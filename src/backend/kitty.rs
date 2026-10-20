//! kitty implementation of [`SessionBackend`], built on the remote-control
//! protocol verified in the kitty backend spike:
//!
//! - Dedicated mode: `init_session` spawns its own kitty with a per-session
//!   Unix socket (`<runtime_dir>/kitty.sock`) and
//!   `--override allow_remote_control=socket-only`.
//! - Floats are overlay windows; `SAT_FLOAT_WINDOW=1` marks them because
//!   `kitten @ ls` has no overlay flag.
//! - Fullscreen is the `stack` layout: switching to stack shows only the
//!   focused window, switching back to `splits` restores geometry exactly.
//! - Window moves are not possible (no attach-window in any kitty), so
//!   `move_pane` is a documented no-op: docking placement is decided at
//!   spawn time (`--location`), which docks panes beside the editor.

use super::{
    Axis, Direction, Key, PaneId, SessionBackend, SessionInit, SessionStatus, SpawnOutcome,
    SpawnSpec,
};
use crate::kitty::client::{KittyClient, FLOAT_ENV, ROLE_ENV, ROLE_VAR, SESSION_ENV};
use crate::kitty::runtime_config::{build_runtime_config, KittyRuntimeConfigInput};
use crate::kitty::session::{session_file, KittySessionInput};
use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SOCKET_READY_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(250);
pub struct KittyBackend {
    kitty: PathBuf,
    /// Resolved `kitten` CLI used for remote control.
    kitten: PathBuf,
    /// `$XDG_RUNTIME_DIR/sat-helix-ide` — sessions live below it.
    runtime_root: PathBuf,
}

impl KittyBackend {
    pub fn new(kitty: PathBuf, kitten: PathBuf, runtime_root: PathBuf) -> Self {
        Self {
            kitty,
            kitten,
            runtime_root,
        }
    }

    fn client_for(&self, session: &str) -> KittyClient {
        KittyClient::for_session(self.kitten.clone(), &self.runtime_root.join(session))
    }

    fn client(&self) -> KittyClient {
        let session = self
            .session_name()
            .unwrap_or_else(|_| "session".to_string());
        self.client_for(&session)
    }

    /// Runtime file recording which role is zoomed (stack layout), so the
    /// fullscreen state survives helper overlays stealing window focus.
    fn zoom_state_file(&self) -> PathBuf {
        let session = self
            .session_name()
            .unwrap_or_else(|_| "session".to_string());
        self.runtime_root.join(session).join("zoom.state")
    }

    fn read_zoomed_role(&self) -> Option<String> {
        fs::read_to_string(self.zoom_state_file())
            .ok()
            .map(|role| role.trim().to_string())
            .filter(|role| !role.is_empty())
    }

