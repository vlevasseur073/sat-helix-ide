use crate::protocol::{Request, Response};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

/// Generate socket path for a given project directory
///
/// Socket naming convention: /run/user/<uid>/sat-hx-ide-<project_hash>.sock
/// Uses project directory hash for stability across session restarts.
pub fn socket_path(project_dir: &Path) -> PathBuf {
    // Hash the project directory path to avoid filesystem issues with special characters
    let hash = project_dir
        .to_string_lossy()
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

    // Use XDG_RUNTIME_DIR if available, otherwise /tmp
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);

    runtime_dir.join(format!("sat-hx-ide-{hash}.sock"))
}

/// Generate socket path for the current session based on project directory
/// Falls back to session name if project directory cannot be determined
pub fn current_socket_path() -> PathBuf {
    // Try to use project directory first (more stable)
    if let Ok(project_dir) = std::env::current_dir() {
        return socket_path(&project_dir);
    }

    // Fallback to session name
    let session_name =
        std::env::var("ZELLIJ_SESSION_NAME").unwrap_or_else(|_| "default".to_string());

    // For backward compatibility, hash the session name
    let hash = session_name
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);

    runtime_dir.join(format!("sat-hx-ide-{hash}.sock"))
}

/// Generate socket path for a given session name (legacy)
///
/// Socket naming convention: /run/user/<uid>/sat-hx-ide-<session_hash>.sock
/// This follows helix-ide's pattern but with sat-hx-ide naming.
pub fn session_socket_path(session_name: &str) -> PathBuf {
    // Hash the session name to avoid filesystem issues with special characters
    let hash = session_name
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

    // Use XDG_RUNTIME_DIR if available, otherwise /tmp
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);

    runtime_dir.join(format!("sat-hx-ide-{hash}.sock"))
}

/// Start the IPC server on the given socket
pub async fn serve(socket: &PathBuf, app: std::sync::Arc<crate::app::App>) -> Result<()> {
    // Remove stale socket if it exists
    if socket.exists() {
        std::fs::remove_file(socket)
            .with_context(|| format!("failed to remove stale socket {}", socket.display()))?;
    }

    // Create listener
    let listener = UnixListener::bind(socket)
        .with_context(|| format!("failed to bind socket {}", socket.display()))?;

    log::info!("IPC daemon listening on {}", socket.display());

    // Accept loop
    loop {
        let (stream, _) = listener
            .accept()
            .await
            .with_context(|| "failed to accept IPC connection")?;

        let app = std::sync::Arc::clone(&app);

        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, app).await {
                log::error!("IPC error: {error:#}");
            }
        });
    }
}

/// Handle a single IPC connection
async fn handle_connection(stream: UnixStream, app: std::sync::Arc<crate::app::App>) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read request (line-delimited JSON)
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .with_context(|| "failed to read IPC request")?;

    // Parse request
    let request: Request = serde_json::from_str(&line)
        .with_context(|| format!("failed to parse IPC request: {line}"))?;

    // Handle request
    let response = app.handle_ipc(request).await;

    // Send response
    let mut serialized = serde_json::to_vec(&response)?;
    serialized.push(b'\n');
    writer
        .write_all(&serialized)
        .await
        .with_context(|| "failed to write IPC response")?;

    Ok(())
}

/// Send a request to the IPC daemon
pub async fn send_request(socket: &PathBuf, request: &Request) -> Result<Response> {
    let stream = UnixStream::connect(socket)
        .await
        .with_context(|| format!("cannot connect to daemon at {}", socket.display()))?;

    let (reader, mut writer) = stream.into_split();

    // Send request
    let mut message = serde_json::to_vec(request)?;
    message.push(b'\n');
    writer
        .write_all(&message)
        .await
        .with_context(|| "failed to write IPC request")?;

    // Read response
    let mut reader = BufReader::new(reader);
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .await
        .with_context(|| "failed to read IPC response")?;

    // Parse response
    serde_json::from_str(&response)
        .with_context(|| format!("failed to parse IPC response: {response}"))
}

/// Check if the daemon is alive and responsive
pub async fn is_daemon_alive(socket: &PathBuf) -> bool {
    // Try to connect and send a ping to ensure daemon is responsive
    let stream = match UnixStream::connect(socket).await {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Try to send a ping and get a pong
    let ping_request = Request::Ping;
    let (r, w) = stream.into_split();
    let (reader, mut writer) = (r, w);

    let message = match serde_json::to_vec(&ping_request) {
        Ok(mut m) => {
            m.push(b'\n');
            m
        }
        Err(_) => return false,
    };

    if writer.write_all(&message).await.is_err() {
        return false;
    }

    let mut reader = BufReader::new(reader);
    let mut response = String::new();
    if reader.read_line(&mut response).await.is_err() {
        return false;
    }

    // Check if we got a pong response
    matches!(serde_json::from_str(&response), Ok(Response::Pong))
}

/// Spawn the daemon process
pub fn spawn_daemon(executable: &std::path::Path, socket: &PathBuf) -> Result<()> {
    let mut command = std::process::Command::new(executable);
    command.arg("daemon").arg("--socket").arg(socket);

    command.spawn().with_context(|| "failed to spawn daemon")?;

    Ok(())
}
