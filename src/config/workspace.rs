use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Default workspace directory
    #[serde(default = "default_workspace_dir")]
    pub default_dir: PathBuf,

    /// Auto-detect project type
    #[serde(default = "default_auto_detect")]
    pub auto_detect: bool,

    /// Open in new Zellij session by default
    #[serde(default)]
    pub new_session_default: bool,

    /// Remember last workspace
    #[serde(default)]
    pub remember_last: bool,

    /// Last used workspace path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_workspace: Option<PathBuf>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            default_dir: default_workspace_dir(),
            auto_detect: default_auto_detect(),
            new_session_default: false,
            remember_last: true,
            last_workspace: None,
        }
    }
}

fn default_workspace_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn default_auto_detect() -> bool {
    true
}
