//! Zellij runtime file generators: `layout.kdl` and the merged `config.kdl`.
//! Driving the Zellij binary lives in [`crate::backend::zellij`].

mod layout;
mod runtime_config;

pub use layout::{session_layout, StatusBarLaunch};
pub use runtime_config::{build_runtime_config, RuntimeConfigInput};
