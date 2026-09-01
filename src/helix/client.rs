use crate::error::HxIdeError;
use std::path::PathBuf;
use std::process::Command;

/// Helix editor client
pub struct HelixClient {
    path: String,
}

impl HelixClient {
    /// Create a new Helix client
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    /// Check if Helix is available
    pub fn is_available(&self) -> bool {
        which::which(&self.path).is_ok()
    }

    /// Launch Helix
    pub fn launch(&self) -> Result<(), HxIdeError> {
        Command::new(&self.path).spawn().map_err(|e| {
            HxIdeError::ToolExecutionFailed(format!("Failed to launch Helix: {}", e))
        })?;
        Ok(())
    }

    /// Launch Helix with a specific file
    pub fn launch_with_file(&self, file: &PathBuf) -> Result<(), HxIdeError> {
        Command::new(&self.path).arg(file).spawn().map_err(|e| {
            HxIdeError::ToolExecutionFailed(format!("Failed to launch Helix: {}", e))
        })?;
        Ok(())
    }

    /// Launch Helix in a specific directory
    pub fn launch_in_dir(&self, dir: &PathBuf) -> Result<(), HxIdeError> {
        Command::new(&self.path)
            .current_dir(dir)
            .spawn()
            .map_err(|e| {
                HxIdeError::ToolExecutionFailed(format!("Failed to launch Helix: {}", e))
            })?;
        Ok(())
    }

    /// Get Helix version
    pub fn version(&self) -> Result<String, HxIdeError> {
        let output = Command::new(&self.path)
            .arg("--version")
            .output()
            .map_err(|e| {
                HxIdeError::ToolExecutionFailed(format!("Failed to get Helix version: {}", e))
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(HxIdeError::ToolExecutionFailed(format!(
                "Failed to get Helix version: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }
}
