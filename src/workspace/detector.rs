use std::path::Path;

/// Detects project types based on directory contents
#[derive(Default)]
pub struct ProjectDetector;

impl ProjectDetector {
    /// Create a new project detector
    pub fn new() -> Self {
        Self
    }

    /// Detect project type from directory
    pub fn detect_project(&self, path: &Path) -> Option<String> {
        const MARKERS: &[(&str, &str)] = &[
            ("Cargo.toml", "rust"),
            ("package.json", "node"),
            ("pyproject.toml", "python"),
            ("setup.py", "python"),
            ("requirements.txt", "python"),
            ("Gemfile", "ruby"),
            ("go.mod", "go"),
            ("Makefile", "c"),
            (".git", "git"),
        ];

        for (marker, project_type) in MARKERS {
            if path.join(marker).exists() {
                return Some((*project_type).to_string());
            }
        }

        None
    }
}
