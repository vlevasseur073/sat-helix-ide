#![cfg(unix)]

use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

/// A workspace whose tools are stub scripts, so `init` can run end to end
/// without a terminal. `ai` is only put on PATH when `with_ai` is set.
struct Fixture {
    temp: tempfile::TempDir,
    project: std::path::PathBuf,
    runtime: std::path::PathBuf,
    capture: std::path::PathBuf,
    app_config: std::path::PathBuf,
    user_zellij: std::path::PathBuf,
}

impl Fixture {
    fn new(with_ai: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        let bin = temp.path().join("bin");
        let runtime = temp.path().join("runtime");
        let capture = temp.path().join("zellij-args");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&bin).unwrap();

        let zellij = bin.join("zellij");
        executable(
            &zellij,
            &format!(
                r#"#!/bin/sh
if [ "$1" = "list-sessions" ]; then
    exit 1
fi
printf '%s\n' "$@" > '{}'
"#,
                capture.display()
            ),
        );
        for command in ["hx", "yazi", "lazygit", "revdiff", "glab-tui"] {
            executable(&bin.join(command), "#!/bin/sh\nexit 0\n");
        }
        if with_ai {
            executable(&bin.join("agent"), "#!/bin/sh\nexit 0\n");
        }

        let user_zellij = temp.path().join("user-config.kdl");
        fs::write(&user_zellij, "theme \"dracula\"\n").unwrap();
        let app_config = temp.path().join("sat.toml");
        fs::write(
            &app_config,
            format!(
                r#"
[session]
zellij_config = "{}"

[tools.zellij]
command = "{}"

[tools.editor]
command = "{}"

[tools.file_manager]
command = "{}"

[tools.git]
command = "{}"

[tools.review]
command = "{}"

[tools.workflow]
command = "{}"

[tools.ai]
command = "{}"
"#,
                user_zellij.display(),
                zellij.display(),
                bin.join("hx").display(),
                bin.join("yazi").display(),
                bin.join("lazygit").display(),
                bin.join("revdiff").display(),
                bin.join("glab-tui").display(),
                bin.join("agent").display(),
            ),
        )
        .unwrap();

        Self {
            temp,
            project,
            runtime,
            capture,
            app_config,
            user_zellij,
        }
    }

    fn set_passthrough(&self, keys: &str) {
        let mut config = fs::read_to_string(&self.app_config).unwrap();
        config.push_str(&format!("\n[keybindings]\npassthrough = {keys}\n"));
        fs::write(&self.app_config, config).unwrap();
    }

    fn init(&self, extra: &[&str]) -> assert_cmd::assert::Assert {
        let mut command = Command::cargo_bin("sat-hx-ide").unwrap();
        command
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .args([
                "--config",
                self.app_config.to_str().unwrap(),
                self.project.to_str().unwrap(),
                "--session",
                "integration",
            ])
            .args(extra);
        command.assert()
    }

    fn layout(&self) -> String {
        fs::read_to_string(
            self.runtime
                .join("sat-helix-ide/integration")
                .join("layout.kdl"),
        )
        .unwrap()
    }
}

#[test]
fn init_preserves_user_config_and_launches_with_runtime_files() {
    let fixture = Fixture::new(false);
    fixture.init(&[]).success();

    assert_eq!(
        fs::read_to_string(&fixture.user_zellij).unwrap(),
        "theme \"dracula\"\n"
    );
    let merged = fs::read_to_string(
        fixture
            .runtime
            .join("sat-helix-ide/integration")
            .join("config.kdl"),
    )
    .unwrap();
    assert!(merged.contains("dracula"));
    assert!(merged.contains("bind \"Ctrl y\""));
    assert!(merged.contains("bind \"Alt y\""));
    assert!(merged.contains("bind \"Alt t\""));
    assert!(merged.contains("bind \"Alt Shift t\""));
    assert!(merged.contains("bind \"Alt g\""));
    assert!(!merged.contains("unbind"));

    let layout = fixture.layout();
    assert!(layout.contains("tab name=\"code\""));
    assert!(layout.contains("name=\"terminal\" size=\"15%\""));
    assert!(!layout.contains("yazi"));
    assert!(!layout.contains("lazygit"));

    let args = fs::read_to_string(&fixture.capture).unwrap();
    assert!(args.contains("--config"));
    assert!(args.contains("--new-session-with-layout"));
    assert!(args.contains("integration"));
    drop(fixture.temp);
}

