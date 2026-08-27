use crate::config::{expand_tilde, Config};
use crate::resolve::resolve_executable;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const FILE_MANAGER_PANE: &str = "file-manager";
const EDITOR_PANE: &str = "editor";
const TERMINAL_PANE: &str = "terminal";

#[derive(Debug, Clone, Copy)]
pub enum FileManagerAction {
    Open,
    ToggleDock,
    Run,
}

#[derive(Debug, Clone, Copy)]
pub enum TerminalAction {
    /// Hide the terminal behind a fullscreen editor, or bring it back.
    Toggle,
    /// Switch the terminal between its docked height and fullscreen.
    Zoom,
}

#[derive(Debug, Clone, Deserialize)]
struct PaneInfo {
    id: u64,
    is_plugin: bool,
    is_floating: bool,
    is_fullscreen: bool,
    title: String,
    tab_id: u64,
    pane_columns: usize,
    pane_rows: usize,
}

impl PaneInfo {
    fn cli_id(&self) -> String {
        format!("terminal_{}", self.id)
    }
}

pub fn file_manager_action(
    config: &Config,
    config_path: &Path,
    action: FileManagerAction,
) -> Result<()> {
    let zellij = resolve(&config.tools.zellij.command)?;
    let panes = list_panes(&zellij)?;
    let existing = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == FILE_MANAGER_PANE);

    match action {
        FileManagerAction::ToggleDock => {
            if let Some(pane) = existing {
                let editor = panes
                    .iter()
                    .find(|candidate| !candidate.is_plugin && candidate.title == EDITOR_PANE)
                    .context("Cannot find the sat-hx-ide editor pane")?;
                toggle_dock(config, config_path, &zellij, pane, editor)?;
                return Ok(());
            }
        }
        FileManagerAction::Open => {
            if let Some(pane) = existing {
                zellij_status(
                    Command::new(&zellij)
                        .args(["action", "focus-pane-id"])
                        .arg(pane.cli_id()),
                )?;
                return Ok(());
            }
        }
        FileManagerAction::Run => {}
    }

    let current_id = current_pane_id()?;
    let current = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.id == current_id);
    let editor = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == EDITOR_PANE)
        .context("Cannot find the sat-hx-ide editor pane")?;

    if matches!(action, FileManagerAction::ToggleDock) {
        return become_floating_manager(config, &zellij, current_id);
    }

    if action_is_request(action) && current.is_some_and(|pane| pane.tab_id != editor.tab_id) {
        spawn_on_code_tab(config, config_path, &zellij, editor.tab_id)?;
        return Ok(());
    }

    zellij_status(
        Command::new(&zellij)
            .args(["action", "rename-pane", "--pane-id"])
            .arg(format!("terminal_{current_id}"))
            .arg(FILE_MANAGER_PANE),
    )?;
    run_file_manager(config, &zellij, current_id)
}

fn action_is_request(action: FileManagerAction) -> bool {
    matches!(
        action,
        FileManagerAction::Open | FileManagerAction::ToggleDock
    )
}

fn spawn_on_code_tab(
    config: &Config,
    config_path: &Path,
    zellij: &Path,
    tab_id: u64,
) -> Result<()> {
    let pane_id = spawn_manager(config, config_path, zellij, tab_id, true)?;
    wait_for_pane_close(zellij, &pane_id)
}

