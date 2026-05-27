// MODULE_CONTRACT
// MODULE_ID: M-WORKSPACE-ENGINE
// PURPOSE: Engine crate root — bundles indexer, graphrag, and GRACE methodology due to 3-node dependency cycle
// SCOPE: Module declarations, crate-level re-exports of public APIs
// DEPENDS: M-WORKSPACE-CORE
// LINKS:
//   → M-WORKSPACE-CORE (depends) — foundation types and utilities
//   → Phase-97 (implements) — workspace split
//   ← V-M-WORKSPACE-ENGINE (verified_by) — verification shard

#![allow(clippy::if_same_then_else, clippy::new_without_default)]

// START_MODULE_MAP
// indexer — Code indexer, storage, search, parsing, embeddings, walker
// graphrag — Knowledge graph, builder, impact analysis, Mermaid rendering
// grace — GRACE methodology engine (verify, review, contracts, etc.)
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Extracted from synapse-agent monolith into syn-engine workspace crate]
// END_CHANGE_SUMMARY

pub mod grace;
pub mod graphrag;
pub mod indexer;

// Re-export key public types for downstream crates
pub use grace::GraceEngine;
pub use grace::GraceProfile;
pub use graphrag::GraphRag;
pub use indexer::Indexer;
pub use indexer::SearchResult;

// START_CONTRACT_public_api
// PURPOSE: Export compile-time crate version constant
// OUTPUTS: { VERSION — crate version string }
// LINKS:
//   → Phase-97 (traces_to) — workspace split metadata
// START_public_api
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
// END_public_api
