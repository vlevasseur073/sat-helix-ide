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
    pub mindmap: MindMapConfig,

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

    /// Add the status bar in zellij tabs
    #[serde(default)]
    pub status_bar: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            attach_existing: true,
            ai_by_default: true,
            zellij_config: None,
            status_bar: false,
        }
    }
}

/// The shell pane docked under Helix in the code tab. It runs Zellij's default
/// shell, so there is no command to configure here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Height or width of the docked terminal as a percentage of the code tab,
    /// depending on `dock_position`.
    #[serde(default = "default_terminal_percent")]
    pub dock_percent: u8,

    /// Where the terminal sits relative to the editor: `down` (default) or `right`.
    #[serde(default)]
    pub dock_position: DockPosition,
}

/// Terminal placement relative to the editor in the code tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DockPosition {
    #[default]
    Down,
    Right,
}

impl DockPosition {
    /// KDL `split_direction` for the editor/terminal split.
    pub fn zellij_split_direction(self) -> &'static str {
        match self {
            Self::Down => "horizontal",
            Self::Right => "vertical",
        }
    }

    /// `zellij action new-pane --direction` when respawning a closed terminal.
    pub fn zellij_new_pane_direction(self) -> &'static str {
        match self {
            Self::Down => "down",
            Self::Right => "right",
        }
    }
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dock_percent: default_terminal_percent(),
            dock_position: DockPosition::Down,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindMapConfig {
    /// When true, include the mind-map pane in the code tab at session start.
    /// When false (default), the pane is omitted until opened with the toggle
    /// keybinding. Unlike `terminal.enabled`, this does not block Alt-m.
    #[serde(default = "default_false")]
    pub enabled: bool,

    /// Height or width of the docked mind map as a percentage of the code tab,
    /// depending on `dock_position`.
    #[serde(default = "default_mindmap_percent")]
    pub dock_percent: u8,

    /// Where the mind map sits relative to the editor: `down` or `right` (default).
    #[serde(default = "default_mindmap_dock_position")]
    pub dock_position: DockPosition,
}

impl Default for MindMapConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            dock_percent: default_mindmap_percent(),
            dock_position: DockPosition::Right,
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

    #[serde(default = "default_review_key")]
    pub review: String,

    #[serde(default = "default_workflow_key")]
    pub workflow: String,

    #[serde(default = "default_terminal_key")]
    pub terminal: String,

    #[serde(default = "default_terminal_zoom_key")]
    pub terminal_zoom: String,

    #[serde(default = "default_mindmap_key")]
    pub mindmap: String,

    #[serde(default = "default_mindmap_zoom_key")]
    pub mindmap_zoom: String,
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            file_manager: default_file_manager_key(),
            file_manager_dock: default_file_manager_dock_key(),
            git: default_git_key(),
            review: default_review_key(),
            workflow: default_workflow_key(),
            terminal: default_terminal_key(),
            terminal_zoom: default_terminal_zoom_key(),
            mindmap: default_mindmap_key(),
            mindmap_zoom: default_mindmap_zoom_key(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
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

fn default_review_key() -> String {
    "Alt r".to_string()
}

fn default_workflow_key() -> String {
    "Alt w".to_string()
}

fn default_terminal_key() -> String {
    "Alt t".to_string()
}

fn default_mindmap_key() -> String {
    "Alt m".to_string()
}

fn default_terminal_zoom_key() -> String {
    "Alt Shift t".to_string()
}

fn default_mindmap_zoom_key() -> String {
    "Alt Shift m".to_string()
}

fn default_terminal_percent() -> u8 {
    15
}

fn default_mindmap_percent() -> u8 {
    50
}

fn default_mindmap_dock_position() -> DockPosition {
    DockPosition::Right
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

            [tools.workflow]
            command = "my-glab-tui"
            "#,
        )
        .unwrap();

        assert_eq!(config.tools.editor.command, "/snap/bin/hx");
        assert_eq!(config.tools.file_manager.command, "yazi");
        assert_eq!(config.tools.git.command, "gitui");
        assert_eq!(config.tools.workflow.command, "my-glab-tui");
    }

    #[test]
    fn terminal_dock_position_deserializes() {
        let config: Config = toml::from_str(
            r#"
            [terminal]
            dock_position = "right"
            "#,
        )
        .unwrap();
        assert_eq!(config.terminal.dock_position, DockPosition::Right);

        let default_config = Config::default();
        assert_eq!(default_config.terminal.dock_position, DockPosition::Down);
    }

    #[test]
    fn mindmap_defaults_to_disabled_on_the_right() {
        let default_config = Config::default();
        assert!(!default_config.mindmap.enabled);
        assert_eq!(default_config.mindmap.dock_position, DockPosition::Right);
        assert_eq!(default_config.tools.mindmap.command, "shiki");
    }

    #[test]
    fn default_tool_commands_are_correct() {
        let default_config = Config::default();
        assert_eq!(default_config.tools.editor.command, "hx");
        assert_eq!(default_config.tools.file_manager.command, "yazi");
        assert_eq!(default_config.tools.git.command, "lazygit");
        assert_eq!(default_config.tools.review.command, "revdiff");
        assert_eq!(default_config.tools.workflow.command, "glab-tui");
    }
}
