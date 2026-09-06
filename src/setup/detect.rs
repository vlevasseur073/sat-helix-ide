//! Host capability detection for the setup command.

use crate::setup::catalog::ToolId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PkgManager {
    Brew,
    Apt,
    Dnf,
    Pacman,
}

impl PkgManager {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Brew => "brew",
            Self::Apt => "apt-get",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
        }
    }

    pub fn install_args(self, package: &str) -> Vec<String> {
        match self {
            Self::Brew => vec!["install".into(), package.into()],
            Self::Apt => vec!["install".into(), "-y".into(), package.into()],
            Self::Dnf => vec!["install".into(), "-y".into(), package.into()],
            Self::Pacman => vec!["-S".into(), "--noconfirm".into(), package.into()],
        }
    }

    pub fn needs_sudo(self) -> bool {
        matches!(self, Self::Apt | Self::Dnf | Self::Pacman)
    }
}

#[derive(Debug, Clone)]
pub struct HostInfo {
    pub target: String,
    pub pkg_manager: Option<PkgManager>,
    pub has_cargo: bool,
    pub has_curl: bool,
    pub install_dir: std::path::PathBuf,
}

impl HostInfo {
    pub fn detect() -> anyhow::Result<Self> {
        let target = detect_target()?;
        let pkg_manager = detect_pkg_manager();
        let has_cargo = which::which("cargo").is_ok();
        let has_curl = which::which("curl").is_ok();
        let install_dir = std::env::var_os("INSTALL_DIR")
            .map(std::path::PathBuf::from)
            .or_else(|| home::home_dir().map(|h| h.join(".local/bin")))
            .unwrap_or_else(|| std::path::PathBuf::from("/usr/local/bin"));

        Ok(Self {
            target,
            pkg_manager,
            has_cargo,
            has_curl,
            install_dir,
        })
    }

    pub fn method_available(
        &self,
        tool: ToolId,
        method: crate::setup::catalog::InstallMethod,
    ) -> bool {
        use crate::setup::catalog::InstallMethod::*;
        if !tool.methods().contains(&method) {
            return false;
        }
        match method {
            Cargo => self.has_cargo && tool.cargo_crate().is_some(),
            Package => self
                .pkg_manager
                .and_then(|m| tool.package_name(m))
                .is_some(),
            Binary => {
                self.has_curl
                    && tool.github_repo().is_some()
                    && !tool.asset_matchers(&self.target).is_empty()
            }
        }
    }

    pub fn available_methods(&self, tool: ToolId) -> Vec<crate::setup::catalog::InstallMethod> {
        tool.methods()
            .iter()
            .copied()
            .filter(|m| self.method_available(tool, *m))
            .collect()
    }
}

pub fn detect_target() -> anyhow::Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let target = match (os, arch) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        _ => anyhow::bail!(
            "Unsupported platform for setup binaries ({os}/{arch}). \
             Install tools manually or use package/cargo methods where available."
        ),
    };
    Ok(target.into())
}

pub fn detect_pkg_manager() -> Option<PkgManager> {
    if cfg!(target_os = "macos") {
        if command_exists("brew") {
            return Some(PkgManager::Brew);
        }
        return None;
    }

    if command_exists("brew") {
        return Some(PkgManager::Brew);
    }
    if command_exists("apt-get") {
        return Some(PkgManager::Apt);
    }
    if command_exists("dnf") {
        return Some(PkgManager::Dnf);
    }
    if command_exists("pacman") {
        return Some(PkgManager::Pacman);
    }
    None
}

pub fn command_exists(name: &str) -> bool {
    which::which(name).is_ok()
}

pub fn tool_installed(tool: ToolId) -> bool {
    tool.probe_commands().iter().any(|cmd| command_exists(cmd))
}

/// True when we should offer Helix in the setup list.
pub fn should_offer_helix() -> bool {
    !tool_installed(ToolId::Helix)
}

pub fn ensure_path_hint(install_dir: &std::path::Path) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    let in_path = std::env::split_paths(&path).any(|p| p == install_dir);
    if in_path {
        None
    } else {
        Some(format!(
            "{} is not on PATH; add it or move installed binaries",
            install_dir.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_target_on_this_host() {
        // May fail on unsupported hosts; skip soft.
        if matches!(
            (std::env::consts::OS, std::env::consts::ARCH),
            ("linux", "x86_64") | ("macos", "x86_64") | ("macos", "aarch64")
        ) {
            assert!(detect_target().is_ok());
        }
    }

    #[test]
    fn brew_install_args() {
        let args = PkgManager::Brew.install_args("zellij");
        assert_eq!(args, vec!["install", "zellij"]);
    }
}
