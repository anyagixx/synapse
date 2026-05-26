#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-CI
# PURPOSE: CI gate for the UPGRADE_4 token economy MCP surface.
# SCOPE: Focused checks for profiles, terse schemas, response trimming, cache hints, recommendations, evidence compaction, budget status, and context pressure.
# DEPENDS: M-MCP-SERVER,M-MCP-SERVER-TOOLS,M-MCP-SERVER-GRACE-TOOLS,M-MCP-SERVER-RUN-TOOLS,M-TRACKING,M-RUNNER
# LINKS:
#   -> docs/phases/Phase-88.xml (implements) - UPGRADE_4 token economy integration gate
#   <- docs/verification/V-M-CI.xml (verified_by) - CI verification shard

# START_MODULE_MAP
# run_token_economy_gate - Execute focused UPGRADE_4 token economy checks
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.0.0 - Added UPGRADE_4 token economy CI gate]
# END_CHANGE_SUMMARY

# START_CONTRACT_run_token_economy_gate
# PURPOSE: Run focused token-economy checks used by local and hosted CI.
# OUTPUTS: { exit code 0 — token economy gate passed }
# SIDE_EFFECTS: invokes cargo tests and writes test artifacts under target/
# LINKS:
#   -> M-MCP-SERVER-TOOLS (depends) - profiles and terse tools/list schemas
#   -> M-MCP-SERVER-RESPONSE (depends) - max_tokens response trimming
#   -> M-MCP-SERVER-RUN-TOOLS (depends) - compact_evidence MCP behavior
#   -> M-MCP-SERVER-GRACE-TOOLS (depends) - budget and context pressure tools
# START_run_token_economy_gate
set -euo pipefail

echo "[CI][token_economy][MCP_PROTOCOL] Checking profiles, terse schemas, cache, recommendations, compaction, budget, and pressure"
SYNAPSE_SESSION_ID=ci-token-economy cargo test --test mcp_protocol

echo "[CI][token_economy][E2E_MCP] Checking real stdio token economy smoke"
SYNAPSE_SESSION_ID=ci-token-economy cargo test --test e2e_mcp token_economy

echo "[CI][token_economy][TRIM] Checking response max_tokens trimming"
cargo test mcp::server_code_tools::tests::test_semantic_search_max_tokens_trims_response --lib

echo "[CI][token_economy][PRESSURE] Checking context pressure metadata and forced terse policy"
cargo test mcp::server_pressure::tests --lib
cargo test mcp::server_tools_pressure::tests --lib

echo "[CI][token_economy][BUDGET] Checking budget and context pressure tools"
cargo test mcp::server_budget_tools::tests --lib

echo "[CI][token_economy][COMPACTION] Checking evidence compaction"
cargo test run::evidence_compaction::tests --lib
# END_run_token_economy_gate
