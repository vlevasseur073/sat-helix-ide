mod git;
mod yazi;

pub use git::GitClient;
pub use yazi::YaziClient;

use crate::config::Config;
use which::which;

/// Manages external tools
pub struct ToolManager {
    pub yazi: YaziClient,
    pub git: GitClient,
}

impl ToolManager {
    /// Create a new tool manager
    pub fn new(config: &Config) -> Self {
        Self {
            yazi: YaziClient::new(&config.tools.yazi),
            git: GitClient::new(&config.tools.git),
        }
    }

    /// Check if a command is available
    pub fn is_available(&self, command: &str) -> bool {
        which(command).is_ok()
    }
}
