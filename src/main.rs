use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sat_helix_ide::actions::{
    file_manager_action, git_action, review_action, terminal_action, workflow_action,
    FileManagerAction, GitAction, ReviewAction, TerminalAction, WorkflowAction,
};
use sat_helix_ide::{Config, WorkspaceManager};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "sat-hx-ide")]
#[command(author = "Vincent Levasseur")]
#[command(version)]
#[command(about = "Launch a Helix-centered Zellij workspace")]
struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,

    #[arg(
        short,
        long,
        global = true,
        default_value = "~/.config/sat-helix-ide/config.toml"
    )]
    config: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start or attach to a project workspace
    #[command(alias = "start")]
    Init {
        /// Project directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Add the AI tab even when session.ai_by_default is off
        #[arg(long, overrides_with = "no_ai")]
        ai: bool,

        /// Skip the AI tab for this session
        #[arg(long, overrides_with = "ai")]
        no_ai: bool,

        #[arg(long)]
        status_bar: Option<bool>,

        /// Override the project-derived Zellij session name
        #[arg(short, long)]
        session: Option<String>,
    },

    /// Show the effective sat-hx-ide configuration
    Config,

    /// Check configured executable paths
    #[command(alias = "check")]
    Doctor,

    /// Interactively install companion tools (zellij, helix, yazi, …)
    #[command(alias = "install-tools")]
    Setup {
        /// Print install commands without executing them
        #[arg(long)]
        dry_run: bool,
    },

    /// Show version information
    Version,

    #[command(name = "__file-manager", hide = true)]
    FileManager {
        #[command(subcommand)]
        action: FileManagerCommands,
    },

    #[command(name = "__terminal", hide = true)]
    Terminal {
        #[command(subcommand)]
        action: TerminalCommands,
    },

    #[command(name = "__git", hide = true)]
    Git {
        #[command(subcommand)]
        action: GitCommands,
    },

    #[command(name = "__review", hide = true)]
    Review {
        #[command(subcommand)]
        action: ReviewCommands,
    },

    #[command(name = "__workflow", hide = true)]
    Workflow {
        #[command(subcommand)]
        action: WorkflowCommands,
    },
}

#[derive(Subcommand, Debug)]
enum FileManagerCommands {
    Open,
    ToggleDock,
    Run,
}

#[derive(Subcommand, Debug)]
enum TerminalCommands {
    Toggle,
    Zoom,
}

#[derive(Subcommand, Debug)]
enum GitCommands {
    Open,
}

#[derive(Subcommand, Debug)]
enum ReviewCommands {
    Open,
}

#[derive(Subcommand, Debug)]
enum WorkflowCommands {
    Open,
}

impl Default for Commands {
    fn default() -> Self {
        Commands::Init {
            path: PathBuf::from("."),
            ai: Default::default(),    // false
            no_ai: Default::default(), // false
            status_bar: Default::default(),
            session: Default::default(),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    match cli.command.unwrap_or_default() {
        Commands::Init {
            path,
            ai,
            no_ai,
            status_bar,
            session,
        } => {
            let config = Config::load(&cli.config)?;
            let ai_override = match (ai, no_ai) {
                (true, _) => Some(true),
                (_, true) => Some(false),
                _ => None,
            };
            WorkspaceManager::new(&config, &cli.config)
                .init_workspace(&path, session.as_deref(), ai_override, status_bar)
                .context("Failed to initialize workspace")?;
        }
        Commands::Config => {
            println!("{:#?}", Config::load(&cli.config)?);
        }
        Commands::Doctor => {
            let config = Config::load(&cli.config)?;
            WorkspaceManager::new(&config, &cli.config).check_tools()?;
        }
        Commands::Setup { dry_run } => {
            sat_helix_ide::setup::run(&cli.config, dry_run)?;
        }
        Commands::Version => {
            println!("sat-hx-ide {}", env!("CARGO_PKG_VERSION"));
        }
        Commands::FileManager { action } => {
            let config = Config::load(&cli.config)?;
            let action = match action {
                FileManagerCommands::Open => FileManagerAction::Open,
                FileManagerCommands::ToggleDock => FileManagerAction::ToggleDock,
                FileManagerCommands::Run => FileManagerAction::Run,
            };
            file_manager_action(&config, &cli.config, action)?;
        }
        Commands::Terminal { action } => {
            let config = Config::load(&cli.config)?;
            let action = match action {
                TerminalCommands::Toggle => TerminalAction::Toggle,
                TerminalCommands::Zoom => TerminalAction::Zoom,
            };
            terminal_action(&config, action)?;
        }
        Commands::Git { action } => {
            let config = Config::load(&cli.config)?;
            let action = match action {
                GitCommands::Open => GitAction::Open,
            };
            git_action(&config, action)?;
        }
        Commands::Review { action } => {
            let config = Config::load(&cli.config)?;
            let action = match action {
                ReviewCommands::Open => ReviewAction::Open,
            };
            review_action(&config, action)?;
        }
        Commands::Workflow { action } => {
            let config = Config::load(&cli.config)?;
            let action = match action {
                WorkflowCommands::Open => WorkflowAction::Open,
            };
            workflow_action(&config, action)?;
        }
    }
    Ok(())
}