fn spawn_manager(
    config: &Config,
    config_path: &Path,
    zellij: &Path,
    tab_id: u64,
    floating: bool,
) -> Result<String> {
    let executable = std::env::current_exe().context("Cannot locate sat-hx-ide executable")?;
    let mut command = Command::new(zellij);
    command
        .args(["action", "new-pane", "--close-on-exit"])
        .args(["--name", FILE_MANAGER_PANE, "--tab-id"])
        .arg(tab_id.to_string());
    if floating {
        command
            .arg("--floating")
            .args(["--x", "0%", "--y", "0%"])
            .arg("--width")
            .arg(&config.tools.file_manager.float_width)
            .arg("--height")
            .arg(&config.tools.file_manager.float_height);
    }
    let output = command
        .arg("--")
        .arg(executable)
        .arg("--config")
        .arg(expand_tilde(config_path))
        .args(["__file-manager", "run"])
        .output()
        .context("Failed to create file-manager pane")?;
    if !output.status.success() {
        bail!(
            "Failed to create file-manager pane: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let pane_id = String::from_utf8(output.stdout)
        .map(|id| id.trim().to_string())
        .context("Zellij returned a non-UTF-8 pane ID")?;
    wait_for_pane(zellij, &pane_id)?;
    Ok(pane_id)
}

fn run_file_manager(config: &Config, zellij: &Path, pane_id: u64) -> Result<()> {
    if config.tools.file_manager.adapter != "yazi" {
        bail!(
            "Unsupported file-manager adapter '{}'; currently supported: yazi",
            config.tools.file_manager.adapter
        );
    }
    let manager = resolve(&config.tools.file_manager.command)?;
    let session = std::env::var("ZELLIJ_SESSION_NAME").unwrap_or_else(|_| "session".to_string());
    let chooser = config
        .runtime_dir(&session)
        .join(format!("chooser-{pane_id}-{}.txt", std::process::id()));
    if let Some(parent) = chooser.parent() {
        fs::create_dir_all(parent)?;
    }

    loop {
        let _ = fs::remove_file(&chooser);
        let status = Command::new(&manager)
            .args(&config.tools.file_manager.args)
            .arg("--chooser-file")
            .arg(&chooser)
            .status()
            .with_context(|| format!("Failed to launch {}", manager.display()))?;
        if !status.success() {
            let _ = fs::remove_file(&chooser);
            bail!("File manager exited with status {status}");
        }

        let selections = read_selections(&chooser)?;
        let _ = fs::remove_file(&chooser);
        if selections.is_empty() {
            return Ok(());
        }
        open_in_editor(zellij, &selections[0])?;

        let pane = list_panes(zellij)?
            .into_iter()
            .find(|pane| !pane.is_plugin && pane.id == pane_id);
        if pane.is_none_or(|pane| pane.is_floating) {
            return Ok(());
        }
    }
}

fn become_floating_manager(config: &Config, zellij: &Path, pane_id: u64) -> Result<()> {
    let pane_id = format!("terminal_{pane_id}");
    zellij_status(
        Command::new(zellij)
            .args(["action", "change-floating-pane-coordinates", "--pane-id"])
            .arg(&pane_id)
            .args(["--x", "0%", "--y", "0%"])
            .arg("--width")
            .arg(&config.tools.file_manager.float_width)
            .arg("--height")
            .arg(&config.tools.file_manager.float_height),
    )?;
    zellij_status(
        Command::new(zellij)
            .args(["action", "rename-pane", "--pane-id"])
            .arg(&pane_id)
            .arg(FILE_MANAGER_PANE),
    )?;
    run_file_manager(
        config,
        zellij,
        pane_id.trim_start_matches("terminal_").parse()?,
    )
}

fn read_selections(path: &Path) -> Result<Vec<PathBuf>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect())
}

fn open_in_editor(zellij: &Path, selected: &Path) -> Result<()> {
    let editor = list_panes(zellij)?
        .into_iter()
        .find(|pane| !pane.is_plugin && pane.title == EDITOR_PANE)
        .context("Cannot find the sat-hx-ide editor pane")?;
    let path = helix_quote(selected)?;
    let pane_id = editor.cli_id();

    zellij_status(
        Command::new(zellij)
            .args(["action", "send-keys", "--pane-id"])
            .arg(&pane_id)
            .arg("Esc"),
    )?;
    zellij_status(
        Command::new(zellij)
            .args(["action", "write-chars", "--pane-id"])
            .arg(&pane_id)
            .arg(format!(":open {path}")),
    )?;
    zellij_status(
        Command::new(zellij)
            .args(["action", "send-keys", "--pane-id"])
            .arg(pane_id)
            .arg("Enter"),
    )
}

