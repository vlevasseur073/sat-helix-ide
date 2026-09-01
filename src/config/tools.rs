use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool configurations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Helix configuration
    #[serde(default)]
    pub helix: HelixConfig,

    /// Zellij configuration
    #[serde(default)]
    pub zellij: ZellijConfig,

    /// Yazi configuration
    #[serde(default)]
    pub yazi: YaziConfig,

    /// Git client configuration
    #[serde(default)]
    pub git: GitConfig,

    /// Custom tool commands
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

/// Helix editor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelixConfig {
    /// Path to Helix executable
    #[serde(default = "default_helix_path")]
    pub path: String,

    /// Default theme
    #[serde(default = "default_helix_theme")]
    pub theme: String,

    /// Enable LSP
    #[serde(default = "default_helix_lsp")]
    pub lsp: bool,

    /// Additional command line arguments
    #[serde(default)]
    pub args: Vec<String>,
}

impl Default for HelixConfig {
    fn default() -> Self {
        Self {
            path: default_helix_path(),
            theme: default_helix_theme(),
            lsp: default_helix_lsp(),
            args: vec![],
        }
    }
}

fn default_helix_path() -> String {
    "hx".to_string()
}

fn default_helix_theme() -> String {
    "catppuccin_mocha".to_string()
}

fn default_helix_lsp() -> bool {
    true
}

/// Zellij configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZellijConfig {
    /// Path to Zellij executable
    #[serde(default = "default_zellij_path")]
    pub path: String,

    /// Default theme
    #[serde(default = "default_zellij_theme")]
    pub theme: String,

    /// Enable mouse support
    #[serde(default = "default_zellij_mouse")]
    pub mouse: bool,
}

impl Default for ZellijConfig {
    fn default() -> Self {
        Self {
            path: default_zellij_path(),
            theme: default_zellij_theme(),
            mouse: default_zellij_mouse(),
        }
    }
}

fn default_zellij_path() -> String {
    "zellij".to_string()
}

fn default_zellij_theme() -> String {
    "catppuccin-mocha".to_string()
}

fn default_zellij_mouse() -> bool {
    true
}

/// Yazi file explorer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YaziConfig {
    /// Path to Yazi executable
    #[serde(default = "default_yazi_path")]
    pub path: String,

    /// Open files in Helix
    #[serde(default = "default_yazi_open_in_helix")]
    pub open_in_helix: bool,

    /// Preview configuration
    #[serde(default)]
    pub preview: PreviewConfig,
}

impl Default for YaziConfig {
    fn default() -> Self {
        Self {
            path: default_yazi_path(),
            open_in_helix: default_yazi_open_in_helix(),
            preview: PreviewConfig::default(),
        }
    }
}

fn default_yazi_path() -> String {
    "yazi".to_string()
}

fn default_yazi_open_in_helix() -> bool {
    true
}

/// Preview configuration for Yazi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewConfig {
    /// Maximum width
    #[serde(default = "default_preview_width")]
    pub max_width: usize,

    /// Maximum height
    #[serde(default = "default_preview_height")]
    pub max_height: usize,

    /// Tab size
    #[serde(default = "default_preview_tab_size")]
    pub tab_size: usize,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            max_width: default_preview_width(),
            max_height: default_preview_height(),
            tab_size: default_preview_tab_size(),
        }
    }
}

fn default_preview_width() -> usize {
    120
}

fn default_preview_height() -> usize {
    60
}

fn default_preview_tab_size() -> usize {
    4
}

/// Git client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// Preferred git client (lazygit or gitui)
    #[serde(default = "default_git_client")]
    pub client: String,

    /// Use git-delta for diffs
    #[serde(default = "default_git_delta")]
    pub delta: bool,

    /// Delta configuration
    #[serde(default)]
    pub delta_config: DeltaConfig,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            client: default_git_client(),
            delta: default_git_delta(),
            delta_config: DeltaConfig::default(),
        }
    }
}

fn default_git_client() -> String {
    "lazygit".to_string()
}

fn default_git_delta() -> bool {
    true
}

/// Git-delta configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaConfig {
    /// Syntax theme
    #[serde(default = "default_delta_theme")]
    pub theme: String,

    /// Show line numbers
    #[serde(default = "default_delta_line_numbers")]
    pub line_numbers: bool,

    /// Side-by-side diff
    #[serde(default = "default_delta_side_by_side")]
    pub side_by_side: bool,
}

impl Default for DeltaConfig {
    fn default() -> Self {
        Self {
            theme: default_delta_theme(),
            line_numbers: default_delta_line_numbers(),
            side_by_side: default_delta_side_by_side(),
        }
    }
}

fn default_delta_theme() -> String {
    "Catppuccin-mocha".to_string()
}

fn default_delta_line_numbers() -> bool {
    true
}

fn default_delta_side_by_side() -> bool {
    true
}
