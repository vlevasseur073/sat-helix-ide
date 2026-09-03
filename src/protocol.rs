use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Requests that can be sent to the daemon
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Request {
    // File manager actions
    FileManagerOpen,
    FileManagerToggleDock,
    FileManagerRun,

    // Terminal actions
    TerminalToggle,
    TerminalZoom,

    // Git actions
    GitOpen,

    // State queries
    GetPaneList,
    GetContext,

    // Lifecycle
    Ping,
    Shutdown,
}

/// Responses from the daemon
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response {
    Ok,

    // Data responses
    PaneList(Vec<crate::actions::PaneInfo>),
    Context {
        cwd: PathBuf,
        project_root: PathBuf,
        git_root: Option<PathBuf>,
    },

    // Error response
    Error {
        message: String,
    },

    // Lifecycle
    Pong,
}
