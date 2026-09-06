//! Static catalog of installable companion tools.

use std::fmt;

/// High-level grouping shown in the setup TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Core,
    FileManager,
    Git,
    Review,
    Workflow,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::FileManager => "File manager",
            Self::Git => "Git",
            Self::Review => "Review",
            Self::Workflow => "Workflow",
        }
    }

    pub fn all() -> &'static [Category] {
        &[
            Self::Core,
            Self::FileManager,
            Self::Git,
            Self::Review,
            Self::Workflow,
        ]
    }
}

/// How a tool can be installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstallMethod {
    Cargo,
    Package,
    Binary,
}

impl InstallMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::Package => "package",
            Self::Binary => "binary",
        }
    }

    pub fn next(self, available: &[InstallMethod]) -> InstallMethod {
        if available.is_empty() {
            return self;
        }
        let Some(idx) = available.iter().position(|m| *m == self) else {
            return available[0];
        };
        available[(idx + 1) % available.len()]
    }
}

impl fmt::Display for InstallMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Stable tool identity used throughout setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolId {
    Zellij,
    Helix,
    Yazi,
    Lazygit,
    Gitui,
    Revdiff,
    Delta,
    Gh,
    Glab,
    GlabTui,
}

impl ToolId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zellij => "zellij",
            Self::Helix => "helix",
            Self::Yazi => "yazi",
            Self::Lazygit => "lazygit",
            Self::Gitui => "gitui",
            Self::Revdiff => "revdiff",
            Self::Delta => "delta",
            Self::Gh => "gh",
            Self::Glab => "glab",
            Self::GlabTui => "glab-tui",
        }
    }

    /// Executable names to probe for "already installed".
    pub fn probe_commands(self) -> &'static [&'static str] {
        match self {
            Self::Helix => &["hx", "helix"],
            Self::Zellij => &["zellij"],
            Self::Yazi => &["yazi"],
            Self::Lazygit => &["lazygit"],
            Self::Gitui => &["gitui"],
            Self::Revdiff => &["revdiff"],
            Self::Delta => &["delta"],
            Self::Gh => &["gh"],
            Self::Glab => &["glab"],
            Self::GlabTui => &["glab-tui"],
        }
    }

    pub fn category(self) -> Category {
        match self {
            Self::Zellij | Self::Helix => Category::Core,
            Self::Yazi => Category::FileManager,
            Self::Lazygit | Self::Gitui => Category::Git,
            Self::Revdiff | Self::Delta => Category::Review,
            Self::Gh | Self::Glab | Self::GlabTui => Category::Workflow,
        }
    }

    pub fn recommended(self) -> bool {
        matches!(
            self,
            Self::Zellij | Self::Helix | Self::Yazi | Self::Lazygit | Self::Revdiff | Self::GlabTui
        )
    }

    /// Command written into sat-hx-ide config when this tool wins its category.
    pub fn config_command(self) -> Option<&'static str> {
        match self {
            Self::Zellij => Some("zellij"),
            Self::Helix => Some("hx"),
            Self::Yazi => Some("yazi"),
            Self::Lazygit => Some("lazygit"),
            Self::Gitui => Some("gitui"),
            Self::Revdiff => Some("revdiff"),
            Self::GlabTui => Some("glab-tui"),
            Self::Glab => Some("glab"),
            Self::Gh => Some("gh"),
            Self::Delta => None,
        }
    }

    pub fn methods(self) -> &'static [InstallMethod] {
        match self {
            Self::Zellij => &[
                InstallMethod::Cargo,
                InstallMethod::Package,
                InstallMethod::Binary,
            ],
            Self::Helix => &[InstallMethod::Package, InstallMethod::Binary],
            Self::Yazi => &[
                InstallMethod::Cargo,
                InstallMethod::Package,
                InstallMethod::Binary,
            ],
            Self::Lazygit => &[InstallMethod::Package, InstallMethod::Binary],
            Self::Gitui => &[
                InstallMethod::Cargo,
                InstallMethod::Package,
                InstallMethod::Binary,
            ],
            Self::Revdiff => &[InstallMethod::Binary],
            Self::Delta => &[
                InstallMethod::Cargo,
                InstallMethod::Package,
                InstallMethod::Binary,
            ],
            Self::Gh => &[InstallMethod::Package, InstallMethod::Binary],
            Self::Glab => &[InstallMethod::Package, InstallMethod::Binary],
            Self::GlabTui => &[InstallMethod::Cargo, InstallMethod::Binary],
        }
    }

    pub fn default_method(self) -> InstallMethod {
        self.methods()[0]
    }

    pub fn cargo_crate(self) -> Option<&'static str> {
        match self {
            Self::Zellij => Some("zellij"),
            Self::Yazi => Some("yazi-fm"),
            Self::Gitui => Some("gitui"),
            Self::Delta => Some("git-delta"),
            Self::GlabTui => Some("glab-tui"),
            _ => None,
        }
    }

    /// Package names keyed by detected manager id.
    pub fn package_name(self, manager: super::detect::PkgManager) -> Option<&'static str> {
        use super::detect::PkgManager::*;
        match (self, manager) {
            (Self::Zellij, Brew) => Some("zellij"),
            (Self::Zellij, Apt) => Some("zellij"),
            (Self::Zellij, Dnf) => Some("zellij"),
            (Self::Zellij, Pacman) => Some("zellij"),

            (Self::Helix, Brew) => Some("helix"),
            (Self::Helix, Apt) => Some("helix"),
            (Self::Helix, Dnf) => Some("helix"),
            (Self::Helix, Pacman) => Some("helix"),

            (Self::Yazi, Brew) => Some("yazi"),
            (Self::Yazi, Apt) => Some("yazi"),
            (Self::Yazi, Dnf) => Some("yazi"),
            (Self::Yazi, Pacman) => Some("yazi"),

            (Self::Lazygit, Brew) => Some("lazygit"),
            (Self::Lazygit, Apt) => Some("lazygit"),
            (Self::Lazygit, Dnf) => Some("lazygit"),
            (Self::Lazygit, Pacman) => Some("lazygit"),

            (Self::Gitui, Brew) => Some("gitui"),
            (Self::Gitui, Apt) => Some("gitui"),
            (Self::Gitui, Dnf) => Some("gitui"),
            (Self::Gitui, Pacman) => Some("gitui"),

            (Self::Delta, Brew) => Some("git-delta"),
            (Self::Delta, Apt) => Some("git-delta"),
            (Self::Delta, Dnf) => Some("git-delta"),
            (Self::Delta, Pacman) => Some("git-delta"),

            (Self::Gh, Brew) => Some("gh"),
            (Self::Gh, Apt) => Some("gh"),
            (Self::Gh, Dnf) => Some("gh"),
            (Self::Gh, Pacman) => Some("github-cli"),

            (Self::Glab, Brew) => Some("glab"),
            (Self::Glab, Apt) => Some("glab"),
            (Self::Glab, Dnf) => Some("glab"),
            (Self::Glab, Pacman) => Some("glab"),

            _ => None,
        }
    }

    pub fn github_repo(self) -> Option<&'static str> {
        match self {
            Self::Zellij => Some("zellij-org/zellij"),
            Self::Helix => Some("helix-editor/helix"),
            Self::Yazi => Some("sxyazi/yazi"),
            Self::Lazygit => Some("jesseduffield/lazygit"),
            Self::Gitui => Some("gitui-org/gitui"),
            Self::Revdiff => Some("umputun/revdiff"),
            Self::Delta => Some("dandavison/delta"),
            Self::Gh => Some("cli/cli"),
            Self::Glab => Some("glab-cli/glab"),
            Self::GlabTui => Some("rcieri/glab-tui"),
        }
    }

    /// Substrings used to pick a release asset for the current host target.
    pub fn asset_matchers(self, target: &str) -> Vec<&'static str> {
        match (self, target) {
            (Self::Zellij, "x86_64-unknown-linux-gnu") => {
                vec!["x86_64-unknown-linux-musl", "x86_64-unknown-linux-gnu"]
            }
            (Self::Zellij, "x86_64-apple-darwin") => vec!["x86_64-apple-darwin"],
            (Self::Zellij, "aarch64-apple-darwin") => vec!["aarch64-apple-darwin"],

            (Self::Helix, "x86_64-unknown-linux-gnu") => vec!["x86_64-linux"],
            (Self::Helix, "x86_64-apple-darwin") => vec!["x86_64-macos"],
            (Self::Helix, "aarch64-apple-darwin") => vec!["aarch64-macos"],

            (Self::Yazi, "x86_64-unknown-linux-gnu") => vec!["x86_64-unknown-linux-gnu"],
            (Self::Yazi, "x86_64-apple-darwin") => vec!["x86_64-apple-darwin"],
            (Self::Yazi, "aarch64-apple-darwin") => vec!["aarch64-apple-darwin"],

            (Self::Lazygit, "x86_64-unknown-linux-gnu") => vec!["Linux_x86_64", "linux_x86_64"],
            (Self::Lazygit, "x86_64-apple-darwin") => vec!["Darwin_x86_64", "darwin_x86_64"],
            (Self::Lazygit, "aarch64-apple-darwin") => vec!["Darwin_arm64", "darwin_arm64"],

            (Self::Gitui, "x86_64-unknown-linux-gnu") => {
                vec!["linux-x86_64", "x86_64-unknown-linux"]
            }
            (Self::Gitui, "x86_64-apple-darwin") => vec!["mac-x86_64", "x86_64-apple-darwin"],
            (Self::Gitui, "aarch64-apple-darwin") => vec!["mac-aarch64", "aarch64-apple-darwin"],

            (Self::Revdiff, "x86_64-unknown-linux-gnu") => {
                vec!["linux_amd64", "linux-amd64", "x86_64"]
            }
            (Self::Revdiff, "x86_64-apple-darwin") => vec!["darwin_amd64", "macos"],
            (Self::Revdiff, "aarch64-apple-darwin") => vec!["darwin_arm64", "macos_arm"],

            (Self::Delta, "x86_64-unknown-linux-gnu") => {
                vec!["x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"]
            }
            (Self::Delta, "x86_64-apple-darwin") => vec!["x86_64-apple-darwin"],
            (Self::Delta, "aarch64-apple-darwin") => vec!["aarch64-apple-darwin"],

            (Self::Gh, "x86_64-unknown-linux-gnu") => vec!["linux_amd64"],
            (Self::Gh, "x86_64-apple-darwin") => vec!["macOS_amd64", "macOS_amd64"],
            (Self::Gh, "aarch64-apple-darwin") => vec!["macOS_arm64"],

            (Self::Glab, "x86_64-unknown-linux-gnu") => vec!["Linux_x86_64", "linux_amd64"],
            (Self::Glab, "x86_64-apple-darwin") => vec!["Darwin_x86_64", "macOS_amd64"],
            (Self::Glab, "aarch64-apple-darwin") => vec!["Darwin_arm64", "macOS_arm64"],

            (Self::GlabTui, "x86_64-unknown-linux-gnu") => {
                vec!["x86_64-unknown-linux", "linux-x86_64"]
            }
            (Self::GlabTui, "x86_64-apple-darwin") => vec!["x86_64-apple-darwin", "macos-x86_64"],
            (Self::GlabTui, "aarch64-apple-darwin") => {
                vec!["aarch64-apple-darwin", "macos-aarch64"]
            }

            _ => vec![],
        }
    }

    /// Binary file name expected inside the downloaded archive.
    pub fn archive_binary_name(self) -> &'static str {
        match self {
            Self::Helix => "hx",
            other => other.as_str(),
        }
    }
}

