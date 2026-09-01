use crate::config::GitConfig;
use crate::error::HxIdeError;
use std::process::Command;

/// Git client wrapper
pub struct GitClient {
    config: GitConfig,
}

impl GitClient {
    /// Create a new Git client
    pub fn new(config: &GitConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Check if the configured git client is available
    pub fn is_available(&self) -> bool {
        which::which(&self.config.client).is_ok()
    }

    /// Check if git-delta is available
    pub fn delta_available(&self) -> bool {
        which::which("delta").is_ok()
    }

    /// Launch the configured git client
    pub fn launch(&self) -> Result<(), HxIdeError> {
        Command::new(&self.config.client).spawn().map_err(|e| {
            HxIdeError::ToolExecutionFailed(format!(
                "Failed to launch {}: {}",
                self.config.client, e
            ))
        })?;
        Ok(())
    }
}
