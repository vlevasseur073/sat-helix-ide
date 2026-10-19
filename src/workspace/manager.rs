use crate::backend::{self, SessionInit};
use crate::config::{expand_tilde, BackendKind, CommandConfig, Config, MindMapConfig};
use crate::resolve::resolve_executable;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

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

    pub fn init_workspace(
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

        let _file_manager_path = resolve_executable(&self.config.tools.file_manager.command)
            .with_context(|| {
                format!(
                    "Cannot find file manager '{}'",
                    self.config.tools.file_manager.command
                )
            })?;

        let ai_enabled = ai_override.unwrap_or(self.config.session.ai_by_default);
        let ai = if ai_enabled { self.resolve_ai() } else { None };
        let mindmap = self.resolve_mindmap();

        let backend = backend::from_config(self.config)?;
        let executable = std::env::current_exe().context("Cannot locate sat-hx-ide executable")?;
        let editor_path = resolve_command(&self.config.tools.editor)?;
        let git_path = resolve_command(&self.config.tools.git)?;
        let user_config = match backend::resolve_kind(self.config) {
            BackendKind::Zellij => self.config.zellij_config_path(),
            BackendKind::Kitty => self.config.kitty_config_path(),
            BackendKind::Auto => unreachable!("resolve_kind never returns Auto"),
        };
        let init = SessionInit {
            session_name: &session_name,
            project_dir: &project_dir,
            runtime_dir: &self.config.runtime_dir(&session_name),
            user_config: user_config.as_deref(),
            attach_existing: self.config.session.attach_existing,
            native_status_bar: status_bar.unwrap_or(self.config.session.status_bar),
            editor: &self.config.tools.editor,
            editor_path: &editor_path,
            ai: ai
                .as_ref()
                .map(|(command, path)| (*command, path.as_path())),
            terminal: self
                .config
                .terminal
                .enabled
                .then_some(&self.config.terminal),
            mindmap: mindmap
                .as_ref()
                .map(|(config, command, path)| (*config, *command, path.as_path())),
            keybindings: &self.config.keybindings,
            executable: &executable,
            app_config: &self.app_config_path,
            git_path: &git_path,
            git_args: &self.config.tools.git.args,
            float_width: &self.config.tools.file_manager.float_width,
            float_height: &self.config.tools.file_manager.float_height,
        };
        backend
            .init_session(&init)
            .context("Failed to initialize workspace")
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

    /// Include the mind-map dock only when enabled and the tool binary resolves.
    fn resolve_mindmap(&self) -> Option<(&'a MindMapConfig, &'a CommandConfig, PathBuf)> {
        if !self.config.mindmap.enabled {
            return None;
        }
        let command = &self.config.tools.mindmap;
        match resolve_executable(&command.command) {
            Ok(path) => Some((&self.config.mindmap, command, path)),
            Err(_) => {
                eprintln!(
                    "sat-hx-ide: skipping the mind-map pane, cannot find '{}'",
                    command.command
                );
                None
            }
        }
    }

    pub fn check_tools(&self) -> Result<()> {
        println!("Checking configured commands...");
        let mut failures = Vec::new();

        match backend::resolve_kind(self.config) {
            BackendKind::Zellij => {
                check_required(&mut failures, "Zellij", &self.config.tools.zellij);
            }
            BackendKind::Kitty => {
                check_required(&mut failures, "Kitty", &self.config.tools.kitty);
                // Remote control needs the kitten CLI next to kitty.
                let kitten = resolve_executable(&self.config.tools.kitty.command)
                    .ok()
                    .and_then(|kitty| backend::resolve_kitten(&kitty));
                match kitten {
                    Some(path) => println!("  ✓ Kitten: {}", path.display()),
                    None => {
                        println!("  ✗ Kitten: not found next to kitty or on PATH");
                        failures.push("Kitten".to_string());
                    }
                }
            }
            BackendKind::Auto => unreachable!("resolve_kind never returns Auto"),
        }
        check_required(&mut failures, "Editor", &self.config.tools.editor);
        check_required_file_manager(&mut failures, &self.config.tools.file_manager.command);
        check_required(&mut failures, "Git client", &self.config.tools.git);
        check_required(&mut failures, "Review", &self.config.tools.review);
        check_required(&mut failures, "Workflow", &self.config.tools.workflow);
        if self.config.mindmap.enabled {
            check_required(&mut failures, "Mind map", &self.config.tools.mindmap);
        } else {
            match resolve_executable(&self.config.tools.mindmap.command) {
                Ok(path) => println!("  ✓ Mind map: {}", path.display()),
                Err(_) => println!(
                    "  - Mind map: '{}' not found (optional while mindmap.enabled is false)",
                    self.config.tools.mindmap.command
                ),
            }
        }
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

        if failures.is_empty() {
            println!("\nAll required commands are available.");
            Ok(())
        } else {
            println!(
                "\nSummary: {} required command(s) missing or misconfigured:",
                failures.len()
            );
            for label in &failures {
                println!("  • {label}");
            }
            bail!(
                "doctor found {} missing or misconfigured required command(s)",
                failures.len()
            );
        }
    }
}

fn check_required(failures: &mut Vec<String>, label: &str, command: &CommandConfig) {
    match resolve_command(command) {
        Ok(path) => println!("  ✓ {label}: {}", path.display()),
        Err(err) => {
            println!("  ✗ {label}: {err:#}");
            failures.push(label.to_string());
        }
    }
}

fn check_required_file_manager(failures: &mut Vec<String>, command: &str) {
    match resolve_executable(command) {
        Ok(path) => println!("  ✓ File manager: {}", path.display()),
        Err(err) => {
            let err = err.context(format!("Cannot find file manager '{command}'"));
            println!("  ✗ File manager: {err:#}");
            failures.push("File manager".to_string());
        }
    }
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
