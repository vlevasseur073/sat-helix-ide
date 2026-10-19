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

    #[serde(default)]
    pub venv: VenvConfig,

    #[serde(default)]
    pub launcher: LauncherConfig,
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str(include_str!("../../configs/config.toml"))
            .expect("bundled configuration must be valid TOML")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Workspace shell that hosts the session: Zellij (default) or kitty.
    #[serde(default)]
    pub backend: BackendKind,

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

    /// Optional explicit path to the kitty config used as merge input.
    #[serde(default)]
    pub kitty_config: Option<PathBuf>,

    /// Show Zellij's built-in input-mode status bar under the sat-hx-ide summary line
    #[serde(default)]
    pub status_bar: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            backend: BackendKind::default(),
            attach_existing: true,
            ai_by_default: true,
            zellij_config: None,
            kitty_config: None,
            status_bar: false,
        }
    }
}

/// How to choose a project directory when the CLI is run without a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LauncherPicker {
    #[default]
    Auto,
    Yazi,
    Fzf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    #[serde(default)]
    pub picker: LauncherPicker,

    /// Directory where Yazi or fzf starts (tilde expanded).
    #[serde(default = "default_launcher_start_dir")]
    pub start_dir: PathBuf,

    /// Executable used when `picker = "fzf"` or as the auto fallback.
    #[serde(default = "default_fzf_command")]
    pub fzf_command: String,

    /// Maximum directory depth when listing folders for fzf.
    #[serde(default = "default_launcher_max_depth")]
    pub max_depth: u32,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            picker: LauncherPicker::default(),
            start_dir: default_launcher_start_dir(),
            fzf_command: default_fzf_command(),
            max_depth: default_launcher_max_depth(),
        }
    }
}

fn default_launcher_start_dir() -> PathBuf {
    PathBuf::from("~")
}

fn default_fzf_command() -> String {
    "fzf".to_string()
}

fn default_launcher_max_depth() -> u32 {
    8
}

/// The workspace shell hosting the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    #[default]
    Zellij,
    Kitty,
    /// Pick kitty when running inside a kitty window, Zellij otherwise.
    Auto,
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

    #[serde(default = "default_venv_key")]
    pub venv: String,

    #[serde(default = "default_venv_select_key")]
    pub venv_select: String,

    /// Zellij keys released to the focused pane (Helix) instead of being
    /// handled by Zellij, so locked mode (`Ctrl g`) is not needed for them.
    /// Empty by default; entries use Zellij key notation, e.g. "Ctrl b".
    #[serde(default)]
    pub passthrough: Vec<String>,
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
            venv: default_venv_key(),
            venv_select: default_venv_select_key(),
            passthrough: Vec::new(),
        }
    }
}

/// Virtual environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenvConfig {
    /// Enable virtual environment management
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Path to a specific virtual environment to use
    /// If set, this environment will be included in the selection list
    /// Can be used alongside auto-detection
    #[serde(default)]
    pub path: Option<String>,

    /// Type of virtual environment for the configured path: "auto", "uv", "venv", "poetry", "conda"
    #[serde(default = "default_venv_type")]
    pub venv_type: String,

    /// Enable auto-detection of virtual environments
    #[serde(default = "default_true")]
    pub auto_detection: bool,

    /// Additional directories to search for virtual environments
    /// Default includes the project directory. Add paths like "~" to search home directory.
    #[serde(default)]
    pub search_paths: Vec<String>,
}

impl Default for VenvConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: None,
            venv_type: default_venv_type(),
            auto_detection: true,
            search_paths: Vec::new(),
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

fn default_venv_type() -> String {
    "auto".to_string()
}

fn default_venv_key() -> String {
    "Alt v".to_string()
}

fn default_venv_select_key() -> String {
    "Alt Shift v".to_string()
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

    /// Locate the user's kitty config. This path is read-only.
    pub fn kitty_config_path(&self) -> Option<PathBuf> {
        self.session
            .kitty_config
            .as_deref()
            .map(expand_tilde)
            .or_else(|| {
                let base = std::env::var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .or_else(|| home::home_dir().map(|home| home.join(".config")))?;
                Some(base.join("kitty").join("kitty.conf"))
            })
            .filter(|path| path.exists())
    }

    /// Root of the per-session runtime state
    /// (`$XDG_RUNTIME_DIR/sat-helix-ide`, or the temp dir as fallback).
    pub fn runtime_root(&self) -> PathBuf {
        let base = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        base.join("sat-helix-ide")
    }

    pub fn runtime_dir(&self, session_name: &str) -> PathBuf {
        self.runtime_root().join(session_name)
    }
}

pub fn expand_tilde(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if text == "~" {
        return home::home_dir().unwrap_or_else(|| path.to_path_buf());
    }
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
    fn expand_tilde_handles_home_alone() {
        let home = home::home_dir().expect("home directory");
        assert_eq!(expand_tilde(Path::new("~")), home);
        assert_eq!(expand_tilde(Path::new("~/src")), home.join("src"));
    }

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
    fn backend_kind_deserializes_all_variants() {
        let config: Config = toml::from_str(r#"[session]
backend = "auto"
"#)
        .unwrap();
        assert_eq!(config.session.backend, BackendKind::Auto);

        let config: Config = toml::from_str(r#"[session]
backend = "kitty"
"#)
        .unwrap();
        assert_eq!(config.session.backend, BackendKind::Kitty);

        assert_eq!(Config::default().session.backend, BackendKind::Zellij);
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

    #[test]
    fn launcher_defaults_to_auto_home_yazi() {
        let config = Config::default();
        assert_eq!(config.launcher.picker, LauncherPicker::Auto);
        assert_eq!(config.launcher.start_dir, PathBuf::from("~"));
        assert_eq!(config.launcher.fzf_command, "fzf");
        assert_eq!(config.launcher.max_depth, 8);
    }

    #[test]
    fn passthrough_defaults_to_empty_and_parses_an_override() {
        let default_config = Config::default();
        assert!(default_config.keybindings.passthrough.is_empty());

        let config: Config = toml::from_str(
            r#"
            [keybindings]
            passthrough = ["Ctrl b", "Ctrl o"]
            "#,
        )
        .unwrap();

        assert_eq!(config.keybindings.passthrough, ["Ctrl b", "Ctrl o"]);
    }
}
