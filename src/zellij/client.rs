use crate::error::HxIdeError;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    NotFound,
    Active,
    Exited,
}

pub struct ZellijClient {
    path: PathBuf,
}

impl ZellijClient {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn session_status(&self, name: &str) -> Result<SessionStatus, HxIdeError> {
        let output = Command::new(&self.path)
            .args(["list-sessions", "--no-formatting"])
            .output()
            .map_err(|error| {
                HxIdeError::ZellijError(format!("Failed to list sessions: {error}"))
            })?;

        // Zellij exits non-zero when no sessions exist.
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some((session_name, status)) = parse_session_line(line) {
                if session_name == name {
                    return Ok(status);
                }
            }
        }
        Ok(SessionStatus::NotFound)
    }

    pub fn delete_session(&self, name: &str) -> Result<(), HxIdeError> {
        self.run(Command::new(&self.path).arg("delete-session").arg(name))
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
        // sat-hx-ide is often launched from inside an existing Zellij pane while
        // developing. Inherited session env would otherwise pin the child process to
        // that session name instead of the project-derived one we pass on the CLI.
        command
            .env_remove("ZELLIJ")
            .env_remove("ZELLIJ_SESSION_NAME")
            .env_remove("ZELLIJ_CONFIG_FILE")
            .env_remove("ZELLIJ_CONFIG_DIR")
            .env_remove("ZELLIJ_AUTO_ATTACH")
            .env_remove("ZELLIJ_AUTO_EXIT");

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

fn parse_session_line(line: &str) -> Option<(&str, SessionStatus)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with("No active zellij sessions") {
        return None;
    }

    let boundary = line.find([' ', '['])?;
    let session_name = &line[..boundary];
    let status = if line.contains("(EXITED") {
        SessionStatus::Exited
    } else {
        SessionStatus::Active
    };
    Some((session_name, status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_active_and_exited_session_lines() {
        assert_eq!(
            parse_session_line("sat-helix-ide [Created 1s ago]"),
            Some(("sat-helix-ide", SessionStatus::Active))
        );
        assert_eq!(
            parse_session_line("sat-helix-ide [Created 9m ago] (EXITED - attach to resurrect)"),
            Some(("sat-helix-ide", SessionStatus::Exited))
        );
        assert!(parse_session_line("No active zellij sessions found.").is_none());
    }
}
