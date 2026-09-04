use crate::config::{expand_tilde, Config, TerminalDockPosition};
use crate::resolve::resolve_executable;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

const FILE_MANAGER_PANE: &str = "file-manager";
const EDITOR_PANE: &str = "editor";
const TERMINAL_PANE: &str = "terminal";

/// Default TTL for pane cache in milliseconds
const PANE_CACHE_TTL_MS: u64 = 500;

/// Cached pane information with timestamp
struct CachedPanes {
    timestamp: Instant,
    panes: Vec<PaneInfo>,
    zellij_path: PathBuf,
}

/// Cache for Zellij pane information to avoid repeated list-panes calls
/// This cache is per-process and helps when list_panes is called multiple
/// times within a single action execution.
struct PaneCache {
    data: RwLock<Option<CachedPanes>>,
}

impl PaneCache {
    fn new() -> Self {
        Self {
            data: RwLock::new(None),
        }
    }

    /// Get panes from cache if fresh, otherwise fetch and cache
    fn get(&self, zellij: &Path) -> Result<Vec<PaneInfo>> {
        // Read cache
        {
            let guard = self.data.read().unwrap();
            if let Some(ref cached) = *guard {
                // If the Zellij path matches and cache is fresh, return cached data
                if cached.zellij_path == zellij
                    && cached.timestamp.elapsed() < Duration::from_millis(PANE_CACHE_TTL_MS)
                {
                    return Ok(cached.panes.clone());
                }
            }
        }

        // Fetch fresh data
        let panes = fetch_panes(zellij)?;

        // Update cache
        {
            let mut guard = self.data.write().unwrap();
            *guard = Some(CachedPanes {
                timestamp: Instant::now(),
                panes: panes.clone(),
                zellij_path: zellij.to_path_buf(),
            });
        }

        Ok(panes)
    }

    fn invalidate(&self) {
        *self.data.write().unwrap() = None;
    }
}

/// Global pane cache instance - one per process
static PANE_CACHE: OnceLock<PaneCache> = OnceLock::new();

/// Get the global pane cache, initializing if necessary
fn pane_cache() -> &'static PaneCache {
    PANE_CACHE.get_or_init(PaneCache::new)
}

