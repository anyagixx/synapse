// MODULE_CONTRACT
// MODULE_ID: M-LIB
// PURPOSE: Crate root — declares all public modules, version, and crate-level lints
// SCOPE: Module declarations, VERSION/NAME constants, clippy allows
// DEPENDS: all sub-modules
// LINKS: Cargo.toml, M-ALL

#![allow(clippy::if_same_then_else, clippy::new_without_default)]

// START_MODULE_MAP
// VERSION — Crate version from Cargo.toml
// NAME — Binary name from Cargo.toml
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added to all source files]
// END_CHANGE_SUMMARY

pub mod capabilities;
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

// START_public_api
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "syn";
// END_public_api
