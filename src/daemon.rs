use crate::app::App;
use crate::config::Config;
use crate::ipc::{self, serve};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// Daemon state
pub struct Daemon {
    app: Arc<App>,
}

impl Daemon {
    pub fn new(config: &Config) -> Result<Self> {
        let app = Arc::new(App::new(config));
        Ok(Self { app })
    }

    pub async fn run(&self, socket: Option<PathBuf>) -> Result<()> {
        let socket = socket.unwrap_or_else(ipc::current_socket_path);
        serve(&socket, Arc::clone(&self.app)).await
    }
}

/// Run the daemon
pub async fn run_daemon(socket: Option<PathBuf>) -> Result<()> {
    let config = Config::load(
        std::env::var("SAT_HX_IDE_CONFIG")
            .unwrap_or_else(|_| "~/.config/sat-helix-ide/config.toml".into()),
    )?;

    let daemon = Daemon::new(&config)?;
    daemon.run(socket).await
}
