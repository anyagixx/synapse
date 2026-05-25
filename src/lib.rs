// MODULE_CONTRACT
// MODULE_ID: M-LIB
// PURPOSE: Crate root — declares all public modules, version, and crate-level lints
// SCOPE: Module declarations, VERSION/NAME constants, clippy allows
// DEPENDS: all sub-modules
// LINKS: Cargo.toml

#![allow(clippy::if_same_then_else, clippy::new_without_default)]

// START_MODULE_MAP
// VERSION — Crate version from Cargo.toml
// NAME — Binary name from Cargo.toml
// test — UPGRADE_3 E2E and regression test harness facade
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.14.0 — Exposed UPGRADE_3 test harness facade]
// END_CHANGE_SUMMARY

// START_CONTRACT_public_api
// PURPOSE: Expose crate modules and compile-time metadata constants
// OUTPUTS: { public module namespace }, { VERSION }, { NAME }
pub mod agent_console;
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
pub mod memory;
pub mod proxy;
pub mod run;
pub mod skills;
pub mod test;
pub mod tracking;
pub mod utils;

// START_public_api
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "syn";
// END_public_api
