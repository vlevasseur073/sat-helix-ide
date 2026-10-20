//! Freedesktop launcher files (Linux).

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub const APP_ID: &str = "sat-helix-ide";
const LOGO_SVG: &str = include_str!("../book/src/assets/logo.svg");

/// Install `~/.local/share/applications/sat-helix-ide.desktop` and the app icon.
pub fn install_desktop_entry(exec: Option<&Path>) -> Result<()> {
    if !cfg!(target_os = "linux") {
        anyhow::bail!("install-desktop is only supported on Linux");
    }
    let exec = match exec {
        Some(path) => path.to_path_buf(),
        None => std::env::current_exe().context("Cannot locate sat-hx-ide executable")?,
    };

    let data_home = xdg_data_home();
    let applications = data_home.join("applications");
    let icon_dir = data_home
        .join("icons")
        .join("hicolor")
        .join("scalable")
        .join("apps");
    let desktop_path = applications.join(format!("{APP_ID}.desktop"));
    let icon_path = icon_dir.join(format!("{APP_ID}.svg"));

    fs::create_dir_all(&applications)
        .with_context(|| format!("Failed to create {}", applications.display()))?;
    fs::create_dir_all(&icon_dir)
        .with_context(|| format!("Failed to create {}", icon_dir.display()))?;

    fs::write(&icon_path, LOGO_SVG)
        .with_context(|| format!("Failed to write {}", icon_path.display()))?;

    let desktop = render_desktop_entry(&exec);
    fs::write(&desktop_path, desktop)
        .with_context(|| format!("Failed to write {}", desktop_path.display()))?;

    println!("Installed desktop entry: {}", desktop_path.display());
    println!("Installed icon: {}", icon_path.display());
    println!("Executable: {}", exec.display());
    println!(
        "Launch from your application menu, or run: {}",
        exec.display()
    );
    Ok(())
}

fn render_desktop_entry(exec: &Path) -> String {
    let exec = exec.display();
    format!(
        r#"[Desktop Entry]
Type=Application
Name=sat-hx-ide
GenericName=Helix IDE Workspace
Comment=Open a Helix-centered workspace for a project
Exec={exec}
Icon={APP_ID}
Terminal=true
Categories=Development;IDE;
Keywords=helix;zellij;ide;editor;
StartupNotify=true
"#
    )
}

fn xdg_data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home::home_dir().map(|home| home.join(".local").join("share")))
        .unwrap_or_else(|| PathBuf::from(".local/share"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_entry_points_at_executable() {
        let entry = render_desktop_entry(Path::new("/opt/bin/sat-hx-ide"));
        assert!(entry.contains("Exec=/opt/bin/sat-hx-ide"));
        assert!(entry.contains("Icon=sat-helix-ide"));
        assert!(entry.contains("Terminal=true"));
    }

    #[test]
    fn logo_is_embedded() {
        assert!(LOGO_SVG.contains("<svg"));
    }
}
