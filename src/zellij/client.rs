use crate::error::HxIdeError;
use std::path::Path;
use std::process::Command;

/// Zellij terminal multiplexer client
pub struct ZellijClient {
    path: String,
}

impl ZellijClient {
    /// Create a new Zellij client
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    /// Check if Zellij is available
    pub fn is_available(&self) -> bool {
        which::which(&self.path).is_ok()
    }

    /// Start a new Zellij session with a layout
    pub fn new_session(
        &self,
        layout_path: &Path,
        config_path: Option<&Path>,
        yazi_config_dir: Option<&Path>,
    ) -> Result<(), HxIdeError> {
        self.run_with_layout(
            "--new-session-with-layout",
            layout_path,
            config_path,
            yazi_config_dir,
        )
    }

    /// Open the layout in the current session, or start one when outside Zellij.
    pub fn open_layout(
        &self,
        layout_path: &Path,
        config_path: Option<&Path>,
        yazi_config_dir: Option<&Path>,
    ) -> Result<(), HxIdeError> {
        self.run_with_layout("--layout", layout_path, config_path, yazi_config_dir)
    }

    fn run_with_layout(
        &self,
        layout_flag: &str,
        layout_path: &Path,
        config_path: Option<&Path>,
        yazi_config_dir: Option<&Path>,
    ) -> Result<(), HxIdeError> {
        let mut command = Command::new(&self.path);
        if let Some(config_path) = config_path {
            command.arg("--config").arg(config_path);
        }
        if let Some(yazi_config_dir) = yazi_config_dir {
            command.env("YAZI_CONFIG_HOME", yazi_config_dir);
        }
        let status = command
            .arg(layout_flag)
            .arg(layout_path)
            .status()
            .map_err(|e| HxIdeError::ZellijError(format!("Failed to start Zellij: {}", e)))?;

        if status.success() {
            Ok(())
        } else {
            Err(HxIdeError::ZellijError(format!(
                "Zellij exited with status {}",
                status
            )))
        }
    }
}
