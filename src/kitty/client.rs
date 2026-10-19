//! `kitten @` remote-control wrapper (the analog of the Zellij CLI plumbing).
//!
//! All commands go through the session's Unix socket. Window identity uses
//! `SAT_HX_ROLE` env (set at launch, immutable) with a `SAT_ROLE` user var
//! (set by renames) taking precedence; `SAT_FLOAT_WINDOW=1` marks overlay
//! windows, which `kitten @ ls` does not distinguish otherwise.

use crate::backend::{PaneInfo, SessionStatus};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Environment variable kitty propagates to every window via kitty.conf,
/// used by helper processes to find their session's runtime state.
pub const SESSION_ENV: &str = "SAT_SESSION_NAME";
/// Environment variable set at launch on windows that sat-hx-ide treats as
/// floating (overlay) windows.
pub const FLOAT_ENV: &str = "SAT_FLOAT_WINDOW";
/// Role user variable set when a window is renamed.
pub const ROLE_VAR: &str = "SAT_ROLE";
/// Role environment variable set at launch.
pub const ROLE_ENV: &str = "SAT_HX_ROLE";

pub struct KittyClient {
    kitten: PathBuf,
    socket: PathBuf,
}

impl KittyClient {
    pub fn new(kitten: PathBuf, socket: PathBuf) -> Self {
        Self { kitten, socket }
    }