#[test]
fn passthrough_keys_are_unbound_in_the_runtime_config() {
    let fixture = Fixture::new(false);
    fixture.set_passthrough("[\"Ctrl b\", \"Ctrl o\"]");
    fixture.init(&[]).success();

    let merged = fs::read_to_string(
        fixture
            .runtime
            .join("sat-helix-ide/integration")
            .join("config.kdl"),
    )
    .unwrap();
    assert!(merged.contains(r#"unbind "Ctrl b" "Ctrl o""#));
}

#[test]
fn ai_tab_is_added_by_default_when_the_agent_exists() {
    let fixture = Fixture::new(true);
    fixture.init(&[]).success();
    assert!(fixture.layout().contains("tab name=\"ai\""));
}

#[test]
fn missing_agent_drops_the_ai_tab_instead_of_failing() {
    let fixture = Fixture::new(false);
    fixture.init(&[]).success();
    assert!(!fixture.layout().contains("tab name=\"ai\""));
}

#[test]
fn no_ai_skips_the_tab_even_when_the_agent_exists() {
    let fixture = Fixture::new(true);
    fixture.init(&["--no-ai"]).success();
    assert!(!fixture.layout().contains("tab name=\"ai\""));
}

#[test]
fn doctor_reports_every_missing_required_tool() {
    let temp = tempfile::tempdir().unwrap();
    let app_config = temp.path().join("sat.toml");
    let prefix = "sat-hx-ide-doctor-missing-";
    fs::write(
        &app_config,
        format!(
            r#"
[tools.zellij]
command = "{prefix}zellij"

[tools.editor]
command = "{prefix}editor"

[tools.file_manager]
command = "{prefix}yazi"

[tools.git]
command = "{prefix}lazygit"

[tools.review]
command = "{prefix}revdiff"

[tools.workflow]
command = "{prefix}glab-tui"

[tools.mindmap]
command = "{prefix}shiki"
"#
        ),
    )
    .unwrap();

    let output = Command::cargo_bin("sat-hx-ide")
        .unwrap()
        .args(["--config", app_config.to_str().unwrap(), "doctor"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for label in [
        "✗ Zellij",
        "✗ Editor",
        "✗ File manager",
        "✗ Git client",
        "✗ Review",
        "✗ Workflow",
    ] {
        assert!(
            stdout.contains(label),
            "expected doctor output to contain {label:?}, got:\n{stdout}"
        );
    }
    assert!(stdout.contains("Summary: 6 required command(s)"));
}

/// Stub kitty + kitten so the kitty backend's init flow runs end to end.
/// The stub kitty creates the listen socket (a plain file) so the next
/// `kitten @ ls` probe succeeds, like a real instance opening its socket.
struct KittyFixture {
    temp: tempfile::TempDir,
    project: std::path::PathBuf,
    runtime: std::path::PathBuf,
    kitty_args: std::path::PathBuf,
    kitten_args: std::path::PathBuf,
    user_kitty: std::path::PathBuf,
    app_config: std::path::PathBuf,
}

impl KittyFixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        let bin = temp.path().join("bin");
        let runtime = temp.path().join("runtime");
        let kitty_args = temp.path().join("kitty-args");
        let kitten_args = temp.path().join("kitten-args");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&bin).unwrap();

        let kitty = bin.join("kitty");
        executable(
            &kitty,
            &format!(
                r#"#!/bin/sh
printf '%s\n' "$@" > '{kitty_args}'
listen=""
prev=""
for arg in "$@"; do
    if [ "$prev" = "--listen-on" ]; then listen="$arg"; fi
    prev="$arg"
done
case "$listen" in unix:*) touch "${{listen#unix:}}";; esac
echo "[0.000000] Ignoring unknown config key: font" >&2
exit 0
"#,
                kitty_args = kitty_args.display()
            ),
        );
        let kitten = bin.join("kitten");
        executable(
            &kitten,
            &format!(
                r#"#!/bin/sh
printf '%s\n' "$@" >> '{kitten_args}'
to="$3"
path="${{to#unix:}}"
[ -e "$path" ] || exit 1
case "$4" in
    ls)
        cat <<'JSON'
[{{ "tabs": [{{ "id": 1, "layout": "splits", "windows": [
    {{"id": 1, "title": "hx", "is_focused": true, "columns": 160, "lines": 20,
      "env": {{"SAT_HX_ROLE": "editor"}}, "user_vars": {{}}}},
    {{"id": 2, "title": "sh", "is_focused": false, "columns": 160, "lines": 10,
      "env": {{"SAT_HX_ROLE": "terminal"}}, "user_vars": {{}}}},
    {{"id": 3, "title": "bar", "is_focused": false, "columns": 160, "lines": 10,
      "env": {{"SAT_HX_ROLE": "statusbar"}}, "user_vars": {{}}}}
]}}]}}]
JSON
        ;;
    launch) echo 99 ;;
esac
exit 0
"#,
                kitten_args = kitten_args.display()
            ),
        );
        for command in ["hx", "yazi", "lazygit", "revdiff", "glab-tui"] {
            executable(&bin.join(command), "#!/bin/sh\nexit 0\n");
        }

        let user_kitty = temp.path().join("user-kitty.conf");
        fs::write(&user_kitty, "font_size 11\n").unwrap();
        let app_config = temp.path().join("sat.toml");
        fs::write(
            &app_config,
            format!(
                r#"
[session]
backend = "kitty"
kitty_config = "{user_kitty}"

[tools.kitty]
command = "{kitty}"

[tools.editor]
command = "{hx}"

[tools.file_manager]
command = "{yazi}"

[terminal]
dock_percent = 15
"#,
                user_kitty = user_kitty.display(),
                kitty = kitty.display(),
                hx = bin.join("hx").display(),
                yazi = bin.join("yazi").display(),
            ),
        )
        .unwrap();

        Self {
            temp,
            project,
            runtime,
            kitty_args,
            kitten_args,
            user_kitty,
            app_config,
        }
    }

    fn session_dir(&self, session: &str) -> std::path::PathBuf {
        self.runtime.join("sat-helix-ide").join(session)
    }

    fn init(&self, session: &str) {
        let mut command = Command::cargo_bin("sat-hx-ide").unwrap();
        let path = format!(
            "{}:{}",
            self.temp.path().join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        command
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .env("PATH", path)
            .args([
                "--config",
                self.app_config.to_str().unwrap(),
                "init",
                self.project.to_str().unwrap(),
                "--session",
                session,
            ])
            .assert()
            .success();
    }
}

