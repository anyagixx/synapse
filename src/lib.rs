pub mod cli;
pub mod config;
pub mod indexer;
pub mod graphrag;
pub mod proxy;
pub mod compress;
pub mod mcp;
pub mod hooks;
pub mod tracking;
pub mod skills;
pub mod utils;
pub mod grace;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "syn";
