use crate::actions::{FileManagerAction, GitAction, PaneInfo, TerminalAction};
use crate::config::Config;
use crate::protocol::{Request, Response};
use crate::resolve::resolve_executable;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::sync::RwLock;

// Constants from actions module
const FILE_MANAGER_PANE: &str = "file-manager";
const EDITOR_PANE: &str = "editor";
const TERMINAL_PANE: &str = "terminal";

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
            Request::FileManagerOpen => self.handle_file_manager_open().await,
            Request::FileManagerToggleDock => self.handle_file_manager_toggle_dock().await,
            Request::FileManagerRun => self.handle_file_manager_run().await,
            Request::TerminalToggle => self.handle_terminal_toggle().await,
            Request::TerminalZoom => self.handle_terminal_zoom().await,
            Request::GitOpen => self.handle_git_open().await,
            Request::GetPaneList => self.get_pane_list().await,
            Request::GetContext => self.get_context().await,
            Request::Ping => Response::Pong,
            Request::Shutdown => {
                // Handle shutdown
                Response::Ok
            }
        }
    }

    // Handler implementations
    async fn handle_file_manager_open(&self) -> Response {
        match self
            .perform_file_manager_action(FileManagerAction::Open)
            .await
        {
            Ok(_) => Response::Ok,
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_file_manager_toggle_dock(&self) -> Response {
        match self
            .perform_file_manager_action(FileManagerAction::ToggleDock)
            .await
        {
            Ok(_) => Response::Ok,
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_file_manager_run(&self) -> Response {
        match self
            .perform_file_manager_action(FileManagerAction::Run)
            .await
        {
            Ok(_) => Response::Ok,
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_terminal_toggle(&self) -> Response {
        match self.perform_terminal_action(TerminalAction::Toggle).await {
            Ok(_) => Response::Ok,
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_terminal_zoom(&self) -> Response {
        match self.perform_terminal_action(TerminalAction::Zoom).await {
            Ok(_) => Response::Ok,
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    }

    async fn handle_git_open(&self) -> Response {
        match self.perform_git_action(GitAction::Open).await {
            Ok(_) => Response::Ok,
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
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

    // Action implementations using cached panes
    async fn perform_file_manager_action(&self, action: FileManagerAction) -> Result<()> {
        let zellij = self.resolved_tool("zellij").await?;
        let panes = self.get_panes().await?;

        let existing = panes
            .iter()
            .find(|pane| !pane.is_plugin && pane.title == FILE_MANAGER_PANE);

        match action {
            FileManagerAction::Open => {
                if let Some(pane) = existing {
                    self.focus_pane(&zellij, pane).await?;
                    return Ok(());
                }

                // If no file manager exists, we cannot spawn one from daemon context
                // because we don't have ZELLIJ_PANE_ID. Return error to fall back to process model.
                anyhow::bail!("No file manager pane exists; cannot spawn from daemon")
            }
            // For ToggleDock and Run, we cannot handle them in daemon context
            // because they require spawning new panes which needs ZELLIJ_PANE_ID
            FileManagerAction::ToggleDock | FileManagerAction::Run => {
                anyhow::bail!("File manager action requires process context")
            }
        }
    }

    async fn perform_terminal_action(&self, action: TerminalAction) -> Result<()> {
        let zellij = self.resolved_tool("zellij").await?;
        let panes = self.get_panes().await?;

        let editor = panes
            .iter()
            .find(|p| !p.is_plugin && p.title == EDITOR_PANE)
            .ok_or_else(|| anyhow::anyhow!("Cannot find editor pane"))?;

        let Some(terminal) = panes
            .iter()
            .find(|p| !p.is_plugin && p.title == TERMINAL_PANE)
        else {
            return self.respawn_terminal(&zellij, editor).await;
        };

        let hidden = editor.is_fullscreen;
        let zoomed = terminal.is_fullscreen;

        match action {
            TerminalAction::Toggle if hidden => {
                self.toggle_fullscreen(&zellij, editor).await?;
                self.focus_pane(&zellij, terminal).await?;
            }
            TerminalAction::Toggle => {
                if zoomed {
                    self.toggle_fullscreen(&zellij, terminal).await?;
                }
                self.toggle_fullscreen(&zellij, editor).await?;
            }
            TerminalAction::Zoom if zoomed => {
                self.toggle_fullscreen(&zellij, terminal).await?;
            }
            TerminalAction::Zoom => {
                if hidden {
                    self.toggle_fullscreen(&zellij, editor).await?;
                }
                self.toggle_fullscreen(&zellij, terminal).await?;
            }
        }

        self.invalidate_pane_cache().await;

        Ok(())
    }

    /// Dock a fresh shell when the previous terminal pane was closed by the user.
    async fn respawn_terminal(&self, zellij: &Path, editor: &PaneInfo) -> Result<()> {
        let config = self.config.clone();
        let zellij = zellij.to_path_buf();
        let editor = editor.clone();
        tokio::task::block_in_place(|| {
            crate::actions::respawn_terminal(&config, &zellij, &editor)
        })?;
        self.invalidate_pane_cache().await;
        Ok(())
    }

    async fn perform_git_action(&self, action: GitAction) -> Result<()> {
        let zellij = self.resolved_tool("zellij").await?;

        match action {
            GitAction::Open => {
                // Spawn a floating git pane
                // Similar to file manager but for git client
                let panes = self.get_panes().await?;
                let editor = panes
                    .iter()
                    .find(|p| !p.is_plugin && p.title == EDITOR_PANE)
                    .ok_or_else(|| anyhow::anyhow!("Cannot find editor pane"))?;

                self.spawn_git_pane(&zellij, editor).await?;
                Ok(())
            }
        }
    }

    /// Spawn a floating git client pane
    async fn spawn_git_pane(&self, zellij: &PathBuf, editor: &PaneInfo) -> Result<()> {
        // Resolve the git command from config
        let git_command =
            resolve_executable(&self.config.tools.git.command).with_context(|| {
                format!("Cannot find git client '{}'", self.config.tools.git.command)
            })?;

        let output = Command::new(zellij)
            .args(["action", "new-pane", "--close-on-exit"])
            .args(["--name", "git", "--tab-id"])
            .arg(editor.tab_id.to_string())
            .arg("--floating")
            .args(["--x", "0%", "--y", "0%"])
            .arg("--width")
            .arg("100%")
            .arg("--height")
            .arg("100%")
            .arg("--")
            .arg(&git_command)
            .args(&self.config.tools.git.args)
            .output()
            .await
            .context("Failed to create git pane")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to create git pane: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Get panes for a session, using cache if fresh
    pub async fn get_panes(&self) -> Result<Vec<PaneInfo>> {
        // Generate cache key based on session
        let session_name =
            std::env::var("ZELLIJ_SESSION_NAME").unwrap_or_else(|_| "default".to_string());
        let zellij = self.resolved_tool("zellij").await?;

        // Check cache
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

        // Fetch fresh
        let panes = self.fetch_panes(&zellij).await?;

        // Update cache
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

    /// Fetch panes from Zellij
    async fn fetch_panes(&self, zellij: &PathBuf) -> Result<Vec<PaneInfo>> {
        let output = tokio::process::Command::new(zellij)
            .args(["action", "list-panes", "--json", "--all"])
            .output()
            .await
            .with_context(|| "Failed to query Zellij panes")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to query Zellij panes: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        serde_json::from_slice(&output.stdout)
            .with_context(|| "Invalid pane JSON returned by Zellij")
    }

    /// Get resolved tool path, using cache
    pub async fn resolved_tool(&self, tool_name: &str) -> Result<PathBuf> {
        // Check cache
        {
            let cache = self.tool_cache.read().await;
            if let Some(path) = cache.get(tool_name) {
                return Ok(path.clone());
            }
        }

        // Resolve path
        let path = resolve_executable(tool_name)?;

        // Update cache
        {
            let mut cache = self.tool_cache.write().await;
            cache.insert(tool_name.to_string(), path.clone());
        }

        Ok(path)
    }

    // Helper methods for Zellij actions
    async fn toggle_fullscreen(&self, zellij: &PathBuf, pane: &PaneInfo) -> Result<()> {
        let output = tokio::process::Command::new(zellij)
            .args(["action", "toggle-fullscreen", "--pane-id"])
            .arg(pane.cli_id())
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to toggle fullscreen: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    async fn focus_pane(&self, zellij: &PathBuf, pane: &PaneInfo) -> Result<()> {
        let output = tokio::process::Command::new(zellij)
            .args(["action", "focus-pane-id"])
            .arg(pane.cli_id())
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to focus pane: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }
}
