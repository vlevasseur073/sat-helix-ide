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
        for command in ["hx", "yazi", "lazygit", "revdiff"] {
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

[tools.ai]
command = "{}"
"#,
                user_zellij.display(),
                zellij.display(),
                bin.join("hx").display(),
                bin.join("yazi").display(),
                bin.join("lazygit").display(),
                bin.join("revdiff").display(),
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

    fn init(&self, extra: &[&str]) -> assert_cmd::assert::Assert {
        let mut command = Command::cargo_bin("sat-hx-ide").unwrap();
        command
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .args([
                "--config",
                self.app_config.to_str().unwrap(),
                "init",
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
