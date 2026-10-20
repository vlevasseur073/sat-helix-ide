//! kitty runtime file generators and remote-control client.
//! Driving kitty itself lives in [`crate::backend::kitty`].

pub mod client;
pub mod runtime_config;
pub mod session;

pub use runtime_config::build_runtime_config;
