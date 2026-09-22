//! Virtual environment management module
//!
//! This module provides functionality for detecting and activating virtual environments
//! in Python projects.

use crate::config::{expand_tilde, VenvConfig};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Detected virtual environment information
#[derive(Debug, Clone)]
pub struct DetectedVenv {
    pub path: PathBuf,
    pub venv_type: String,
}

/// Auto-detect virtual environment in the given directory
pub fn auto_detect_venv(dir: &Path) -> Result<Option<DetectedVenv>> {
    // Check for .venv directory (standard Python venv or uv)
    let dot_venv = dir.join(".venv");
    if dot_venv.exists() && dot_venv.is_dir() {
        let activate_path = dot_venv.join("bin").join("activate");
        if activate_path.exists() {
            return Ok(Some(DetectedVenv {
                path: dot_venv,
                venv_type: "venv".to_string(),
            }));
        }
    }

    // Check for pyproject.toml to detect uv or poetry
    let pyproject = dir.join("pyproject.toml");
    if pyproject.exists() {
        let content = fs::read_to_string(&pyproject)?;

        // Check for uv
        if content.contains("[tool.uv]") {
            let uv_venv = dir.join(".venv");
            if uv_venv.exists() {
                return Ok(Some(DetectedVenv {
                    path: uv_venv,
                    venv_type: "uv".to_string(),
                }));
            }
            // uv can work without .venv directory
            return Ok(Some(DetectedVenv {
                path: dir.to_path_buf(),
                venv_type: "uv".to_string(),
            }));
        }

        // Check for poetry
        if content.contains("[tool.poetry]") {
            return Ok(Some(DetectedVenv {
                path: dir.to_path_buf(),
                venv_type: "poetry".to_string(),
            }));
        }
    }

    // Check for .python-version (often used with pyenv or uv)
    let python_version = dir.join(".python-version");
    if python_version.exists() {
        return Ok(Some(DetectedVenv {
            path: dir.to_path_buf(),
            venv_type: "uv".to_string(),
        }));
    }

    // Check for environment.yml (conda)
    let env_yaml = dir.join("environment.yml");
    if env_yaml.exists() {
        return Ok(Some(DetectedVenv {
            path: dir.to_path_buf(),
            venv_type: "conda".to_string(),
        }));
    }

    // Check for Pipfile (pipenv)
    let pipfile = dir.join("Pipfile");
    if pipfile.exists() {
        return Ok(Some(DetectedVenv {
            path: dir.to_path_buf(),
            venv_type: "pipenv".to_string(),
        }));
    }

    Ok(None)
}

/// Detect the type of virtual environment from its path
pub fn detect_venv_type(path: &Path) -> Result<String> {
    // If it's a .venv directory
    if path.file_name().and_then(|n| n.to_str()) == Some(".venv") {
        // Check if it has uv markers
        if let Some(parent) = path.parent() {
            let pyproject = parent.join("pyproject.toml");
            if pyproject.exists() {
                let content = fs::read_to_string(&pyproject)?;
                if content.contains("[tool.uv]") {
                    return Ok("uv".to_string());
                }
            }
        }
        return Ok("venv".to_string());
    }

    Ok("auto".to_string())
}

/// Generate activation and deactivation commands for a given venv type and path
pub fn generate_commands(venv_type: &str, path: Option<&Path>) -> Result<(String, String)> {
    let shell = detect_shell();

    match venv_type {
        "uv" => {
            // uv shell activates the environment in the current directory
            let activate = "uv shell".to_string();
            let deactivate = if shell == "fish" {
                "conda deactivate 2>/dev/null; deactivate 2>/dev/null; set -gx VIRTUAL_ENV"
                    .to_string()
            } else {
                "deactivate 2>/dev/null || true".to_string()
            };
            Ok((activate, deactivate))
        }
        "venv" => {
            let path = path.context("Path required for venv type")?;
            let activate_path = path.join("bin").join("activate");
            let activate = format!(
                "source {}",
                shell_quote(activate_path.display().to_string())
            );
            let deactivate = if shell == "fish" {
                "conda deactivate 2>/dev/null; deactivate 2>/dev/null; set -gx VIRTUAL_ENV"
                    .to_string()
            } else {
                "deactivate 2>/dev/null || true".to_string()
            };
            Ok((activate, deactivate))
        }
        "poetry" => {
            let activate = "poetry shell".to_string();
            let deactivate = "exit".to_string();
            Ok((activate, deactivate))
        }
        "conda" => {
            // For conda, we need the environment name
            // Try to extract from environment.yml or use directory name
            let activate = format!(
                "conda activate {} 2>/dev/null || conda activate base 2>/dev/null || echo 'No conda env'",
                detect_conda_env_name()
            );
            let deactivate = "conda deactivate 2>/dev/null || true".to_string();
            Ok((activate, deactivate))
        }
        "pipenv" => {
            let activate = "pipenv shell".to_string();
            let deactivate = "exit".to_string();
            Ok((activate, deactivate))
        }
        _ => {
            // Default to venv-style activation for "auto" and unknown types
            let path = path.context("Path required for venv type")?;
            let activate_path = path.join("bin").join("activate");
            let activate = format!(
                "source {}",
                shell_quote(activate_path.display().to_string())
            );
            let deactivate = if shell == "fish" {
                "conda deactivate 2>/dev/null; deactivate 2>/dev/null; set -gx VIRTUAL_ENV"
                    .to_string()
            } else {
                "deactivate 2>/dev/null || true".to_string()
            };
            Ok((activate, deactivate))
        }
    }
}

/// Detect conda environment name from environment.yml or directory name
fn detect_conda_env_name() -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(name) = cwd.file_name() {
            if let Some(name_str) = name.to_str() {
                return name_str.to_string();
            }
        }
    }
    "base".to_string()
}

/// Detect the current shell
fn detect_shell() -> String {
    std::env::var("SHELL")
        .unwrap_or_else(|_| "/bin/sh".to_string())
        .split('/')
        .next_back()
        .unwrap_or("sh")
        .to_string()
}

/// Quote a path for use in shell commands
fn shell_quote(path: String) -> String {
    if path.contains('\'')
        || path.contains('"')
        || path.contains('$')
        || path.contains('`')
        || path.contains(' ')
    {
        format!("\"{}\"", path.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        path
    }
}

/// Determine the activation and deactivation commands based on configuration or auto-detection.
pub fn determine_venv_commands(venv_config: &VenvConfig) -> Result<(String, String)> {
    let project_dir =
        std::env::current_dir().context("Cannot determine current directory for venv detection")?;

    // If path is explicitly configured, use it
    if let Some(ref path) = venv_config.path {
        let resolved_path = expand_tilde(Path::new(path));
        let venv_type = if venv_config.venv_type == "auto" {
            detect_venv_type(&resolved_path)?
        } else {
            venv_config.venv_type.clone()
        };
        return generate_commands(&venv_type, Some(&resolved_path));
    }

    // Auto-detection: scan project directory for virtual environment indicators
    if venv_config.auto_detection {
        if let Some(detected) = auto_detect_venv(&project_dir)? {
            let venv_type = if venv_config.venv_type == "auto" {
                detected.venv_type
            } else {
                venv_config.venv_type.clone()
            };
            return generate_commands(&venv_type, Some(&detected.path));
        }
    }

    // Fallback: use default activation commands based on type
    generate_commands(&venv_config.venv_type, None)
}
