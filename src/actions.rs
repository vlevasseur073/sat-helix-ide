use crate::backend::{
    self, Axis, Direction, FloatGeometry, Key, PaneId, PaneInfo, Placement, SessionBackend,
    SpawnSpec,
};
use crate::config::{expand_tilde, CommandConfig, Config, DockPosition};
use crate::resolve::resolve_executable;
use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

const FILE_MANAGER_PANE: &str = "file-manager";
const EDITOR_PANE: &str = "editor";
const TERMINAL_PANE: &str = "terminal";
const MIND_MAP_PANE: &str = "mindmapping";

/// Path to the state file indicating Yazi should use docked (single-panel) mode
fn yazi_docked_state_file(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join("yazi-docked.state")
}

/// Path to the docked Yazi configuration directory
fn docked_yazi_config_dir(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join("yazi-docked")
}

/// User's Yazi configuration directory
fn user_yazi_config_dir() -> Option<PathBuf> {
    std::env::var_os("YAZI_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            home::home_dir().and_then(|home| {
                let path = home.join(".config").join("yazi");
                if path.exists() {
                    Some(path)
                } else {
                    None
                }
            })
        })
}

/// Create or ensure the docked Yazi config directory and file exist
/// Copies user's config files (keymap.toml, theme.toml) and creates a custom yazi.toml with single-panel layout
fn ensure_docked_yazi_config(runtime_dir: &Path) -> Result<PathBuf> {
    let config_dir = docked_yazi_config_dir(runtime_dir);
    let config_path = config_dir.join("yazi.toml");

    if !config_path.exists() {
        std::fs::create_dir_all(&config_dir)?;

        // Copy user's config files if they exist
        if let Some(user_config_dir) = user_yazi_config_dir() {
            let files_to_copy = ["keymap.toml", "theme.toml", "init.lua"];
            for filename in files_to_copy {
                let src = user_config_dir.join(filename);
                let dst = config_dir.join(filename);
                if src.exists() {
                    if let Err(e) = std::fs::copy(&src, &dst) {
                        eprintln!("Warning: Failed to copy {}: {}", filename, e);
                    }
                }
            }
        }

        // Create custom yazi.toml with single-panel layout
        let content = r#"[mgr]
ratio = [0, 8, 0]
"#;
        std::fs::write(&config_path, content)?;
    }
    Ok(config_dir)
}

/// Default TTL for pane cache in milliseconds
const PANE_CACHE_TTL_MS: u64 = 500;

/// Cached pane information with timestamp
struct CachedPanes {
    timestamp: Instant,
    panes: Vec<PaneInfo>,
    backend: String,
}

