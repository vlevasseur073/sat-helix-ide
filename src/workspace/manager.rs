use crate::config::{Config, LayoutConfig, LayoutPane};
use crate::error::HxIdeError;
use crate::helix::HelixClient;
use crate::tools::ToolManager;
use crate::workspace::ProjectDetector;
use crate::zellij::ZellijClient;
use anyhow::{Context, Result};
use log::info;
use std::path::{Path, PathBuf};

/// Manages workspace operations
pub struct WorkspaceManager<'a> {
    config: &'a Config,
    zellij: ZellijClient,
    helix: HelixClient,
    tools: ToolManager,
    detector: ProjectDetector,
}

impl<'a> WorkspaceManager<'a> {
    /// Create a new workspace manager
    pub fn new(config: &'a Config) -> Self {
        Self {
            config,
            zellij: ZellijClient::new(&config.tools.zellij.path),
            helix: HelixClient::new(&config.tools.helix.path),
            tools: ToolManager::new(config),
            detector: ProjectDetector::new(),
        }
    }

    /// Initialize a workspace
    pub fn init_workspace(
        &mut self,
        path: &Path,
        layout_name: &str,
        new_session: bool,
    ) -> Result<()> {
        info!("Initializing workspace at: {}", path.display());

        let project_path = std::fs::canonicalize(path)
            .with_context(|| format!("Failed to resolve project path {}", path.display()))?;

        // Change to project directory
        std::env::set_current_dir(&project_path)
            .context("Failed to change to project directory")?;

        // Detect project type
        let project_type = if self.config.workspace.auto_detect {
            self.detector.detect_project(&project_path)
        } else {
            None
        };
        info!("Detected project type: {:?}", project_type);

        // Get layout
        let layout = self
            .config
            .get_layout(layout_name)
            .ok_or_else(|| HxIdeError::LayoutNotFound(layout_name.to_string()))?;

        // Apply project-specific overrides
        let mut layout = self.apply_project_overrides(layout, &project_type)?;
        self.resolve_pane_commands(&mut layout.parts)?;
        self.apply_generated_helix_config(&mut layout)?;
        let layout_path = self.save_layout(&layout)?;

        // Check tools
        self.check_tools()?;

        // Launch Zellij with the layout
        let (zellij_config, yazi_config) = self.generated_configs()?;

        // Persist state before Zellij takes over the terminal.
        if self.config.workspace.remember_last {
            self.config.save_last_workspace(&project_path)?;
        }

        if new_session {
            self.zellij
                .new_session(
                    &layout_path,
                    zellij_config.as_deref(),
                    yazi_config.as_deref(),
                )
                .context("Failed to start new Zellij session")?;
        } else {
            self.zellij
                .open_layout(
                    &layout_path,
                    zellij_config.as_deref(),
                    yazi_config.as_deref(),
                )
                .context("Failed to open Zellij layout")?;
        }

        Ok(())
    }

    fn generated_configs(&self) -> Result<(Option<PathBuf>, Option<PathBuf>)> {
        if !self.config.use_generated_configs {
            return Ok((None, None));
        }

        let zellij = self.config.generated_zellij_config();
        let yazi = self.config.generated_yazi_config_dir();
        let helix = self.config.generated_helix_config();
        for path in [&zellij, &yazi, &helix] {
            if !path.exists() {
                return Err(HxIdeError::ConfigError(format!(
                    "Generated config {} is missing; run `sat-hx-ide generate --all` first",
                    path.display()
                ))
                .into());
            }
        }
        Ok((Some(zellij), Some(yazi)))
    }

    /// Apply project-specific overrides to layout
    fn apply_project_overrides(
        &self,
        layout: &LayoutConfig,
        project_type: &Option<String>,
    ) -> Result<LayoutConfig> {
        use std::collections::HashMap;

        let mut layout = layout.clone();

        // Add project-specific panes
        if let Some(p_type) = project_type {
            match p_type.as_str() {
                "rust" => {
                    // Add cargo pane for Rust projects
                    let first_part = layout.parts.first_mut().ok_or_else(|| {
                        HxIdeError::InvalidLayout(format!("Layout '{}' has no parts", layout.name))
                    })?;
                    first_part.panes.push(LayoutPane {
                        id: Some("cargo".to_string()),
                        command: "cargo".to_string(),
                        args: vec![],
                        run: false,
                        config: HashMap::new(),
                    });
                }
                "node" => {
                    // Add npm pane for Node.js projects
                    let first_part = layout.parts.first_mut().ok_or_else(|| {
                        HxIdeError::InvalidLayout(format!("Layout '{}' has no parts", layout.name))
                    })?;
                    first_part.panes.push(LayoutPane {
                        id: Some("npm".to_string()),
                        command: "npm".to_string(),
                        args: vec![],
                        run: false,
                        config: HashMap::new(),
                    });
                }
                "python" => {
                    // Add pip pane for Python projects
                    let first_part = layout.parts.first_mut().ok_or_else(|| {
                        HxIdeError::InvalidLayout(format!("Layout '{}' has no parts", layout.name))
                    })?;
                    first_part.panes.push(LayoutPane {
                        id: Some("pip".to_string()),
                        command: "pip".to_string(),
                        args: vec![],
                        run: false,
                        config: HashMap::new(),
                    });
                }
                _ => {}
            }
        }

        Ok(layout)
    }

