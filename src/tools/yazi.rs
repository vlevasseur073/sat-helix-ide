use crate::config::YaziConfig;
use crate::error::HxIdeError;
use std::process::Command;

/// Yazi file explorer client
pub struct YaziClient {
    config: YaziConfig,
}

impl YaziClient {
    /// Create a new Yazi client
    pub fn new(config: &YaziConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Check if Yazi is available
    pub fn is_available(&self) -> bool {
        which::which(&self.config.path).is_ok()
    }

    /// Launch Yazi
    pub fn launch(&self) -> Result<(), HxIdeError> {
        Command::new(&self.config.path).spawn().map_err(|e| {
            HxIdeError::ToolExecutionFailed(format!("Failed to launch Yazi: {}", e))
        })?;
        Ok(())
    }

    /// Launch Yazi with specific directory
    pub fn launch_in_dir(&self, dir: &str) -> Result<(), HxIdeError> {
        Command::new(&self.config.path)
            .arg(dir)
            .spawn()
            .map_err(|e| {
                HxIdeError::ToolExecutionFailed(format!("Failed to launch Yazi: {}", e))
            })?;
        Ok(())
    }
}