/// Fetch panes directly from Zellij (uncached)
fn fetch_panes(zellij: &Path) -> Result<Vec<PaneInfo>> {
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

#[derive(Debug, Clone, Copy)]
pub enum GitAction {
    Open,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneInfo {
    pub id: u64,
    pub is_plugin: bool,
    pub is_floating: bool,
    pub is_fullscreen: bool,
    pub title: String,
    pub tab_id: u64,
    pub pane_columns: usize,
    pub pane_rows: usize,
}

impl PaneInfo {
    pub fn cli_id(&self) -> String {
        format!("terminal_{}", self.id)
    }
}

/// Whether the daemon can handle this file-manager action without a helper TTY.
///
/// Open/run paths rename the keybinding helper pane and run yazi in that process.
/// The background daemon must not execute those — it would exit the helper early
/// (`close_on_exit`) while yazi never attaches to the pane.
pub fn file_manager_ipc_capable(action: FileManagerAction, file_manager_exists: bool) -> bool {
    matches!(
        (action, file_manager_exists),
        (FileManagerAction::Open, true)
    )
}

pub fn file_manager_action(
    config: &Config,
    config_path: &Path,
    action: FileManagerAction,
    source_pane_id: Option<u64>,
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

    let current_id = source_pane_id
        .or_else(|| current_pane_id().ok())
        .context("ZELLIJ_PANE_ID is not set")?;
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
pub fn respawn_terminal(config: &Config, zellij: &Path, editor: &PaneInfo) -> Result<()> {
    let terminal = &config.terminal;
    if !terminal.enabled {
        bail!("Terminal is disabled in configuration");
    }

    focus_pane(zellij, editor)?;

    let mut command = Command::new(zellij);
    command.args([
        "action",
        "new-pane",
        "--direction",
        terminal.dock_position.zellij_new_pane_direction(),
        "--name",
        TERMINAL_PANE,
        "--near-current-pane",
    ]);
    if let Ok(cwd) = std::env::current_dir() {
        command.arg("--cwd").arg(cwd);
    }

    let output = command
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

    // `--name` on `new-pane` does not always stick; match layout.kdl naming explicitly.
    zellij_status(
        Command::new(zellij)
            .args(["action", "rename-pane", "--pane-id"])
            .arg(&pane_id)
            .arg(TERMINAL_PANE),
    )?;

    // Once Zellij ships `--size` on tiled `new-pane` (zellij-org/zellij#4735), prefer
    // that over post-create resize.
    let (axis, grow_border) = match terminal.dock_position {
        TerminalDockPosition::Down => (Axis::Rows, "up"),
        TerminalDockPosition::Right => (Axis::Columns, "left"),
    };
    resize_against_editor(
        zellij,
        &pane_id,
        TERMINAL_PANE,
        axis,
        grow_border,
        usize::from(terminal.dock_percent),
    )?;

    pane_cache().invalidate();
    Ok(())
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
            "right",
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
}

/// Share of the split (terminal + editor) taken by `pane`, as an integer percent.
fn pane_area_share(pane: &PaneInfo, editor: &PaneInfo, axis: Axis) -> Option<usize> {
    let total = axis.measure(pane) + axis.measure(editor);
    (total > 0).then_some(axis.measure(pane) * 100 / total)
}

fn resize_pane(zellij: &Path, pane_id: &str, resize: &str, direction: Option<&str>) -> Result<()> {
    let mut command = Command::new(zellij);
    command.args(["action", "resize", "--pane-id", pane_id, resize]);
    if let Some(direction) = direction {
        command.arg(direction);
    }
    zellij_status(&mut command)
}

/// Zellij resizes in fixed steps, so nudge the pane until it is close enough
/// to the requested share of the space it splits with the editor.
///
/// Uses coarse `+`/`-` (≈5% steps) when far from the target, then fine-grained
/// border resizes. Always reads fresh pane geometry — the pane cache must not
/// be used here or measurements go stale mid-loop.
fn resize_against_editor(
    zellij: &Path,
    pane_id: &str,
    subject: &str,
    axis: Axis,
    grow_border: &'static str,
    target: usize,
) -> Result<()> {
    const MAX_STEPS: usize = 100;
    const TOLERANCE_PERCENT: usize = 1;
    /// Switch to Zellij's coarse resize when more than this many points away.
    const COARSE_THRESHOLD: usize = 8;

    for _ in 0..MAX_STEPS {
        let panes = fetch_panes(zellij)?;
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

        let Some(current) = pane_area_share(pane, editor, axis) else {
            return Ok(());
        };
        let diff = current.abs_diff(target);
        if diff <= TOLERANCE_PERCENT {
            return Ok(());
        }

        let coarse = diff > COARSE_THRESHOLD;
        if coarse {
            let step = if current > target { "-" } else { "+" };
            resize_pane(zellij, pane_id, step, None)?;
        } else {
            let step = if current > target {
                "decrease"
            } else {
                "increase"
            };
            resize_pane(zellij, pane_id, step, Some(grow_border))?;
        }

        // Stop if a step overshoots the target (common with coarse `-`/`+`).
        let panes = fetch_panes(zellij)?;
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
        let Some(after) = pane_area_share(pane, editor, axis) else {
            return Ok(());
        };
        if after.abs_diff(target) <= TOLERANCE_PERCENT {
            return Ok(());
        }
        if after.abs_diff(target) >= diff && !coarse {
            let undo = if current > target {
                "increase"
            } else {
                "decrease"
            };
            let _ = resize_pane(zellij, pane_id, undo, Some(grow_border));
            return Ok(());
        }
    }

    Ok(())
}

/// Get the list of panes from Zellij, using cache if available
///
/// This function uses a per-process cache to avoid calling `zellij action list-panes`
/// multiple times within a single action execution. The cache has a TTL of 500ms
/// to ensure data freshness.
fn list_panes(zellij: &Path) -> Result<Vec<PaneInfo>> {
    pane_cache().get(zellij)
}

fn current_pane_id() -> Result<u64> {
    let value = std::env::var("ZELLIJ_PANE_ID").context("ZELLIJ_PANE_ID is not set")?;
    parse_zellij_pane_id(&value)
}

/// Parse Zellij's `ZELLIJ_PANE_ID` value (`terminal_N` or bare `N`).
pub fn parse_zellij_pane_id(value: &str) -> Result<u64> {
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

pub fn git_action(config: &Config, _action: GitAction) -> Result<()> {
    let zellij = resolve(&config.tools.zellij.command)?;
    let panes = list_panes(&zellij)?;
    let editor = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == EDITOR_PANE)
        .context("Cannot find the sat-hx-ide editor pane")?;

    // Resolve the git command
    let git_command = resolve_executable(&config.tools.git.command)
        .with_context(|| format!("Cannot find git client '{}'", config.tools.git.command))?;

    // Spawn a floating git client pane with the actual git command
    let output = Command::new(zellij)
        .args(["action", "new-pane", "--close-on-exit"])
        .args(["--name", "git", "--tab-id"])
        .arg(editor.tab_id.to_string())
        .arg("--floating")
        .args(["--x", "0%", "--y", "0%"])
        .arg("--width")
        .arg("100%")
        .arg("--height")
        .arg("100%")
        .arg("--")
        .arg(&git_command)
        .args(&config.tools.git.args)
        .output()
        .context("Failed to create git pane")?;

    if !output.status.success() {
        bail!(
            "Failed to create git pane: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
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

    #[test]
    fn file_manager_ipc_capable_only_for_focus() {
        assert!(file_manager_ipc_capable(FileManagerAction::Open, true));
        assert!(!file_manager_ipc_capable(FileManagerAction::Open, false));
        assert!(!file_manager_ipc_capable(FileManagerAction::Run, true));
        assert!(!file_manager_ipc_capable(
            FileManagerAction::ToggleDock,
            true
        ));
    }

    #[test]
    fn parse_zellij_pane_id_accepts_cli_form() {
        assert_eq!(parse_zellij_pane_id("terminal_4").unwrap(), 4);
        assert_eq!(parse_zellij_pane_id("42").unwrap(), 42);
    }

    #[test]
    fn parse_zellij_pane_id_rejects_invalid_values() {
        assert!(parse_zellij_pane_id("terminal_").is_err());
        assert!(parse_zellij_pane_id("not-a-number").is_err());
    }

    #[test]
    fn pane_cache_starts_empty() {
        let cache = PaneCache::new();
        // Verify cache starts empty
        assert!(cache.data.read().unwrap().is_none());
    }

    #[test]
    fn pane_cache_stores_data() {
        let cache = PaneCache::new();

        // Add some data
        {
            let mut guard = cache.data.write().unwrap();
            *guard = Some(CachedPanes {
                timestamp: Instant::now(),
                panes: vec![PaneInfo {
                    id: 1,
                    is_plugin: false,
                    is_floating: false,
                    is_fullscreen: false,
                    title: "editor".to_string(),
                    tab_id: 0,
                    pane_columns: 80,
                    pane_rows: 24,
                }],
                zellij_path: PathBuf::from("/usr/bin/zellij"),
            });
        }

        // Verify it's there
        assert!(cache.data.read().unwrap().is_some());
    }

    #[test]
    fn pane_area_share_percent() {
        let terminal = PaneInfo {
            id: 2,
            is_plugin: false,
            is_floating: false,
            is_fullscreen: false,
            title: TERMINAL_PANE.to_string(),
            tab_id: 0,
            pane_columns: 80,
            pane_rows: 15,
        };
        let editor = PaneInfo {
            id: 1,
            is_plugin: false,
            is_floating: false,
            is_fullscreen: false,
            title: EDITOR_PANE.to_string(),
            tab_id: 0,
            pane_columns: 80,
            pane_rows: 85,
        };
        assert_eq!(pane_area_share(&terminal, &editor, Axis::Rows), Some(15));
        assert_eq!(pane_area_share(&terminal, &editor, Axis::Columns), Some(50));
    }

    #[test]
    fn pane_cache_invalidate_clears_data() {
        let cache = PaneCache::new();
        {
            let mut guard = cache.data.write().unwrap();
            *guard = Some(CachedPanes {
                timestamp: Instant::now(),
                panes: vec![],
                zellij_path: PathBuf::from("/usr/bin/zellij"),
            });
        }
        cache.invalidate();
        assert!(cache.data.read().unwrap().is_none());
    }
}
