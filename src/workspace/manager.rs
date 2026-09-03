use crate::config::{expand_tilde, CommandConfig, Config};

use crate::ipc;
use crate::resolve::resolve_executable;
use crate::zellij::{
    build_runtime_config, session_layout, RuntimeConfigInput, SessionStatus, ZellijClient,
};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct WorkspaceManager<'a> {
    config: &'a Config,
    app_config_path: PathBuf,
}

impl<'a> WorkspaceManager<'a> {
    pub fn new(config: &'a Config, app_config_path: impl AsRef<Path>) -> Self {
        Self {
            config,
            app_config_path: absolute_path(&expand_tilde(app_config_path.as_ref())),
        }
    }

    pub async fn init_workspace(
        &self,
        path: &Path,
        session_override: Option<&str>,
        ai_override: Option<bool>,
        status_bar: Option<bool>,
    ) -> Result<()> {
        let project_dir = resolve_project_dir(path)?;
        let session_name = session_override
            .map(sanitize_session_name)
            .unwrap_or_else(|| project_session_name(&project_dir));
        if session_name.is_empty() {
            bail!("Session name cannot be empty");
        }

        let zellij_path = resolve_command(&self.config.tools.zellij)?;
        let editor_path = resolve_command(&self.config.tools.editor)?;
        let git_path = resolve_command(&self.config.tools.git)?;
        let _file_manager_path = resolve_executable(&self.config.tools.file_manager.command)
            .with_context(|| {
                format!(
                    "Cannot find file manager '{}'",
                    self.config.tools.file_manager.command
                )
            })?;

        let ai_enabled = ai_override.unwrap_or(self.config.session.ai_by_default);
        let ai = if ai_enabled { self.resolve_ai() } else { None };

        let status_bar = status_bar.unwrap_or(self.config.session.status_bar);

        // Calculate socket path based on project directory for daemon persistence
        let socket_path = ipc::socket_path(&project_dir);

        // Spawn daemon if not already running (like helix-ide's Start command)
        // This ensures daemon persists for the entire session lifetime
        let executable = std::env::current_exe().context("Cannot locate sat-hx-ide executable")?;

        if !ipc::is_daemon_alive(&socket_path).await {
            log::info!("Spawning daemon for session: {}", session_name);
            std::process::Command::new(&executable)
                .arg("daemon")
                .arg("--socket")
                .arg(&socket_path)
                .env("ZELLIJ_SESSION_NAME", &session_name)
                .spawn()
                .context("Failed to spawn daemon")?;

            // Wait for daemon to start listening (with timeout)
            for _ in 0..50 {
                if ipc::is_daemon_alive(&socket_path).await {
                    log::info!("Daemon is now listening on {}", socket_path.display());
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        } else {
            log::debug!("Daemon already running for session: {}", session_name);
        }

        let runtime_dir = self.config.runtime_dir(&session_name);
        fs::create_dir_all(&runtime_dir)?;
        let layout_path = runtime_dir.join("layout.kdl");
        let config_path = runtime_dir.join("config.kdl");
        let layout = session_layout(
            &self.config.tools.editor,
            &editor_path,
            ai.as_ref()
                .map(|(command, path)| (*command, path.as_path())),
            self.config
                .terminal
                .enabled
                .then_some(self.config.terminal.dock_percent),
            &project_dir,
            status_bar,
        );
        fs::write(&layout_path, layout)?;
        build_runtime_config(&RuntimeConfigInput {
            source: self.config.zellij_config_path().as_deref(),
            destination: &config_path,
            session_name: &session_name,
            keys: &self.config.keybindings,
            executable: &executable,
            app_config: &self.app_config_path,
            git_command: &git_path,
            git_args: &self.config.tools.git.args,
            project_dir: &project_dir,
            float_width: &self.config.tools.file_manager.float_width,
            float_height: &self.config.tools.file_manager.float_height,
        })?;

        let zellij = ZellijClient::new(zellij_path);
        match zellij.session_status(&session_name)? {
            SessionStatus::Active => {
                if self.config.session.attach_existing {
                    zellij
                        .attach(&session_name, &config_path)
                        .context("Failed to attach to existing Zellij session")?
                } else {
                    bail!(
                        "Session '{session_name}' already exists; enable session.attach_existing \
                         or choose another --session name"
                    );
                }
            }
            SessionStatus::Exited => {
                // Zellij resurrection replays serialized pane commands, which can
                // drift from the configured layout (for example after Helix spawns
                // an LSP child). Always recreate exited sessions from layout.kdl.
                zellij
                    .delete_session(&session_name)
                    .context("Failed to delete exited Zellij session")?;
                zellij
                    .create(&session_name, &layout_path, &config_path, &project_dir)
                    .context("Failed to create Zellij session")?
            }
            SessionStatus::NotFound => zellij
                .create(&session_name, &layout_path, &config_path, &project_dir)
                .context("Failed to create Zellij session")?,
        }

        // Zellij session has ended (attach/create returned)
        // Clean up the daemon
        log::info!("Zellij session ended, cleaning up daemon");
        if let Err(e) = crate::daemon::kill_daemon(&socket_path) {
            log::warn!("Failed to kill daemon: {}", e);
        }

        Ok(())
    }

    /// The AI tab is a convenience, not a requirement: an agent that is
    /// unconfigured or missing from PATH drops the tab instead of the session.
    fn resolve_ai(&self) -> Option<(&'a CommandConfig, PathBuf)> {
        let command = self.config.tools.ai.as_ref()?;
        match resolve_executable(&command.command) {
            Ok(path) => Some((command, path)),
            Err(_) => {
                eprintln!(
                    "sat-hx-ide: skipping the AI tab, cannot find '{}'",
                    command.command
                );
                None
            }
        }
    }

    pub fn check_tools(&self) -> Result<()> {
        println!("Checking configured commands...");
        check("Zellij", &self.config.tools.zellij)?;
        check("Editor", &self.config.tools.editor)?;
        check_file_manager(&self.config.tools.file_manager.command)?;
        check("Git client", &self.config.tools.git)?;
        match self.config.tools.ai.as_ref() {
            Some(ai) => match resolve_executable(&ai.command) {
                Ok(path) => println!("  ✓ AI: {}", path.display()),
                Err(_) => println!(
                    "  - AI: '{}' not found, the AI tab will be skipped",
                    ai.command
                ),
            },
            None => println!("  - AI: not configured (optional)"),
        }
        println!("\nAll required commands are available.");
        Ok(())
    }
}

fn check(label: &str, command: &CommandConfig) -> Result<()> {
    let path = resolve_command(command)?;
    println!("  ✓ {label}: {}", path.display());
    Ok(())
}

fn check_file_manager(command: &str) -> Result<()> {
    let path = resolve_executable(command)
        .with_context(|| format!("Cannot find file manager '{command}'"))?;
    println!("  ✓ File manager: {}", path.display());
    Ok(())
}

fn resolve_command(command: &CommandConfig) -> Result<PathBuf> {
    resolve_executable(&command.command)
        .with_context(|| format!("Cannot find executable '{}'", command.command))
}

fn resolve_project_dir(path: &Path) -> Result<PathBuf> {
    let project_dir = fs::canonicalize(path)
        .with_context(|| format!("Failed to resolve project path {}", path.display()))?;
    if !project_dir.is_dir() {
        bail!(
            "Project path '{}' is not a directory",
            project_dir.display()
        );
    }
    Ok(project_dir)
}

fn project_session_name(project_dir: &Path) -> String {
    let raw = project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace");
    sanitize_session_name(raw)
}

fn sanitize_session_name(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_safe_session_name() {
        assert_eq!(sanitize_session_name("my project!"), "my-project");
    }

    #[test]
    fn session_name_uses_project_directory_basename() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("my-project");
        fs::create_dir(&project).unwrap();

        let project_dir = resolve_project_dir(&project).unwrap();
        assert_eq!(project_session_name(&project_dir), "my-project");
    }
}