/// Cache for pane information to avoid repeated backend queries
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
    fn get(&self, backend: &dyn SessionBackend) -> Result<Vec<PaneInfo>> {
        // Read cache
        {
            let guard = self.data.read().unwrap();
            if let Some(ref cached) = *guard {
                // If the backend matches and cache is fresh, return cached data
                if cached.backend == backend.name()
                    && cached.timestamp.elapsed() < Duration::from_millis(PANE_CACHE_TTL_MS)
                {
                    return Ok(cached.panes.clone());
                }
            }
        }

        // Fetch fresh data
        let panes = backend.panes()?;

        // Update cache
        {
            let mut guard = self.data.write().unwrap();
            *guard = Some(CachedPanes {
                timestamp: Instant::now(),
                panes: panes.clone(),
                backend: backend.name().to_string(),
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
pub enum MindMapAction {
    /// Hide the MindMap window behind a fullscreen editor, or bring it back.
    Toggle,
    /// Switch the MindMap window between its docked height and fullscreen.
    Zoom,
}

#[derive(Debug, Clone, Copy)]
pub enum GitAction {
    Open,
}

#[derive(Debug, Clone, Copy)]
pub enum ReviewAction {
    Open,
}

#[derive(Debug, Clone, Copy)]
pub enum WorkflowAction {
    Open,
}

#[derive(Debug, Clone, Copy)]
pub enum VenvAction {
    /// Toggle virtual environment activation/deactivation
    Toggle,
    /// Open a TUI to select from detected environments
    Select,
}

pub fn file_manager_action(
    config: &Config,
    config_path: &Path,
    action: FileManagerAction,
) -> Result<()> {
    let backend = backend::from_config(config)?;
    let panes = list_panes(backend.as_ref())?;
    // The helper may itself be the file-manager pane (kitty map-launched
    // overlays are born with the file-manager role), so it never counts as
    // an existing pane to focus: the action runs the manager in place
    // instead.
    let current_id = backend.current_pane_id()?;
    let existing = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == FILE_MANAGER_PANE && pane.id != current_id);

    match action {
        FileManagerAction::ToggleDock => {
            if let Some(pane) = existing {
                let editor = panes
                    .iter()
                    .find(|candidate| !candidate.is_plugin && candidate.title == EDITOR_PANE)
                    .context("Cannot find the sat-hx-ide editor pane")?;
                toggle_dock(config, config_path, backend.as_ref(), pane, editor)?;
                return Ok(());
            }
        }
        FileManagerAction::Open => {
            if let Some(pane) = existing {
                backend.focus(pane.id)?;
                return Ok(());
            }
        }
        FileManagerAction::Run => {}
    }

    let current = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.id == current_id);
    let editor = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == EDITOR_PANE)
        .context("Cannot find the sat-hx-ide editor pane")?;

    if matches!(action, FileManagerAction::ToggleDock) {
        return become_floating_manager(config, backend.as_ref(), current_id);
    }

    if action_is_request(action) && current.is_some_and(|pane| pane.tab_id != editor.tab_id) {
        spawn_on_code_tab(config, config_path, backend.as_ref(), editor.tab_id)?;
        return Ok(());
    }

    backend.rename(current_id, FILE_MANAGER_PANE)?;

    run_file_manager(config, backend.as_ref(), current_id)
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
    backend: &dyn SessionBackend,
    tab_id: u64,
) -> Result<()> {
    let pane_id = spawn_manager(config, config_path, backend, tab_id, true)?;
    if backend.helper_waits_for_spawned_pane() {
        wait_for_pane_close(backend, pane_id)
    } else {
        Ok(())
    }
}

/// Spawn the in-pane file-manager helper and wait for its pane to appear.
fn spawn_manager(
    config: &Config,
    config_path: &Path,
    backend: &dyn SessionBackend,
    tab_id: u64,
    floating: bool,
) -> Result<PaneId> {
    let executable = std::env::current_exe().context("Cannot locate sat-hx-ide executable")?;
    let args = vec![
        OsString::from("--config"),
        expand_tilde(config_path).into_os_string(),
        OsString::from("__file-manager"),
        OsString::from("run"),
    ];
    let spec = SpawnSpec {
        title: FILE_MANAGER_PANE,
        tab_id,
        placement: if floating {
            Placement::Floating(file_manager_geometry(config))
        } else {
            Placement::Tiled
        },
        close_on_exit: true,
        cwd: None,
        program: Some(&executable),
        args: &args,
    };
    let pane_id = backend.spawn(&spec)?;
    wait_for_pane(backend, pane_id)?;
    Ok(pane_id)
}

/// Floating geometry for the file manager, shared by open and re-float.
fn file_manager_geometry(config: &Config) -> FloatGeometry<'_> {
    FloatGeometry {
        x: "0%",
        y: "0%",
        width: config.tools.file_manager.float_width.as_str(),
        height: config.tools.file_manager.float_height.as_str(),
    }
}

