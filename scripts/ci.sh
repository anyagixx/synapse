#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-CI
# PURPOSE: CI quality gate — runs Rust checks and MyGRACE truth gates in one reproducible entrypoint
# SCOPE: Formatting, linting, runtime panic guard, tests, isolated XDG data path, release tag guard, local release-candidate dry-run with release-context freshness skipped, optional full RTK release gate, release/install smoke, canonical MyGRACE verification, review, refresh, and status checks
# DEPENDS: M-CI-RUNTIME-GUARD, M-CI-RELEASE-SMOKE, M-RTK-FULL-PARITY, M-GRACE-VERIFY, M-GRACE-REVIEW, M-GRACE-REFRESH, M-GRACE-STATUS
# LINKS: .github/workflows/ci.yml, docs/verification-index.xml

# START_MODULE_MAP
# run_ci_gate — Executes all local and hosted CI gates
# ci_runtime_guard.py — Blocks production panic markers outside tests
# release_freshness_guard.sh — Prevents stale release tags from masquerading as current Cargo versions
# release_candidate_dry_run.sh — Validates release-candidate metadata and installer truth before publishing
# rtk_full_release_gate.sh — Validates source-derived full RTK release parity when explicitly enabled
# release_install_smoke.sh — Builds and validates the packaged release artifact
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.9.0 - Skipped release freshness in local non-release candidate gate]
# END_CHANGE_SUMMARY

# START_CONTRACT_run_ci_gate
# PURPOSE: Execute the full quality gate expected by CI and maintainers
# OUTPUTS: { exit code 0 — all checks passed }
# SIDE_EFFECTS: invokes cargo, Python guard, release version guard, release candidate dry-run, release smoke, and syn verification commands; writes build artifacts under target/
# LINKS:
#   -> M-CI-RUNTIME-GUARD (depends) - production panic guard
#   -> M-CI-RELEASE-SMOKE (depends) - release policy gates
#   -> M-GRACE-VERIFY (depends) - verification gate
#   -> M-GRACE-REVIEW (depends) - integrity review gate
#   -> M-GRACE-REFRESH (depends) - canonical drift gate
#   -> M-GRACE-STATUS (depends) - status gate
# START_run_ci_gate
set -euo pipefail
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
export XDG_DATA_HOME="${XDG_DATA_HOME:-$tmp_dir/xdg-data}"

echo "[CI][run_ci_gate][FMT] Checking formatting"
cargo fmt --all -- --check

echo "[CI][run_ci_gate][CLIPPY] Checking lint warnings"
cargo clippy --all-targets --all-features -- -D warnings

echo "[CI][run_ci_gate][RUNTIME_GUARD] Checking production panic markers"
python3 scripts/ci_runtime_guard.py --self-test
python3 scripts/ci_runtime_guard.py

echo "[CI][run_ci_gate][TEST] Running all target tests"
cargo test --all-targets

echo "[CI][run_ci_gate][RELEASE_VERSION] Checking release tag policy"
bash scripts/release_version_guard.sh

echo "[CI][run_ci_gate][RELEASE_FRESHNESS] Checking release tag freshness"
bash scripts/release_freshness_guard.sh

echo "[CI][run_ci_gate][RELEASE_CANDIDATE] Checking release candidate policy"
SYN_RC_SKIP_SMOKE=1 SYN_RC_SKIP_FULL_RTK=1 SYN_RC_SKIP_FRESHNESS=1 bash scripts/release_candidate_dry_run.sh

if [[ "${SYN_CI_RUN_FULL_RTK:-0}" = "1" ]]; then
    echo "[CI][run_ci_gate][RTK_FULL] Running full RTK release gate"
    bash scripts/rtk_full_release_gate.sh
else
    echo "[CI][run_ci_gate][RTK_FULL] Skipping full RTK release gate in general CI; release candidate workflow runs it by default"
fi

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
