use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Requests that can be sent to the daemon
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Request {
    // File manager actions
    FileManagerOpen {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source_pane_id: Option<u64>,
    },
    FileManagerToggleDock {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source_pane_id: Option<u64>,
    },
    FileManagerRun {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source_pane_id: Option<u64>,
    },

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
