mod tools;

pub use tools::{CommandConfig, FileManagerConfig, ToolConfig};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub session: SessionConfig,

    #[serde(default)]
    pub tools: ToolConfig,

    #[serde(default)]
    pub terminal: TerminalConfig,

    #[serde(default)]
    pub keybindings: KeybindingConfig,
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str(include_str!("../../configs/config.toml"))
            .expect("bundled configuration must be valid TOML")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Attach to an existing project-named session when one exists.
    #[serde(default = "default_true")]
    pub attach_existing: bool,

    /// Add the AI tab when the configured agent is available. Skipped
    /// silently when it is not, so this can stay on.
    #[serde(default = "default_true")]
    pub ai_by_default: bool,

    /// Optional explicit path to the Zellij config used as merge input.
    #[serde(default)]
    pub zellij_config: Option<PathBuf>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            attach_existing: true,
            ai_by_default: true,
            zellij_config: None,
        }
    }
}

/// The shell pane docked under Helix in the code tab. It runs Zellij's default
/// shell, so there is no command to configure here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Height of the docked terminal as a percentage of the code tab.
    #[serde(default = "default_terminal_percent")]
    pub dock_percent: u8,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dock_percent: default_terminal_percent(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingConfig {
    #[serde(default = "default_file_manager_key")]
    pub file_manager: String,

    #[serde(default = "default_file_manager_dock_key")]
    pub file_manager_dock: String,

    #[serde(default = "default_git_key")]
    pub git: String,

    #[serde(default = "default_terminal_key")]
    pub terminal: String,

    #[serde(default = "default_terminal_zoom_key")]
    pub terminal_zoom: String,
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            file_manager: default_file_manager_key(),
            file_manager_dock: default_file_manager_dock_key(),
            git: default_git_key(),
            terminal: default_terminal_key(),
            terminal_zoom: default_terminal_zoom_key(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_file_manager_key() -> String {
    "Ctrl y".to_string()
}

fn default_file_manager_dock_key() -> String {
    "Alt y".to_string()
}

fn default_git_key() -> String {
    "Alt g".to_string()
}

fn default_terminal_key() -> String {
    "Alt t".to_string()
}

fn default_terminal_zoom_key() -> String {
    "Alt Shift t".to_string()
}

fn default_terminal_percent() -> u8 {
    15
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = expand_tilde(path.as_ref());
        if !path.exists() {
            log::debug!(
                "Config {} does not exist; using bundled defaults",
                path.display()
            );
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse sat-hx-ide config {}", path.display()))
    }

    /// Locate the user's Zellij config. This path is read-only.
    pub fn zellij_config_path(&self) -> Option<PathBuf> {
        self.session
            .zellij_config
            .as_deref()
            .map(expand_tilde)
            .or_else(|| std::env::var_os("ZELLIJ_CONFIG_FILE").map(PathBuf::from))
            .or_else(|| {
                let base = std::env::var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .or_else(|| home::home_dir().map(|home| home.join(".config")))?;
                Some(base.join("zellij").join("config.kdl"))
            })
            .filter(|path| path.exists())
    }

    pub fn runtime_dir(&self, session_name: &str) -> PathBuf {
        let base = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        base.join("sat-helix-ide").join(session_name)
    }
}

pub fn expand_tilde(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix("~/") {
        home::home_dir()
            .map(|home| home.join(rest))
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_is_not_created() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing.toml");

        let config = Config::load(&path).unwrap();

        assert_eq!(config.keybindings.file_manager, "Ctrl y");
        assert!(!path.exists());
    }

    #[test]
    fn old_tool_names_still_deserialize() {
        let config: Config = toml::from_str(
            r#"
            [tools.helix]
            path = "/snap/bin/hx"
            theme = "ignored"

            [tools.yazi]
            path = "yazi"

            [tools.git]
            client = "gitui"
            "#,
        )
        .unwrap();

        assert_eq!(config.tools.editor.command, "/snap/bin/hx");
        assert_eq!(config.tools.file_manager.command, "yazi");
        assert_eq!(config.tools.git.command, "gitui");
    }
}