    /// Fresh client for the given session's socket.
    pub fn for_session(kitten: PathBuf, runtime_dir: &Path) -> Self {
        Self::new(kitten, runtime_dir.join("kitty.sock"))
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// `kitten @ --to=<socket> <args>`.
    fn base_command(&self) -> Command {
        let mut command = Command::new(&self.kitten);
        command
            .arg("@")
            .arg("--to")
            .arg(format!("unix:{}", self.socket.display()));
        command
    }

    pub fn run(&self, args: &[&str]) -> Result<Output> {
        let mut command = self.base_command();
        command.args(args);
        let display = format!("{command:?}");
        command
            .output()
            .with_context(|| format!("Failed to run {display}"))
    }

    /// Run a command and require a success exit status.
    pub fn run_ok(&self, args: &[&str]) -> Result<()> {
        let output = self.run(args)?;
        if output.status.success() {
            Ok(())
        } else {
            bail!(
                "kitten @ {} failed: {}",
                args.first().unwrap_or(&""),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
    }

    /// `run_ok` with arbitrary arguments (launch commands carry user paths).
    pub fn run_ok_args(&self, args: &[OsString]) -> Result<()> {
        let display = format!("kitten @ {:?}", args.first());
        let output = self.run_args(args)?;
        if output.status.success() {
            Ok(())
        } else {
            bail!(
                "{display} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
    }

    /// Run a command and return its stdout.
    pub fn run_output(&self, args: &[&str]) -> Result<String> {
        let output = self.run(args)?;
        if !output.status.success() {
            bail!(
                "kitten @ {} failed: {}",
                args.first().unwrap_or(&""),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// `run_output` with arbitrary arguments.
    pub fn run_output_args(&self, args: &[OsString]) -> Result<String> {
        let display = format!("kitten @ {:?}", args.first());
        let output = self.run_args(args)?;
        if !output.status.success() {
            bail!(
                "{display} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn run_args(&self, args: &[OsString]) -> Result<Output> {
        let mut command = self.base_command();
        command.args(args);
        let display = format!("{command:?}");
        command
            .output()
            .with_context(|| format!("Failed to run {display}"))
    }

    /// Whether the session's kitty instance answers on its socket.
    pub fn probe(&self) -> SessionStatus {
        match self.run(&["ls"]) {
            Ok(output) if output.status.success() => SessionStatus::Active,
            _ => SessionStatus::NotFound,
        }
    }

    pub fn ls(&self) -> Result<Vec<OsWindow>> {
        let stdout = self.run_output(&["ls"])?;
        serde_json::from_str(&stdout).context("Invalid JSON returned by kitten @ ls")
    }

    /// Map `kitten @ ls` output onto the backend-neutral pane list.
    ///
    /// `zoomed_role` is the role the backend recorded when it entered the
    /// stack layout; the fullscreen flag derives from layout + that record
    /// instead of window focus, because map-launched helper overlays steal
    /// focus (and even the active-window history), which would defeat any
    /// focus-based test.
    pub fn panes(&self, zoomed_role: Option<&str>) -> Result<Vec<PaneInfo>> {
        map_panes(&self.ls()?, zoomed_role)
    }
}

/// Map `kitten @ ls` output onto the backend-neutral pane list.
///
/// `title` resolves in favor of the rename user var, then the launch role
/// env, then the window title. `is_fullscreen` derives from the stack
/// layout (kitty's fullscreen substitute: only the focused window of a
/// stacked tab is visible).
fn map_panes(os_windows: &[OsWindow], zoomed_role: Option<&str>) -> Result<Vec<PaneInfo>> {
    let mut panes = Vec::new();
    for os_window in os_windows {
        for tab in &os_window.tabs {
            for window in &tab.windows {
                let role = window
                    .user_vars
                    .get(ROLE_VAR)
                    .or_else(|| window.env.get(ROLE_ENV))
                    .unwrap_or(&window.title)
                    .clone();
                let zoomed = tab.layout == "stack" && zoomed_role == Some(role.as_str());
                panes.push(PaneInfo {
                    id: window.id,
                    is_plugin: false,
                    is_floating: window.env.get(FLOAT_ENV).map(String::as_str) == Some("1"),
                    is_fullscreen: zoomed,
                    title: role,
                    tab_id: tab.id,
                    pane_columns: window.columns,
                    pane_rows: window.lines,
                });
            }
        }
    }
    Ok(panes)
}

#[derive(Debug, Deserialize)]
pub struct OsWindow {
    #[serde(default)]
    pub tabs: Vec<Tab>,
}

#[derive(Debug, Deserialize)]
pub struct Tab {
    pub id: u64,
    #[serde(default)]
    pub layout: String,
    #[serde(default)]
    pub windows: Vec<Window>,
}

#[derive(Debug, Deserialize)]
pub struct Window {
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub is_focused: bool,
    #[serde(default)]
    pub columns: usize,
    #[serde(default)]
    pub lines: usize,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub user_vars: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const LS_JSON: &str = r#"[{
        "id": 1,
        "tabs": [{
            "id": 1,
            "layout": "splits",
            "windows": [{
                "id": 10,
                "title": "hx",
                "is_focused": true,
                "columns": 120,
                "lines": 40,
                "env": {"SAT_HX_ROLE": "editor", "SAT_FLOAT_WINDOW": "1"},
                "user_vars": {}
            }, {
                "id": 11,
                "title": "sh",
                "is_focused": false,
                "columns": 120,
                "lines": 6,
                "env": {"SAT_HX_ROLE": "terminal"},
                "user_vars": {"SAT_ROLE": "terminal"}
            }]
        }, {
            "id": 2,
            "layout": "stack",
            "windows": [{
                "id": 12,
                "title": "yazi",
                "is_focused": true,
                "columns": 120,
                "lines": 47,
                "env": {"SAT_HX_ROLE": "sat-helper", "SAT_FLOAT_WINDOW": "1"},
                "user_vars": {"SAT_ROLE": "file-manager"}
            }]
        }]
    }]"#;

    #[test]
    fn maps_ls_json_to_pane_info() {
        let windows: Vec<OsWindow> = serde_json::from_str(LS_JSON).unwrap();
        let panes = map_panes(&windows, Some("file-manager")).unwrap();
        let editor = panes.iter().find(|p| p.id == 10).unwrap();
        assert_eq!(editor.title, "editor");
        assert!(editor.is_floating);
        assert!(!editor.is_fullscreen);
        let terminal = panes.iter().find(|p| p.id == 11).unwrap();
        assert_eq!(terminal.title, "terminal");
        let yazi = panes.iter().find(|p| p.id == 12).unwrap();
        assert_eq!(yazi.title, "file-manager");
        assert!(yazi.is_floating);
        // The stacked-tab window reports fullscreen only when its role is
        // the recorded zoomed role (tab 2 is stack in the fixture).
        assert!(yazi.is_fullscreen);
    }

    #[test]
    fn fullscreen_requires_the_recorded_zoom_role() {
        let windows: Vec<OsWindow> = serde_json::from_str(LS_JSON).unwrap();
        // Same ls fixture but zoomed role recorded as the terminal: the
        // file-manager window in the stack tab is NOT fullscreen, and the
        // terminal in the splits tab is not either (wrong layout).
        let panes = map_panes(&windows, Some("terminal")).unwrap();
        let yazi = panes.iter().find(|p| p.id == 12).unwrap();
        assert!(!yazi.is_fullscreen);
        let terminal = panes.iter().find(|p| p.id == 11).unwrap();
        assert!(!terminal.is_fullscreen);
        // No recorded role: nothing reports fullscreen, regardless of focus.
        let panes = map_panes(&windows, None).unwrap();
        let yazi = panes.iter().find(|p| p.id == 12).unwrap();
        assert!(!yazi.is_fullscreen);
    }
}