    fn write_zoomed_role(&self, role: Option<&str>) -> Result<()> {
        let path = self.zoom_state_file();
        let result = match role {
            Some(role) => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&path, role)
            }
            None => match fs::remove_file(&path) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(err) => Err(err),
            },
        };
        result.with_context(|| format!("Failed to update {}", path.display()))
    }

    /// Argument stream for `kitten @ launch` implementing a spawn request.
    fn launch_args(spec: &SpawnSpec<'_>, floating_marker: bool) -> Vec<OsString> {
        let mut args: Vec<OsString> = vec![
            "launch".into(),
            "--match".into(),
            format!("id:{}", spec.tab_id).into(),
        ];
        match spec.placement {
            super::Placement::Floating(_) => {
                args.push("--type=overlay".into());
                if floating_marker {
                    // kitty remote control takes argv, not a shell line:
                    // --env and its value are separate arguments.
                    args.push("--env".into());
                    args.push(format!("{FLOAT_ENV}=1").into());
                }
            }
            // `--location=before` places the new window before the active
            // one: to the LEFT in a vertical split, matching Zellij's
            // move-pane-left dock for the file manager.
            super::Placement::Tiled => args.push("--location=before".into()),
            super::Placement::TiledBeside(direction) => args.push(
                match direction {
                    Direction::Down | Direction::Up => "--location=hsplit",
                    Direction::Left => "--location=before",
                    Direction::Right => "--location=vsplit",
                }
                .into(),
            ),
        }
        // Pane identity: the role env is immutable for the window's lifetime,
        // so every spawned pane gets one from its title.
        args.push("--env".into());
        args.push(format!("{ROLE_ENV}={}", spec.title).into());
        args.push("--title".into());
        args.push(spec.title.into());
        if let Some(cwd) = spec.cwd {
            args.push("--cwd".into());
            args.push(cwd.into());
        }
        if let Some(program) = spec.program {
            args.push(program.into());
            args.extend(spec.args.iter().map(OsString::from));
        }
        args
    }

    /// Spawn a pane and return its window id (kitty prints it to stdout).
    fn launch(&self, spec: &SpawnSpec<'_>) -> Result<PaneId> {
        let args = Self::launch_args(spec, true);
        let stdout = self.client().run_output_args(&args)?;
        stdout
            .trim()
            .parse::<u64>()
            .with_context(|| format!("kitty returned a non-numeric window id: '{stdout}'"))
    }

    /// One-shot resize of a window to an exact cell count along an axis.
    fn resize_by(&self, client: &KittyClient, id: PaneId, axis: Axis, delta: i32) -> Result<()> {
        if delta == 0 {
            return Ok(());
        }
        let args: Vec<OsString> = vec![
            "resize-window".into(),
            "--match".into(),
            format!("id:{id}").into(),
            "--axis".into(),
            match axis {
                Axis::Columns => "horizontal",
                Axis::Rows => "vertical",
            }
            .into(),
            "--increment".into(),
            format!("{delta}").into(),
        ];
        client.run_ok_args(&args)
    }

    /// Bring the code tab to the Zellij layout geometry: the docked
    /// terminal at its percent, a one-line status bar, the editor filling
    /// the remainder, and the mind map at its share of the editor split.
    ///
    /// kitty resizes move one border and hand the cells to a neighbor, so
    /// each role is nudged one step per pass and the whole layout is
    /// re-measured between passes until it stabilizes.
    fn apply_initial_layout(&self, client: &KittyClient, init: &SessionInit<'_>) {
        // The mind map cannot be expressed in the session file (kitty
        // always splits the ACTIVE window, and by now the active window is
        // the status bar). Focusing the editor first docks it beside Helix
        // exactly like the Zellij layout.
        if let Some((_config, command, path)) = init.mindmap.filter(|(config, _, _)| config.enabled)
        {
            let _ = client.run_ok_args(&[
                "focus-window".into(),
                "--match".into(),
                format!("env:{ROLE_ENV}=editor").into(),
            ]);
            let mut args: Vec<OsString> = vec![
                "launch".into(),
                "--location=vsplit".into(),
                "--env".into(),
                format!("{ROLE_ENV}=mindmapping").into(),
                "--title".into(),
                "mindmapping".into(),
            ];
            args.push(path.into());
            args.extend(command.args.iter().map(OsString::from));
            if let Err(err) = client.run_output_args(&args) {
                log::warn!("failed to launch the mind map: {err:#}");
            }
        }

        const MAX_PASSES: usize = 10;
        for pass in 0..MAX_PASSES {
            let stable = self.layout_pass(client, init);
            if stable {
                log::debug!("kitty layout stable after {pass} pass(es)");
                return;
            }
        }
        log::warn!("kitty layout did not fully stabilize after {MAX_PASSES} passes");
    }

    /// One measured resize step per role; returns whether the layout is at
    /// its target geometry.
    fn layout_pass(&self, client: &KittyClient, init: &SessionInit<'_>) -> bool {
        let Ok(panes) = client.panes(None) else {
            return true;
        };
        let mut stable = true;
        // A step is stable only when the pane already measures its target;
        // a successful resize means progress and another measured pass.
        let step = |pane: &super::PaneInfo, axis: Axis, target: usize| -> bool {
            let current = axis.measure(pane);
            if current.abs_diff(target) <= 1 {
                return true;
            }
            if let Err(err) = self.resize_by(client, pane.id, axis, target as i32 - current as i32)
            {
                log::warn!("failed to resize '{}': {err:#}", pane.title);
            }
            false
        };

        let terminal_config = init.terminal.filter(|terminal| terminal.enabled);
        let mindmap_config = init
            .mindmap
            .filter(|(config, _, _)| config.enabled)
            .map(|(config, _, _)| config);

        let mut tabs: Vec<u64> = panes.iter().map(|pane| pane.tab_id).collect();
        tabs.sort_unstable();
        tabs.dedup();
        for tab_id in tabs {
            let in_tab: Vec<&super::PaneInfo> =
                panes.iter().filter(|pane| pane.tab_id == tab_id).collect();
            let find = |title: &str| in_tab.iter().copied().find(|pane| pane.title == title);
            // Vertical chain: windows stacked full-width. Column-split
            // windows (mind map, right-docked terminal) are excluded —
            // their rows are shared with a sibling, not added.
            let column_split: Vec<&str> = ["mindmapping"]
                .into_iter()
                .chain(
                    terminal_config
                        .filter(|t| t.dock_position == crate::config::DockPosition::Right)
                        .map(|_| "terminal"),
                )
                .collect();
            let rows_chain: usize = in_tab
                .iter()
                .filter(|pane| !column_split.contains(&pane.title.as_str()))
                .map(|pane| pane.pane_rows)
                .sum();
            if rows_chain == 0 {
                continue;
            }

            let statusbars: Vec<&super::PaneInfo> = in_tab
                .iter()
                .filter(|pane| pane.title == "statusbar")
                .copied()
                .collect();
            let terminal_rows_target = match terminal_config {
                Some(config)
                    if config.dock_position == crate::config::DockPosition::Down
                        && find("terminal").is_some() =>
                {
                    (rows_chain * usize::from(config.dock_percent) / 100).max(1)
                }
                _ => 0,
            };
            let main_rows_target = rows_chain
                .saturating_sub(terminal_rows_target)
                .saturating_sub(statusbars.len())
                .max(1);

            // The main window of the tab (editor, AI agent) fills the
            // remainder of the vertical chain.
            if let Some(main) = in_tab.iter().find(|pane| {
                !column_split.contains(&pane.title.as_str()) && pane.title != "statusbar"
            }) {
                if !step(main, Axis::Rows, main_rows_target) {
                    stable = false;
                }
            }
            if let Some(terminal) = find("terminal") {
                let Some(config) = terminal_config else {
                    continue;
                };
                let stable_step = match config.dock_position {
                    crate::config::DockPosition::Down => {
                        step(terminal, Axis::Rows, terminal_rows_target)
                    }
                    crate::config::DockPosition::Right => {
                        let cols_chain: usize = in_tab
                            .iter()
                            .filter(|pane| pane.title != "statusbar")
                            .map(|pane| pane.pane_columns)
                            .sum();
                        step(
                            terminal,
                            Axis::Columns,
                            (cols_chain * usize::from(config.dock_percent) / 100).max(1),
                        )
                    }
                };
                if !stable_step {
                    stable = false;
                }
            }
            // One-line status bars, docked after the taller panes have
            // converged so neighbors cannot hand cells back to them.
            for bar in statusbars {
                if !step(bar, Axis::Rows, 1) {
                    stable = false;
                }
            }
            // Mind map share of the horizontal split with the editor.
            if let Some(mindmap) = find("mindmapping") {
                let Some(config) = mindmap_config else {
                    continue;
                };
                let cols_chain =
                    mindmap.pane_columns + find("editor").map(|e| e.pane_columns).unwrap_or(0);
                if !step(
                    mindmap,
                    Axis::Columns,
                    (cols_chain * usize::from(config.dock_percent) / 100).max(1),
                ) {
                    stable = false;
                }
            }
        }
        stable
    }

    fn wait_for_window_close(&self, id: PaneId) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(300);
        while Instant::now() < deadline {
            let gone = self
                .client()
                .panes(self.read_zoomed_role().as_deref())
                .map(|panes| !panes.iter().any(|pane| pane.id == id))
                .unwrap_or(true);
            if gone {
                return Ok(());
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        Ok(())
    }

    fn init_session_files(&self, init: &SessionInit<'_>) -> Result<(PathBuf, PathBuf)> {
        let session_path = init.runtime_dir.join("session.conf");
        let config_path = init.runtime_dir.join("kitty.conf");
        fs::create_dir_all(init.runtime_dir)?;
        fs::write(
            &session_path,
            session_file(&KittySessionInput {
                session_name: init.session_name,
                project_dir: init.project_dir,
                editor: init.editor,
                editor_path: init.editor_path,
                ai: init.ai,
                terminal: init.terminal,
                mindmap: init.mindmap,
                executable: init.executable,
                app_config: init.app_config,
            }),
        )
        .with_context(|| format!("Failed to write {}", session_path.display()))?;
        build_runtime_config(&KittyRuntimeConfigInput {
            source: init.user_config,
            destination: &config_path,
            session_name: init.session_name,
            keys: init.keybindings,
            executable: init.executable,
            app_config: init.app_config,
        })?;
        Ok((session_path, config_path))
    }
}

