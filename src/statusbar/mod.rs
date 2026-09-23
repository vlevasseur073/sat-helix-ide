//! Session status line: Zellij session name, git branch/state, and active venv.

use crate::config::{expand_tilde, Config};
use crate::resolve::resolve_git_cli;
use crate::venv::load_venv_selection;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusBarSnapshot {
    pub session_name: String,
    pub branch: String,
    pub git_state: GitState,
    pub venv_label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitState {
    Clean,
    Modified,
    Staged,
    Untracked,
    NoRepo,
}

impl GitState {
    fn label(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Modified => "modified",
            Self::Staged => "staged",
            Self::Untracked => "untracked",
            Self::NoRepo => "no git",
        }
    }
}

/// Collect git and venv information for the status line.
pub fn collect_status(
    config: &Config,
    project_dir: &Path,
    session_name: &str,
) -> Result<StatusBarSnapshot> {
    let project_dir = expand_tilde(project_dir);
    let git = resolve_git_cli()?;
    let (branch, git_state) = git_summary(&git, &project_dir)?;
    let venv_label = active_venv_label(config, session_name);
    Ok(StatusBarSnapshot {
        session_name: session_name.to_string(),
        branch,
        git_state,
        venv_label,
    })
}

fn git_summary(git: &Path, project_dir: &Path) -> Result<(String, GitState)> {
    let inside = Command::new(git)
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(project_dir)
        .output()
        .context("Failed to run git rev-parse --is-inside-work-tree")?;
    if !inside.status.success() || String::from_utf8_lossy(&inside.stdout).trim() != "true" {
        return Ok(("—".to_string(), GitState::NoRepo));
    }

    let branch = Command::new(git)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(project_dir)
        .output()
        .context("Failed to run git rev-parse")?;
    if !branch.status.success() {
        return Ok(("—".to_string(), GitState::NoRepo));
    }
    let branch = String::from_utf8_lossy(&branch.stdout).trim().to_string();

    let status = Command::new(git)
        .args(["status", "--porcelain"])
        .current_dir(project_dir)
        .output()
        .context("Failed to run git status")?;
    if !status.status.success() {
        return Ok((branch, GitState::NoRepo));
    }

    let porcelain = String::from_utf8_lossy(&status.stdout);
    Ok((branch, classify_porcelain(&porcelain)))
}

fn classify_porcelain(porcelain: &str) -> GitState {
    let lines: Vec<&str> = porcelain
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.is_empty() {
        return GitState::Clean;
    }

    let mut has_staged = false;
    let mut has_unstaged = false;
    let mut only_untracked = true;

    for line in lines {
        if line.starts_with("??") {
            continue;
        }
        only_untracked = false;
        let index = line.chars().next().unwrap_or(' ');
        let worktree = line.chars().nth(1).unwrap_or(' ');
        if index != ' ' && index != '?' {
            has_staged = true;
        }
        if worktree != ' ' && worktree != '?' {
            has_unstaged = true;
        }
    }

    if only_untracked {
        return GitState::Untracked;
    }
    if has_staged && !has_unstaged {
        return GitState::Staged;
    }
    GitState::Modified
}

fn active_venv_label(config: &Config, session_name: &str) -> Option<String> {
    let runtime_dir = config.runtime_dir(session_name);
    if !runtime_dir.join("venv_active").exists() {
        return None;
    }
    if let Ok(Some(selection)) = load_venv_selection(session_name) {
        let path = expand_tilde(&selection.path);
        return Some(short_venv_name(&selection.name, &path));
    }
    Some("active".to_string())
}

fn short_venv_name(display_name: &str, path: &Path) -> String {
    if let Some(stripped) = display_name.strip_prefix("Custom: ") {
        return stripped.to_string();
    }
    if let Some(stripped) = display_name.strip_prefix("Configured: ") {
        return stripped.to_string();
    }
    if path.file_name().and_then(|n| n.to_str()) == Some(".venv") {
        if let Some(parent) = path.parent().and_then(|p| p.file_name()) {
            return format!("{}/.venv", parent.to_string_lossy());
        }
    }
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Render a single status line with ANSI styling (Catppuccin-inspired).
pub fn format_status_line(snapshot: &StatusBarSnapshot) -> String {
    const RESET: &str = "\x1b[0m";
    const BAR: &str = "\x1b[38;2;49;50;68m"; // surface1
    const LABEL: &str = "\x1b[38;2;166;173;200m"; // subtext
    const BRANCH: &str = "\x1b[1;38;2;137;180;250m"; // blue
    const CLEAN: &str = "\x1b[38;2;166;227;161m"; // green
    const MODIFIED: &str = "\x1b[38;2;250;179;135m"; // peach
    const STAGED: &str = "\x1b[38;2;148;226;213m"; // teal
    const UNTRACKED: &str = "\x1b[38;2;243;139;168m"; // red
    const NOGIT: &str = "\x1b[38;2;108;112;134m"; // overlay
    const SESSION: &str = "\x1b[38;2;203;166;247m"; // mauve
    const VENV: &str = "\x1b[38;2;180;190;254m"; // lavender
    const VENV_OFF: &str = "\x1b[38;2;108;112;134m";

    let git_color = match snapshot.git_state {
        GitState::Clean => CLEAN,
        GitState::Modified => MODIFIED,
        GitState::Staged => STAGED,
        GitState::Untracked => UNTRACKED,
        GitState::NoRepo => NOGIT,
    };

    let venv_part = match &snapshot.venv_label {
        Some(name) => format!("{VENV}{name}{RESET}"),
        None => format!("{VENV_OFF}none{RESET}"),
    };

    format!(
        "{BAR} {LABEL}session{RESET} {SESSION}{}{RESET} {BAR}│{RESET} {LABEL}git{RESET} {BRANCH}{}{RESET} {BAR}│{RESET} {LABEL}status{RESET} {git_color}{}{RESET} {BAR}│{RESET} {LABEL}venv{RESET} {venv_part}",
        snapshot.session_name,
        snapshot.branch,
        snapshot.git_state.label(),
    )
}

/// Print one formatted line (for tests and one-shot use).
pub fn print_status_line(config: &Config, project_dir: &Path, session_name: &str) -> Result<()> {
    let snapshot = collect_status(config, project_dir, session_name)?;
    println!("{}", format_status_line(&snapshot));
    Ok(())
}

/// Refresh the status line in the current terminal (used by the layout pane).
pub fn run_status_bar_loop(config: &Config, project_dir: &Path, session_name: &str) -> Result<()> {
    loop {
        let snapshot = collect_status(config, project_dir, session_name)?;
        let line = format_status_line(&snapshot);
        print!("\x1b[2K\r{line}");
        std::io::stdout().flush()?;
        thread::sleep(REFRESH_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_clean_tree() {
        assert_eq!(classify_porcelain(""), GitState::Clean);
    }

    #[test]
    fn classify_modified_and_untracked() {
        assert_eq!(classify_porcelain(" M file\n"), GitState::Modified);
        assert_eq!(classify_porcelain("?? new\n"), GitState::Untracked);
        assert_eq!(classify_porcelain("A  staged\n"), GitState::Staged);
    }

    #[test]
    fn format_includes_session_before_git() {
        let line = format_status_line(&StatusBarSnapshot {
            session_name: "my-project".to_string(),
            branch: "main".to_string(),
            git_state: GitState::Clean,
            venv_label: None,
        });
        assert!(line.contains("session"));
        assert!(line.contains("my-project"));
        let session_pos = line.find("my-project").unwrap();
        let git_pos = line.find("git").unwrap();
        assert!(session_pos < git_pos);
    }
}
