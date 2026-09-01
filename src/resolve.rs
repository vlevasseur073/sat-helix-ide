use anyhow::{bail, Context, Result};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// Resolve a configured tool name to an absolute executable path.
///
/// Zellij panes do not load shell aliases, so a bare name like `hx` must be
/// resolved here before it is written into a layout or keybinding.
pub fn resolve_executable(command: &str) -> Result<PathBuf> {
    let path = Path::new(command);
    if path.is_absolute() || command.contains('/') {
        return normalize_executable_path(path);
    }

    if let Ok(found) = which::which(command) {
        return normalize_executable_path(&found);
    }

    if let Some(found) = resolve_via_login_shell(command)? {
        return Ok(found);
    }

    bail!(
        "Cannot find executable '{command}'. If it is a shell alias, set an \
         absolute path in your sat-hx-ide config."
    )
}

fn resolve_via_login_shell(command: &str) -> Result<Option<PathBuf>> {
    if !is_safe_command_name(command) {
        return Ok(None);
    }

    let output = Command::new("sh")
        .args(["-lc", &format!("command -v {command}")])
        .output()
        .context("Failed to query the login shell for an executable path")?;
    if !output.status.success() {
        return Ok(None);
    }

    let resolved = String::from_utf8(output.stdout)
        .context("Login shell returned non-UTF-8 output")?
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    if resolved.is_empty() {
        return Ok(None);
    }

    normalize_executable_path(Path::new(&resolved))
        .map(Some)
        .with_context(|| {
            format!("Login shell resolved '{command}' to '{resolved}', but that path is unusable")
        })
}

/// Normalize a path to an absolute, usable executable location.
///
/// Snap apps are exposed as symlinks under `/snap/bin/` that ultimately point
/// at `/usr/bin/snap`. Canonicalizing those wrappers strips the app identity
/// snap needs, so they are returned unchanged after an existence check.
fn normalize_executable_path(path: &Path) -> Result<PathBuf> {
    if is_snap_wrapper(path) {
        return path
            .exists()
            .then(|| path.to_path_buf())
            .with_context(|| format!("Cannot find executable '{}'", path.display()));
    }

    let needs_canonicalize = !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir));

    if needs_canonicalize {
        return path
            .canonicalize()
            .with_context(|| format!("Cannot find executable '{}'", path.display()));
    }

    path.exists()
        .then(|| path.to_path_buf())
        .with_context(|| format!("Cannot find executable '{}'", path.display()))
}

fn is_snap_wrapper(path: &Path) -> bool {
    path.starts_with("/snap/bin")
}

fn is_safe_command_name(command: &str) -> bool {
    !command.is_empty()
        && command
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_absolute_paths() {
        let path = resolve_executable("/bin/sh").unwrap();
        assert!(path.is_absolute());
    }

    #[test]
    fn rejects_unsafe_shell_lookup_names() {
        assert!(resolve_via_login_shell("hx; rm -rf /").unwrap().is_none());
    }

    #[test]
    fn recognizes_snap_wrappers() {
        assert!(is_snap_wrapper(Path::new("/snap/bin/hx")));
        assert!(is_snap_wrapper(Path::new("/snap/bin/helix")));
        assert!(!is_snap_wrapper(Path::new("/snap/helix/current/bin/hx")));
        assert!(!is_snap_wrapper(Path::new("/usr/bin/hx")));
    }

    #[test]
    fn preserves_snap_wrapper_paths_without_canonicalizing() {
        let path = Path::new("/snap/bin/hx");
        if !path.exists() {
            return;
        }

        let normalized = normalize_executable_path(path).unwrap();
        assert_eq!(normalized, path);
    }
}
