// MODULE_CONTRACT
// MODULE_ID: M-WORKSPACE-CLI
// PURPOSE: CLI crate root — command dispatch, dashboard, workspace, test infrastructure
// DEPENDS: M-WORKSPACE-CORE, M-WORKSPACE-ENGINE, M-WORKSPACE-PROXY, M-WORKSPACE-RUN, M-WORKSPACE-SKILLS, M-WORKSPACE-MCP
// LINKS:
//   → Phase-97 (implements) — workspace split
//   ← V-M-WORKSPACE-CLI (verified_by) — verification shard

#![allow(clippy::if_same_then_else, clippy::new_without_default)]

// START_MODULE_MAP
// cli — CLI command dispatch and schema (42+ commands)
// dashboard — Web dashboard with observability routes
// workspace — Multi-project workspace orchestration
// test — Test harness, coverage matrix, e2e runner, snapshots
// VERSION — Crate version from workspace
// NAME — Binary name
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Extracted from synapse-agent monolith into syn-cli workspace crate]
// END_CHANGE_SUMMARY

pub mod cli;
pub mod dashboard;
pub mod workspace;
pub mod test;

// START_CONTRACT_public_api
// PURPOSE: Export compile-time crate metadata constants
// OUTPUTS: { VERSION — crate version string }, { NAME — binary name }
// LINKS:
//   → Phase-97 (traces_to) — workspace split metadata
// START_public_api
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "syn";
// END_public_api