fn run_file_manager(config: &Config, backend: &dyn SessionBackend, pane_id: PaneId) -> Result<()> {
    if config.tools.file_manager.adapter != "yazi" {
        bail!(
            "Unsupported file-manager adapter '{}'; currently supported: yazi",
            config.tools.file_manager.adapter
        );
    }
    let manager = resolve_executable(&config.tools.file_manager.command)?;
    let session = backend.session_name()?;
    let runtime_dir = config.runtime_dir(&session);
    let state_file = yazi_docked_state_file(&runtime_dir);
    let chooser = runtime_dir.join(format!("chooser-{pane_id}-{}.txt", std::process::id()));
    if let Some(parent) = chooser.parent() {
        fs::create_dir_all(parent)?;
    }

    // Check if Yazi should be in docked (single-panel) mode
    let use_docked_config = state_file.exists();

    loop {
        let _ = fs::remove_file(&chooser);

        let mut yazi_cmd = Command::new(&manager);
        yazi_cmd.args(&config.tools.file_manager.args);

        // Use docked config if state file exists
        if use_docked_config {
            let config_dir = ensure_docked_yazi_config(&runtime_dir)?;
            yazi_cmd.env("YAZI_CONFIG_HOME", &config_dir);
        }

        let status = yazi_cmd
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
        open_in_editor(backend, &selections[0])?;

        let pane = list_panes(backend)?
            .into_iter()
            .find(|pane| !pane.is_plugin && pane.id == pane_id);
        if pane.is_none_or(|pane| pane.is_floating) {
            return Ok(());
        }
    }
}

fn become_floating_manager(
    config: &Config,
    backend: &dyn SessionBackend,
    pane_id: PaneId,
) -> Result<()> {
    backend.set_floating_geometry(pane_id, &file_manager_geometry(config))?;
    backend.rename(pane_id, FILE_MANAGER_PANE)?;
    run_file_manager(config, backend, pane_id)
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

fn open_in_editor(backend: &dyn SessionBackend, selected: &Path) -> Result<()> {
    let editor = list_panes(backend)?
        .into_iter()
        .find(|pane| !pane.is_plugin && pane.title == EDITOR_PANE)
        .context("Cannot find the sat-hx-ide editor pane")?;
    let path = helix_quote(selected)?;

    backend.send_key(editor.id, Key::Escape)?;
    backend.send_text(editor.id, &format!(":open {path}"))?;
    backend.send_key(editor.id, Key::Enter)
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

/// Both terminal actions are built on backend fullscreen, which hides the other
/// panes without touching the processes inside them.
pub fn terminal_action(config: &Config, action: TerminalAction) -> Result<()> {
    let backend = backend::from_config(config)?;
    let panes = list_panes(backend.as_ref())?;
    let editor = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == EDITOR_PANE)
        .context("Cannot find the sat-hx-ide editor pane")?;
    let Some(terminal) = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == TERMINAL_PANE)
    else {
        return respawn_terminal(config, backend.as_ref(), editor);
    };

    let hidden = editor.is_fullscreen;
    let zoomed = terminal.is_fullscreen;

    match action {
        TerminalAction::Toggle if hidden => {
            backend.toggle_fullscreen(editor.id)?;
            backend.focus(terminal.id)
        }
        TerminalAction::Toggle => {
            if zoomed {
                backend.toggle_fullscreen(terminal.id)?;
            }
            backend.toggle_fullscreen(editor.id)
        }
        TerminalAction::Zoom if zoomed => backend.toggle_fullscreen(terminal.id),
        TerminalAction::Zoom => {
            if hidden {
                backend.toggle_fullscreen(editor.id)?;
            }
            backend.toggle_fullscreen(terminal.id)
        }
    }
}

