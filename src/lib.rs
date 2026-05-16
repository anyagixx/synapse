#![allow(clippy::if_same_then_else, clippy::new_without_default)]
pub mod cli;
pub mod compress;
pub mod config;
pub mod dashboard;
pub mod grace;
pub mod graphrag;
pub mod hooks;
pub mod indexer;
pub mod mcp;
pub mod proxy;
pub mod skills;
pub mod tracking;
pub mod utils;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "syn";
