//! Workspace shell abstraction.
//!
//! sat-hx-ide drives a terminal multiplexer (Zellij today, kitty as a second
//! backend) to compose the IDE workspace. Everything the helpers do — list,
//! spawn, focus, resize, rename, close panes, and bootstrap a session — goes
//! through [`SessionBackend`] so the action layer stays backend-neutral.

pub mod kitty;
pub mod zellij;

use crate::config::{
    BackendKind, CommandConfig, KeybindingConfig, MindMapConfig, TerminalConfig,
};
use crate::resolve::resolve_executable;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub use kitty::KittyBackend;
pub use zellij::ZellijBackend;

/// Backend-agnostic pane identifier. Backends map it to their own CLI form
/// (Zellij uses `terminal_<id>`, kitty uses bare window ids).
pub type PaneId = u64;

/// A pane/window in the session, as reported by the backend.
#[derive(Debug, Clone, Deserialize)]
pub struct PaneInfo {
    pub id: PaneId,
    /// Zellij plugin panes carry no process and must be skipped by actions.
    /// Backends without plugins report `false`.
    #[serde(default)]
    pub is_plugin: bool,
    pub is_floating: bool,
    pub is_fullscreen: bool,
    pub title: String,
    pub tab_id: u64,
    pub pane_columns: usize,
    pub pane_rows: usize,
}

/// Axis a resize applies to. Zellij infers it from the border being resized;
/// kitty requires an explicit `--axis`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Columns,
    Rows,
}

impl Axis {
    pub fn measure(self, pane: &PaneInfo) -> usize {
        match self {
            Axis::Columns => pane.pane_columns,
            Axis::Rows => pane.pane_rows,
        }
    }
}

/// Direction on screen, used for docking panes and describing resize borders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    /// Backend-native lowercase name (`left`, `right`, `up`, `down`).
    pub fn name(self) -> &'static str {
        match self {
            Direction::Left => "left",
            Direction::Right => "right",
            Direction::Up => "up",
            Direction::Down => "down",
        }
    }
}

/// Special keys the helpers send to panes (Helix `:open`, shell `Enter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Escape,
    Enter,
}

/// Floating-pane geometry. Values are percentage strings from configuration
/// (e.g. `0%`, `80%`); backends that cannot place windows freely (kitty
/// overlays) may ignore position and use the size as a hint.
#[derive(Debug, Clone, Copy)]
pub struct FloatGeometry<'a> {
    pub x: &'a str,
    pub y: &'a str,
    pub width: &'a str,
    pub height: &'a str,
}

/// Where a spawned pane lands.
#[derive(Debug, Clone, Copy)]
pub enum Placement<'a> {
    /// Floating pane with explicit geometry (Zellij floating panes, kitty
    /// overlay windows).
    Floating(FloatGeometry<'a>),
    /// Tiled pane in the default position.
    Tiled,
    /// Tiled pane split in the given direction relative to the active pane.
    TiledBeside(Direction),
}

/// How a spawned program is launched.
#[derive(Debug, Clone, Copy)]
pub struct SpawnSpec<'a> {
    /// Pane title used by actions to find the pane again (editor, terminal,
    /// file-manager, ...).
    pub title: &'a str,
    /// Backend tab id the pane is spawned in.
    pub tab_id: u64,
    pub placement: Placement<'a>,
    /// Close the pane when the program exits.
    pub close_on_exit: bool,
    /// Working directory for the program.
    pub cwd: Option<&'a Path>,
    /// Program to run; `None` runs the backend's default shell.
    pub program: Option<&'a Path>,
    pub args: &'a [OsString],
}

/// Result of a blocking spawn (`--block-until-exit` on Zellij).
#[derive(Debug)]
pub struct SpawnOutcome {
    pub success: bool,
    pub stderr: String,
}

/// Inputs for session bootstrap: layout generation and session creation are
/// backend-specific, so the whole init flow lives behind the trait.
pub struct SessionInit<'a> {
    pub session_name: &'a str,
    pub project_dir: &'a Path,
    pub runtime_dir: &'a Path,
    /// User's own multiplexer config, used read-only as merge input.
    pub user_config: Option<&'a Path>,
    /// Reuse an existing session instead of failing when one exists.
    pub attach_existing: bool,
    /// Show the multiplexer's built-in status bar alongside ours.
    pub native_status_bar: bool,
    pub editor: &'a CommandConfig,
    pub editor_path: &'a Path,
    pub ai: Option<(&'a CommandConfig, &'a Path)>,
    pub terminal: Option<&'a TerminalConfig>,
    pub mindmap: Option<(&'a MindMapConfig, &'a CommandConfig, &'a Path)>,
    pub keybindings: &'a KeybindingConfig,
    /// sat-hx-ide executable, referenced by runtime keybindings.
    pub executable: &'a Path,
    /// Path of the sat-hx-ide config file.
    pub app_config: &'a Path,
    pub git_path: &'a Path,
    pub git_args: &'a [String],
    pub float_width: &'a str,
    pub float_height: &'a str,
}

/// State of a named session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    NotFound,
    Active,
    Exited,
}

/// The workspace shell sat-hx-ide drives.
///
/// Implementations wrap one multiplexer (Zellij, kitty). Methods describe
/// intent, not CLI syntax: `resize` takes a request the backend maps to its
/// own step semantics, `send_text`/`send_key` hide key-notation differences,
/// and pane ids are plain numbers that each backend formats as needed.
pub trait SessionBackend: Send {
    /// Stable backend name ("zellij", "kitty").
    fn name(&self) -> &'static str;