/// Mind-map toggle/zoom mirrors the terminal: fullscreen hides sibling
/// panes without stopping the tool process inside them.
pub fn mind_map_action(config: &Config, action: MindMapAction) -> Result<()> {
    let backend = backend::from_config(config)?;
    let panes = list_panes(backend.as_ref())?;
    let editor = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == EDITOR_PANE)
        .context("Cannot find the sat-hx-ide editor pane")?;
    let Some(mindmap) = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == MIND_MAP_PANE)
    else {
        return respawn_mindmap(config, backend.as_ref(), editor);
    };

    let hidden = editor.is_fullscreen;
    let zoomed = mindmap.is_fullscreen;

    match action {
        MindMapAction::Toggle if hidden => {
            backend.toggle_fullscreen(editor.id)?;
            backend.focus(mindmap.id)
        }
        MindMapAction::Toggle => {
            if zoomed {
                backend.toggle_fullscreen(mindmap.id)?;
            }
            backend.toggle_fullscreen(editor.id)
        }
        MindMapAction::Zoom if zoomed => backend.toggle_fullscreen(mindmap.id),
        MindMapAction::Zoom => {
            if hidden {
                backend.toggle_fullscreen(editor.id)?;
            }
            backend.toggle_fullscreen(mindmap.id)
        }
    }
}

/// Dock a fresh mind-map tool when the previous pane was closed or never opened.
///
/// Unlike the terminal, `mindmap.enabled` only controls whether the pane is
/// present at session start. Alt-m can always spawn it on demand.
fn respawn_mindmap(config: &Config, backend: &dyn SessionBackend, editor: &PaneInfo) -> Result<()> {
    let mindmap = &config.mindmap;
    let tool = &config.tools.mindmap;
    let command = resolve_executable(&tool.command)
        .with_context(|| format!("Cannot find '{}'", tool.command))?;
    let args: Vec<OsString> = tool.args.iter().map(OsString::from).collect();

    // Focus the editor and open beside it. Do not dock relative to the
    // current helper pane: this action runs from a floating helper whose
    // pane is destroyed on exit, and docking next to that helper takes the
    // mind-map pane down with it.
    backend.focus(editor.id)?;
    let cwd = std::env::current_dir().ok();

    let (direction, grow_border) = dock_layout(mindmap.dock_position);
    let spec = SpawnSpec {
        title: MIND_MAP_PANE,
        tab_id: editor.tab_id,
        placement: Placement::TiledBeside(direction),
        close_on_exit: true,
        cwd: cwd.as_deref(),
        program: Some(command.as_path()),
        args: &args,
    };
    let pane_id = backend.spawn(&spec)?;
    wait_for_pane(backend, pane_id)?;

    // Spawn naming does not always stick; match layout naming explicitly.
    backend.rename(pane_id, MIND_MAP_PANE)?;

    backend.resize_to_share(
        pane_id,
        editor.id,
        dock_axis(mindmap.dock_position),
        grow_border,
        mindmap.dock_percent,
    )?;

    pane_cache().invalidate();
    Ok(())
}

/// Dock a fresh shell when the previous one was exited by the user.
fn respawn_terminal(
    config: &Config,
    backend: &dyn SessionBackend,
    editor: &PaneInfo,
) -> Result<()> {
    let terminal = &config.terminal;
    if !terminal.enabled {
        bail!("Terminal is disabled in configuration");
    }

    // Same docking rule as respawn_mindmap: follow the focused editor, not the
    // helper pane that invoked this action.
    backend.focus(editor.id)?;
    let cwd = std::env::current_dir().ok();

    let (direction, grow_border) = dock_layout(terminal.dock_position);
    let spec = SpawnSpec {
        title: TERMINAL_PANE,
        tab_id: editor.tab_id,
        placement: Placement::TiledBeside(direction),
        close_on_exit: false,
        cwd: cwd.as_deref(),
        program: None,
        args: &[],
    };
    let pane_id = backend.spawn(&spec)?;
    wait_for_pane(backend, pane_id)?;

    // Spawn naming does not always stick; match layout naming explicitly.
    backend.rename(pane_id, TERMINAL_PANE)?;

    backend.resize_to_share(
        pane_id,
        editor.id,
        dock_axis(terminal.dock_position),
        grow_border,
        terminal.dock_percent,
    )?;

    pane_cache().invalidate();
    Ok(())
}

