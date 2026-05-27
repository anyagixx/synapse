#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-CI
# PURPOSE: UPGRADE_5 integration gate for cross-cutting command, docs, coverage, workspace, hooks, tools, telemetry, and benchmark surfaces
# SCOPE: Targeted docs parity, workspace CLI smoke, user tool safety tests, hook CLI smoke, code coverage parser tests, Makefile declaration checks, and telemetry config tests
# DEPENDS: M-CAPABILITIES, M-TESTS-PARITY, M-WORKSPACE, M-MCP-USER-TOOLS, M-HOOKS, M-TEST-COVERAGE-MATRIX, M-CONFIG
# LINKS: docs/phases/Phase-95.xml, docs/verification/V-M-CI.xml

# START_MODULE_MAP
# run_upgrade5_gate - Executes targeted UPGRADE_5 integration checks
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.1.0 - Clarified local declaration checks versus hosted coverage and benchmark execution]
# END_CHANGE_SUMMARY

# START_CONTRACT_run_upgrade5_gate
# PURPOSE: Execute targeted UPGRADE_5 checks without duplicating hosted coverage and benchmark gates
# OUTPUTS: { exit code 0 - all UPGRADE_5 checks passed }
# SIDE_EFFECTS: invokes cargo tests and Makefile declaration dry-runs
# LINKS:
#   -> Phase-95 (implements) - UPGRADE_5 integration gate
#   -> NFR-002 (traces_to) - release verification commands must fail clearly
# START_run_upgrade5_gate
set -euo pipefail

echo "[CI][upgrade5][DOCS_PARITY] Checking UPGRADE_5 docs and capability truth"
cargo test --test docs_parity upgrade5 -- --test-threads=1

echo "[CI][upgrade5][WORKSPACE] Checking workspace CLI smoke"
cargo test --test workspace_cli -- --test-threads=1

echo "[CI][upgrade5][TOOLS] Checking user-defined MCP tool safety"
cargo test mcp::user_tools --lib -- --test-threads=8
cargo test cli::tools_commands --lib -- --test-threads=8

echo "[CI][upgrade5][HOOKS] Checking pre-commit hook CLI smoke"
cargo test --test integration_test test_hook_pre_commit_cli_round_trip -- --test-threads=1

echo "[CI][upgrade5][COVERAGE] Checking coverage parser and local Makefile declaration"
cargo test code_coverage --lib -- --test-threads=8
make -n coverage

echo "[CI][upgrade5][BENCH] Checking benchmark target declaration; hosted CI runs real benchmark jobs"
make -n bench

echo "[CI][upgrade5][TELEMETRY] Checking telemetry config defaults"
cargo test config::tests::config_key_helpers_get_set_and_unset_typed_values --lib -- --test-threads=8
cargo test config::tests::config_key_helpers_persist_typed_update_to_toml --lib -- --test-threads=8

echo "[CI][upgrade5][PASS] UPGRADE_5 integration gate passed"
# END_run_upgrade5_gate
