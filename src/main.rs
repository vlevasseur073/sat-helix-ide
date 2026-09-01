use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::debug;
use sat_helix_ide::{Config, WorkspaceManager};
use std::path::PathBuf;

/// Helix IDE - A workspace orchestrator for terminal-based development
#[derive(Parser, Debug)]
#[command(name = "sat-hx-ide")]
#[command(author = "Vincent Levasseur")]
#[command(version = "0.1.0")]
#[command(about = "Orchestrate Helix, Zellij, Yazi, Lazygit, and git-delta into a cohesive IDE")]
#[command(long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Configuration file path
    #[arg(
        short,
        long,
        global = true,
        default_value = "~/.config/sat-helix-ide/config.toml"
    )]
    config: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

/// Available commands
#[derive(Subcommand, Debug)]
enum Commands {
    /// Start a new workspace session
    #[command(alias = "start")]
    Init {
        /// Path to the project (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Layout to use (default, coding, review, minimal)
        #[arg(short, long)]
        layout: Option<String>,

        /// Open in new Zellij session
        #[arg(short, long)]
        new_session: bool,
    },

    /// List available layouts
    #[command(alias = "ls")]
    ListLayouts,

    /// Show current workspace configuration
    Config,

    /// Generate optional configs in sat-helix-ide's private config directory
    #[command(alias = "gen")]
    Generate {
        /// Generate Zellij configuration
        #[arg(short, long)]
        zellij: bool,

        /// Generate Yazi configuration
        #[arg(short, long)]
        yazi: bool,

        /// Generate Helix configuration
        #[arg(long)]
        helix: bool,

        /// Generate all configurations
        #[arg(short, long)]
        all: bool,
    },

    /// Check tool availability
    #[command(alias = "check")]
    Doctor,

    /// Show version information
    Version,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    debug!("Parsed CLI arguments: {:?}", cli);

    match cli.command {
        Commands::Init {
            path,
            layout,
            new_session,
        } => {
            let config = Config::load(&cli.config)?;
            let layout = layout.as_deref().unwrap_or(&config.default_layout);
            let new_session = new_session || config.workspace.new_session_default;
            let mut manager = WorkspaceManager::new(&config);
            manager
                .init_workspace(&path, layout, new_session)
                .context("Failed to initialize workspace")?;
        }
        Commands::ListLayouts => {
            let config = Config::load(&cli.config)?;
            let layouts = config.list_layouts();
            println!("Available layouts:");
            for layout in layouts {
                println!("  - {}", layout);
            }
        }
        Commands::Config => {
            let config = Config::load(&cli.config)?;
            println!("{:#?}", config);
        }
        Commands::Generate {
            zellij,
            yazi,
            helix,
            all,
        } => {
            let config = Config::load(&cli.config)?;
            let manager = WorkspaceManager::new(&config);

            if all || zellij {
                manager.generate_zellij_config()?;
            }
            if all || yazi {
                manager.generate_yazi_config()?;
            }
            if all || helix {
                manager.generate_helix_config()?;
            }
        }
        Commands::Doctor => {
            let config = Config::load(&cli.config)?;
            let manager = WorkspaceManager::new(&config);
            manager.check_tools()?;
        }
        Commands::Version => {
            println!("Helix IDE v0.1.0");
            println!("A workspace orchestrator for terminal-based development");
        }
    }

    Ok(())
}
