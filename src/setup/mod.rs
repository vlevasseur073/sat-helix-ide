//! Interactive companion-tool setup (`sat-hx-ide setup`).

mod catalog;
mod config_write;
mod detect;
mod execute;
mod plan;
mod ui;

use anyhow::{Context, Result};
use detect::{ensure_path_hint, HostInfo};
use std::path::Path;

/// Run the setup TUI, then install (or dry-run) and update config.
pub fn run(config_path: &Path, dry_run: bool) -> Result<()> {
    if cfg!(windows) {
        anyhow::bail!(
            "sat-hx-ide setup does not support Windows yet. \
             Download tools from their GitHub releases manually."
        );
    }

    let host = HostInfo::detect().context("Failed to detect host capabilities")?;
    println!(
        "Host: target={} pkg={} cargo={} curl={} install_dir={}",
        host.target,
        host.pkg_manager.map(|m| m.as_str()).unwrap_or("none"),
        host.has_cargo,
        host.has_curl,
        host.install_dir.display()
    );
    if let Some(hint) = ensure_path_hint(&host.install_dir) {
        println!("note: {hint}");
    }

    let Some(selections) = ui::run_ui(host.clone(), dry_run)? else {
        println!("Setup cancelled.");
        return Ok(());
    };

    // Re-detect is unnecessary; reuse host from UI via clone — but UI consumed host.
    // run_ui takes ownership of a clone; we still have `host`.
    let plan = plan::build_plan(&selections, &host)?;
    let report = execute::execute_plan(&plan, dry_run)?;

    let mut installed_ok = report.succeeded.clone();
    // Already-installed tools that were selected still count for config updates.
    for sel in &selections {
        if sel.selected && sel.already_installed {
            installed_ok.push(sel.tool.as_str().into());
        }
    }

    if !dry_run {
        match config_write::update_config_from_selection(config_path, &selections, &installed_ok) {
            Ok(notes) => {
                if !notes.is_empty() {
                    println!("\nConfig:");
                    for note in notes {
                        println!("  • {note}");
                    }
                }
            }
            Err(err) => {
                eprintln!("warning: failed to update config: {err:#}");
            }
        }
    } else {
        println!("\nDry run: config was not modified.");
    }

    println!("\nSummary:");
    if !report.succeeded.is_empty() {
        println!("  installed: {}", report.succeeded.join(", "));
    }
    if !report.failed.is_empty() {
        println!("  failed:");
        for (tool, err) in &report.failed {
            println!("    - {tool}: {err}");
        }
    }
    if dry_run && !report.skipped.is_empty() {
        println!("  planned: {}", report.skipped.join(", "));
    }

    let selected_already: Vec<_> = selections
        .iter()
        .filter(|s| s.selected && s.already_installed)
        .map(|s| s.tool.as_str())
        .collect();
    if !selected_already.is_empty() {
        println!("  already present (kept): {}", selected_already.join(", "));
    }

    if selections
        .iter()
        .any(|s| s.selected && s.tool.as_str() == "delta")
    {
        println!(
            "\nnote: delta was installed but git pager config is left to you \
             (e.g. git config --global core.pager delta)."
        );
    }

    if !report.failed.is_empty() {
        anyhow::bail!("one or more installs failed");
    }

    println!("\nDone. Run `sat-hx-ide doctor` to verify.");
    Ok(())
}
