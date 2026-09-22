//! Virtual environment management module
//!
//! This module provides functionality for detecting and activating virtual environments
//! in Python projects.

use crate::config::{expand_tilde, VenvConfig};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

/// Detected environment for selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectableVenv {
    pub name: String,
    pub path: PathBuf,
    pub venv_type: String,
}

/// All saved virtual environments (detected + custom)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenvCollection {
    pub environments: Vec<SelectableVenv>,
}

/// List all detected virtual environments in a directory
pub fn list_selectable_venvs(dir: &Path) -> Result<Vec<SelectableVenv>> {
    let mut results = Vec::new();

    // Check for .venv directory
    let dot_venv = dir.join(".venv");
    if dot_venv.exists() && dot_venv.is_dir() {
        let activate_path = dot_venv.join("bin").join("activate");
        if activate_path.exists() {
            results.push(SelectableVenv {
                name: ".venv".to_string(),
                path: dot_venv.clone(),
                venv_type: "venv".to_string(),
            });
        }
    }

    // Check for pyproject.toml with uv
    let pyproject = dir.join("pyproject.toml");
    if pyproject.exists() {
        if let Ok(content) = fs::read_to_string(&pyproject) {
            if content.contains("[tool.uv]") {
                let uv_venv = dir.join(".venv");
                results.push(SelectableVenv {
                    name: "uv (pyproject.toml)".to_string(),
                    path: if uv_venv.exists() {
                        uv_venv
                    } else {
                        dir.to_path_buf()
                    },
                    venv_type: "uv".to_string(),
                });
            }
            if content.contains("[tool.poetry]") {
                results.push(SelectableVenv {
                    name: "poetry (pyproject.toml)".to_string(),
                    path: dir.to_path_buf(),
                    venv_type: "poetry".to_string(),
                });
            }
        }
    }

    // Check for .python-version
    let python_version = dir.join(".python-version");
    if python_version.exists() {
        results.push(SelectableVenv {
            name: ".python-version".to_string(),
            path: dir.to_path_buf(),
            venv_type: "uv".to_string(),
        });
    }

    // Check for environment.yml
    let env_yaml = dir.join("environment.yml");
    if env_yaml.exists() {
        results.push(SelectableVenv {
            name: "conda (environment.yml)".to_string(),
            path: dir.to_path_buf(),
            venv_type: "conda".to_string(),
        });
    }

    // Check for Pipfile
    let pipfile = dir.join("Pipfile");
    if pipfile.exists() {
        results.push(SelectableVenv {
            name: "pipenv (Pipfile)".to_string(),
            path: dir.to_path_buf(),
            venv_type: "pipenv".to_string(),
        });
    }

    Ok(results)
}

/// List all selectable virtual environments from multiple search paths and config
pub fn list_all_selectable_venvs(venv_config: &VenvConfig) -> Result<Vec<SelectableVenv>> {
    use crate::config::expand_tilde;

    let mut all_environments = Vec::new();

    // Add user-configured path if set
    if let Some(ref path) = venv_config.path {
        let resolved_path = expand_tilde(Path::new(path));
        let venv_type = if venv_config.venv_type == "auto" {
            detect_venv_type(&resolved_path)?
        } else {
            venv_config.venv_type.clone()
        };
        all_environments.push(SelectableVenv {
            name: format!("Configured: {}", resolved_path.display()),
            path: resolved_path,
            venv_type,
        });
    }

    // Build list of directories to search
    let mut search_dirs = Vec::new();

    // Always search current directory first
    if let Ok(current_dir) = std::env::current_dir() {
        search_dirs.push(current_dir);
    }

    // Add configured search paths
    for search_path in &venv_config.search_paths {
        let expanded = expand_tilde(Path::new(search_path));
        if expanded.exists() && expanded.is_dir() {
            search_dirs.push(expanded);
        }
    }

    // Also search home directory by default
    if let Some(home) = home::home_dir() {
        if !search_dirs.iter().any(|d| d == &home) {
            search_dirs.push(home);
        }
    }

    // Deduplicate search directories
    let unique_search_dirs: Vec<PathBuf> = search_dirs
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // Search each directory for environments
    for search_dir in &unique_search_dirs {
        if let Ok(venvs) = list_selectable_venvs(search_dir) {
            all_environments.extend(venvs);
        }
    }

    // Remove duplicates based on path
    let unique_environments: Vec<SelectableVenv> =
        all_environments
            .into_iter()
            .fold(Vec::new(), |mut acc, env| {
                if !acc.iter().any(|e| e.path == env.path) {
                    acc.push(env);
                }
                acc
            });

    Ok(unique_environments)
}

