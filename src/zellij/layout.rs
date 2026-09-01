use crate::config::LayoutConfig;
use crate::error::HxIdeError;
use std::fs;
use std::path::PathBuf;

/// Generates Zellij layouts from configuration
#[derive(Default)]
pub struct LayoutGenerator;

impl LayoutGenerator {
    /// Create a new layout generator
    pub fn new() -> Self {
        Self
    }

    /// Generate KDL layout from configuration
    pub fn generate_kdl(layout: &LayoutConfig) -> String {
        layout.to_kdl()
    }

    /// Save layout to file
    pub fn save_layout(layout: &LayoutConfig, path: &PathBuf) -> Result<(), HxIdeError> {
        let kdl = Self::generate_kdl(layout);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, kdl)?;
        Ok(())
    }

    /// Load layout from file
    pub fn load_layout(path: &PathBuf) -> Result<LayoutConfig, HxIdeError> {
        let _content = fs::read_to_string(path)?;

        // Parse the KDL content
        // Note: This is a simplified parser. For a full implementation,
        // you might want to use a proper KDL parser crate
        let layout = LayoutConfig {
            name: path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unnamed")
                .to_string(),
            ..Default::default()
        };

        // Simple parsing logic would go here
        // For now, return a basic layout
        Ok(layout)
    }
}
