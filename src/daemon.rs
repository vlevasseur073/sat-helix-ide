use crate::app::App;
use crate::config::Config;
use crate::ipc::{self, serve};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Daemon state
pub struct Daemon {
    app: Arc<App>,
    shutdown_sender: Option<mpsc::Sender<()>>,
    server_handle: Option<tokio::task::JoinHandle<Result<()>>>,
}

impl Daemon {
    pub fn new(config: &Config) -> Result<Self> {
        let app = Arc::new(App::new(config));
        Ok(Self {
            app,
            shutdown_sender: None,
            server_handle: None,
        })
    }

    pub async fn run(&mut self, socket: Option<PathBuf>) -> Result<()> {
        let socket = socket.unwrap_or_else(ipc::current_socket_path);

        // Start server with shutdown capability
        let (shutdown_sender, server_handle) = serve(&socket, Arc::clone(&self.app)).await?;

        // Store for later shutdown
        self.shutdown_sender = Some(shutdown_sender);
        self.server_handle = Some(server_handle);

        // Wait for server to complete (should only happen on shutdown)
        if let Some(handle) = self.server_handle.take() {
            handle.await??;
        }

        Ok(())
    }

    /// Signal the daemon to shutdown gracefully
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(sender) = self.shutdown_sender.take() {
            // Ignore error if channel is closed
            let _ = sender.send(()).await;
        }

        // Wait for server to finish
        if let Some(handle) = self.server_handle.take() {
            handle.await??;
        }

        Ok(())
    }
}

/// Path to the daemon PID file for a given socket
fn daemon_pid_path(socket: &Path) -> PathBuf {
    ipc::pid_path(socket)
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

/// Write the current process ID to the PID file for a given socket
fn write_pid_file(socket: &Path) -> Result<()> {
    let pid = process::id();
    let pid_path = daemon_pid_path(socket);

    // Create directory if it doesn't exist
    if let Some(parent) = pid_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(pid_path, pid.to_string())?;
    Ok(())
}

/// Remove the PID file for a given socket
fn remove_pid_file(socket: &Path) {
    let _ = fs::remove_file(daemon_pid_path(socket));
}

/// Read the daemon PID from the PID file for a given socket
fn read_daemon_pid(socket: &Path) -> Option<u32> {
    let pid_path = daemon_pid_path(socket);
    fs::read_to_string(pid_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Kill the daemon process using the PID file for a given socket
pub fn kill_daemon(socket: &Path) -> Result<()> {
    if let Some(pid) = read_daemon_pid(socket) {
        log::info!("Killing daemon with PID: {}", pid);
        // Send SIGTERM to allow graceful shutdown
        #[cfg(unix)]
        {
            use libc::{kill, SIGTERM};
            unsafe {
                kill(pid as libc::pid_t, SIGTERM);
            }
        }

        // Wait a bit for graceful shutdown
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Force kill only if the same PID is still running
        #[cfg(unix)]
        if process_alive(pid) {
            log::warn!("Daemon did not exit gracefully, sending SIGKILL");
            use libc::{kill, SIGKILL};
            unsafe {
                kill(pid as libc::pid_t, SIGKILL);
            }
        }

        // Clean up PID file
        remove_pid_file(socket);
    }
    Ok(())
}

/// Run the daemon
pub async fn run_daemon(socket: Option<PathBuf>) -> Result<()> {
    // Use current socket path if none provided
    let socket = socket.unwrap_or_else(ipc::current_socket_path);

    // Write PID file
    if let Err(e) = write_pid_file(&socket) {
        log::warn!("Failed to write PID file: {}", e);
    }

    // Clone socket for panic hook
    let socket_for_panic = socket.clone();

    // Set up cleanup on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        remove_pid_file(&socket_for_panic);
        original_hook(panic_info);
    }));

    let config = Config::load(
        std::env::var("SAT_HX_IDE_CONFIG")
            .unwrap_or_else(|_| "~/.config/sat-helix-ide/config.toml".into()),
    )?;

    let mut daemon = Daemon::new(&config)?;

    // Run the daemon
    let result = daemon.run(Some(socket.clone())).await;

    // Clean up PID file on exit
    remove_pid_file(&socket);

    result
}
