//! Build an ordered install plan from UI selections.

use crate::setup::catalog::{InstallMethod, ToolId};
use crate::setup::detect::{HostInfo, PkgManager};

#[derive(Debug, Clone)]
pub struct ToolSelection {
    pub tool: ToolId,
    pub method: InstallMethod,
    pub selected: bool,
    pub already_installed: bool,
}

#[derive(Debug, Clone)]
pub struct InstallStep {
    pub tool: ToolId,
    pub method: InstallMethod,
    pub summary: String,
    pub argv: Vec<String>,
}

pub fn build_plan(
    selections: &[ToolSelection],
    host: &HostInfo,
) -> anyhow::Result<Vec<InstallStep>> {
    let mut steps = Vec::new();
    for sel in selections {
        if !sel.selected {
            continue;
        }
        if sel.already_installed {
            continue;
        }
        steps.push(step_for(sel.tool, sel.method, host)?);
    }
    Ok(steps)
}

fn step_for(tool: ToolId, method: InstallMethod, host: &HostInfo) -> anyhow::Result<InstallStep> {
    match method {
        InstallMethod::Cargo => {
            let crate_name = tool
                .cargo_crate()
                .ok_or_else(|| anyhow::anyhow!("{} has no cargo crate", tool.as_str()))?;
            Ok(InstallStep {
                tool,
                method,
                summary: format!("cargo install {crate_name}"),
                argv: vec![
                    "cargo".into(),
                    "install".into(),
                    crate_name.into(),
                    "--locked".into(),
                ],
            })
        }
        InstallMethod::Package => {
            let manager = host
                .pkg_manager
                .ok_or_else(|| anyhow::anyhow!("no package manager detected"))?;
            let package = tool
                .package_name(manager)
                .ok_or_else(|| anyhow::anyhow!("no package mapping for {}", tool.as_str()))?;
            let mut argv = Vec::new();
            if manager.needs_sudo() && which::which("sudo").is_ok() {
                argv.push("sudo".into());
            }
            argv.push(manager.as_str().into());
            argv.extend(manager.install_args(package));
            Ok(InstallStep {
                tool,
                method,
                summary: format!("{} install {package}", manager.as_str()),
                argv,
            })
        }
        InstallMethod::Binary => {
            let repo = tool
                .github_repo()
                .ok_or_else(|| anyhow::anyhow!("{} has no GitHub repo", tool.as_str()))?;
            let matchers = tool.asset_matchers(&host.target);
            if matchers.is_empty() {
                anyhow::bail!(
                    "no binary asset matchers for {} on {}",
                    tool.as_str(),
                    host.target
                );
            }
            let install_dir = host.install_dir.display().to_string();
            let bin_name = tool.archive_binary_name();
            let matcher_list = matchers
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(" ");
            let script = binary_install_script(repo, &install_dir, bin_name, &matcher_list);
            Ok(InstallStep {
                tool,
                method,
                summary: format!("download {repo} binary → {install_dir}/{bin_name}"),
                argv: vec!["sh".into(), "-lc".into(), script],
            })
        }
    }
}

fn binary_install_script(
    repo: &str,
    install_dir: &str,
    bin_name: &str,
    matcher_list: &str,
) -> String {
    // Keep this self-contained: resolve latest release asset via the GitHub API,
    // download, extract, and install the binary into INSTALL_DIR.
    format!(
        r#"
set -euo pipefail
REPO="{repo}"
INSTALL_DIR="{install_dir}"
BIN_NAME="{bin_name}"
MATCHERS=({matcher_list})
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
api="https://api.github.com/repos/${{REPO}}/releases/latest"
json="$(curl -fsSL "$api")"
asset=""
for m in "${{MATCHERS[@]}}"; do
  candidate="$(printf '%s' "$json" | tr '{{' '\n' | grep -oE '"browser_download_url":[[:space:]]*"[^"]+"' | sed 's/.*"\(https[^"]*\)".*/\1/' | grep -i -- "$m" | grep -vE '\.(deb|rpm|msi|sha256|sha512|sum|txt|asc)$' | head -n1 || true)"
  if [ -n "$candidate" ]; then
    asset="$candidate"
    break
  fi
done
if [ -z "$asset" ]; then
  echo "error: no matching release asset for $REPO (matchers: ${{MATCHERS[*]}})" >&2
  exit 1
fi
echo "Downloading $asset"
fname="$tmpdir/$(basename "$asset")"
curl -fsSL --proto '=https' --tlsv1.2 -o "$fname" "$asset"
mkdir -p "$INSTALL_DIR"
case "$fname" in
  *.tar.gz|*.tgz) tar -xzf "$fname" -C "$tmpdir" ;;
  *.tar.xz) tar -xJf "$fname" -C "$tmpdir" ;;
  *.zip)
    if command -v unzip >/dev/null 2>&1; then
      unzip -q "$fname" -d "$tmpdir"
    else
      echo "error: unzip is required for zip assets" >&2
      exit 1
    fi
    ;;
  *)
    # Single binary blob
    cp "$fname" "$tmpdir/$BIN_NAME"
    ;;
esac
found="$(find "$tmpdir" -type f -name "$BIN_NAME" | head -n1)"
if [ -z "$found" ]; then
  # Helix ships hx inside a versioned directory; also accept the tool name.
  found="$(find "$tmpdir" -type f \( -name "$BIN_NAME" -o -perm -111 \) | head -n1)"
fi
if [ -z "$found" ]; then
  echo "error: binary $BIN_NAME not found in archive" >&2
  find "$tmpdir" -maxdepth 3 -type f >&2 || true
  exit 1
fi
install -m 755 "$found" "$INSTALL_DIR/$BIN_NAME"
echo "Installed $INSTALL_DIR/$BIN_NAME"
"#
    )
}

/// Prefer package manager enum for tests.
#[allow(dead_code)]
pub fn package_step_preview(tool: ToolId, manager: PkgManager) -> Option<String> {
    let package = tool.package_name(manager)?;
    Some(format!(
        "{} {}",
        manager.as_str(),
        manager.install_args(package).join(" ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn host_with(pkg: Option<PkgManager>) -> HostInfo {
        HostInfo {
            target: "x86_64-unknown-linux-gnu".into(),
            pkg_manager: pkg,
            has_cargo: true,
            has_curl: true,
            install_dir: PathBuf::from("/tmp/bin"),
        }
    }

    #[test]
    fn skips_unselected_and_installed() {
        let host = host_with(Some(PkgManager::Brew));
        let sels = vec![
            ToolSelection {
                tool: ToolId::Zellij,
                method: InstallMethod::Cargo,
                selected: false,
                already_installed: false,
            },
            ToolSelection {
                tool: ToolId::Yazi,
                method: InstallMethod::Cargo,
                selected: true,
                already_installed: true,
            },
            ToolSelection {
                tool: ToolId::Gitui,
                method: InstallMethod::Cargo,
                selected: true,
                already_installed: false,
            },
        ];
        let plan = build_plan(&sels, &host).unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].tool, ToolId::Gitui);
        assert!(plan[0].summary.contains("cargo install gitui"));
    }

    #[test]
    fn package_step_uses_sudo_for_apt() {
        let host = host_with(Some(PkgManager::Apt));
        let step = step_for(ToolId::Delta, InstallMethod::Package, &host).unwrap();
        assert_eq!(step.argv[0], "sudo");
        assert!(step.argv.iter().any(|a| a == "git-delta"));
    }
}
