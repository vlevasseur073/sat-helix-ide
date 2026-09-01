//! Helix IDE - Workspace Orchestrator
//!
//! This crate provides a workspace management system that integrates
//! Helix, Zellij, Yazi, Lazygit/GitUI, and git-delta into a cohesive
//! terminal-based IDE experience.

pub mod config;
pub mod error;
pub mod helix;
pub mod tools;
pub mod workspace;
pub mod zellij;

// Re-export main types
pub use config::Config;
pub use error::HxIdeError;
pub use workspace::WorkspaceManager;
