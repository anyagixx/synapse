// MODULE_CONTRACT
// MODULE_ID: M-WORKSPACE-CORE
// PURPOSE: Foundation crate root — publishes config, utils, telemetry, tracking, memory, compress, hooks, and agent_console as a standalone dependency
// SCOPE: Module declarations, crate-level re-exports, and VERSION/NAME constants
// DEPENDS: none (leaf crate)
// LINKS:
//   → Phase-97 (implements) — workspace split
//   ← V-M-WORKSPACE-CORE (verified_by) — verification shard

#![allow(clippy::if_same_then_else, clippy::new_without_default)]

// START_MODULE_MAP
// VERSION — Crate version from workspace
// NAME — Crate name
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Extracted from synapse-agent monolith into syn-core workspace crate]
// END_CHANGE_SUMMARY

pub mod agent_console;
pub mod compress;
pub mod config;
pub mod hooks;
pub mod memory;
pub mod telemetry;
pub mod tracking;
pub mod utils;

// START_CONTRACT_public_api
// PURPOSE: Export compile-time crate metadata constants
// OUTPUTS: { VERSION — crate version string }, { NAME — crate identifier }
// LINKS:
//   → NFR-002 (traces_to) — version constants support reproducible debugging
// START_public_api
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "syn-core";
// END_public_api