/// Save the selected environment path to session runtime
pub fn save_venv_selection(session_name: &str, selection: &SelectableVenv) -> Result<()> {
    let runtime_dir = crate::config::Config::default().runtime_dir(session_name);
    std::fs::create_dir_all(&runtime_dir)?;

    let selection_file = runtime_dir.join("venv_selection.json");
    let json = serde_json::to_string(selection)?;
    std::fs::write(selection_file, json)?;

    Ok(())
}

/// Load the saved environment selection from session runtime
pub fn load_venv_selection(session_name: &str) -> Result<Option<SelectableVenv>> {
    let runtime_dir = crate::config::Config::default().runtime_dir(session_name);
    let selection_file = runtime_dir.join("venv_selection.json");

    if !selection_file.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&selection_file)?;
    let selection: SelectableVenv = serde_json::from_str(&content)?;

    Ok(Some(selection))
}

/// Save all known environments (detected + custom) to session runtime
pub fn save_venv_collection(session_name: &str, collection: &VenvCollection) -> Result<()> {
    let runtime_dir = crate::config::Config::default().runtime_dir(session_name);
    std::fs::create_dir_all(&runtime_dir)?;

    let collection_file = runtime_dir.join("venv_collection.json");
    let json = serde_json::to_string(collection)?;
    std::fs::write(collection_file, json)?;

    Ok(())
}

/// Load all saved environments from session runtime
pub fn load_venv_collection(session_name: &str) -> Result<Option<VenvCollection>> {
    let runtime_dir = crate::config::Config::default().runtime_dir(session_name);
    let collection_file = runtime_dir.join("venv_collection.json");

    if !collection_file.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&collection_file)?;
    let collection: VenvCollection = serde_json::from_str(&content)?;

    Ok(Some(collection))
}

/// Get all available environments including detected and previously saved custom paths
pub fn get_all_available_venvs(
    venv_config: &VenvConfig,
    session_name: &str,
) -> Result<Vec<SelectableVenv>> {
    // Load previously saved collection
    if let Ok(Some(saved_collection)) = load_venv_collection(session_name) {
        // Get freshly detected environments
        let detected = list_all_selectable_venvs(venv_config)?;

        // Merge detected with saved, deduplicating by path
        let mut all_environments = saved_collection.environments;

        // Add detected environments that aren't already in the saved collection
        for detected_env in detected {
            if !all_environments.iter().any(|e| e.path == detected_env.path) {
                all_environments.push(detected_env);
            }
        }

        Ok(all_environments)
    } else {
        // No saved collection yet, just return detected environments
        list_all_selectable_venvs(venv_config)
    }
}

/// Transient handoff files used only while the selector TUI is open (not persisted).
pub const VENV_SELECTOR_OUTPUT: &str = "venv_selector_output";
pub const VENV_SELECTOR_SELECTION: &str = "venv_selector_selection.json";
pub const VENV_SELECTOR_CUSTOM_PATH: &str = "venv_selector_custom_path";

/// Marker written to [`VENV_SELECTOR_OUTPUT`] when the user made a selection.
pub const VENV_SELECTOR_DONE: &str = "OK";

/// Whether the stored selection can be activated as-is (used for list checkmarks).
pub fn is_selection_activatable(env: &SelectableVenv) -> bool {
    validate_selection_activatable(env).is_ok()
}

/// Verify that a saved selection path and type match what `generate_commands` expects.
pub fn validate_selection_activatable(env: &SelectableVenv) -> Result<()> {
    let path = expand_tilde(&env.path);
    match env.venv_type.as_str() {
        "venv" | "auto" => require_activate_at(&path),
        "uv" => validate_uv_activatable(&path),
        "poetry" => validate_poetry(&path),
        "conda" => validate_conda(&path),
        "pipenv" => validate_pipenv(&path),
        _ => require_activate_at(&path),
    }
}