#[test]
fn kitty_init_generates_runtime_files_and_launches_kitty() {
    let fixture = KittyFixture::new();
    fixture.init("kitty-session");

    let dir = fixture.session_dir("kitty-session");
    let session = fs::read_to_string(dir.join("session.conf")).unwrap();
    assert!(session.contains("--env SAT_HX_ROLE=editor --title editor"));
    assert!(session.contains("launch --location=hsplit --env SAT_HX_ROLE=terminal"));
    assert!(session.contains("SAT_HX_ROLE=statusbar"));

    let conf = fs::read_to_string(dir.join("kitty.conf")).unwrap();
    assert!(conf.contains(&format!("include {}", fixture.user_kitty.display())));
    assert!(conf.contains("env SAT_SESSION_NAME=kitty-session"));
    assert!(conf.contains("map ctrl+y launch --type=overlay --env SAT_HX_ROLE=file-manager"));
    // Every helper map must pass the config path with --config, or the
    // helper dies instantly on a stray positional argument.
    assert!(conf
        .lines()
        .filter(|line| line.starts_with("map ") && line.contains("launch"))
        .all(|line| line.contains("--config")));
    // Zellij-style quit parity.
    assert!(conf.contains("map ctrl+q quit"));

    let kitty_args = fs::read_to_string(&fixture.kitty_args).unwrap();
    assert!(kitty_args.contains("--listen-on"));
    assert!(kitty_args.contains("allow_remote_control=socket-only"));
    assert!(kitty_args.contains("--session"));
}

