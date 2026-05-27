// MODULE_CONTRACT
// MODULE_ID: M-WORKSPACE-PROXY
// PURPOSE: Proxy crate root — token-saving command proxy with 30+ filter families
// SCOPE: Module declarations, crate-level re-exports
// DEPENDS: M-WORKSPACE-CORE
// LINKS:
//   → M-WORKSPACE-CORE (depends) — config and tracking
//   → Phase-97 (implements) — workspace split
//   ← V-M-WORKSPACE-PROXY (verified_by) — verification shard

#![allow(clippy::if_same_then_else, clippy::new_without_default)]

// START_MODULE_MAP
// proxy — Token-saving command proxy: router, filter engine, runner
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Extracted from synapse-agent monolith into syn-proxy workspace crate]
// END_CHANGE_SUMMARY

pub mod proxy;

// START_CONTRACT_public_api
// PURPOSE: Re-export proxy types for downstream consumers
// OUTPUTS: { CommandRouter — token-saving command dispatch }, { CommandRunner — sandboxed command execution }, { FilterEngine — TOML-based filter chain }
// LINKS:
//   → Phase-97 (traces_to) — workspace split metadata
// START_public_api
pub use proxy::router::CommandRouter;
pub use proxy::runner::CommandRunner;
pub use proxy::toml_filter::FilterEngine;
// END_public_api
