//! Helix IDE - Workspace Orchestrator
//!
//! A focused Zellij session composer for Helix and terminal tools.

pub mod actions;
pub mod config;
pub mod error;
pub mod resolve;
pub mod setup;
pub mod workspace;
pub mod zellij;

pub use config::Config;
pub use error::HxIdeError;
pub use resolve::resolve_executable;
pub use workspace::WorkspaceManager;