/// Strict validation for a user-typed custom path (used by the selector TUI).
pub fn validate_custom_venv_path(path: &Path) -> Result<()> {
    resolve_custom_venv_path(path).map(|_| ())
}

fn activate_script_path(venv_root: &Path) -> Option<PathBuf> {
    let unix = venv_root.join("bin").join("activate");
    if unix.is_file() {
        return Some(unix);
    }
    let windows = venv_root.join("Scripts").join("activate");
    if windows.is_file() {
        return Some(windows);
    }
    None
}

fn require_activate_at(path: &Path) -> Result<()> {
    if activate_script_path(path).is_some() {
        return Ok(());
    }
    bail!(
        "No bin/activate script at {} (enter the venv directory, e.g. {}/.venv)",
        path.display(),
        path.display()
    );
}

fn validate_uv_activatable(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }
    if activate_script_path(path).is_some() {
        return Ok(());
    }
    if path.is_dir() && uv_project_markers(path) {
        return Ok(());
    }
    bail!(
        "Not a uv project or virtual environment at {}",
        path.display()
    );
}

fn uv_project_markers(path: &Path) -> bool {
    let pyproject = path.join("pyproject.toml");
    if pyproject.is_file() {
        if let Ok(content) = fs::read_to_string(&pyproject) {
            if content.contains("[tool.uv]") {
                return true;
            }
        }
    }
    path.join(".python-version").is_file()
}

fn validate_poetry(path: &Path) -> Result<()> {
    if !path.is_dir() {
        bail!("Path is not a directory: {}", path.display());
    }
    let pyproject = path.join("pyproject.toml");
    if !pyproject.is_file() {
        bail!("No pyproject.toml found at {}", path.display());
    }
    let content = fs::read_to_string(&pyproject)?;
    if content.contains("[tool.poetry]") {
        Ok(())
    } else {
        bail!(
            "pyproject.toml is not a Poetry project at {}",
            path.display()
        );
    }
}

fn validate_conda(path: &Path) -> Result<()> {
    if !path.is_dir() {
        bail!("Path is not a directory: {}", path.display());
    }
    if path.join("environment.yml").is_file() {
        Ok(())
    } else {
        bail!("No environment.yml found at {}", path.display());
    }
}

fn validate_pipenv(path: &Path) -> Result<()> {
    if !path.is_dir() {
        bail!("Path is not a directory: {}", path.display());
    }
    if path.join("Pipfile").is_file() {
        Ok(())
    } else {
        bail!("No Pipfile found at {}", path.display());
    }
}

/// Resolve and validate a user-typed path into a storable selection.
pub fn resolve_custom_venv_path(path: &Path) -> Result<SelectableVenv> {
    let path = expand_tilde(path);
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }

    if activate_script_path(&path).is_some() {
        let venv_type = detect_venv_type(&path).unwrap_or_else(|_| "venv".to_string());
        return Ok(custom_venv_entry(path, venv_type));
    }

    if validate_poetry(&path).is_ok() {
        return Ok(custom_venv_entry(path, "poetry".to_string()));
    }
    if path.is_dir() && uv_project_markers(&path) {
        return Ok(custom_venv_entry(path, "uv".to_string()));
    }
    if validate_conda(&path).is_ok() {
        return Ok(custom_venv_entry(path, "conda".to_string()));
    }
    if validate_pipenv(&path).is_ok() {
        return Ok(custom_venv_entry(path, "pipenv".to_string()));
    }

    let nested = path.join(".venv");
    if nested.is_dir() && activate_script_path(&nested).is_some() {
        bail!(
            "No environment at {}. Enter the venv directory instead: {}",
            path.display(),
            nested.display()
        );
    }

    bail!("No virtual environment found at {}", path.display());
}

fn custom_venv_entry(path: PathBuf, venv_type: String) -> SelectableVenv {
    SelectableVenv {
        name: format!("Custom: {}", path.display()),
        path,
        venv_type,
    }
}

