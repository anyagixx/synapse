// MODULE_CONTRACT
// MODULE_ID: M-WORKSPACE-MCP
// PURPOSE: MCP crate root — JSON-RPC server with 48 tools for code search, GraphRAG, GRACE verification, autonomous runs, and user-defined tools
// DEPENDS: M-WORKSPACE-CORE, M-WORKSPACE-ENGINE, M-WORKSPACE-RUN, M-WORKSPACE-SKILLS
// LINKS:
//   → Phase-97 (implements) — workspace split
//   ← V-M-WORKSPACE-MCP (verified_by) — verification shard

#![allow(clippy::if_same_then_else, clippy::new_without_default)]

// START_MODULE_MAP
// mcp — MCP JSON-RPC server, tools registry, code/grace/cascade/run handlers, LSP, pipeline
// McpServer — Re-exported stdio server entry point
// VERSION — Crate version
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Extracted from synapse-agent monolith into syn-mcp workspace crate]
// END_CHANGE_SUMMARY

pub mod mcp;

pub use mcp::server::McpServer;

// START_CONTRACT_public_api
// PURPOSE: Export compile-time crate version constant
// OUTPUTS: { VERSION — crate version string }
// LINKS:
//   → Phase-97 (traces_to) — workspace split metadata
// START_public_api
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
// END_public_api