fn helix_quote(path: &Path) -> Result<String> {
    let path = path.to_string_lossy();
    if path.chars().any(char::is_control) {
        bail!("Selected path contains unsupported control characters");
    }
    Ok(format!(
        "\"{}\"",
        path.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

/// Both terminal actions are built on Zellij fullscreen, which hides the other
/// panes without touching the processes inside them.
pub fn terminal_action(config: &Config, action: TerminalAction) -> Result<()> {
    let zellij = resolve(&config.tools.zellij.command)?;
    let panes = list_panes(&zellij)?;
    let editor = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == EDITOR_PANE)
        .context("Cannot find the sat-hx-ide editor pane")?;
    let Some(terminal) = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == TERMINAL_PANE)
    else {
        return respawn_terminal(config, &zellij, editor);
    };

    let hidden = editor.is_fullscreen;
    let zoomed = terminal.is_fullscreen;

    match action {
        TerminalAction::Toggle if hidden => {
            toggle_fullscreen(&zellij, editor)?;
            focus_pane(&zellij, terminal)
        }
        TerminalAction::Toggle => {
            if zoomed {
                toggle_fullscreen(&zellij, terminal)?;
            }
            toggle_fullscreen(&zellij, editor)
        }
        TerminalAction::Zoom if zoomed => toggle_fullscreen(&zellij, terminal),
        TerminalAction::Zoom => {
            if hidden {
                toggle_fullscreen(&zellij, editor)?;
            }
            toggle_fullscreen(&zellij, terminal)
        }
    }
}

/// Dock a fresh shell when the previous one was exited by the user.
fn respawn_terminal(config: &Config, zellij: &Path, editor: &PaneInfo) -> Result<()> {
    let output = Command::new(zellij)
        .args(["action", "new-pane", "--name", TERMINAL_PANE, "--tab-id"])
        .arg(editor.tab_id.to_string())
        .output()
        .context("Failed to create the terminal pane")?;
    if !output.status.success() {
        bail!(
            "Failed to create the terminal pane: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let pane_id = String::from_utf8(output.stdout)
        .map(|id| id.trim().to_string())
        .context("Zellij returned a non-UTF-8 pane ID")?;
    wait_for_pane(zellij, &pane_id)?;

    zellij_status(
        Command::new(zellij)
            .args(["action", "move-pane", "--pane-id"])
            .arg(&pane_id)
            .arg("down"),
    )?;
    resize_against_editor(
        zellij,
        &pane_id,
        TERMINAL_PANE,
        Axis::Rows,
        usize::from(config.terminal.dock_percent),
    )
}

fn toggle_fullscreen(zellij: &Path, pane: &PaneInfo) -> Result<()> {
    zellij_status(
        Command::new(zellij)
            .args(["action", "toggle-fullscreen", "--pane-id"])
            .arg(pane.cli_id()),
    )
}

fn focus_pane(zellij: &Path, pane: &PaneInfo) -> Result<()> {
    zellij_status(
        Command::new(zellij)
            .args(["action", "focus-pane-id"])
            .arg(pane.cli_id()),
    )
}

fn toggle_dock(
    config: &Config,
    config_path: &Path,
    zellij: &Path,
    pane: &PaneInfo,
    editor: &PaneInfo,
) -> Result<()> {
    let was_floating = pane.is_floating;
    zellij_status(
        Command::new(zellij)
            .args(["action", "close-pane", "--pane-id"])
            .arg(pane.cli_id()),
    )?;

    if !was_floating {
        return become_floating_manager(config, zellij, current_pane_id()?);
    }

    zellij_status(
        Command::new(zellij)
            .args(["action", "focus-pane-id"])
            .arg(editor.cli_id()),
    )?;
    let pane_id = spawn_manager(config, config_path, zellij, editor.tab_id, !was_floating)?;

    if was_floating {
        zellij_status(
            Command::new(zellij)
                .args(["action", "move-pane", "--pane-id"])
                .arg(&pane_id)
                .arg("left"),
        )?;
        resize_against_editor(
            zellij,
            &pane_id,
            FILE_MANAGER_PANE,
            Axis::Columns,
            usize::from(config.tools.file_manager.dock_percent),
        )?;
    }

    zellij_status(
        Command::new(zellij)
            .args(["action", "focus-pane-id"])
            .arg(&pane_id),
    )?;
    wait_for_pane_close(zellij, &pane_id)
}

/// `new-pane` prints the new pane ID before Zellij registers it, so any action
/// targeting that ID has to wait for it to show up first.
fn wait_for_pane(zellij: &Path, pane_id: &str) -> Result<u64> {
    let id = parse_pane_id(pane_id)?;
    for _ in 0..40 {
        if pane_exists(zellij, id)? {
            return Ok(id);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    bail!("Pane {pane_id} did not appear")
}

fn wait_for_pane_close(zellij: &Path, pane_id: &str) -> Result<()> {
    let id = parse_pane_id(pane_id)?;
    while pane_exists(zellij, id)? {
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    Ok(())
}

fn parse_pane_id(pane_id: &str) -> Result<u64> {
    pane_id
        .trim_start_matches("terminal_")
        .parse()
        .with_context(|| format!("Invalid pane ID returned by Zellij: '{pane_id}'"))
}

fn pane_exists(zellij: &Path, id: u64) -> Result<bool> {
    Ok(list_panes(zellij)?
        .iter()
        .any(|pane| !pane.is_plugin && pane.id == id))
}

#[derive(Debug, Clone, Copy)]
enum Axis {
    Columns,
    Rows,
}

impl Axis {
    fn measure(self, pane: &PaneInfo) -> usize {
        match self {
            Axis::Columns => pane.pane_columns,
            Axis::Rows => pane.pane_rows,
        }
    }

    /// The border to push on to grow a pane docked away from the editor.
    fn grow_border(self) -> &'static str {
        match self {
            Axis::Columns => "right",
            Axis::Rows => "up",
        }
    }
}

/// Zellij resizes in fixed steps, so nudge the pane until it is close enough
/// to the requested share of the space it splits with the editor.
fn resize_against_editor(
    zellij: &Path,
    pane_id: &str,
    subject: &str,
    axis: Axis,
    target: usize,
) -> Result<()> {
    for _ in 0..20 {
        let panes = list_panes(zellij)?;
        let Some(pane) = panes
            .iter()
            .find(|pane| !pane.is_plugin && pane.title == subject)
        else {
            return Ok(());
        };
        let Some(editor) = panes
            .iter()
            .find(|pane| !pane.is_plugin && pane.title == EDITOR_PANE)
        else {
            return Ok(());
        };
        let total = axis.measure(pane) + axis.measure(editor);
        if total == 0 {
            return Ok(());
        }
        let current = axis.measure(pane) * 100 / total;
        if current.abs_diff(target) <= 3 {
            return Ok(());
        }
        let resize = if current > target {
            "decrease"
        } else {
            "increase"
        };
        zellij_status(Command::new(zellij).args([
            "action",
            "resize",
            "--pane-id",
            pane_id,
            resize,
            axis.grow_border(),
        ]))?;
    }
    Ok(())
}

fn list_panes(zellij: &Path) -> Result<Vec<PaneInfo>> {
    let output = Command::new(zellij)
        .args(["action", "list-panes", "--json", "--all"])
        .output()
        .context("Failed to query Zellij panes")?;
    if !output.status.success() {
        bail!(
            "Failed to query Zellij panes: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    serde_json::from_slice(&output.stdout).context("Invalid pane JSON returned by Zellij")
}

fn current_pane_id() -> Result<u64> {
    let value = std::env::var("ZELLIJ_PANE_ID").context("ZELLIJ_PANE_ID is not set")?;
    value
        .trim_start_matches("terminal_")
        .parse()
        .with_context(|| format!("Invalid ZELLIJ_PANE_ID '{value}'"))
}

fn resolve(command: &str) -> Result<PathBuf> {
    resolve_executable(command)
}

fn zellij_status(command: &mut Command) -> Result<()> {
    let display = format!("{command:?}");
    let status: ExitStatus = command
        .status()
        .with_context(|| format!("Failed to run {display}"))?;
    if status.success() {
        Ok(())
    } else {
        bail!("{display} exited with status {status}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_helix_paths() {
        assert_eq!(
            helix_quote(Path::new("/tmp/a \"quoted\" file")).unwrap(),
            "\"/tmp/a \\\"quoted\\\" file\""
        );
    }

    #[test]
    fn parses_pane_json() {
        let panes: Vec<PaneInfo> = serde_json::from_str(
            r#"[{
                "id": 4,
                "is_plugin": false,
                "is_floating": true,
                "is_fullscreen": false,
                "title": "file-manager",
                "tab_id": 0,
                "pane_columns": 80,
                "pane_rows": 24
            }]"#,
        )
        .unwrap();
        assert_eq!(panes[0].cli_id(), "terminal_4");
        assert!(panes[0].is_floating);
    }
}
