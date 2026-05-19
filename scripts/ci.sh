#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-CI
# PURPOSE: CI quality gate — runs Rust checks and MyGRACE truth gates in one reproducible entrypoint
# SCOPE: Formatting, linting, runtime panic guard, tests, release/install smoke, canonical MyGRACE verification, review, refresh, and status checks
# DEPENDS: M-CI-RUNTIME-GUARD, M-CI-RELEASE-SMOKE, M-GRACE-VERIFY, M-GRACE-REVIEW, M-GRACE-REFRESH, M-GRACE-STATUS
# LINKS: .github/workflows/ci.yml, docs/verification-index.xml

# START_MODULE_MAP
# run_ci_gate — Executes all local and hosted CI gates
# ci_runtime_guard.py — Blocks production panic markers outside tests
# release_install_smoke.sh — Builds and validates the packaged release artifact
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.2.0 - Added release/install smoke gate]
# END_CHANGE_SUMMARY

# START_CONTRACT_run_ci_gate
# PURPOSE: Execute the full quality gate expected by CI and maintainers
# OUTPUTS: { exit code 0 — all checks passed }
# SIDE_EFFECTS: invokes cargo, Python guard, release smoke, and syn verification commands; writes build artifacts under target/
# LINKS: M-CI-RUNTIME-GUARD, M-CI-RELEASE-SMOKE, M-GRACE-VERIFY, M-GRACE-REVIEW, M-GRACE-REFRESH, M-GRACE-STATUS
# START_run_ci_gate
set -euo pipefail
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

echo "[CI][run_ci_gate][FMT] Checking formatting"
cargo fmt --all -- --check

echo "[CI][run_ci_gate][CLIPPY] Checking lint warnings"
cargo clippy --all-targets --all-features -- -D warnings

echo "[CI][run_ci_gate][RUNTIME_GUARD] Checking production panic markers"
python3 scripts/ci_runtime_guard.py --self-test
python3 scripts/ci_runtime_guard.py

echo "[CI][run_ci_gate][TEST] Running all target tests"
cargo test --all-targets

echo "[CI][run_ci_gate][RELEASE_SMOKE] Running release/install smoke"
bash scripts/release_install_smoke.sh

echo "[CI][run_ci_gate][VERIFY] Running MyGRACE verification gate"
cargo run --quiet -- verify --ci

echo "[CI][run_ci_gate][REVIEW] Running MyGRACE full review gate"
cargo run --quiet -- review --mode full --ci

echo "[CI][run_ci_gate][REFRESH] Reporting canonical artifact drift"
cargo run --quiet -- refresh --json > "$tmp_dir/refresh.json"
echo "[CI][run_ci_gate][REFRESH] Drift report generated"

echo "[CI][run_ci_gate][STATUS] Reporting project health"
cargo run --quiet -- status --json > "$tmp_dir/status.json"
echo "[CI][run_ci_gate][STATUS] Status report generated"
# END_run_ci_gate
