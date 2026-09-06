//! Persist selected tools into the user sat-hx-ide config.

use crate::config::{expand_tilde, CommandConfig, Config};
use crate::setup::catalog::{config_preference, Category, ToolId};
use crate::setup::detect::tool_installed;
use crate::setup::plan::ToolSelection;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn update_config_from_selection(
    config_path: &Path,
    selections: &[ToolSelection],
    installed_ok: &[String],
) -> Result<Vec<String>> {
    let path = expand_tilde(config_path);
    let mut config = if path.exists() {
        Config::load(&path)?
    } else {
        Config::default()
    };

    let installed: std::collections::HashSet<&str> =
        installed_ok.iter().map(String::as_str).collect();

    let mut notes = Vec::new();

    // Core: zellij / helix independently when selected and available.
    apply_core(&mut config, selections, &installed, &mut notes);

    apply_category(
        Category::FileManager,
        selections,
        &installed,
        |cmd| {
            config.tools.file_manager.command = cmd.to_string();
            config.tools.file_manager.adapter = "yazi".into();
        },
        &mut notes,
    );

    apply_category(
        Category::Git,
        selections,
        &installed,
        |cmd| {
            config.tools.git = CommandConfig::new(cmd);
        },
        &mut notes,
    );

    apply_category(
        Category::Review,
        selections,
        &installed,
        |cmd| {
            config.tools.review = CommandConfig::new(cmd);
        },
        &mut notes,
    );

    apply_category(
        Category::Workflow,
        selections,
        &installed,
        |cmd| {
            config.tools.workflow = CommandConfig::new(cmd);
        },
        &mut notes,
    );

    if notes.is_empty() {
        return Ok(notes);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let serialized = toml::to_string_pretty(&config).context("Failed to serialize config")?;
    fs::write(&path, serialized).with_context(|| format!("Failed to write {}", path.display()))?;
    notes.push(format!("Updated {}", path.display()));
    Ok(notes)
}

fn apply_core(
    config: &mut Config,
    selections: &[ToolSelection],
    installed: &std::collections::HashSet<&str>,
    notes: &mut Vec<String>,
) {
    for tool in [ToolId::Zellij, ToolId::Helix] {
        let Some(sel) = selections.iter().find(|s| s.tool == tool) else {
            continue;
        };
        if !sel.selected {
            continue;
        }
        let available =
            installed.contains(tool.as_str()) || sel.already_installed || tool_installed(tool);
        if !available {
            continue;
        }
        let Some(cmd) = tool.config_command() else {
            continue;
        };
        match tool {
            ToolId::Zellij => {
                config.tools.zellij = CommandConfig::new(cmd);
                notes.push(format!("tools.zellij → {cmd}"));
            }
            ToolId::Helix => {
                config.tools.editor = CommandConfig::new(cmd);
                notes.push(format!("tools.editor → {cmd}"));
            }
            _ => {}
        }
    }
}

fn apply_category(
    category: Category,
    selections: &[ToolSelection],
    installed: &std::collections::HashSet<&str>,
    mut apply: impl FnMut(&str),
    notes: &mut Vec<String>,
) {
    let winner = config_preference(category).iter().find(|tool| {
        let Some(sel) = selections.iter().find(|s| s.tool == **tool) else {
            return false;
        };
        if !sel.selected {
            return false;
        }
        tool.config_command().is_some()
            && (installed.contains(tool.as_str())
                || sel.already_installed
                || tool_installed(**tool))
    });

    let Some(tool) = winner else {
        return;
    };
    let Some(cmd) = tool.config_command() else {
        return;
    };
    apply(cmd);
    notes.push(format!(
        "tools.{} → {cmd}",
        match category {
            Category::FileManager => "file_manager",
            Category::Git => "git",
            Category::Review => "review",
            Category::Workflow => "workflow",
            Category::Core => "core",
        }
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::catalog::InstallMethod;
    use tempfile::tempdir;

    #[test]
    fn writes_gitui_when_selected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let selections = vec![ToolSelection {
            tool: ToolId::Gitui,
            method: InstallMethod::Cargo,
            selected: true,
            already_installed: true,
        }];
        let notes = update_config_from_selection(&path, &selections, &[]).unwrap();
        assert!(notes.iter().any(|n| n.contains("gitui")));
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.tools.git.command, "gitui");
    }
}