#[test]
fn kitty_init_docks_the_layout_after_startup() {
    let fixture = KittyFixture::new();
    fixture.init("dock");

    // The stub geometry never changes, so the backend keeps nudging the
    // docked panes toward their configured shares every pass.
    let kitten_args = fs::read_to_string(&fixture.kitten_args).unwrap();
    assert!(kitten_args.contains("resize-window"));
    assert!(kitten_args.contains("--axis"));
    // The editor is focused at the end of init.
    assert!(kitten_args.contains("focus-window"));
    assert!(kitten_args.contains("env:SAT_HX_ROLE=editor"));
}

#[test]
fn kitty_init_reports_ignored_config_lines() {
    let fixture = KittyFixture::new();
    // The stub kitty logs an ignored-config warning to its stderr, which
    // the backend captures in kitty.log and must surface after init.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sat-hx-ide"))
        .env("XDG_RUNTIME_DIR", &fixture.runtime)
        .env(
            "PATH",
            format!(
                "{}:{}",
                fixture.temp.path().join("bin").display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .args([
            "--config",
            fixture.app_config.to_str().unwrap(),
            "init",
            fixture.project.to_str().unwrap(),
            "--session",
            "warned",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("kitty ignored 1 config line(s)"),
        "expected the ignored-config warning, got:\n{stderr}"
    );
    assert!(stderr.contains("Ignoring unknown config key: font"));
    assert!(stderr.contains("kitty.log"));
}

#[test]
fn auto_backend_picks_kitty_inside_kitty_and_zellij_elsewhere() {
    let fixture = KittyFixture::new();
    // Both backends' binaries are on PATH: the zellij stub from the zellij
    // tests' layout and the kitty/kitten stubs from the fixture.
    let zellij = fixture.temp.path().join("bin").join("zellij");
    executable(
        &zellij,
        r#"#!/bin/sh
if [ "$1" = "list-sessions" ]; then exit 1; fi
exit 0
"#,
    );
    let path = format!(
        "{}:{}",
        fixture.temp.path().join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let app_config = fixture.temp.path().join("auto.toml");
    fs::write(
        &app_config,
        format!(
            r#"
[session]
backend = "auto"

[tools.zellij]
command = "{zellij}"

[tools.kitty]
command = "{kitty}"

[tools.editor]
command = "{hx}"

[tools.file_manager]
command = "{yazi}"
"#,
            zellij = zellij.display(),
            kitty = fixture.temp.path().join("bin/kitty").display(),
            hx = fixture.temp.path().join("bin/hx").display(),
            yazi = fixture.temp.path().join("bin/yazi").display(),
        ),
    )
    .unwrap();

    // Inside kitty (KITTY_WINDOW_ID set): the kitty backend generates
    // session.conf and launches the kitty stub.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sat-hx-ide"))
        .env("XDG_RUNTIME_DIR", &fixture.runtime)
        .env("PATH", &path)
        .env("KITTY_WINDOW_ID", "7")
        .args([
            "--config",
            app_config.to_str().unwrap(),
            "init",
            fixture.project.to_str().unwrap(),
            "--session",
            "auto-kitty",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fixture
        .session_dir("auto-kitty")
        .join("session.conf")
        .exists());

    // Outside kitty: the zellij backend generates layout.kdl.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sat-hx-ide"))
        .env("XDG_RUNTIME_DIR", &fixture.runtime)
        .env("PATH", &path)
        .env_remove("KITTY_WINDOW_ID")
        .args([
            "--config",
            app_config.to_str().unwrap(),
            "init",
            fixture.project.to_str().unwrap(),
            "--session",
            "auto-zellij",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fixture
        .session_dir("auto-zellij")
        .join("layout.kdl")
        .exists());
    assert!(!fixture
        .session_dir("auto-zellij")
        .join("session.conf")
        .exists());
}
