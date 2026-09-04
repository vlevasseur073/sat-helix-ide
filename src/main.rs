use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sat_helix_ide::actions::{parse_zellij_pane_id, FileManagerAction, GitAction, TerminalAction};
use sat_helix_ide::config::Config;
use sat_helix_ide::daemon;
use sat_helix_ide::ipc::{self, send_request};
use sat_helix_ide::protocol::{Request, Response};
use sat_helix_ide::WorkspaceManager;
use std::path::Path;
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

    /// Show version information
    Version,

    #[command(hide = true)]
    Daemon {
        /// Override the IPC socket path
        #[arg(long, short)]
        socket: Option<PathBuf>,
    },

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
    // Set up logging first
    let cli = Cli::parse();
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    // Run the async main function
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main(cli))
}

/// Read the helper pane ID Zellij sets when a keybinding runs `sat-hx-ide`.
fn zellij_source_pane_id() -> Option<u64> {
    match std::env::var("ZELLIJ_PANE_ID") {
        Ok(value) => match parse_zellij_pane_id(&value) {
            Ok(id) => Some(id),
            Err(e) => {
                log::debug!("Invalid ZELLIJ_PANE_ID for IPC: {e}");
                None
            }
        },
        Err(_) => {
            log::debug!("ZELLIJ_PANE_ID missing for IPC file-manager request");
            None
        }
    }
}

/// Convert CLI command to IPC request
fn command_to_request(command: &Commands) -> Option<Request> {
    let source_pane_id = zellij_source_pane_id();
    match command {
        Commands::FileManager { action } => match action {
            FileManagerCommands::Open => Some(Request::FileManagerOpen { source_pane_id }),
            FileManagerCommands::ToggleDock => {
                Some(Request::FileManagerToggleDock { source_pane_id })
            }
            FileManagerCommands::Run => Some(Request::FileManagerRun { source_pane_id }),
        },
        Commands::Terminal { action } => match action {
            TerminalCommands::Toggle => Some(Request::TerminalToggle),
            TerminalCommands::Zoom => Some(Request::TerminalZoom),
        },
        Commands::Git { action } => match action {
            GitCommands::Open => Some(Request::GitOpen),
        },
        // Other commands don't use IPC
        _ => None,
    }
}

/// Try to execute an action via IPC, falling back to process model
async fn try_ipc_or_process(command: Commands, config: &Config, config_path: &Path) -> Result<()> {
    // Check if this command supports IPC
    let Some(request) = command_to_request(&command) else {
        // Command doesn't support IPC, use process model
        return execute_command_process(command, config, config_path).await;
    };

    // Check if daemon is running
    let socket = ipc::current_socket_path();
    if ipc::is_daemon_alive(&socket).await {
        // Try IPC request to daemon
        match send_request(&socket, &request).await {
            Ok(Response::Ok) => return Ok(()),
            Ok(Response::Error { message }) => {
                log::warn!("IPC error: {}", message);
            }
            Ok(other) => {
                log::warn!("Unexpected IPC response: {:?}", other);
            }
            Err(e) => {
                log::warn!("IPC connection error: {}", e);
            }
        }
    } else {
        // Daemon not running, try to spawn it
        log::info!("No daemon running, attempting to spawn...");
        let session_name = std::env::var("ZELLIJ_SESSION_NAME").ok();
        if let Err(e) = spawn_daemon_and_wait(&socket, config_path, session_name.as_deref()).await {
            log::warn!("Failed to spawn daemon: {}", e);
        } else {
            // Daemon should be running now, try IPC again
            match send_request(&socket, &request).await {
                Ok(Response::Ok) => return Ok(()),
                Ok(Response::Error { message }) => {
                    log::warn!("IPC error after spawning daemon: {}", message);
                }
                Ok(other) => {
                    log::warn!("Unexpected IPC response after spawning: {:?}", other);
                }
                Err(e) => {
                    log::warn!("IPC connection error after spawning: {}", e);
                }
            }
            // If we got here, IPC failed after spawning, fall through to process model
        }
        log::debug!("Falling back to process model");
    }

    // Fallback to process model
    execute_command_process(command, config, config_path).await
}

/// Execute command using process model (existing logic)
async fn execute_command_process(
    command: Commands,
    config: &Config,
    config_path: &Path,
) -> Result<()> {
    use sat_helix_ide::actions::{file_manager_action, git_action, terminal_action};
    use std::sync::Arc;

    // Clone the config and config_path for use in the blocking task
    let config = Arc::new(config.clone());
    let config_path = Arc::new(config_path.to_path_buf());

    tokio::task::block_in_place(|| {
        match command {
            Commands::FileManager { action } => {
                let action = match action {
                    FileManagerCommands::Open => FileManagerAction::Open,
                    FileManagerCommands::ToggleDock => FileManagerAction::ToggleDock,
                    FileManagerCommands::Run => FileManagerAction::Run,
                };
                file_manager_action(&config, &config_path, action, None)
            }
            Commands::Terminal { action } => {
                let action = match action {
                    TerminalCommands::Toggle => TerminalAction::Toggle,
                    TerminalCommands::Zoom => TerminalAction::Zoom,
                };
                terminal_action(&config, action)
            }
            Commands::Git { action } => {
                let action = match action {
                    GitCommands::Open => GitAction::Open,
                };
                git_action(&config, action)
            }
            // For other commands, this function shouldn't be called
            _ => Ok(()),
        }
    })
}

/// Spawn the daemon and wait for it to be ready
async fn spawn_daemon_and_wait(
    socket: &PathBuf,
    config_path: &Path,
    session_name: Option<&str>,
) -> Result<()> {
    use std::time::Duration;

    let executable = std::env::current_exe().context("Cannot locate sat-hx-ide executable")?;

    ipc::spawn_daemon(&executable, socket, config_path, session_name)?;

    // Wait for the daemon to start listening
    for _ in 0..50 {
        if ipc::is_daemon_alive(socket).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    anyhow::bail!("Daemon did not start within the timeout period")
}

async fn async_main(cli: Cli) -> Result<()> {
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
                .await
                .context("Failed to initialize workspace")?;
        }
        Commands::Config => {
            println!("{:#?}", Config::load(&cli.config)?);
        }
        Commands::Doctor => {
            let config = Config::load(&cli.config)?;
            WorkspaceManager::new(&config, &cli.config).check_tools()?;
        }
        Commands::Version => {
            println!("sat-hx-ide {}", env!("CARGO_PKG_VERSION"));
        }
        Commands::Daemon { socket } => {
            daemon::run_daemon(socket).await?;
        }
        Commands::FileManager { action } => {
            let config = Config::load(&cli.config)?;
            try_ipc_or_process(Commands::FileManager { action }, &config, &cli.config).await?;
        }
        Commands::Terminal { action } => {
            let config = Config::load(&cli.config)?;
            try_ipc_or_process(Commands::Terminal { action }, &config, &cli.config).await?;
        }
        Commands::Git { action } => {
            let config = Config::load(&cli.config)?;
            try_ipc_or_process(Commands::Git { action }, &config, &cli.config).await?;
        }
    }
    Ok(())
}