    /// Map a layout command onto the configured tool binary and locate it on PATH.
    ///
    /// Zellij spawns pane commands itself, so shell aliases and functions are not
    /// available to it and the binary has to be resolvable.
    fn resolve_command(&self, command: &str) -> Result<String> {
        let tools = &self.config.tools;
        let configured = match command {
            "hx" | "helix" => tools.helix.path.as_str(),
            "yazi" => tools.yazi.path.as_str(),
            "lazygit" | "gitui" => tools.git.client.as_str(),
            other => other,
        };

        let resolved = which::which(configured).map_err(|_| {
            HxIdeError::ToolNotFound(format!(
                "'{command}' resolves to '{configured}', which is not on PATH. \
                 Shell aliases are invisible to Zellij, so set an absolute path \
                 under [tools] in your sat-helix-ide config."
            ))
        })?;

        Ok(resolved.display().to_string())
    }

    fn resolve_pane_commands(&self, parts: &mut [crate::config::LayoutPart]) -> Result<()> {
        for part in parts {
            for pane in &mut part.panes {
                if !pane.run || pane.command.trim().is_empty() {
                    continue;
                }

                let mut tokens = pane.command.split_whitespace();
                let Some(binary) = tokens.next() else {
                    continue;
                };
                let inline_args: Vec<String> = tokens.map(str::to_owned).collect();

                pane.command = self.resolve_command(binary)?;
                if !inline_args.is_empty() {
                    let mut args = inline_args;
                    args.append(&mut pane.args);
                    pane.args = args;
                }
            }
            self.resolve_pane_commands(&mut part.parts)?;
        }
        Ok(())
    }

    fn apply_generated_helix_config(&self, layout: &mut LayoutConfig) -> Result<()> {
        if !self.config.use_generated_configs {
            return Ok(());
        }
        let config_path = self.config.generated_helix_config();
        let helix_command = self.resolve_command(&self.config.tools.helix.path)?;
        Self::add_helix_config_args(&mut layout.parts, &helix_command, &config_path);
        Ok(())
    }

    fn add_helix_config_args(
        parts: &mut [crate::config::LayoutPart],
        helix_command: &str,
        config_path: &Path,
    ) {
        for part in parts {
            for pane in &mut part.panes {
                if pane.command == helix_command {
                    pane.args.push("--config".to_string());
                    pane.args.push(config_path.display().to_string());
                }
            }
            Self::add_helix_config_args(&mut part.parts, helix_command, config_path);
        }
    }

    /// Save layout to temporary file
    fn save_layout(&self, layout: &LayoutConfig) -> Result<PathBuf> {
        use std::fs;
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let layout_path = temp_dir.join(format!(
            "sat-helix-ide-{}-{}.kdl",
            std::process::id(),
            layout.name
        ));

        let mut file = fs::File::create(&layout_path)?;
        file.write_all(layout.to_kdl().as_bytes())?;

        Ok(layout_path)
    }

    /// Check all required tools are available
    pub fn check_tools(&self) -> Result<()> {
        println!("Checking tool availability...");

        let mut errors = vec![];
        let mut warnings: Vec<String> = vec![];

        // Check Zellij
        if !self.zellij.is_available() {
            errors.push(HxIdeError::ToolNotFound(
                "Zellij not found. Install with: cargo install --git https://github.com/zellij-org/zellij".to_string(),
            ));
        } else {
            println!("  ✓ Zellij: found");
        }

        // Check Helix
        if !self.helix.is_available() {
            errors.push(HxIdeError::ToolNotFound(
                "Helix not found. Install with: cargo install --git https://github.com/helix-editor/helix".to_string(),
            ));
        } else {
            println!("  ✓ Helix: found");
        }

        // Check Yazi
        if !self.tools.yazi.is_available() {
            warnings.push("Yazi not found. File explorer will not be available.".to_string());
        } else {
            println!("  ✓ Yazi: found");
        }

        // Check Git client
        match self.config.tools.git.client.as_str() {
            "lazygit" => {
                if !self.tools.git.is_available() {
                    warnings
                        .push("Lazygit not found. Git client will not be available.".to_string());
                } else {
                    println!("  ✓ Lazygit: found");
                }
            }
            "gitui" => {
                if !self.tools.git.is_available() {
                    warnings.push("GitUI not found. Git client will not be available.".to_string());
                } else {
                    println!("  ✓ GitUI: found");
                }
            }
            _ => {
                warnings.push(format!(
                    "Unknown git client: {}",
                    self.config.tools.git.client
                ));
            }
        }

        // Check git-delta
        if self.config.tools.git.delta && !self.tools.git.delta_available() {
            warnings.push("git-delta not found. Git diffs will use default pager.".to_string());
        } else if self.config.tools.git.delta {
            println!("  ✓ git-delta: found");
        }

        // Print warnings
        for warning in &warnings {
            println!("  ! {}", warning);
        }

        if !errors.is_empty() {
            for error in &errors {
                eprintln!("Error: {}", error);
            }
            return Err(errors.into_iter().nth(0).unwrap().into());
        }

        println!("\nAll required tools are available!");
        Ok(())
    }

