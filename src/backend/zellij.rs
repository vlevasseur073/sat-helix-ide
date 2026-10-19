//! Zellij implementation of [`SessionBackend`].
//!
//! Every method shells out to the `zellij` binary, preserving the exact CLI
//! sequences the helpers used before the backend abstraction existed.

use super::{
    Axis, Direction, FloatGeometry, Key, PaneId, PaneInfo, Placement, SessionBackend, SessionInit,
    SessionStatus, SpawnOutcome, SpawnSpec,
};
use crate::zellij::{
    build_runtime_config, session_layout, RuntimeConfigInput, StatusBarLaunch,
};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

pub struct ZellijBackend {
    path: PathBuf,
}

impl ZellijBackend {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Zellij identifies panes on the CLI as `terminal_<id>`.
    fn cli_id(id: PaneId) -> String {
        format!("terminal_{id}")
    }

    fn action(&self) -> Command {
        let mut command = Command::new(&self.path);
        command.arg("action");
        command
    }

    /// Run a command to completion and require a success exit status.
    fn run_status(command: &mut Command) -> Result<()> {
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

    /// Build the `action new-pane` argument stream for a spawn request.
    fn new_pane_command(&self, spec: &SpawnSpec<'_>, blocking: bool) -> Command {
        let mut command = self.action();
        command.arg("new-pane");
        if spec.close_on_exit {
            command.arg("--close-on-exit");
        }
        if blocking {
            command.arg("--block-until-exit");
        }
        match spec.placement {
            Placement::Floating(geometry) => {
                command
                    .arg("--floating")
                    .args(["--x", geometry.x, "--y", geometry.y])
                    .arg("--width")
                    .arg(geometry.width)
                    .arg("--height")
                    .arg(geometry.height);
            }
            Placement::Tiled => {}
            Placement::TiledBeside(direction) => {
                command
                    .arg("--direction")
                    .arg(direction.name());
            }
        }
        command
            .arg("--name")
            .arg(spec.title)
            .arg("--tab-id")
            .arg(spec.tab_id.to_string());
        if let Some(cwd) = spec.cwd {
            command.arg("--cwd").arg(cwd);
        }
        if let Some(program) = spec.program {
            command.arg("--").arg(program).args(spec.args);
        }
        command
    }

    /// Zellij prints the new pane id (as `terminal_<n>` or a bare number)
    /// before registering the pane.
    fn parse_pane_id(value: &str) -> Result<PaneId> {
        value
            .trim()
            .trim_start_matches("terminal_")
            .parse()
            .with_context(|| format!("Invalid pane ID returned by Zellij: '{value}'"))
    }

    /// Zellij env vars are only meaningful inside a Zellij pane; scrub them so
    /// launching from an existing session cannot pin the new session name.
    fn scrub_session_env(command: &mut Command) {
        command
            .env_remove("ZELLIJ")
            .env_remove("ZELLIJ_SESSION_NAME")
            .env_remove("ZELLIJ_CONFIG_FILE")
            .env_remove("ZELLIJ_CONFIG_DIR")
            .env_remove("ZELLIJ_AUTO_ATTACH")
            .env_remove("ZELLIJ_AUTO_EXIT");
    }

    fn attach(&self, name: &str, config: &Path) -> Result<()> {
        let mut command = Command::new(&self.path);
        command.arg("--config").arg(config).arg("attach").arg(name);
        Self::scrub_session_env(&mut command);
        Self::run_status(&mut command).context("Failed to attach to existing Zellij session")
    }

    /// Share of the split (pane + editor) taken by `pane`, as an integer
    /// percent; `None` when either pane is gone or the split is empty.
    fn pane_share(&self, pane_id: PaneId, editor_id: PaneId, axis: Axis) -> Option<usize> {
        let panes = self.panes().ok()?;
        let pane = panes
            .iter()
            .find(|pane| !pane.is_plugin && pane.id == pane_id)?;
        let editor = panes
            .iter()
            .find(|pane| !pane.is_plugin && pane.id == editor_id)?;
        let total = axis.measure(pane) + axis.measure(editor);
        (total > 0).then(|| axis.measure(pane) * 100 / total)
    }

    /// One Zellij resize nudge: coarse `+`/`-` when far from the target,
    /// fine border `increase`/`decrease` when close.
    fn resize_step(
        &self,
        pane_id: PaneId,
        shrink: bool,
        coarse: bool,
        border: &str,
    ) -> Result<()> {
        let mut command = self.action();
        command
            .args(["resize", "--pane-id"])
            .arg(Self::cli_id(pane_id));
        match (coarse, shrink) {
            (true, true) => {
                command.arg("-");
            }
            (true, false) => {
                command.arg("+");
            }
            (false, true) => {
                command.arg("decrease").arg(border);
            }
            (false, false) => {
                command.arg("increase").arg(border);
            }
        }
        Self::run_status(&mut command)
    }

    fn create(&self, name: &str, layout: &Path, config: &Path, project_dir: &Path) -> Result<()> {
        let mut command = Command::new(&self.path);
        command
            .current_dir(project_dir)
            .arg("--config")
            .arg(config)
            .arg("--session")
            .arg(name)
            .arg("--new-session-with-layout")
            .arg(layout);
        Self::scrub_session_env(&mut command);
        Self::run_status(&mut command).context("Failed to create Zellij session")
    }
}

impl SessionBackend for ZellijBackend {
    fn name(&self) -> &'static str {
        "zellij"
    }

