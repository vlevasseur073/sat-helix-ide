use thiserror::Error;

/// Error types for Helix IDE
#[derive(Error, Debug)]
pub enum HxIdeError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Tool not found
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Tool execution failed
    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    /// Layout not found
    #[error("Layout not found: {0}")]
    LayoutNotFound(String),

    /// Invalid layout definition
    #[error("Invalid layout definition: {0}")]
    InvalidLayout(String),

    /// Workspace error
    #[error("Workspace error: {0}")]
    WorkspaceError(String),

    /// Zellij error
    #[error("Zellij error: {0}")]
    ZellijError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// TOML parsing error
    #[error("TOML parsing error: {0}")]
    TomlError(#[from] toml::de::Error),

    /// Serde JSON error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl HxIdeError {
    /// Check if error is due to missing tool
    pub fn is_tool_not_found(&self) -> bool {
        matches!(self, HxIdeError::ToolNotFound(_))
    }
}
