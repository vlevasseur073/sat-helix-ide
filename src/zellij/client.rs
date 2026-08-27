use crate::error::HxIdeError;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct ZellijClient {
    path: PathBuf,
}

impl ZellijClient {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn session_exists(&self, name: &str) -> Result<bool, HxIdeError> {
        let output = Command::new(&self.path)
            .args(["list-sessions", "--short", "--no-formatting"])
            .output()
            .map_err(|error| {
                HxIdeError::ZellijError(format!("Failed to list sessions: {error}"))
            })?;

        // Zellij exits non-zero when no sessions exist.
        let sessions = String::from_utf8_lossy(&output.stdout);
        Ok(sessions.lines().any(|session| session.trim() == name))
    }

    pub fn attach(&self, name: &str, config: &Path) -> Result<(), HxIdeError> {
        self.run(
            Command::new(&self.path)
                .arg("--config")
                .arg(config)
                .arg("attach")
                .arg(name),
        )
    }

    pub fn create(
        &self,
        name: &str,
        layout: &Path,
        config: &Path,
        project_dir: &Path,
    ) -> Result<(), HxIdeError> {
        self.run(
            Command::new(&self.path)
                .current_dir(project_dir)
                .arg("--config")
                .arg(config)
                .arg("--session")
                .arg(name)
                .arg("--new-session-with-layout")
                .arg(layout),
        )
    }

    fn run(&self, command: &mut Command) -> Result<(), HxIdeError> {
        let status = command
            .status()
            .map_err(|error| HxIdeError::ZellijError(format!("Failed to start Zellij: {error}")))?;
        if status.success() {
            Ok(())
        } else {
            Err(HxIdeError::ZellijError(format!(
                "Zellij exited with status {status}"
            )))
        }
    }
}
