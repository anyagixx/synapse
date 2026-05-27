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
// mcp_regression_module - Includes the MCP stdio regression helper module in the e2e_mcp test target
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 - Added strict MyGRACE function contract for module inclusion]
// END_CHANGE_SUMMARY

// START_public_api

// START_CONTRACT_mcp_regression_module
// PURPOSE: Include the MCP stdio regression helper module as the e2e_mcp integration-test body
// OUTPUTS: { module mcp }
// SIDE_EFFECTS: compiles tests/e2e/mcp/mod.rs into the cargo integration-test target
// LINKS:
//   -> M-TEST-MCP-REGRESSION (depends) - MCP regression helper implementation
//   -> NFR-002 (traces_to) - MCP regression target must preserve protocol reliability evidence
//   <- V-M-TEST-MCP-REGRESSION (verified_by) - MCP regression verification
// START_mcp_regression_module
#[path = "e2e/mcp/mod.rs"]
mod mcp;
// END_mcp_regression_module
// END_public_api