/// Split direction for docking below/right of the editor, plus the border the
/// docked pane grows along when resized.
fn dock_layout(position: DockPosition) -> (Direction, &'static str) {
    match position {
        DockPosition::Down => (Direction::Down, "up"),
        DockPosition::Right => (Direction::Right, "left"),
    }
}

fn dock_axis(position: DockPosition) -> Axis {
    match position {
        DockPosition::Down => Axis::Rows,
        DockPosition::Right => Axis::Columns,
    }
}

fn toggle_dock(
    config: &Config,
    config_path: &Path,
    backend: &dyn SessionBackend,
    pane: &PaneInfo,
    editor: &PaneInfo,
) -> Result<()> {
    let was_floating = pane.is_floating;
    let session = backend.session_name()?;
    let runtime_dir = config.runtime_dir(&session);
    let state_file = yazi_docked_state_file(&runtime_dir);

    backend.close(pane.id)?;

    if !was_floating {
        // Undocking: remove state file for floating mode
        let _ = std::fs::remove_file(&state_file);
        return become_floating_manager(config, backend, backend.current_pane_id()?);
    }

    // Docking: create state file for docked mode
    if let Some(parent) = state_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&state_file, "docked")?;

    backend.focus(editor.id)?;
    let pane_id = spawn_manager(config, config_path, backend, editor.tab_id, !was_floating)?;

    if was_floating {
        backend.move_pane(pane_id, Direction::Left)?;
        backend.resize_to_share(
            pane_id,
            editor.id,
            Axis::Columns,
            "right",
            config.tools.file_manager.dock_percent,
        )?;
    }

    backend.focus(pane_id)?;
    if backend.helper_waits_for_spawned_pane() {
        wait_for_pane_close(backend, pane_id)
    } else {
        Ok(())
    }
}