/// Full catalog entry list in display order.
pub fn catalog_tools() -> &'static [ToolId] {
    &[
        ToolId::Zellij,
        ToolId::Helix,
        ToolId::Yazi,
        ToolId::Lazygit,
        ToolId::Gitui,
        ToolId::Revdiff,
        ToolId::Delta,
        ToolId::Gh,
        ToolId::Glab,
        ToolId::GlabTui,
    ]
}

/// Preference order when choosing which selected tool updates config.
pub fn config_preference(category: Category) -> &'static [ToolId] {
    match category {
        Category::Core => &[ToolId::Zellij, ToolId::Helix],
        Category::FileManager => &[ToolId::Yazi],
        Category::Git => &[ToolId::Lazygit, ToolId::Gitui],
        Category::Review => &[ToolId::Revdiff],
        Category::Workflow => &[ToolId::GlabTui, ToolId::Glab, ToolId::Gh],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helix_has_no_cargo_method() {
        assert!(!ToolId::Helix.methods().contains(&InstallMethod::Cargo));
    }

    #[test]
    fn lazygit_has_no_cargo_method() {
        assert!(!ToolId::Lazygit.methods().contains(&InstallMethod::Cargo));
    }

    #[test]
    fn cycle_method_wraps() {
        let methods = ToolId::Zellij.methods();
        let next = InstallMethod::Binary.next(methods);
        assert_eq!(next, InstallMethod::Cargo);
    }

    #[test]
    fn yazi_cargo_crate_name() {
        assert_eq!(ToolId::Yazi.cargo_crate(), Some("yazi-fm"));
    }
}
