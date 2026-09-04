use serde::{Deserialize, Serialize};

/// An executable and the arguments passed to it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandConfig {
    #[serde(alias = "path", alias = "client")]
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,
}

impl CommandConfig {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
        }
    }
}

/// File-manager behavior. Yazi is the first adapter because it supports
/// chooser files, which lets us hand a selected path to the existing Helix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileManagerConfig {
    #[serde(default = "default_file_manager_adapter")]
    pub adapter: String,

    #[serde(alias = "path", default = "default_file_manager_command")]
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default = "default_float_width")]
    pub float_width: String,

    #[serde(default = "default_float_height")]
    pub float_height: String,

    #[serde(default = "default_dock_percent")]
    pub dock_percent: u8,
}

impl Default for FileManagerConfig {
    fn default() -> Self {
        Self {
            adapter: default_file_manager_adapter(),
            command: default_file_manager_command(),
            args: Vec::new(),
            float_width: default_float_width(),
            float_height: default_float_height(),
            dock_percent: default_dock_percent(),
        }
    }
}

/// Commands launched by the session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    #[serde(default = "default_zellij")]
    pub zellij: CommandConfig,

    #[serde(default = "default_editor", alias = "helix")]
    pub editor: CommandConfig,

    #[serde(default, alias = "yazi")]
    pub file_manager: FileManagerConfig,

    #[serde(default = "default_git")]
    pub git: CommandConfig,

    #[serde(default = "default_review")]
    pub review: CommandConfig,

    #[serde(default = "default_ai")]
    pub ai: Option<CommandConfig>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            zellij: default_zellij(),
            editor: default_editor(),
            file_manager: FileManagerConfig::default(),
            git: default_git(),
            review: default_review(),
            ai: default_ai(),
        }
    }
}

fn default_zellij() -> CommandConfig {
    CommandConfig::new("zellij")
}

fn default_editor() -> CommandConfig {
    CommandConfig::new("hx")
}

fn default_git() -> CommandConfig {
    CommandConfig::new("lazygit")
}

fn default_review() -> CommandConfig {
    CommandConfig::new("revdiff")
}

fn default_ai() -> Option<CommandConfig> {
    Some(CommandConfig::new("cursor-agent"))
}

fn default_file_manager_adapter() -> String {
    "yazi".to_string()
}

fn default_file_manager_command() -> String {
    "yazi".to_string()
}

fn default_float_width() -> String {
    "100%".to_string()
}

fn default_float_height() -> String {
    "100%".to_string()
}

fn default_dock_percent() -> u8 {
    28
}
