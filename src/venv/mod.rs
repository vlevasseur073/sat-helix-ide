//! Virtual environment management module
//!
//! This module provides functionality for detecting and activating virtual environments
//! in Python projects.

mod ui;

use crate::config::{expand_tilde, Config, VenvConfig};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
pub use ui::{run_selector_ui, SelectorRun};

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
            if uv_venv.exists() && uv_venv.join("bin").join("activate").exists() {
                return Ok(Some(DetectedVenv {
                    path: uv_venv,
                    venv_type: "venv".to_string(),
                }));
            }
        }

        // Check for poetry
        if content.contains("[tool.poetry]") {
            return Ok(Some(DetectedVenv {
                path: dir.to_path_buf(),
                venv_type: "poetry".to_string(),
            }));
        }
    }

    // .python-version alone is not enough without a local .venv

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
        "uv" | "venv" | "auto" => activate_via_script(path, &shell),
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
        _ => activate_via_script(path, &shell),
    }
}

fn activate_via_script(path: Option<&Path>, shell: &str) -> Result<(String, String)> {
    let path = path.context("Path required for venv activation")?;
    let venv_root = resolve_activate_root(path)?;
    let activate_path = venv_root.join("bin").join("activate");
    let activate = format!(
        "source {}",
        shell_quote(activate_path.display().to_string())
    );
    let deactivate = if shell == "fish" {
        "conda deactivate 2>/dev/null; deactivate 2>/dev/null; set -gx VIRTUAL_ENV".to_string()
    } else {
        "deactivate 2>/dev/null || true".to_string()
    };
    Ok((activate, deactivate))
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

    // Check for pyproject.toml with uv — only listable when .venv exists (handled above)
    let pyproject = dir.join("pyproject.toml");
    if pyproject.exists() {
        if let Ok(content) = fs::read_to_string(&pyproject) {
            if content.contains("[tool.poetry]") {
                results.push(SelectableVenv {
                    name: "poetry (pyproject.toml)".to_string(),
                    path: dir.to_path_buf(),
                    venv_type: "poetry".to_string(),
                });
            }
        }
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
        if let Ok(normalized) = normalize_selectable_venv(SelectableVenv {
            name: format!("Configured: {}", resolved_path.display()),
            path: resolved_path,
            venv_type,
        }) {
            all_environments.push(normalized);
        }
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

    Ok(filter_activatable_environments(unique_environments))
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

        Ok(filter_activatable_environments(all_environments))
    } else {
        // No saved collection yet, just return detected environments
        list_all_selectable_venvs(venv_config)
    }
}

/// Path to the environment currently activated in the session, if any.
pub fn active_venv_path(config: &Config, session_name: &str) -> Option<PathBuf> {
    let runtime_dir = config.runtime_dir(session_name);
    if !runtime_dir.join("venv_active").exists() {
        return None;
    }
    load_venv_selection(session_name)
        .ok()
        .flatten()
        .map(|s| expand_tilde(&s.path))
}

/// Whether two paths refer to the same environment (best-effort).
pub fn same_venv_path(a: &Path, b: &Path) -> bool {
    let a = expand_tilde(a);
    let b = expand_tilde(b);
    if a == b {
        return true;
    }
    if let (Ok(a_canon), Ok(b_canon)) = (a.canonicalize(), b.canonicalize()) {
        return a_canon == b_canon;
    }
    false
}

pub fn filter_activatable_environments(environments: Vec<SelectableVenv>) -> Vec<SelectableVenv> {
    let mut out: Vec<SelectableVenv> = Vec::new();
    for env in environments {
        if let Ok(normalized) = normalize_selectable_venv(env) {
            if !out
                .iter()
                .any(|e: &SelectableVenv| same_venv_path(&e.path, &normalized.path))
            {
                out.push(normalized);
            }
        }
    }
    out
}

/// Transient handoff files used only while the selector TUI is open (not persisted).
pub const VENV_SELECTOR_OUTPUT: &str = "venv_selector_output";
pub const VENV_SELECTOR_SELECTION: &str = "venv_selector_selection.json";
pub const VENV_SELECTOR_CUSTOM_PATH: &str = "venv_selector_custom_path";

/// Marker written to [`VENV_SELECTOR_OUTPUT`] when the user made a selection.
pub const VENV_SELECTOR_DONE: &str = "OK";

/// Whether the stored selection can be activated as-is.
pub fn is_selection_activatable(env: &SelectableVenv) -> bool {
    normalize_selectable_venv(env.clone()).is_ok()
}

