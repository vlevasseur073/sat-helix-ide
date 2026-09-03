//! Helix IDE - Workspace Orchestrator
//!
//! A focused Zellij session composer for Helix and terminal tools.

pub mod actions;
pub mod app;
pub mod config;
pub mod daemon;
pub mod error;
pub mod ipc;
pub mod protocol;
pub mod resolve;
pub mod workspace;
pub mod zellij;

pub use config::Config;
pub use error::HxIdeError;
pub use resolve::resolve_executable;
pub use workspace::WorkspaceManager;