/// Print the config lines kitty ignored at startup (kept in `kitty.log`),
/// deduplicated. Unknown keys usually mean the user's kitty.conf uses
/// settings newer than the installed kitty — silently dropped otherwise.
fn report_ignored_config_lines(runtime_dir: &Path) {
    let Ok(log) = fs::read_to_string(runtime_dir.join("kitty.log")) else {
        return;
    };
    let ignored = summarize_ignored_lines(&log);
    if ignored.is_empty() {
        return;
    }
    eprintln!(
        "sat-hx-ide: kitty ignored {} config line(s); a setting in the merged kitty.conf \
         is likely newer than the installed kitty (details: {}):",
        ignored.len(),
        runtime_dir.join("kitty.log").display()
    );
    for line in &ignored {
        eprintln!("  {line}");
    }
}

/// Extract the `Ignoring ...` lines from a kitty log, without timestamps.
fn summarize_ignored_lines(log: &str) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for line in log.lines().filter(|line| line.contains("Ignoring")) {
        let message = line.rsplit("] ").next().unwrap_or(line).trim();
        if !seen.iter().any(|existing| existing == message) {
            seen.push(message.to_string());
        }
    }
    seen
}

impl SessionBackend for KittyBackend {
    fn name(&self) -> &'static str {
        "kitty"
    }

    fn session_name(&self) -> Result<String> {
        Ok(std::env::var(SESSION_ENV).unwrap_or_else(|_| "session".to_string()))
    }

    fn current_pane_id(&self) -> Result<PaneId> {
        let value = std::env::var("KITTY_WINDOW_ID").context("KITTY_WINDOW_ID is not set")?;
        value
            .trim()
            .parse::<u64>()
            .with_context(|| format!("Invalid KITTY_WINDOW_ID '{value}'"))
    }

    fn panes(&self) -> Result<Vec<super::PaneInfo>> {
        let zoomed = self.read_zoomed_role();
        self.client().panes(zoomed.as_deref())
    }

    fn spawn(&self, spec: &SpawnSpec<'_>) -> Result<PaneId> {
        self.launch(spec)
    }

    fn spawn_blocking(&self, spec: &SpawnSpec<'_>) -> Result<SpawnOutcome> {
        // kitty closes the overlay window when its program exits, so blocking
        // means waiting for the window to disappear. The program's exit
        // status is not observable through the remote-control protocol.
        let id = self.launch(spec)?;
        self.wait_for_window_close(id)?;
        Ok(SpawnOutcome {
            success: true,
            stderr: String::new(),
        })
    }

    fn focus(&self, id: PaneId) -> Result<()> {
        self.client().run_ok_args(&[
            "focus-window".into(),
            "--match".into(),
            format!("id:{id}").into(),
        ])
    }

    fn rename(&self, id: PaneId, title: &str) -> Result<()> {
        // The display title and the runtime role both move: pane lookups
        // prefer the SAT_ROLE user var over the immutable launch env.
        let client = self.client();
        client.run_ok_args(&[
            "set-window-title".into(),
            "--match".into(),
            format!("id:{id}").into(),
            title.into(),
        ])?;
        client.run_ok_args(&[
            "set-user-vars".into(),
            "--match".into(),
            format!("id:{id}").into(),
            format!("{ROLE_VAR}={title}").into(),
        ])
    }

    fn close(&self, id: PaneId) -> Result<()> {
        self.client().run_ok_args(&[
            "close-window".into(),
            "--match".into(),
            format!("id:{id}").into(),
        ])
    }

    fn toggle_fullscreen(&self, id: PaneId) -> Result<()> {
        // Stack layout shows only the tab's active window; leaving stack
        // restores the splits geometry exactly (verified in the kitty
        // spike). Which window is "the stacked one" cannot be derived from
        // `ls` when a helper overlay holds focus, so the backend records
        // the zoomed role in the session's runtime state.
        let client = self.client();
        let mut target_tab = None;
        let mut stacked = false;
        let mut role = String::new();
        for os_window in &client.ls()? {
            for tab in &os_window.tabs {
                let Some(window) = tab.windows.iter().find(|w| w.id == id) else {
                    continue;
                };
                target_tab = Some(tab.id);
                stacked = tab.layout == "stack";
                role = window
                    .user_vars
                    .get(ROLE_VAR)
                    .or_else(|| window.env.get(ROLE_ENV))
                    .unwrap_or(&window.title)
                    .clone();
            }
        }
        let Some(tab) = target_tab else {
            bail!("Cannot toggle fullscreen: window {id} not found");
        };
        let zoomed_role = self.read_zoomed_role();
        let restoring = stacked && zoomed_role.as_deref() == Some(role.as_str());
        if restoring {
            self.write_zoomed_role(None)?;
        } else {
            self.write_zoomed_role(Some(&role))?;
            self.focus(id)?;
        }
        let layout = if restoring { "splits" } else { "stack" };
        client.run_ok_args(&[
            "goto-layout".into(),
            "--match".into(),
            format!("id:{tab}").into(),
            layout.into(),
        ])
    }

    fn set_floating_geometry(
        &self,
        _id: PaneId,
        _geometry: &super::FloatGeometry<'_>,
    ) -> Result<()> {
        // Overlay windows cover the focused window; kitty has no free
        // geometry for floating panes, so re-floating is already done.
        Ok(())
    }

    fn move_pane(&self, _id: PaneId, _direction: Direction) -> Result<()> {
        // kitty has no attach-window; docking placement is decided at
        // spawn time via --location (docked panes sit beside the editor).
        Ok(())
    }

    fn resize_to_share(
        &self,
        pane_id: PaneId,
        editor_id: PaneId,
        axis: Axis,
        _border: &str,
        target_percent: u8,
    ) -> Result<()> {
        // kitty takes an exact cell count, so request the full remaining
        // delta in one call. kitty may still clamp the applied increment
        // (neighbors must be able to absorb it), so re-measure until the
        // target is reached or no progress is left.
        const MAX_ATTEMPTS: usize = 5;
        let client = self.client();
        let mut last_current = usize::MAX;
        for _ in 0..MAX_ATTEMPTS {
            let panes = client.panes(self.read_zoomed_role().as_deref())?;
            let (Some(pane), Some(_editor)) = (
                panes
                    .iter()
                    .find(|pane| !pane.is_plugin && pane.id == pane_id),
                panes
                    .iter()
                    .find(|pane| !pane.is_plugin && pane.id == editor_id),
            ) else {
                return Ok(());
            };
            let total = axis.measure(pane) + axis.measure(_editor);
            if total == 0 {
                return Ok(());
            }
            let target = (total * usize::from(target_percent) / 100).max(1);
            let current = axis.measure(pane);
            if current.abs_diff(target) <= 1 {
                return Ok(());
            }
            self.resize_by(&client, pane_id, axis, target as i32 - current as i32)?;
            if current == last_current {
                log::warn!("resize of pane {pane_id} made no progress; giving up");
                return Ok(());
            }
            last_current = current;
        }
        Ok(())
    }

    fn send_text(&self, id: PaneId, text: &str) -> Result<()> {
        self.client().run_ok_args(&[
            "send-text".into(),
            "--match".into(),
            format!("id:{id}").into(),
            text.into(),
        ])
    }

    fn send_key(&self, id: PaneId, key: Key) -> Result<()> {
        self.client().run_ok_args(&[
            "send-key".into(),
            "--match".into(),
            format!("id:{id}").into(),
            match key {
                Key::Escape => "esc",
                Key::Enter => "enter",
            }
            .into(),
        ])
    }

    fn helper_waits_for_spawned_pane(&self) -> bool {
        false
    }

    fn focus_previous_pane(&self) -> Result<()> {
        // kitty has no focus-previous command; the status-bar guard bounces
        // focus to the editor pane instead.
        self.client().run_ok_args(&[
            "focus-window".into(),
            "--match".into(),
            format!("env:{ROLE_ENV}=editor").into(),
        ])
    }

    fn init_session(&self, init: &SessionInit<'_>) -> Result<()> {
        let (session_path, config_path) = self.init_session_files(init)?;
        let socket = init.runtime_dir.join("kitty.sock");
        let client = KittyClient::new(self.kitten.clone(), socket.clone());

        if client.probe() == SessionStatus::Active {
            if !init.attach_existing {
                bail!(
                    "Session '{}' already exists; enable session.attach_existing \
                     or choose another --session name",
                    init.session_name
                );
            }
            // Attach = raise the existing kitty window.
            return client.run_ok_args(&[
                "focus-window".into(),
                "--match".into(),
                format!("env:{ROLE_ENV}=editor").into(),
            ]);
        }

        // A stale socket file from a dead instance must not block the new
        // one's listen socket.
        let _ = fs::remove_file(&socket);

        let log = fs::File::create(init.runtime_dir.join("kitty.log")).with_context(|| {
            format!(
                "Failed to create kitty log in {}",
                init.runtime_dir.display()
            )
        })?;
        // The child handle is dropped immediately: kitty keeps running,
        // detached, and the helper exits after the layout fixups below.
        spawn_kitty(
            &self.kitty,
            &config_path,
            &socket,
            &session_path,
            init.project_dir,
            log,
        )?;

        // kitty opens its socket asynchronously; poll until the remote
        // control answers.
        let deadline = Instant::now() + SOCKET_READY_TIMEOUT;
        loop {
            if client.probe() == SessionStatus::Active {
                break;
            }
            if Instant::now() > deadline {
                bail!(
                    "kitty did not open its remote-control socket at {} within {}s",
                    socket.display(),
                    SOCKET_READY_TIMEOUT.as_secs()
                );
            }
            std::thread::sleep(POLL_INTERVAL);
        }

        // Session files cannot express pane sizes portably (no --bias on
        // older kitty), so bring the layout to its configured geometry after
        // startup, over the remote-control socket.
        self.apply_initial_layout(&client, init);
        report_ignored_config_lines(init.runtime_dir);
        client.run_ok_args(&[
            "focus-window".into(),
            "--match".into(),
            format!("env:{ROLE_ENV}=editor").into(),
        ])
    }

    fn session_status(&self, name: &str) -> Result<SessionStatus> {
        Ok(self.client_for(name).probe())
    }

    fn delete_session(&self, name: &str) -> Result<()> {
        // No exited-session concept: quitting the instance is the delete.
        self.client_for(name)
            .run_ok_args(&["action".into(), "quit".into()])
    }
}