/// Read the user's choice from transient selector handoff files.
pub fn read_selector_handoff(runtime_dir: &Path) -> Result<Option<SelectableVenv>> {
    let selection_file = runtime_dir.join(VENV_SELECTOR_SELECTION);
    if selection_file.exists() {
        let content = fs::read_to_string(&selection_file)?;
        let _ = fs::remove_file(&selection_file);
        let selection: SelectableVenv = serde_json::from_str(content.trim())
            .context("Invalid virtual environment selection from selector")?;
        validate_selection_activatable(&selection)?;
        return Ok(Some(selection));
    }

    let custom_path_file = runtime_dir.join(VENV_SELECTOR_CUSTOM_PATH);
    if custom_path_file.exists() {
        let path_line = fs::read_to_string(&custom_path_file)?;
        let _ = fs::remove_file(&custom_path_file);
        let path = PathBuf::from(path_line.trim());
        if path.as_os_str().is_empty() {
            bail!("Custom virtual environment path is empty");
        }
        return resolve_custom_venv_path(&path).map(Some);
    }

    Ok(None)
}

/// Remove leftover selector handoff files from a previous run.
pub fn clear_selector_handoff(runtime_dir: &Path) -> Result<()> {
    for name in [
        VENV_SELECTOR_OUTPUT,
        VENV_SELECTOR_SELECTION,
        VENV_SELECTOR_CUSTOM_PATH,
    ] {
        let path = runtime_dir.join(name);
        if path.exists() {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

/// Generate a shell script that lets the user select a virtual environment
/// The script writes the selection to a file and exits
pub fn generate_selector_script(
    environments: &[SelectableVenv],
    output_file: &Path,
    selection_file: &Path,
    custom_path_file: &Path,
    validator_executable: &Path,
) -> Result<String> {
    use std::fmt::Write;

    let mut script = String::new();

    writeln!(script, "#!/bin/sh")?;
    writeln!(script, "# Virtual Environment Selector")?;
    writeln!(script, "# Generated by sat-hx-ide")?;
    writeln!(script)?;

    if environments.is_empty() {
        writeln!(
            script,
            "echo 'No virtual environments detected in this project.'"
        )?;
        // Write a marker to the output file to signal completion
        writeln!(script, "echo 'NO_ENVIRONMENTS' > {}", output_file.display())?;
        writeln!(script, "exit 0")?;
        return Ok(script);
    }

    writeln!(script, "echo 'Select a virtual environment to activate:'")?;
    writeln!(
        script,
        "echo '  [✓] verified   [ ] not found or incomplete'"
    )?;
    writeln!(script, "echo ''")?;

    for (i, env) in environments.iter().enumerate() {
        let checkbox = if is_selection_activatable(env) {
            "[✓]"
        } else {
            "[ ]"
        };
        writeln!(
            script,
            "echo '{} {} - {} ({})'",
            checkbox,
            i + 1,
            env.name,
            env.venv_type
        )?;
    }

    writeln!(script)?;
    writeln!(script, "echo ''")?;
    writeln!(
        script,
        "echo 'Enter the number of your choice, c to add a custom path, or q to quit:'"
    )?;
    writeln!(script, "read choice")?;
    writeln!(script)?;

    writeln!(script, r##"case "$choice" in"##)?;

    let done = shell_single_quote(VENV_SELECTOR_DONE);

    for (i, env) in environments.iter().enumerate() {
        let index = i + 1;
        let env_json = serde_json::to_string(env)?;
        let env_json_quoted = shell_single_quote(&env_json);
        writeln!(script, "  {}*)", index)?;
        writeln!(
            script,
            "    printf '%s\\n' {} > {}",
            env_json_quoted,
            selection_file.display()
        )?;
        writeln!(
            script,
            "    printf '%s\\n' {} > {}",
            done,
            output_file.display()
        )?;
        writeln!(script, "    exit 0")?;
        writeln!(script, "    ;;")?;
    }

    let validator = shell_single_quote(&validator_executable.display().to_string());

    // Add option to enter a custom path
    writeln!(script, "  c|C)")?;
    writeln!(
        script,
        "    echo 'Enter the path to a custom virtual environment:'"
    )?;
    writeln!(script, "    while true; do")?;
    writeln!(script, "      printf '> '")?;
    writeln!(script, "      read custom_path")?;
    writeln!(script, "      if [ -z \"$custom_path\" ]; then")?;
    writeln!(script, "        echo 'No path entered, cancelling.'")?;
    writeln!(script, "        exit 1")?;
    writeln!(script, "      fi")?;
    writeln!(
        script,
        "      if {} __venv validate-path \"$custom_path\"; then",
        validator
    )?;
    writeln!(script, "        break")?;
    writeln!(script, "      fi")?;
    writeln!(script, "      echo ''")?;
    writeln!(script, "    done")?;
    writeln!(
        script,
        "    printf '%s\\n' \"$custom_path\" > {}",
        custom_path_file.display()
    )?;
    writeln!(
        script,
        "    printf '%s\\n' {} > {}",
        done,
        output_file.display()
    )?;
    writeln!(script, "    exit 0")?;
    writeln!(script, "    ;;")?;

    writeln!(script, "  q|Q)")?;
    writeln!(script, "    exit 1")?;
    writeln!(script, "    ;;")?;
    writeln!(script, "  *)")?;
    writeln!(script, "    echo 'Invalid choice'")?;
    writeln!(script, "    exit 1")?;
    writeln!(script, "    ;;")?;
    writeln!(script, "esac")?;

    Ok(script)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn read_selector_handoff_parses_json_selection() {
        let venv = tempfile::tempdir().unwrap();
        let bin = venv.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("activate"), "").unwrap();
        let handoff = tempfile::tempdir().unwrap();
        let selection = SelectableVenv {
            name: "test".to_string(),
            path: venv.path().to_path_buf(),
            venv_type: "venv".to_string(),
        };
        fs::write(
            handoff.path().join(VENV_SELECTOR_SELECTION),
            serde_json::to_string(&selection).unwrap(),
        )
        .unwrap();

        let parsed = read_selector_handoff(handoff.path()).unwrap().unwrap();
        assert_eq!(parsed.path, selection.path);
        assert!(!handoff.path().join(VENV_SELECTOR_SELECTION).exists());
    }

    #[test]
    fn read_selector_handoff_reads_custom_path_file() {
        let venv = tempfile::tempdir().unwrap();
        let bin = venv.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("activate"), "").unwrap();
        let handoff = tempfile::tempdir().unwrap();
        fs::write(
            handoff.path().join(VENV_SELECTOR_CUSTOM_PATH),
            format!("{}\n", venv.path().display()),
        )
        .unwrap();

        let parsed = read_selector_handoff(handoff.path()).unwrap().unwrap();
        assert_eq!(parsed.path, venv.path());
        assert!(parsed.name.starts_with("Custom:"));
    }

    #[test]
    fn resolve_custom_path_requires_activate_at_entered_path() {
        let project = tempfile::tempdir().unwrap();
        let dot_venv = project.path().join(".venv");
        let bin = dot_venv.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("activate"), "").unwrap();

        assert!(resolve_custom_venv_path(project.path()).is_err());
        assert!(resolve_custom_venv_path(&dot_venv).is_ok());
    }

    #[test]
    fn resolve_custom_path_accepts_poetry_project() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("pyproject.toml"),
            "[tool.poetry]\nname = \"demo\"\n",
        )
        .unwrap();
        let resolved = resolve_custom_venv_path(dir.path()).unwrap();
        assert_eq!(resolved.venv_type, "poetry");
    }

    #[test]
    fn selection_checkbox_false_when_project_root_auto_without_activate() {
        let project = tempfile::tempdir().unwrap();
        let dot_venv = project.path().join(".venv");
        let bin = dot_venv.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("activate"), "").unwrap();
        let env = SelectableVenv {
            name: "Custom: demo".to_string(),
            path: project.path().to_path_buf(),
            venv_type: "auto".to_string(),
        };
        assert!(!is_selection_activatable(&env));
    }

    #[test]
    fn clear_selector_handoff_removes_transient_files() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            VENV_SELECTOR_OUTPUT,
            VENV_SELECTOR_SELECTION,
            VENV_SELECTOR_CUSTOM_PATH,
        ] {
            fs::write(dir.path().join(name), "x").unwrap();
        }
        clear_selector_handoff(dir.path()).unwrap();
        for name in [
            VENV_SELECTOR_OUTPUT,
            VENV_SELECTOR_SELECTION,
            VENV_SELECTOR_CUSTOM_PATH,
        ] {
            assert!(!dir.path().join(name).exists());
        }
    }
}
