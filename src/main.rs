use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sat_helix_ide::actions::{
    file_manager_action, git_action, mind_map_action, review_action, terminal_action, venv_action,
    workflow_action, FileManagerAction, GitAction, MindMapAction, ReviewAction, TerminalAction,
    VenvAction, WorkflowAction,
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

    /// Project directory (omit to pick interactively)
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Add the AI tab even when session.ai_by_default is off
    #[arg(long, global = true, overrides_with = "no_ai")]
    ai: bool,

    /// Skip the AI tab for this session
    #[arg(long, global = true, overrides_with = "ai")]
    no_ai: bool,

    #[arg(long, global = true)]
    status_bar: Option<bool>,

    /// Override the project-derived Zellij session name
    #[arg(short, long, global = true)]
    session: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start or attach to a project workspace (optional; same as `sat-hx-ide [PATH]`)
    #[command(alias = "start")]
    Init {
        /// Project directory when using the `init` subcommand
        path: Option<PathBuf>,
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

    /// Install a Freedesktop launcher and icon (Linux)
    InstallDesktop,

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

    #[command(name = "__mindmap", hide = true)]
    MindMap {
        #[command(subcommand)]
        action: MindMapCommands,
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

    #[command(name = "__venv", hide = true)]
    Venv {
        #[command(subcommand)]
        action: VenvCommands,
    },

    #[command(name = "__status-bar", hide = true)]
    StatusBar {
        #[command(subcommand)]
        action: StatusBarCommands,
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
enum MindMapCommands {
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

#[derive(Subcommand, Debug)]
enum StatusBarCommands {
    /// Refresh the session status line until the pane closes.
    Run {
        #[arg(long)]
        session: String,
        #[arg(long)]
        project_dir: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum VenvCommands {
    Toggle,
    Select,
    /// Interactive ratatui selector (spawned in a floating Zellij pane).
    RunSelector {
        #[arg(long)]
        session: String,
    },
    /// Verify that a path exists and looks like an activatable environment.
    ValidatePath {
        path: PathBuf,
    },
}

fn run_workspace(cli: &Cli, path: Option<PathBuf>) -> Result<()> {
    let config = Config::load(&cli.config)?;
    let path = match path {
        Some(path) => path,
        None => match sat_helix_ide::workspace::pick_project_dir(&config)? {
            Some(path) => path,
            None => return Ok(()),
        },
    };
    let ai_override = match (cli.ai, cli.no_ai) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    };
    WorkspaceManager::new(&config, &cli.config)
        .init_workspace(&path, cli.session.as_deref(), ai_override, cli.status_bar)
        .context("Failed to initialize workspace")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    let workspace_path = match &cli.command {
        None => cli.path.clone(),
        Some(Commands::Init { path }) => path.clone().or(cli.path.clone()),
        Some(_) => None,
    };

    match cli.command {
        None | Some(Commands::Init { .. }) => run_workspace(&cli, workspace_path)?,
        Some(Commands::Config) => {
            println!("{:#?}", Config::load(&cli.config)?);
        }
        Some(Commands::Doctor) => {
            let config = Config::load(&cli.config)?;
            WorkspaceManager::new(&config, &cli.config).check_tools()?;
        }
        Some(Commands::Setup { dry_run }) => {
            sat_helix_ide::setup::run(&cli.config, dry_run)?;
        }
        Some(Commands::Version) => {
            println!("sat-hx-ide {}", env!("CARGO_PKG_VERSION"));
        }
        Some(Commands::InstallDesktop) => {
            sat_helix_ide::desktop::install_desktop_entry(None)?;
        }
        Some(Commands::FileManager { action }) => {
            let config = Config::load(&cli.config)?;
            let action = match action {
                FileManagerCommands::Open => FileManagerAction::Open,
                FileManagerCommands::ToggleDock => FileManagerAction::ToggleDock,
                FileManagerCommands::Run => FileManagerAction::Run,
            };
            file_manager_action(&config, &cli.config, action)?;
        }
        Some(Commands::Terminal { action }) => {
            let config = Config::load(&cli.config)?;
            let action = match action {
                TerminalCommands::Toggle => TerminalAction::Toggle,
                TerminalCommands::Zoom => TerminalAction::Zoom,
            };
            terminal_action(&config, action)?;
        }
        Some(Commands::MindMap { action }) => {
            let config = Config::load(&cli.config)?;
            let action = match action {
                MindMapCommands::Toggle => MindMapAction::Toggle,
                MindMapCommands::Zoom => MindMapAction::Zoom,
            };
            mind_map_action(&config, action)?;
        }
        Some(Commands::Git { action }) => {
            let config = Config::load(&cli.config)?;
            let action = match action {
                GitCommands::Open => GitAction::Open,
            };
            git_action(&config, action)?;
        }
        Some(Commands::Review { action }) => {
            let config = Config::load(&cli.config)?;
            let action = match action {
                ReviewCommands::Open => ReviewAction::Open,
            };
            review_action(&config, action)?;
        }
        Some(Commands::Workflow { action }) => {
            let config = Config::load(&cli.config)?;
            let action = match action {
                WorkflowCommands::Open => WorkflowAction::Open,
            };
            workflow_action(&config, action)?;
        }
        Some(Commands::Venv { action }) => match action {
            VenvCommands::ValidatePath { path } => {
                if let Err(err) = sat_helix_ide::venv::validate_custom_venv_path(&path) {
                    eprintln!("{err:#}");
                    std::process::exit(1);
                }
            }
            VenvCommands::RunSelector { session } => {
                let config = Config::load(&cli.config)?;
                match sat_helix_ide::venv::run_selector_for_session(&config, &session) {
                    Ok(sat_helix_ide::venv::SelectorRun::Cancelled) => std::process::exit(1),
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("{err:#}");
                        std::process::exit(1);
                    }
                }
            }
            other => {
                let config = Config::load(&cli.config)?;
                let action = match other {
                    VenvCommands::Toggle => VenvAction::Toggle,
                    VenvCommands::Select => VenvAction::Select,
                    VenvCommands::RunSelector { .. } | VenvCommands::ValidatePath { .. } => {
                        unreachable!()
                    }
                };
                venv_action(&config, action)?;
            }
        },
        Some(Commands::StatusBar { action }) => {
            let config = Config::load(&cli.config)?;
            match action {
                StatusBarCommands::Run {
                    session,
                    project_dir,
                } => {
                    sat_helix_ide::statusbar::run_status_bar_loop(&config, &project_dir, &session)?;
                }
            }
        }
    }
    Ok(())
}
