use crate::actions::{FileManagerAction, GitAction, PaneInfo, TerminalAction};
use crate::config::Config;
use crate::protocol::{Request, Response};
use crate::resolve::resolve_executable;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Default TTL for pane cache in milliseconds
const PANE_CACHE_TTL_MS: u64 = 500;

/// Cached pane information with timestamp
struct CachedPanes {
    timestamp: Instant,
    panes: Vec<PaneInfo>,
    zellij_path: PathBuf,
}

/// Shared application state for the daemon
pub struct App {
    config: Config,
    /// Cache of pane information per session
    pane_cache: RwLock<HashMap<String, CachedPanes>>,
    /// Cache of resolved tool paths
    tool_cache: RwLock<HashMap<String, PathBuf>>,
}

impl App {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            pane_cache: RwLock::new(HashMap::new()),
            tool_cache: RwLock::new(HashMap::new()),
        }
    }

    pub async fn handle_ipc(&self, request: Request) -> Response {
        match request {
            Request::FileManagerOpen { .. } => self.handle_file_manager_open().await,
            Request::FileManagerToggleDock { .. } => self.handle_file_manager_toggle_dock().await,
            Request::FileManagerRun { .. } => self.handle_file_manager_run().await,
            Request::TerminalToggle => self.handle_terminal_toggle().await,
            Request::TerminalZoom => self.handle_terminal_zoom().await,
            Request::GitOpen => self.handle_git_open().await,
            Request::GetPaneList => self.get_pane_list().await,
            Request::GetContext => self.get_context().await,
            Request::Ping => Response::Pong,
            Request::Shutdown => Response::Ok,
        }
    }

    async fn handle_file_manager_open(&self) -> Response {
        self.to_response(self.run_file_manager_ipc(FileManagerAction::Open).await)
    }

    async fn handle_file_manager_toggle_dock(&self) -> Response {
        self.to_response(
            self.run_file_manager_ipc(FileManagerAction::ToggleDock)
                .await,
        )
    }

    async fn handle_file_manager_run(&self) -> Response {
        self.to_response(self.run_file_manager_ipc(FileManagerAction::Run).await)
    }

    async fn handle_terminal_toggle(&self) -> Response {
        self.to_response(self.run_terminal_action(TerminalAction::Toggle).await)
    }

    async fn handle_terminal_zoom(&self) -> Response {
        self.to_response(self.run_terminal_action(TerminalAction::Zoom).await)
    }

    async fn handle_git_open(&self) -> Response {
        self.to_response(self.run_git_action(GitAction::Open).await)
    }

    fn to_response(&self, result: Result<()>) -> Response {
        match result {
            Ok(()) => Response::Ok,
            Err(error) => Response::Error {
                message: error.to_string(),
            },
        }
    }

    async fn run_file_manager_ipc(&self, action: FileManagerAction) -> Result<()> {
        let config = self.config.clone();
        tokio::task::block_in_place(|| match action {
            FileManagerAction::Open => crate::actions::file_manager_focus(&config),
            FileManagerAction::ToggleDock | FileManagerAction::Run => {
                anyhow::bail!("file manager action must run in helper process")
            }
        })?;
        self.invalidate_pane_cache().await;
        Ok(())
    }

    async fn run_terminal_action(&self, action: TerminalAction) -> Result<()> {
        let config = self.config.clone();
        tokio::task::block_in_place(|| crate::actions::terminal_action(&config, action))?;
        self.invalidate_pane_cache().await;
        Ok(())
    }

    async fn run_git_action(&self, action: GitAction) -> Result<()> {
        let config = self.config.clone();
        tokio::task::block_in_place(|| crate::actions::git_action(&config, action))?;
        self.invalidate_pane_cache().await;
        Ok(())
    }

    async fn get_pane_list(&self) -> Response {
        match self.get_panes().await {
            Ok(panes) => Response::PaneList(panes),
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn get_context(&self) -> Response {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let project_root = self
            .config
            .session
            .zellij_config
            .as_deref()
            .map(|p| p.parent().unwrap_or(p).to_path_buf())
            .unwrap_or_else(|| cwd.clone());

        Response::Context {
            cwd,
            project_root,
            git_root: None,
        }
    }

    /// Get panes for a session, using cache if fresh
    pub async fn get_panes(&self) -> Result<Vec<PaneInfo>> {
        let session_name =
            std::env::var("ZELLIJ_SESSION_NAME").unwrap_or_else(|_| "default".to_string());
        let zellij = self.resolved_tool("zellij").await?;

        {
            let cache = self.pane_cache.read().await;
            if let Some(cached) = cache.get(&session_name) {
                if cached.zellij_path == zellij
                    && cached.timestamp.elapsed() < Duration::from_millis(PANE_CACHE_TTL_MS)
                {
                    return Ok(cached.panes.clone());
                }
            }
        }

        let panes = self.fetch_panes(&zellij).await?;

        {
            let mut cache = self.pane_cache.write().await;
            cache.insert(
                session_name,
                CachedPanes {
                    timestamp: Instant::now(),
                    panes: panes.clone(),
                    zellij_path: zellij,
                },
            );
        }

        Ok(panes)
    }

    /// Invalidate pane cache for current session
    pub async fn invalidate_pane_cache(&self) {
        let session_name =
            std::env::var("ZELLIJ_SESSION_NAME").unwrap_or_else(|_| "default".to_string());

        let mut cache = self.pane_cache.write().await;
        cache.remove(&session_name);
    }

    async fn fetch_panes(&self, zellij: &PathBuf) -> Result<Vec<PaneInfo>> {
        let output = tokio::process::Command::new(zellij)
            .args(["action", "list-panes", "--json", "--all"])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to query Zellij panes: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(serde_json::from_slice(&output.stdout)?)
    }

    /// Get resolved tool path, using cache
    pub async fn resolved_tool(&self, tool_name: &str) -> Result<PathBuf> {
        {
            let cache = self.tool_cache.read().await;
            if let Some(path) = cache.get(tool_name) {
                return Ok(path.clone());
            }
        }

        let path = resolve_executable(tool_name)?;

        {
            let mut cache = self.tool_cache.write().await;
            cache.insert(tool_name.to_string(), path.clone());
        }

        Ok(path)
    }
}
