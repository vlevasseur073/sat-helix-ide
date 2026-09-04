#![cfg(unix)]

use assert_cmd::cargo::CommandCargoExt;
use sat_helix_ide::daemon;
use sat_helix_ide::ipc::{self, pid_path, send_request, socket_path};
use sat_helix_ide::protocol::{Request, Response};
use std::fs;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

#[test]
fn pid_files_are_unique_per_project_socket() {
    let sock_a = socket_path(Path::new("/home/user/project-a"));
    let sock_b = socket_path(Path::new("/home/user/project-b"));
    assert_ne!(pid_path(&sock_a), pid_path(&sock_b));
}

#[test]
fn protocol_roundtrip_serialization() {
    let request = serde_json::to_string(&Request::TerminalToggle).unwrap();
    assert!(request.contains("\"command\":\"terminal_toggle\""));

    let with_pane = Request::FileManagerOpen {
        source_pane_id: Some(42),
    };
    let encoded = serde_json::to_string(&with_pane).unwrap();
    assert!(encoded.contains("\"command\":\"file_manager_open\""));
    assert!(encoded.contains("\"source_pane_id\":42"));
    let decoded: Request = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, with_pane);

    let legacy = serde_json::from_str::<Request>(r#"{"command":"file_manager_open"}"#).unwrap();
    assert_eq!(
        legacy,
        Request::FileManagerOpen {
            source_pane_id: None
        }
    );

    let response = serde_json::to_string(&Response::Pong).unwrap();
    assert!(response.contains("\"status\":\"pong\""));
}

#[tokio::test]
async fn daemon_responds_to_ping_and_shutdown() {
    let runtime = tempfile::tempdir().unwrap();
    let config = runtime.path().join("config.toml");
    fs::write(&config, "[session]\n").unwrap();

    let project = runtime.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let socket = socket_path(&project);

    let mut child = std::process::Command::cargo_bin("sat-hx-ide")
        .unwrap()
        .arg("daemon")
        .arg("--socket")
        .arg(&socket)
        .env("XDG_RUNTIME_DIR", runtime.path())
        .env("SAT_HX_IDE_CONFIG", &config)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut ready = false;
    for _ in 0..50 {
        if ipc::is_daemon_alive(&socket).await {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(ready, "daemon did not become ready");

    let ping = send_request(&socket, &Request::Ping).await.unwrap();
    assert!(matches!(ping, Response::Pong));

    send_request(&socket, &Request::Shutdown).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!ipc::is_daemon_alive(&socket).await);

    let _ = child.kill();
    let _ = child.wait();
}

#[tokio::test]
async fn concurrent_projects_use_separate_pid_files() {
    let runtime = tempfile::tempdir().unwrap();
    let config = runtime.path().join("config.toml");
    fs::write(&config, "[session]\n").unwrap();

    let project_a = runtime.path().join("project-a");
    let project_b = runtime.path().join("project-b");
    fs::create_dir_all(&project_a).unwrap();
    fs::create_dir_all(&project_b).unwrap();

    let socket_a = socket_path(&project_a);
    let socket_b = socket_path(&project_b);
    let pid_a = pid_path(&socket_a);
    let pid_b = pid_path(&socket_b);

    let spawn = |socket: &Path| {
        std::process::Command::cargo_bin("sat-hx-ide")
            .unwrap()
            .arg("daemon")
            .arg("--socket")
            .arg(socket)
            .env("XDG_RUNTIME_DIR", runtime.path())
            .env("SAT_HX_IDE_CONFIG", &config)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    };

    let mut child_a = spawn(&socket_a);
    let mut child_b = spawn(&socket_b);

    for _ in 0..50 {
        if ipc::is_daemon_alive(&socket_a).await && ipc::is_daemon_alive(&socket_b).await {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    assert!(pid_a.exists());
    assert!(pid_b.exists());
    assert_ne!(pid_a, pid_b);
    assert_ne!(
        fs::read_to_string(&pid_a).unwrap(),
        fs::read_to_string(&pid_b).unwrap()
    );

    daemon::kill_daemon(&socket_a).unwrap();
    daemon::kill_daemon(&socket_b).unwrap();

    let _ = child_a.kill();
    let _ = child_b.kill();
    let _ = child_a.wait();
    let _ = child_b.wait();
}