    /// Generate Zellij configuration
    pub fn generate_zellij_config(&self) -> Result<()> {
        info!("Generating Zellij configuration...");

        let config_kdl = format!(
            "// Generated by sat-helix-ide; never installed globally.\ntheme \"{}\"\nmouse_mode {}\n",
            self.config.tools.zellij.theme,
            self.config.tools.zellij.mouse
        );

        let config_path = self.config.generated_zellij_config();
        std::fs::create_dir_all(config_path.parent().unwrap())?;
        std::fs::write(&config_path, config_kdl)?;

        info!("Zellij configuration saved to: {}", config_path.display());
        Ok(())
    }

    /// Generate Yazi configuration
    pub fn generate_yazi_config(&self) -> Result<()> {
        info!("Generating Yazi configuration...");

        let config_dir = self.config.generated_yazi_config_dir();
        std::fs::create_dir_all(&config_dir)?;

        // Generate config.toml
        let config_toml = format!(
            r#"
[preview]
tab_size = {}
max_width = {}
max_height = {}
"#,
            self.config.tools.yazi.preview.tab_size,
            self.config.tools.yazi.preview.max_width,
            self.config.tools.yazi.preview.max_height
        );

        let config_path = config_dir.join("yazi.toml");
        std::fs::write(&config_path, config_toml)?;

        info!("Yazi configuration saved to: {}", config_dir.display());
        Ok(())
    }

    /// Generate Helix configuration additions
    pub fn generate_helix_config(&self) -> Result<()> {
        info!("Generating Helix configuration...");

        let helix_config = format!(
            r#"
# Generated by sat-helix-ide; never installed globally.
theme = "{}"

[editor]
true-color = true

[editor.cursor-shape]
insert = "bar"
normal = "block"
select = "underline"

[editor.lsp]
display-messages = true
display-signature-help-docs = true
"#,
            self.config.tools.helix.theme
        );

        let config_path = self.config.generated_helix_config();
        std::fs::create_dir_all(config_path.parent().unwrap())?;
        std::fs::write(&config_path, helix_config)?;

        info!("Helix configuration saved to: {}", config_path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LayoutPane, LayoutPart};

    fn config_with_helix_path(path: &str) -> Config {
        let mut config = Config::default();
        config.tools.helix.path = path.to_string();
        config
    }

    #[test]
    fn layout_commands_use_the_configured_tool_path() {
        let config = config_with_helix_path("/bin/sh");
        let manager = WorkspaceManager::new(&config);

        let mut parts = vec![LayoutPart {
            panes: vec![LayoutPane {
                command: "hx".to_string(),
                run: true,
                ..Default::default()
            }],
            ..Default::default()
        }];

        manager.resolve_pane_commands(&mut parts).unwrap();

        assert!(parts[0].panes[0].command.ends_with("/sh"));
        assert!(parts[0].panes[0].command.starts_with('/'));
    }

    #[test]
    fn unresolvable_commands_fail_before_launching_zellij() {
        let config = config_with_helix_path("definitely-not-a-real-binary");
        let manager = WorkspaceManager::new(&config);

        let mut parts = vec![LayoutPart {
            panes: vec![LayoutPane {
                command: "hx".to_string(),
                run: true,
                ..Default::default()
            }],
            ..Default::default()
        }];

        let error = manager.resolve_pane_commands(&mut parts).unwrap_err();
        assert!(error.to_string().contains("definitely-not-a-real-binary"));
    }

    #[test]
    fn panes_that_do_not_run_are_left_alone() {
        let config = config_with_helix_path("definitely-not-a-real-binary");
        let manager = WorkspaceManager::new(&config);

        let mut parts = vec![LayoutPart {
            panes: vec![LayoutPane {
                command: "git delta".to_string(),
                run: false,
                ..Default::default()
            }],
            ..Default::default()
        }];

        manager.resolve_pane_commands(&mut parts).unwrap();
        assert_eq!(parts[0].panes[0].command, "git delta");
    }
}
