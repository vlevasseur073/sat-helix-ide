//! Run or print install steps.

use crate::setup::plan::InstallStep;
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::process::{Command, Stdio};

#[derive(Debug, Default)]
pub struct ExecuteReport {
    pub succeeded: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub skipped: Vec<String>,
}

pub fn execute_plan(steps: &[InstallStep], dry_run: bool) -> Result<ExecuteReport> {
    let mut report = ExecuteReport::default();
    if steps.is_empty() {
        println!("Nothing to install.");
        return Ok(report);
    }

    if dry_run {
        println!("Dry run — planned install steps:\n");
        for step in steps {
            println!("  • {} ({})", step.summary, step.method);
            println!("    $ {}", format_argv(&step.argv));
        }
        for step in steps {
            report.skipped.push(step.tool.as_str().into());
        }
        return Ok(report);
    }

    for step in steps {
        println!(
            "\n==> Installing {} via {}",
            step.tool.as_str(),
            step.method
        );
        println!("    {}", step.summary);
        match run_step(step) {
            Ok(()) => {
                println!("    ✓ {}", step.tool.as_str());
                report.succeeded.push(step.tool.as_str().into());
            }
            Err(err) => {
                eprintln!("    ✗ {}: {err:#}", step.tool.as_str());
                report
                    .failed
                    .push((step.tool.as_str().into(), format!("{err:#}")));
            }
        }
    }
    Ok(report)
}

fn run_step(step: &InstallStep) -> Result<()> {
    let (program, args) = step
        .argv
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("empty install command"))?;

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn `{program}`"))?;

    let status = child.wait().context("failed waiting for install command")?;
    if !status.success() {
        anyhow::bail!(
            "command exited with status {}",
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".into())
        );
    }
    let _ = io::stdout().flush();
    Ok(())
}

fn format_argv(argv: &[String]) -> String {
    argv.iter()
        .map(|a| {
            if a.chars().any(|c| c.is_whitespace()) {
                format!("'{a}'")
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_quotes_shell_script() {
        let argv = vec!["sh".into(), "-lc".into(), "echo hello world".into()];
        let s = format_argv(&argv);
        assert!(s.contains("'echo hello world'"));
    }
}