/// Spawn the dedicated kitty instance detached, logging stderr to the
/// runtime dir.
fn spawn_kitty(
    kitty: &Path,
    config: &Path,
    socket: &Path,
    session: &Path,
    project_dir: &Path,
    log: fs::File,
) -> Result<std::process::Child> {
    use std::process::{Command, Stdio};
    Command::new(kitty)
        .current_dir(project_dir)
        .arg("--config")
        .arg(config)
        .arg("--listen-on")
        .arg(format!("unix:{}", socket.display()))
        .arg("--override")
        .arg("allow_remote_control=socket-only")
        .arg("--session")
        .arg(session)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()
        .with_context(|| format!("Failed to start {}", kitty.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{FloatGeometry, Placement};

    fn spec<'a>(
        title: &'a str,
        placement: Placement<'a>,
        program: Option<&'a std::path::Path>,
    ) -> SpawnSpec<'a> {
        SpawnSpec {
            title,
            tab_id: 2,
            placement,
            close_on_exit: true,
            cwd: None,
            program,
            args: &[],
        }
    }

    fn strings(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn floating_spawn_is_an_overlay_with_role_and_float_marker() {
        let geometry = FloatGeometry {
            x: "10%",
            y: "20%",
            width: "80%",
            height: "60%",
        };
        let args = KittyBackend::launch_args(
            &spec("sat-venv-selector", Placement::Floating(geometry), None),
            true,
        );
        assert_eq!(
            strings(&args),
            vec![
                "launch",
                "--match",
                "id:2",
                "--type=overlay",
                "--env",
                "SAT_FLOAT_WINDOW=1",
                "--env",
                "SAT_HX_ROLE=sat-venv-selector",
                "--title",
                "sat-venv-selector",
            ]
        );
    }

    #[test]
    fn tiled_beside_spawn_splits_toward_the_dock() {
        let args = KittyBackend::launch_args(
            &spec("terminal", Placement::TiledBeside(Direction::Down), None),
            true,
        );
        assert_eq!(
            strings(&args),
            vec![
                "launch",
                "--match",
                "id:2",
                "--location=hsplit",
                "--env",
                "SAT_HX_ROLE=terminal",
                "--title",
                "terminal",
            ]
        );
    }

    #[test]
    fn plain_tiled_spawn_docks_left_of_the_editor() {
        let args = KittyBackend::launch_args(&spec("file-manager", Placement::Tiled, None), true);
        assert_eq!(
            strings(&args),
            vec![
                "launch",
                "--match",
                "id:2",
                "--location=before",
                "--env",
                "SAT_HX_ROLE=file-manager",
                "--title",
                "file-manager",
            ]
        );
    }

    #[test]
    fn spawn_passes_program_and_args() {
        let program = std::path::Path::new("/usr/bin/lazygit");
        let args: Vec<OsString> = vec!["-p".into(), "/some dir".into()];
        let s = SpawnSpec {
            title: "git",
            tab_id: 1,
            placement: Placement::Tiled,
            close_on_exit: true,
            cwd: Some(std::path::Path::new("/proj")),
            program: Some(program),
            args: &args,
        };
        let out = strings(&KittyBackend::launch_args(&s, true));
        assert_eq!(
            out[..6],
            vec![
                "launch",
                "--match",
                "id:1",
                "--location=before",
                "--env",
                "SAT_HX_ROLE=git"
            ]
        );
        assert!(out.contains(&"SAT_HX_ROLE=git".to_string()));
        assert!(out.contains(&"--cwd".to_string()));
        assert!(out.contains(&"/proj".to_string()));
        assert!(out.ends_with(&[
            "/usr/bin/lazygit".to_string(),
            "-p".to_string(),
            "/some dir".to_string()
        ]));
    }

    #[test]
    fn summarizes_ignored_config_lines_without_timestamps_or_duplicates() {
        let log = "\
[292 10:42:23.805735] Ignoring unknown config key: font\n\
[292 10:42:23.805812] Ignoring unknown config key: window_border_radius\n\
[292 10:44:01.111111] Ignoring unknown config key: font\n\
[292 10:44:01.111112] some other log line\n";
        assert_eq!(
            summarize_ignored_lines(log),
            vec![
                "Ignoring unknown config key: font".to_string(),
                "Ignoring unknown config key: window_border_radius".to_string()
            ]
        );
        assert!(summarize_ignored_lines("no warnings here\n").is_empty());
    }
}
