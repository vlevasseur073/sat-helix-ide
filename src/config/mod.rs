mod layout;
mod tools;
mod workspace;

pub use layout::{LayoutConfig, LayoutPane, LayoutPart};
pub use tools::{GitConfig, ToolConfig, YaziConfig};
pub use workspace::WorkspaceConfig;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Main configuration for Helix IDE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Workspace configuration
    #[serde(default)]
    pub workspace: WorkspaceConfig,

    /// Tool configurations
    #[serde(default)]
    pub tools: ToolConfig,

    /// Layout configurations
    #[serde(default)]
    pub layouts: Vec<LayoutConfig>,

    /// Default layout
    #[serde(default = "default_default_layout")]
    pub default_layout: String,

    /// Use configurations generated in sat-helix-ide's private config directory.
    ///
    /// Disabled by default so the tools continue to use the user's existing
    /// configuration unchanged.
    #[serde(default)]
    pub use_generated_configs: bool,

    /// sat-helix-ide-owned directory for generated tool configurations.
    #[serde(skip, default = "default_generated_config_dir")]
    generated_config_dir: PathBuf,

    /// sat-helix-ide-owned file for volatile workspace state.
    #[serde(skip, default = "default_state_file")]
    state_file: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let mut config: Self = toml::from_str(include_str!("../../configs/config.toml"))
            .expect("bundled configuration must be valid TOML");
        config.generated_config_dir = default_generated_config_dir();
        config.state_file = default_state_file();
        config
    }
}

fn default_default_layout() -> String {
    "default".to_string()
}

fn default_app_config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home::home_dir().map(|p| p.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("sat-helix-ide")
}

fn default_generated_config_dir() -> PathBuf {
    default_app_config_dir().join("generated")
}

fn default_state_file() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| home::home_dir().map(|p| p.join(".local").join("state")))
        .unwrap_or_else(|| PathBuf::from(".local/state"))
        .join("sat-helix-ide")
        .join("state.toml")
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct WorkspaceState {
    #[serde(skip_serializing_if = "Option::is_none")]
    last_workspace: Option<PathBuf>,
}

impl Config {
    /// Load configuration from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Convert to string to handle tilde expansion
        let path_str = path.to_string_lossy();

        // If path starts with ~, expand it using home directory
        let path = if path_str.starts_with("~/") {
            home::home_dir()
                .map(|p| p.join(&path_str[2..]))
                .unwrap_or_else(|| path.to_path_buf())
        } else {
            path.to_path_buf()
        };

        // Make it absolute
        let path = path_absolutize::Absolutize::absolutize(&path)?;
        log::debug!("Loading config from: {}", path.display());

        if !path.exists() {
            log::debug!("Config does not exist; using in-memory defaults");
            let mut config = Config::default();
            config.load_state()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&path)?;
        let mut config: Config = toml::from_str(&content)?;
        config.generated_config_dir = default_generated_config_dir();
        config.state_file = default_state_file();
        config.load_state()?;

        Ok(config)
    }

    /// Persist volatile application state outside all tool configuration trees.
    pub fn save_last_workspace(&self, path: &Path) -> Result<()> {
        let state = WorkspaceState {
            last_workspace: Some(path.to_path_buf()),
        };
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string(&state)?;
        fs::write(&self.state_file, content)
            .with_context(|| format!("Failed to save state to {}", self.state_file.display()))?;
        Ok(())
    }

    fn load_state(&mut self) -> Result<()> {
        if !self.state_file.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&self.state_file)?;
        let state: WorkspaceState = toml::from_str(&content)?;
        self.workspace.last_workspace = state.last_workspace;
        Ok(())
    }

    pub fn generated_zellij_config(&self) -> PathBuf {
        self.generated_config_dir.join("zellij").join("config.kdl")
    }

    pub fn generated_yazi_config_dir(&self) -> PathBuf {
        self.generated_config_dir.join("yazi")
    }

    pub fn generated_helix_config(&self) -> PathBuf {
        self.generated_config_dir.join("helix").join("config.toml")
    }

    /// Get layout by name
    pub fn get_layout(&self, name: &str) -> Option<&LayoutConfig> {
        self.layouts.iter().find(|l| l.name == name)
    }

    /// List all available layouts
    pub fn list_layouts(&self) -> Vec<String> {
        self.layouts.iter().map(|l| l.name.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_a_missing_config_never_creates_it() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("zellij").join("config.kdl");

        let _ = Config::load(&path).unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn workspace_state_does_not_touch_tool_configuration() {
        let temp = tempfile::tempdir().unwrap();
        let tool_config = temp.path().join("config.kdl");
        fs::write(&tool_config, "user-owned-zellij-config").unwrap();

        let config = Config {
            state_file: temp.path().join("state").join("state.toml"),
            ..Default::default()
        };
        config
            .save_last_workspace(Path::new("/tmp/example-project"))
            .unwrap();

        assert_eq!(
            fs::read_to_string(&tool_config).unwrap(),
            "user-owned-zellij-config"
        );
        let state = fs::read_to_string(&config.state_file).unwrap();
        assert!(state.contains("/tmp/example-project"));
    }

    #[test]
    fn generated_configs_live_under_the_application_directory() {
        let config = Config::default();
        for path in [
            config.generated_zellij_config(),
            config.generated_yazi_config_dir(),
            config.generated_helix_config(),
        ] {
            assert!(path.to_string_lossy().contains("sat-helix-ide/generated"));
        }
    }
}
