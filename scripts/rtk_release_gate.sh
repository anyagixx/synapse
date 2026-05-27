#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-CI-RELEASE-SMOKE
# PURPOSE: RTK release gate validates token-saving parity, hook rewrite diagnostics, economics, and MyGRACE gates before release.
# SCOPE: RTK parity inventory, representative filter tests, hook rewrite checks, discover/learn diagnostics, gain economics, and MyGRACE verify/review checks.
# DEPENDS: M-CLI-RTK-COMMANDS, M-HOOKS, M-TRACKING, M-CI-RELEASE-SMOKE
# LINKS: docs/phases/Phase-50.xml, docs/verification/V-M-CI-RELEASE-SMOKE.xml

# START_MODULE_MAP
# run_syn - Executes the selected Synapse binary or cargo run fallback
# run_syn_at - Executes Synapse in a selected working directory
# assert_contains - Fails when command output does not include a marker
# run_rtk_release_gate - Executes the release-grade RTK verification sequence
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.0.0 - Added RTK release gate for Synapse 2.6]
# END_CHANGE_SUMMARY

# START_CONTRACT_run_rtk_release_gate
# PURPOSE: Exercise the release-critical RTK layer without invoking destructive external tools
# OUTPUTS: { exit code 0 - RTK release gate passed }
# SIDE_EFFECTS: runs cargo-backed Synapse commands and reads/writes temporary tracking data under XDG_DATA_HOME when unset
# LINKS:
#   -> M-CLI-RTK-COMMANDS (depends) - parity, rewrite, discover, and learn commands
#   -> M-HOOKS (depends) - hook audit/check command surface
#   -> M-TRACKING (depends) - gain economics command surface
#   -> M-GRACE-VERIFY (depends) - MyGRACE release verification
# START_run_rtk_release_gate
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

export XDG_DATA_HOME="${XDG_DATA_HOME:-$tmp_dir/xdg-data}"
export SYNAPSE_SESSION_ID="${SYNAPSE_SESSION_ID:-rtk-release-gate}"

if [[ -n "${SYN_BIN:-}" ]]; then
    syn_cmd=("$SYN_BIN")
else
    syn_cmd=(cargo run --manifest-path "$repo_root/Cargo.toml" --quiet --)
fi

# START_CONTRACT_run_syn
# PURPOSE: Execute Synapse from the release gate with a stable working directory
# INPUTS: { $@: syn command arguments }
# OUTPUTS: { command stdout/stderr }
# SIDE_EFFECTS: may build the local crate through cargo run
# START_run_syn
run_syn() {
    (cd "$repo_root" && "${syn_cmd[@]}" "$@")
}
# END_run_syn

# START_CONTRACT_run_syn_at
# PURPOSE: Execute Synapse from an isolated project directory
# INPUTS: { $1: working directory }, { remaining args: syn command arguments }
# OUTPUTS: { command stdout/stderr }
# SIDE_EFFECTS: may write Synapse project files in the selected working directory
# START_run_syn_at
run_syn_at() {
    workdir="$1"
    shift
    (cd "$workdir" && "${syn_cmd[@]}" "$@")
}
# END_run_syn_at

# START_CONTRACT_assert_contains
# PURPOSE: Require a release-gate command output marker
# INPUTS: { $1: haystack }, { $2: marker }, { $3: failure message }
# OUTPUTS: { exit code 0 - marker exists }
# SIDE_EFFECTS: writes failure evidence to stderr
# START_assert_contains
assert_contains() {
    haystack="$1"
    marker="$2"
    message="$3"
    if ! printf '%s\n' "$haystack" | grep -F "$marker" >/dev/null 2>&1; then
        echo "[CI][rtk_release_gate][FAIL] ${message}: ${marker}" >&2
        printf '%s\n' "$haystack" >&2
        exit 1
    fi
}
# END_assert_contains

echo "[CI][rtk_release_gate][PARITY] Checking RTK parity inventory"
run_syn rtk-parity --ci

echo "[CI][rtk_release_gate][FILTERS] Checking representative RTK filters"
run_syn filters verify --filter container-ps
run_syn filters verify --filter jq

echo "[CI][rtk_release_gate][REWRITE] Checking representative hook rewrites"
rewrite_output="$(run_syn rewrite docker ps)"
assert_contains "$rewrite_output" "syn proxy -- docker ps" "docker rewrite must route through syn proxy"
rewrite_output="$(run_syn rewrite git status)"
assert_contains "$rewrite_output" "syn proxy -- git status" "git rewrite must route through syn proxy"

echo "[CI][rtk_release_gate][HOOKS] Checking installable hook audit trust"
hook_project="$tmp_dir/hook-project"
mkdir -p "$hook_project"
run_syn_at "$hook_project" hooks install opencode >/dev/null
run_syn_at "$hook_project" hooks audit --json >/dev/null

echo "[CI][rtk_release_gate][DISCOVER] Checking route discovery diagnostics"
discover_output="$(run_syn discover docker ps)"
assert_contains "$discover_output" "syn docker ps" "discover must recommend container shortcut"

echo "[CI][rtk_release_gate][LEARN] Checking static learning diagnostics"
learn_output="$(run_syn learn --json)"
assert_contains "$learn_output" "suggestions" "learn JSON must include suggestions"

echo "[CI][rtk_release_gate][ECONOMICS] Checking gain analytics surface"
run_syn gain --sessions --adapters

echo "[CI][rtk_release_gate][VERIFY] Running MyGRACE verification gate"
run_syn verify --ci

echo "[CI][rtk_release_gate][REVIEW] Running MyGRACE full review gate"
run_syn review --mode full --ci

echo "[CI][rtk_release_gate][PASS] RTK release gate passed"
# END_run_rtk_release_gate