/// `spawn` returns before the pane shows up in the pane list, so actions
/// targeting the new pane have to wait for it to appear first.
fn wait_for_pane(backend: &dyn SessionBackend, pane_id: PaneId) -> Result<PaneId> {
    for _ in 0..40 {
        if pane_exists(backend, pane_id)? {
            return Ok(pane_id);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    bail!("Pane {pane_id} did not appear")
}

fn wait_for_pane_close(backend: &dyn SessionBackend, pane_id: PaneId) -> Result<()> {
    while pane_exists(backend, pane_id)? {
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    Ok(())
}

fn pane_exists(backend: &dyn SessionBackend, id: PaneId) -> Result<bool> {
    Ok(list_panes(backend)?
        .iter()
        .any(|pane| !pane.is_plugin && pane.id == id))
}

/// Get the list of panes from the backend, using cache if available
///
/// This function uses a per-process cache to avoid querying the backend
/// multiple times within a single action execution. The cache has a TTL of
/// 500ms to ensure data freshness.
fn list_panes(backend: &dyn SessionBackend) -> Result<Vec<PaneInfo>> {
    pane_cache().get(backend)
}

pub fn git_action(config: &Config, _action: GitAction) -> Result<()> {
    spawn_floating_tool(config, "git", &config.tools.git)
}

pub fn review_action(config: &Config, _action: ReviewAction) -> Result<()> {
    spawn_floating_tool(config, "review", &config.tools.review)
}

pub fn workflow_action(config: &Config, _action: WorkflowAction) -> Result<()> {
    spawn_floating_tool(config, "workflow", &config.tools.workflow)
}

/// Manages virtual environment activation/deactivation in the terminal pane.
/// Uses auto-detection when path is not configured, or uses the configured path.
pub fn venv_action(config: &Config, action: VenvAction) -> Result<()> {
    use crate::venv::determine_venv_commands;

    let backend = backend::from_config(config)?;
    let venv_config = &config.venv;

    if !venv_config.enabled {
        bail!("Virtual environment management is disabled in configuration. Enable it in [venv] section.");
    }

    let panes = list_panes(backend.as_ref()).context("Not running in a sat-hx-ide session")?;

    let terminal = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == TERMINAL_PANE)
        .context("Cannot find the sat-hx-ide terminal pane. The terminal may have been closed.")?;

    match action {
        VenvAction::Toggle => {
            let session_name = backend.session_name()?;
            let runtime_dir = config.runtime_dir(&session_name);
            let venv_state_file = runtime_dir.join("venv_active");

            let is_active = venv_state_file.exists();

            // Check if we have a saved selection
            if let Ok(Some(saved_selection)) = crate::venv::load_venv_selection(&session_name) {
                // Use the saved selection's activation command
                let (activate_cmd, deactivate_cmd) = crate::venv::generate_commands(
                    &saved_selection.venv_type,
                    Some(&saved_selection.path),
                )?;

                if is_active {
                    // Deactivate
                    send_command_to_pane(backend.as_ref(), terminal, &deactivate_cmd)?;
                    let _ = std::fs::remove_file(venv_state_file);
                } else {
                    // Activate the saved selection
                    send_command_to_pane(backend.as_ref(), terminal, &activate_cmd)?;
                    std::fs::create_dir_all(&runtime_dir)?;
                    std::fs::write(venv_state_file, "active")?;
                }
            } else {
                // Fall back to auto-detection
                let (activate_cmd, deactivate_cmd) = determine_venv_commands(venv_config)?;

                if is_active {
                    // Deactivate
                    send_command_to_pane(backend.as_ref(), terminal, &deactivate_cmd)?;
                    let _ = std::fs::remove_file(venv_state_file);
                } else {
                    // Activate
                    send_command_to_pane(backend.as_ref(), terminal, &activate_cmd)?;
                    std::fs::create_dir_all(&runtime_dir)?;
                    std::fs::write(venv_state_file, "active")?;
                }
            }
        }
        VenvAction::Select => {
            select_venv_interactive(config, backend.as_ref(), terminal)?;
        }
    }

    Ok(())
}

/// Spawn an interactive selector to choose a virtual environment
fn select_venv_interactive(
    config: &Config,
    backend: &dyn SessionBackend,
    terminal: &PaneInfo,
) -> Result<()> {
    use crate::venv::{
        clear_selector_handoff, filter_activatable_environments, generate_commands,
        get_all_available_venvs, normalize_selectable_venv, read_selector_handoff, same_venv_path,
        save_venv_collection, save_venv_selection, VenvCollection, VENV_SELECTOR_DONE,
        VENV_SELECTOR_OUTPUT,
    };
    use std::fs;

    let venv_config = &config.venv;
    let session_name = backend.session_name()?;

    // Get all available environments (detected + previously saved custom paths)
    let environments = get_all_available_venvs(venv_config, &session_name)?;

    let runtime_dir = config.runtime_dir(&session_name);
    std::fs::create_dir_all(&runtime_dir)?;
    clear_selector_handoff(&runtime_dir)?;

    let selector_output_file = runtime_dir.join(VENV_SELECTOR_OUTPUT);

    let executable = std::env::current_exe().context("Cannot locate sat-hx-ide executable")?;

    let args = vec![
        OsString::from("__venv"),
        OsString::from("run-selector"),
        OsString::from("--session"),
        OsString::from(session_name.clone()),
    ];
    let spec = SpawnSpec {
        title: "sat-venv-selector",
        tab_id: terminal.tab_id,
        placement: Placement::Floating(FloatGeometry {
            x: "10%",
            y: "20%",
            width: "80%",
            height: "60%",
        }),
        close_on_exit: true,
        cwd: None,
        program: Some(&executable),
        args: &args,
    };
    let outcome = backend
        .spawn_blocking(&spec)
        .context("Failed to spawn venv selector pane")?;

    if !outcome.success && !selector_output_file.exists() {
        return Ok(());
    }

    if !outcome.success {
        bail!("Virtual environment selector failed: {}", outcome.stderr);
    }

    if !selector_output_file.exists() {
        return Ok(());
    }

    let marker = fs::read_to_string(&selector_output_file)?;
    let _ = fs::remove_file(&selector_output_file);

    if marker.trim() == "NO_ENVIRONMENTS" {
        bail!("No virtual environments detected");
    }

    if marker.trim() != VENV_SELECTOR_DONE {
        bail!("Virtual environment selector returned an unexpected result");
    }

    let selection = read_selector_handoff(&runtime_dir)?
        .context("Selector finished but no environment was chosen")?;
    let selection = normalize_selectable_venv(selection)?;

    let (activate_cmd, _) = generate_commands(&selection.venv_type, Some(&selection.path))?;
    send_command_to_pane(backend, terminal, &activate_cmd)?;

    save_venv_selection(&session_name, &selection)?;

    let mut updated_environments = environments;
    if !updated_environments
        .iter()
        .any(|e| same_venv_path(&e.path, &selection.path))
    {
        updated_environments.push(selection.clone());
    }
    save_venv_collection(
        &session_name,
        &VenvCollection {
            environments: filter_activatable_environments(updated_environments),
        },
    )?;
    std::fs::write(runtime_dir.join("venv_active"), "active")?;

    Ok(())
}

/// Send a command to a specific pane and execute it
fn send_command_to_pane(
    backend: &dyn SessionBackend,
    pane: &PaneInfo,
    command: &str,
) -> Result<()> {
    // Typed text avoids keybinding parsing
    backend.send_text(pane.id, command)?;
    // Send Enter to execute
    backend.send_key(pane.id, Key::Enter)
}

fn spawn_floating_tool(config: &Config, pane_name: &str, tool: &CommandConfig) -> Result<()> {
    let backend = backend::from_config(config)?;
    let panes = list_panes(backend.as_ref())?;
    let editor = panes
        .iter()
        .find(|pane| !pane.is_plugin && pane.title == EDITOR_PANE)
        .context("Cannot find the sat-hx-ide editor pane")?;

    let command = resolve_executable(&tool.command)
        .with_context(|| format!("Cannot find '{}'", tool.command))?;
    let args: Vec<OsString> = tool.args.iter().map(OsString::from).collect();

    let spec = SpawnSpec {
        title: pane_name,
        tab_id: editor.tab_id,
        placement: Placement::Floating(FloatGeometry {
            x: tool.x_pos.as_str(),
            y: tool.y_pos.as_str(),
            width: tool.float_width.as_str(),
            height: tool.float_height.as_str(),
        }),
        close_on_exit: true,
        cwd: None,
        program: Some(command.as_path()),
        args: &args,
    };
    backend.spawn(&spec)?;

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
                backend: "zellij".to_string(),
            });
        }

        // Verify it is there
        assert!(cache.data.read().unwrap().is_some());
    }

    #[test]
    fn pane_cache_invalidate_clears_data() {
        let cache = PaneCache::new();
        {
            let mut guard = cache.data.write().unwrap();
            *guard = Some(CachedPanes {
                timestamp: Instant::now(),
                panes: vec![],
                backend: "zellij".to_string(),
            });
        }
        cache.invalidate();
        assert!(cache.data.read().unwrap().is_none());
    }

    #[test]
    fn dock_layout_maps_positions() {
        assert_eq!(dock_layout(DockPosition::Down), (Direction::Down, "up"));
        assert_eq!(dock_layout(DockPosition::Right), (Direction::Right, "left"));
        assert_eq!(dock_axis(DockPosition::Down), Axis::Rows);
        assert_eq!(dock_axis(DockPosition::Right), Axis::Columns);
    }
}
