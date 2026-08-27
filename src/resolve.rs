use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve a configured tool name to an absolute executable path.
///
/// Zellij panes do not load shell aliases, so a bare name like `hx` must be
/// resolved here before it is written into a layout or keybinding.
pub fn resolve_executable(command: &str) -> Result<PathBuf> {
    let path = Path::new(command);
    if path.is_absolute() || command.contains('/') {
        return path
            .canonicalize()
            .with_context(|| format!("Cannot find executable '{command}'"));
    }

    if let Ok(found) = which::which(command) {
        return Ok(found);
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

    Path::new(&resolved)
        .canonicalize()
        .map(Some)
        .with_context(|| format!("Login shell resolved '{command}' to '{resolved}', but that path is unusable"))
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
}
