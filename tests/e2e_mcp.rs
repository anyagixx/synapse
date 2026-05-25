// MODULE_CONTRACT
// MODULE_ID: M-TEST-MCP-REGRESSION
// PURPOSE: Cargo integration-test target for MCP stdio regression helpers.
// SCOPE: Exposes tests/e2e/mcp/mod.rs as the e2e_mcp test target required by Phase-78 gates.
// DEPENDS: M-TEST-MCP-REGRESSION
// LINKS:
//   -> Phase-78 (implements) - cargo test --test e2e_mcp gate
//   <- V-M-TEST-MCP-REGRESSION (verified_by) - MCP regression verification

// START_MODULE_MAP
// mcp - MCP stdio regression test module
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added e2e_mcp integration test target]
// END_CHANGE_SUMMARY

// START_public_api
#[path = "e2e/mcp/mod.rs"]
mod mcp;
// END_public_api