    /// Session name for runtime state, usually from the environment
    /// (`ZELLIJ_SESSION_NAME` for Zellij, `SAT_SESSION_NAME` for kitty).
    fn session_name(&self) -> Result<String>;

    /// Pane the current helper process runs in.
    fn current_pane_id(&self) -> Result<PaneId>;

    /// Fresh pane list. The action layer caches on top of this.
    fn panes(&self) -> Result<Vec<PaneInfo>>;

    /// Spawn a pane and return its id. Returns before the program exits.
    fn spawn(&self, spec: &SpawnSpec<'_>) -> Result<PaneId>;

    /// Spawn a pane and wait for the program to exit, returning its outcome.
    fn spawn_blocking(&self, spec: &SpawnSpec<'_>) -> Result<SpawnOutcome>;

    fn focus(&self, id: PaneId) -> Result<()>;
    fn rename(&self, id: PaneId, title: &str) -> Result<()>;
    fn close(&self, id: PaneId) -> Result<()>;
    /// Make the pane fill the tab, or restore it. Kitty emulates this with
    /// its stack layout.
    fn toggle_fullscreen(&self, id: PaneId) -> Result<()>;
    /// Re-position an existing floating pane. Backends without movable
    /// floats (kitty overlays always cover the focused window) accept and
    /// ignore the geometry.
    fn set_floating_geometry(&self, id: PaneId, geometry: &FloatGeometry<'_>) -> Result<()>;
    /// Move a tiled pane within the tab. Backends that cannot move panes
    /// (kitty has no attach-window) treat this as a no-op: their docking
    /// placement is already decided at spawn time.
    fn move_pane(&self, id: PaneId, direction: Direction) -> Result<()>;
    /// Resize `pane` until it takes `target_percent` of the space it splits
    /// with `editor` along `axis`. `border` names the edge the pane grows
    /// along (Zellij fine resizes); backends with exact resize sizes may
    /// ignore it.
    ///
    /// Zellij only has relative steps (coarse `+`/`-` and fine border
    /// `increase`/`decrease`), so its implementation loops with fresh
    /// measurements. kitty takes an exact cell count, so its implementation
    /// requests the full remaining delta and only re-measures when kitty
    /// clamps the applied increment.
    fn resize_to_share(
        &self,
        pane_id: PaneId,
        editor_id: PaneId,
        axis: Axis,
        border: &str,
        target_percent: u8,
    ) -> Result<()>;
    /// Type text into a pane without triggering keybindings.
    fn send_text(&self, id: PaneId, text: &str) -> Result<()>;
    /// Send a special key to a pane.
    fn send_key(&self, id: PaneId, key: Key) -> Result<()>;
    /// Whether a helper must stay alive while a pane it spawned lives.
    ///
    /// Zellij helpers run in tiny floating panes and wait for the panes
    /// they spawn (its pane model keeps them around). kitty helpers are
    /// overlay windows covering the session: a waiting helper would blank
    /// the window underneath it, so they exit as soon as their work is
    /// done and let the spawned pane live on its own.
    fn helper_waits_for_spawned_pane(&self) -> bool;
    /// Focus the pane focused before the current one (status-bar focus guard).
    fn focus_previous_pane(&self) -> Result<()>;

    /// Create or attach the session described by `init`, generating the
    /// backend's runtime files under `init.runtime_dir`.
    fn init_session(&self, init: &SessionInit<'_>) -> Result<()>;

    fn session_status(&self, name: &str) -> Result<SessionStatus>;
    fn delete_session(&self, name: &str) -> Result<()>;
}

/// Resolve `backend = "auto"`: kitty when sat-hx-ide runs inside a kitty
/// window, Zellij otherwise.
///
/// `KITTY_WINDOW_ID` is set for every kitty child process and is not
/// inherited over SSH, unlike `TERM=xterm-kitty` which remote shells keep.
pub fn detect_kind() -> BackendKind {
    if std::env::var_os("KITTY_WINDOW_ID").is_some() {
        BackendKind::Kitty
    } else {
        BackendKind::Zellij
    }
}

/// The backend a configuration selects, with `auto` resolved against the
/// current environment.
pub fn resolve_kind(config: &crate::config::Config) -> BackendKind {
    match config.session.backend {
        BackendKind::Auto => detect_kind(),
        kind => kind,
    }
}

/// Resolve the `kitten` CLI: by name on PATH, or as a sibling of the kitty
/// binary (the upstream install ships them together).
pub fn resolve_kitten(kitty: &Path) -> Option<PathBuf> {
    resolve_executable("kitten").ok().or_else(|| {
        let sibling = kitty.parent()?.join("kitten");
        sibling.exists().then_some(sibling)
    })
}

/// Build the configured session backend: Zellij or kitty, selected by
/// `[session] backend` (with `auto` resolving per [`detect_kind`]).
pub fn from_config(config: &crate::config::Config) -> Result<Box<dyn SessionBackend>> {
    match resolve_kind(config) {
        BackendKind::Zellij => {
            let zellij = resolve_executable(&config.tools.zellij.command)?;
            Ok(Box::new(ZellijBackend::new(zellij)))
        }
        BackendKind::Kitty => {
            let kitty = resolve_executable(&config.tools.kitty.command)?;
            // Remote control needs the kitten CLI; `kitty +kitten` is not a
            // valid substitute for it.
            let kitten = resolve_kitten(&kitty)
                .context("The kitty backend requires the 'kitten' binary")?;
            Ok(Box::new(KittyBackend::new(
                kitty,
                kitten,
                config.runtime_root(),
            )))
        }
        BackendKind::Auto => unreachable!("resolve_kind never returns Auto"),
    }
}