/// Normalize paths and types so activation uses `source …/bin/activate` where possible.
pub fn normalize_selectable_venv(env: SelectableVenv) -> Result<SelectableVenv> {
    validate_selection_activatable(&env)?;
    match env.venv_type.as_str() {
        "poetry" | "conda" | "pipenv" => Ok(env),
        _ => {
            let root = resolve_activate_root(&env.path)?;
            Ok(SelectableVenv {
                name: env.name,
                path: root,
                venv_type: "venv".to_string(),
            })
        }
    }
}

/// Verify that a saved selection path and type match what `generate_commands` expects.
pub fn validate_selection_activatable(env: &SelectableVenv) -> Result<()> {
    let path = expand_tilde(&env.path);
    match env.venv_type.as_str() {
        "venv" | "auto" | "uv" => resolve_activate_root(&path).map(|_| ()),
        "poetry" => validate_poetry(&path),
        "conda" => validate_conda(&path),
        "pipenv" => validate_pipenv(&path),
        _ => resolve_activate_root(&path).map(|_| ()),
    }
}

/// Strict validation for a user-typed custom path (used by the selector TUI).
pub fn validate_custom_venv_path(path: &Path) -> Result<()> {
    resolve_custom_venv_path(path).map(|_| ())
}

/// Directory that contains `bin/activate` (the path itself or a `.venv` child).
pub fn resolve_activate_root(path: &Path) -> Result<PathBuf> {
    let path = expand_tilde(path);
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }
    if activate_script_path(&path).is_some() {
        return Ok(path);
    }
    let nested = path.join(".venv");
    if activate_script_path(&nested).is_some() {
        return Ok(nested);
    }
    bail!(
        "No bin/activate at {} or {}/.venv",
        path.display(),
        path.display()
    );
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

    let nested = path.join(".venv");
    if nested.is_dir() && activate_script_path(&nested).is_some() {
        let venv_type = detect_venv_type(&nested).unwrap_or_else(|_| "venv".to_string());
        return Ok(custom_venv_entry(nested, venv_type));
    }

    if validate_poetry(&path).is_ok() {
        return Ok(custom_venv_entry(path, "poetry".to_string()));
    }
    if validate_conda(&path).is_ok() {
        return Ok(custom_venv_entry(path, "conda".to_string()));
    }
    if validate_pipenv(&path).is_ok() {
        return Ok(custom_venv_entry(path, "pipenv".to_string()));
    }

    if path.is_dir() && uv_project_markers(&path) {
        bail!(
            "No .venv at {}. Run `uv sync` or enter the .venv directory.",
            path.display()
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
        return normalize_selectable_venv(selection).map(Some);
    }

    let custom_path_file = runtime_dir.join(VENV_SELECTOR_CUSTOM_PATH);
    if custom_path_file.exists() {
        let path_line = fs::read_to_string(&custom_path_file)?;
        let _ = fs::remove_file(&custom_path_file);
        let path = PathBuf::from(path_line.trim());
        if path.as_os_str().is_empty() {
            bail!("Custom virtual environment path is empty");
        }
        return resolve_custom_venv_path(&path)
            .and_then(normalize_selectable_venv)
            .map(Some);
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

/// Load environments and run the selector TUI in the session runtime directory.
pub fn run_selector_for_session(config: &Config, session: &str) -> Result<SelectorRun> {
    let environments = get_all_available_venvs(&config.venv, session)?;
    let active_path = active_venv_path(config, session);
    let runtime_dir = config.runtime_dir(session);
    fs::create_dir_all(&runtime_dir)?;
    clear_selector_handoff(&runtime_dir)?;
    run_selector_ui(&runtime_dir, environments, active_path.as_deref())
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
    fn resolve_custom_path_accepts_project_root_with_dot_venv() {
        let project = tempfile::tempdir().unwrap();
        let dot_venv = project.path().join(".venv");
        let bin = dot_venv.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("activate"), "").unwrap();

        let from_root = resolve_custom_venv_path(project.path()).unwrap();
        assert_eq!(from_root.path, dot_venv);
        assert!(resolve_custom_venv_path(&dot_venv).is_ok());
    }

    #[test]
    fn generate_commands_uv_project_uses_activate_script() {
        let project = tempfile::tempdir().unwrap();
        let dot_venv = project.path().join(".venv");
        let bin = dot_venv.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("activate"), "").unwrap();
        fs::write(project.path().join("pyproject.toml"), "[tool.uv]\n").unwrap();

        let (activate, _) =
            generate_commands("uv", Some(project.path())).expect("activate command");
        assert!(activate.contains("bin/activate"));
        assert!(!activate.contains("uv shell"));
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
    fn normalize_maps_project_root_auto_to_dot_venv() {
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
        let normalized = normalize_selectable_venv(env).unwrap();
        assert_eq!(normalized.path, dot_venv);
        assert_eq!(normalized.venv_type, "venv");
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
