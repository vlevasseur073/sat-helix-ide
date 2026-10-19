use crate::config::{expand_tilde, Config, LauncherPicker};
use crate::resolve::resolve_executable;
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Ask the user to pick a project directory (Yazi or fzf). Returns `None` if they cancel.
pub fn pick_project_dir(config: &Config) -> Result<Option<PathBuf>> {
    let start = expand_tilde(&config.launcher.start_dir);
    if !start.is_dir() {
        bail!(
            "launcher.start_dir '{}' is not a directory",
            start.display()
        );
    }

    match resolve_picker(config)? {
        PickerBackend::Yazi(command) => pick_with_yazi(&command, &start),
        PickerBackend::Fzf(command) => pick_with_fzf(&command, &start, config.launcher.max_depth),
    }
}

enum PickerBackend {
    Yazi(PathBuf),
    Fzf(PathBuf),
}

fn resolve_picker(config: &Config) -> Result<PickerBackend> {
    let yazi = resolve_executable(&config.tools.file_manager.command).ok();
    let fzf = resolve_executable(&config.launcher.fzf_command).ok();

    match config.launcher.picker {
        LauncherPicker::Yazi => {
            let command = yazi.with_context(|| {
                format!(
                    "Cannot find file manager '{}' for project picker",
                    config.tools.file_manager.command
                )
            })?;
            Ok(PickerBackend::Yazi(command))
        }
        LauncherPicker::Fzf => {
            let command = fzf.with_context(|| {
                format!(
                    "Cannot find fzf command '{}' for project picker",
                    config.launcher.fzf_command
                )
            })?;
            Ok(PickerBackend::Fzf(command))
        }
        LauncherPicker::Auto => {
            if let Some(command) = yazi {
                return Ok(PickerBackend::Yazi(command));
            }
            if let Some(command) = fzf {
                return Ok(PickerBackend::Fzf(command));
            }
            bail!(
                "No project picker available: install {} (recommended) or {}, then run `sat-hx-ide doctor`",
                config.tools.file_manager.command,
                config.launcher.fzf_command
            );
        }
    }
}

fn pick_with_yazi(yazi: &Path, start: &Path) -> Result<Option<PathBuf>> {
    let chooser_file = std::env::temp_dir().join(format!(
        "sat-hx-ide-picker-chooser.{}.{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let config_dir = prepare_picker_yazi_config()?;
    let _ = fs::remove_file(&chooser_file);

    eprintln!("Select a project folder and press Enter to open it (Esc to cancel).");

    let status = Command::new(yazi)
        .env("YAZI_CONFIG_HOME", &config_dir)
        .arg(start)
        .arg("--chooser-file")
        .arg(&chooser_file)
        .status()
        .with_context(|| format!("Failed to launch {}", yazi.display()))?;

    let _ = fs::remove_dir_all(&config_dir);

    if !status.success() {
        let _ = fs::remove_file(&chooser_file);
        return Ok(None);
    }

    let picked = read_chooser_project_dir(&chooser_file)?;
    let _ = fs::remove_file(&chooser_file);
    match picked {
        Some(path) => Ok(Some(validate_project_dir(&path)?)),
        None => Ok(None),
    }
}

/// Isolated Yazi config so `--chooser-file` handles Enter instead of the user's open rules (e.g. Helix).
fn prepare_picker_yazi_config() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!(
        "sat-hx-ide-yazi-picker.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create {}", dir.display()))?;
    fs::write(
        dir.join("yazi.toml"),
        "[mgr]\nshow_hidden = true\n",
    )
    .with_context(|| format!("Failed to write {}", dir.join("yazi.toml").display()))?;
    Ok(dir)
}

fn pick_with_fzf(fzf: &Path, start: &Path, max_depth: u32) -> Result<Option<PathBuf>> {
    let depth = max_depth.to_string();
    let mut directory_listing = if let Ok(fd) = resolve_executable("fd") {
        Command::new(fd)
            .args([
                "--type",
                "d",
                "--hidden",
                "--exclude",
                ".git",
                "--max-depth",
                &depth,
            ])
            .arg(start)
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to list directories with fd")?
    } else {
        Command::new("find")
            .arg(start)
            .args(["-type", "d", "-maxdepth"])
            .arg(&depth)
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to list directories with find")?
    };

    let listing_stdout = directory_listing
        .stdout
        .take()
        .context("directory listing produced no output")?;

    let chooser = Command::new(fzf)
        .arg("--prompt=Project directory> ")
        .stdin(Stdio::from(listing_stdout))
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to launch {}", fzf.display()))?;

    let output = chooser
        .wait_with_output()
        .with_context(|| format!("Failed to run {}", fzf.display()))?;
    let _ = directory_listing.wait();

    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }

    let line = String::from_utf8(output.stdout)
        .context("fzf output was not valid UTF-8")?
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from);

    match line {
        Some(path) => Ok(Some(validate_project_dir(&path)?)),
        None => Ok(None),
    }
}

fn read_chooser_project_dir(path: &Path) -> Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let file = fs::File::open(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.context("Failed to read picker output")?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let candidate = expand_tilde(Path::new(trimmed));
        if candidate.is_dir() {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn validate_project_dir(path: &Path) -> Result<PathBuf> {
    let path = expand_tilde(path);
    let project_dir = fs::canonicalize(&path)
        .with_context(|| format!("Failed to resolve project path {}", path.display()))?;
    if !project_dir.is_dir() {
        bail!(
            "Project path '{}' is not a directory",
            project_dir.display()
        );
    }
    Ok(project_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn executable(path: &Path, body: &str) {
        fs::write(path, body).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn yazi_stub_writes_chooser_file() {
        let temp = tempfile::tempdir().unwrap();
        let start = temp.path().join("start");
        fs::create_dir(&start).unwrap();
        let project = start.join("my-project");
        fs::create_dir(&project).unwrap();

        let yazi = temp.path().join("yazi");
        executable(
            &yazi,
            r#"#!/bin/sh
start=""
while [ $# -gt 0 ]; do
  case "$1" in
    --chooser-file) file="$2"; shift 2 ;;
    *) start="$1"; shift ;;
  esac
done
printf '%s\n' "$start/my-project" > "$file"
"#,
        );

        let picked = pick_with_yazi(&yazi, &start).unwrap().unwrap();
        assert_eq!(picked, fs::canonicalize(&project).unwrap());
    }

    #[test]
    fn cancelled_yazi_returns_none() {
        let temp = tempfile::tempdir().unwrap();
        let start = temp.path().join("start");
        fs::create_dir(&start).unwrap();
        let yazi = temp.path().join("yazi");
        executable(&yazi, "#!/bin/sh\nexit 1\n");

        assert!(pick_with_yazi(&yazi, &start).unwrap().is_none());
    }
}