    fn session_name(&self) -> Result<String> {
        Ok(std::env::var("ZELLIJ_SESSION_NAME").unwrap_or_else(|_| "session".to_string()))
    }

    fn current_pane_id(&self) -> Result<PaneId> {
        let value = std::env::var("ZELLIJ_PANE_ID").context("ZELLIJ_PANE_ID is not set")?;
        Self::parse_pane_id(&value)
    }

    fn panes(&self) -> Result<Vec<PaneInfo>> {
        let output = Command::new(&self.path)
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

    fn spawn(&self, spec: &SpawnSpec<'_>) -> Result<PaneId> {
        let mut command = self.new_pane_command(spec, false);
        let output = command
            .output()
            .with_context(|| format!("Failed to create the '{}' pane", spec.title))?;
        if !output.status.success() {
            bail!(
                "Failed to create the '{}' pane: {}",
                spec.title,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let pane_id = String::from_utf8(output.stdout)
            .map(|id| id.trim().to_string())
            .context("Zellij returned a non-UTF-8 pane ID")?;
        Self::parse_pane_id(&pane_id)
    }

    fn spawn_blocking(&self, spec: &SpawnSpec<'_>) -> Result<SpawnOutcome> {
        let mut command = self.new_pane_command(spec, true);
        let output = command
            .output()
            .context("Failed to spawn the '{}' pane")?;
        Ok(SpawnOutcome {
            success: output.status.success(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    fn focus(&self, id: PaneId) -> Result<()> {
        let mut command = self.action();
        command.arg("focus-pane-id").arg(Self::cli_id(id));
        Self::run_status(&mut command)
    }

    fn rename(&self, id: PaneId, title: &str) -> Result<()> {
        let mut command = self.action();
        command
            .args(["rename-pane", "--pane-id"])
            .arg(Self::cli_id(id))
            .arg(title);
        Self::run_status(&mut command)
    }

    fn close(&self, id: PaneId) -> Result<()> {
        let mut command = self.action();
        command.args(["close-pane", "--pane-id"]).arg(Self::cli_id(id));
        Self::run_status(&mut command)
    }

    fn toggle_fullscreen(&self, id: PaneId) -> Result<()> {
        let mut command = self.action();
        command
            .args(["toggle-fullscreen", "--pane-id"])
            .arg(Self::cli_id(id));
        Self::run_status(&mut command)
    }

    fn set_floating_geometry(&self, id: PaneId, geometry: &FloatGeometry<'_>) -> Result<()> {
        let mut command = self.action();
        command
            .args(["change-floating-pane-coordinates", "--pane-id"])
            .arg(Self::cli_id(id))
            .args(["--x", geometry.x, "--y", geometry.y])
            .arg("--width")
            .arg(geometry.width)
            .arg("--height")
            .arg(geometry.height);
        Self::run_status(&mut command)
    }

    fn move_pane(&self, id: PaneId, direction: Direction) -> Result<()> {
        let mut command = self.action();
        command
            .args(["move-pane", "--pane-id"])
            .arg(Self::cli_id(id))
            .arg(direction.name());
        Self::run_status(&mut command)
    }

    fn resize_to_share(
        &self,
        pane_id: PaneId,
        editor_id: PaneId,
        axis: Axis,
        border: &str,
        target_percent: u8,
    ) -> Result<()> {
        // Zellij has no exact resize: panes can only be nudged in coarse
        // steps (`+`/`-`, ≈5%) or fine border steps (`increase`/`decrease`).
        // Nudge until the pane is close enough to the requested share of
        // the space it splits with the editor, always reading fresh
        // geometry — measurements go stale mid-loop otherwise.
        const MAX_STEPS: usize = 100;
        const TOLERANCE_PERCENT: usize = 1;
        /// Switch to coarse steps when more than this many points away.
        const COARSE_THRESHOLD: usize = 8;

        let target = usize::from(target_percent);
        for _ in 0..MAX_STEPS {
            let Some(current) = self.pane_share(pane_id, editor_id, axis) else {
                return Ok(());
            };
            let diff = current.abs_diff(target);
            if diff <= TOLERANCE_PERCENT {
                return Ok(());
            }

            let shrink = current > target;
            let coarse = diff > COARSE_THRESHOLD;
            self.resize_step(pane_id, shrink, coarse, border)?;

            // Stop if a step overshoots the target (common with coarse `-`/`+`).
            let Some(after) = self.pane_share(pane_id, editor_id, axis) else {
                return Ok(());
            };
            if after.abs_diff(target) <= TOLERANCE_PERCENT {
                return Ok(());
            }
            if after.abs_diff(target) >= diff && !coarse {
                // Undo the fine step and accept the closest reachable size.
                let _ = self.resize_step(pane_id, !shrink, false, border);
                return Ok(());
            }
        }

        Ok(())
    }

    fn send_text(&self, id: PaneId, text: &str) -> Result<()> {
        let mut command = self.action();
        command
            .args(["write-chars", "--pane-id"])
            .arg(Self::cli_id(id))
            .arg(text);
        Self::run_status(&mut command)
    }

    fn send_key(&self, id: PaneId, key: Key) -> Result<()> {
        let mut command = self.action();
        command
            .args(["send-keys", "--pane-id"])
            .arg(Self::cli_id(id))
            .arg(match key {
                Key::Escape => "Esc",
                Key::Enter => "Enter",
            });
        Self::run_status(&mut command)
    }

    fn helper_waits_for_spawned_pane(&self) -> bool {
        true
    }

    fn focus_previous_pane(&self) -> Result<()> {
        let session = self.session_name()?;
        let mut command = Command::new(&self.path);
        command
            .arg("--session")
            .arg(session)
            .args(["action", "focus-previous-pane"])
            .stdin(Stdio::null());
        Self::run_status(&mut command)
    }

    fn init_session(&self, init: &SessionInit<'_>) -> Result<()> {
        let runtime_dir = init.runtime_dir;
        fs::create_dir_all(runtime_dir)?;
        let layout_path = runtime_dir.join("layout.kdl");
        let config_path = runtime_dir.join("config.kdl");
        let status_launch = StatusBarLaunch {
            executable: init.executable,
            config_path: init.app_config,
            session_name: init.session_name,
        };
        let layout = session_layout(
            init.editor,
            init.editor_path,
            init.ai,
            init.terminal,
            init.mindmap,
            init.project_dir,
            init.native_status_bar,
            status_launch,
        );
        fs::write(&layout_path, layout)?;
        build_runtime_config(&RuntimeConfigInput {
            source: init.user_config,
            destination: &config_path,
            session_name: init.session_name,
            keys: init.keybindings,
            executable: init.executable,
            app_config: init.app_config,
            git_command: init.git_path,
            git_args: init.git_args,
            project_dir: init.project_dir,
            float_width: init.float_width,
            float_height: init.float_height,
        })?;

        match self.session_status(init.session_name)? {
            SessionStatus::Active => {
                if init.attach_existing {
                    self.attach(init.session_name, &config_path)
                } else {
                    bail!(
                        "Session '{}' already exists; enable session.attach_existing \
                         or choose another --session name",
                        init.session_name
                    )
                }
            }
            SessionStatus::Exited => {
                // Zellij resurrection replays serialized pane commands, which can
                // drift from the configured layout (for example after Helix spawns
                // an LSP child). Always recreate exited sessions from layout.kdl.
                self.delete_session(init.session_name)?;
                self.create(
                    init.session_name,
                    &layout_path,
                    &config_path,
                    init.project_dir,
                )
            }
            SessionStatus::NotFound => self.create(
                init.session_name,
                &layout_path,
                &config_path,
                init.project_dir,
            ),
        }
    }

    fn session_status(&self, name: &str) -> Result<SessionStatus> {
        let output = Command::new(&self.path)
            .args(["list-sessions", "--no-formatting"])
            .output()
            .context("Failed to list sessions")?;

        // Zellij exits non-zero when no sessions exist.
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some((session_name, status)) = parse_session_line(line) {
                if session_name == name {
                    return Ok(status);
                }
            }
        }
        Ok(SessionStatus::NotFound)
    }

    fn delete_session(&self, name: &str) -> Result<()> {
        let mut command = Command::new(&self.path);
        command.arg("delete-session").arg(name);
        Self::run_status(&mut command)
    }
}

fn parse_session_line(line: &str) -> Option<(&str, SessionStatus)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with("No active zellij sessions") {
        return None;
    }

    let boundary = line.find([' ', '['])?;
    let session_name = &line[..boundary];
    let status = if line.contains("(EXITED") {
        SessionStatus::Exited
    } else {
        SessionStatus::Active
    };
    Some((session_name, status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_active_and_exited_session_lines() {
        assert_eq!(
            parse_session_line("sat-helix-ide [Created 1s ago]"),
            Some(("sat-helix-ide", SessionStatus::Active))
        );
        assert_eq!(
            parse_session_line("sat-helix-ide [Created 9m ago] (EXITED - attach to resurrect)"),
            Some(("sat-helix-ide", SessionStatus::Exited))
        );
        assert!(parse_session_line("No active zellij sessions found.").is_none());
    }

    #[test]
    fn parses_pane_id_accepts_cli_form() {
        assert_eq!(ZellijBackend::parse_pane_id("terminal_4").unwrap(), 4);
        assert_eq!(ZellijBackend::parse_pane_id("42").unwrap(), 42);
    }

    #[test]
    fn parses_pane_id_rejects_invalid_values() {
        assert!(ZellijBackend::parse_pane_id("terminal_").is_err());
        assert!(ZellijBackend::parse_pane_id("not-a-number").is_err());
    }

    #[test]
    fn cli_id_formats_terminal_prefix() {
        assert_eq!(ZellijBackend::cli_id(7), "terminal_7");
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
        assert_eq!(ZellijBackend::cli_id(panes[0].id), "terminal_4");
        assert!(panes[0].is_floating);
    }
}
