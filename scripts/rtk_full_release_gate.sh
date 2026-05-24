#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-CI-RELEASE-SMOKE
# PURPOSE: Full RTK release gate blocks Synapse 2.6 releases unless standalone RTK parity and runtime token-saving gates pass.
# SCOPE: Source-derived RTK full parity with local or pinned public RTK source resolution, all-agent hook install/audit trust, streaming route previews, ecosystem route previews, measured discover/learn/session/economics analytics, clippy, MyGRACE verify, and MyGRACE review.
# DEPENDS: M-RTK-FULL-PARITY, M-CLI-RTK-COMMANDS, M-HOOKS, M-PROXY-RUNNER, M-TRACKING, M-GRACE-VERIFY, M-GRACE-REVIEW
# LINKS: docs/phases/Phase-57.xml, docs/verification/V-M-CI-RELEASE-SMOKE.xml, docs/verification/V-M-RTK-FULL-PARITY.xml

# START_MODULE_MAP
# select_rtk_source - Resolves or fetches the rtk-develop source tree used for full source-derived parity
# run_syn - Executes the selected Synapse binary or cargo run fallback
# run_syn_at - Executes Synapse in a selected working directory
# assert_contains - Fails when command output does not include a marker
# assert_file_exists - Fails when an expected release-gate artifact is missing
# check_route_preview - Verifies that a command preview routes through the expected adapter
# run_full_rtk_release_gate - Executes the release-blocking full RTK verification sequence
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.1.0 - Added pinned public RTK source fallback for hosted release gates]
# END_CHANGE_SUMMARY

# START_CONTRACT_run_full_rtk_release_gate
# PURPOSE: Exercise the release-critical full RTK layer without invoking destructive external tools.
# INPUTS: { SYNAPSE_RTK_SOURCE: optional path to rtk-develop }, { SYNAPSE_RTK_REPOSITORY: optional RTK repository URL }, { SYNAPSE_RTK_SOURCE_REF: optional RTK git ref }, { SYN_BIN: optional prebuilt Synapse binary }
# OUTPUTS: { exit code 0 - full RTK release gate passed }
# SIDE_EFFECTS: may clone the pinned RTK source into a temporary directory, runs cargo-backed Synapse commands, creates temporary hook manifests, and writes isolated tracking data under XDG_DATA_HOME when unset.
# LINKS:
#   -> M-RTK-FULL-PARITY (depends) - source-derived parity matrix
#   -> M-HOOKS (depends) - all-agent hook install/audit trust
#   -> M-PROXY-RUNNER (depends) - route preview and streaming runner behavior
#   -> M-TRACKING (depends) - measured token economics and adoption analytics
#   -> M-GRACE-VERIFY (depends) - MyGRACE verification gate
# START_run_full_rtk_release_gate
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

export XDG_DATA_HOME="${XDG_DATA_HOME:-$tmp_dir/xdg-data}"
export SYNAPSE_SESSION_ID="${SYNAPSE_SESSION_ID:-rtk-full-release-gate}"

if [[ -n "${SYN_BIN:-}" ]]; then
    syn_cmd=("$SYN_BIN")
else
    syn_cmd=(cargo run --manifest-path "$repo_root/Cargo.toml" --quiet --)
fi

# START_CONTRACT_select_rtk_source
# PURPOSE: Resolve or fetch the RTK source tree used by source-derived full parity.
# OUTPUTS: { stdout - absolute or configured rtk-develop path }
# SIDE_EFFECTS: reads filesystem metadata, may clone a pinned public RTK ref, and may fail the release gate with setup guidance.
# START_select_rtk_source
select_rtk_source() {
    if [[ -n "${SYNAPSE_RTK_SOURCE:-}" ]]; then
        if [[ -d "$SYNAPSE_RTK_SOURCE/src" ]]; then
            printf '%s\n' "$SYNAPSE_RTK_SOURCE"
            return 0
        fi
        echo "[CI][rtk_full_release_gate][FAIL] SYNAPSE_RTK_SOURCE does not look like rtk-develop: $SYNAPSE_RTK_SOURCE" >&2
        exit 1
    fi

    local sibling_source="$repo_root/../GRACEme/rtk-develop"
    if [[ -d "$sibling_source/src" ]]; then
        printf '%s\n' "$sibling_source"
        return 0
    fi

    local local_source="/home/truffle/Загрузки/GRACEme/rtk-develop"
    if [[ -d "$local_source/src" ]]; then
        printf '%s\n' "$local_source"
        return 0
    fi

    local source_repo="${SYNAPSE_RTK_REPOSITORY:-https://github.com/rtk-ai/rtk}"
    local source_ref="${SYNAPSE_RTK_SOURCE_REF:-v0.34.3}"
    local fetched_source="$tmp_dir/rtk-source"
    echo "[CI][rtk_full_release_gate][SOURCE] Fetching ${source_repo}@${source_ref}" >&2
    if git clone --depth 1 --branch "$source_ref" "$source_repo" "$fetched_source" >/dev/null 2>&1; then
        printf '%s\n' "$fetched_source"
        return 0
    fi

    echo "[CI][rtk_full_release_gate][FAIL] rtk-develop source not found and pinned fetch failed; set SYNAPSE_RTK_SOURCE to the RTK source checkout" >&2
    exit 1
}
# END_select_rtk_source

# START_CONTRACT_run_syn
# PURPOSE: Execute Synapse from the release gate with a stable working directory.
# INPUTS: { $@: syn command arguments }
# OUTPUTS: { command stdout/stderr }
# SIDE_EFFECTS: may build the local crate through cargo run.
# START_run_syn
run_syn() {
    (cd "$repo_root" && "${syn_cmd[@]}" "$@")
}
# END_run_syn

# START_CONTRACT_run_syn_at
# PURPOSE: Execute Synapse from an isolated project directory.
# INPUTS: { $1: working directory }, { remaining args: syn command arguments }
# OUTPUTS: { command stdout/stderr }
# SIDE_EFFECTS: may write Synapse project files in the selected working directory.
# START_run_syn_at
run_syn_at() {
    local workdir="$1"
    shift
    (cd "$workdir" && "${syn_cmd[@]}" "$@")
}
# END_run_syn_at

# START_CONTRACT_assert_contains
# PURPOSE: Require a release-gate command output marker.
# INPUTS: { $1: haystack }, { $2: marker }, { $3: failure message }
# OUTPUTS: { exit code 0 - marker exists }
# SIDE_EFFECTS: writes failure evidence to stderr.
# START_assert_contains
assert_contains() {
    local haystack="$1"
    local marker="$2"
    local message="$3"
    if ! printf '%s\n' "$haystack" | grep -F -- "$marker" >/dev/null 2>&1; then
        echo "[CI][rtk_full_release_gate][FAIL] ${message}: ${marker}" >&2
        printf '%s\n' "$haystack" >&2
        exit 1
    fi
}
# END_assert_contains

# START_CONTRACT_assert_file_exists
# PURPOSE: Require a release-gate file artifact.
# INPUTS: { $1: file path }, { $2: failure message }
# OUTPUTS: { exit code 0 - file exists }
# SIDE_EFFECTS: writes failure evidence to stderr.
# START_assert_file_exists
assert_file_exists() {
    local file_path="$1"
    local message="$2"
    if [[ ! -f "$file_path" ]]; then
        echo "[CI][rtk_full_release_gate][FAIL] ${message}: ${file_path}" >&2
        exit 1
    fi
}
# END_assert_file_exists

# START_CONTRACT_check_route_preview
# PURPOSE: Verify that a non-executing route preview selects the expected token-saving adapter.
# INPUTS: { $1: expected adapter marker }, { remaining args: syn command route preview }
# OUTPUTS: { exit code 0 - route preview contains should_proxy and expected adapter }
# SIDE_EFFECTS: executes Synapse route-preview commands only.
# START_check_route_preview
check_route_preview() {
    local expected_adapter="$1"
    shift
    local route_output
    route_output="$(run_syn "$@")"
    assert_contains "$route_output" "should_proxy: true" "route preview must proxy"
    assert_contains "$route_output" "adapter:      ${expected_adapter}" "route preview must use expected adapter"
}
# END_check_route_preview

rtk_source="$(select_rtk_source)"

echo "[CI][rtk_full_release_gate][FULL_PARITY] Checking source-derived full RTK parity"
parity_output="$(run_syn rtk-parity --source "$rtk_source" --full --ci)"
assert_contains "$parity_output" "full-standalone-parity: pass" "full parity must pass"
assert_contains "$parity_output" "missing-total: 0" "full parity must have zero missing RTK surfaces"
assert_contains "$parity_output" "- source-commands: pass" "source command parity must pass"
assert_contains "$parity_output" "- hook-processors: pass" "hook processor parity must pass"
assert_contains "$parity_output" "- hook-install-targets: pass" "hook install target parity must pass"
assert_contains "$parity_output" "- command-modules: pass" "command module parity must pass"
assert_contains "$parity_output" "- filters: pass" "filter parity must pass"

echo "[CI][rtk_full_release_gate][HOOKS] Checking all-agent hook install/audit trust"
hook_project="$tmp_dir/hook-project"
mkdir -p "$hook_project"
run_syn_at "$hook_project" hooks install all >/dev/null
hook_audit_output="$(run_syn_at "$hook_project" hooks audit all --json)"
assert_contains "$hook_audit_output" "\"agent\": \"all\"" "hook audit must cover all agents"
assert_contains "$hook_audit_output" "\"ok\": true" "hook audit must pass"
assert_file_exists "$hook_project/.opencode/hooks/synapse-proxy.sh" "OpenCode shell hook must be installed"
assert_file_exists "$hook_project/.synapse/hooks/claude.json" "Claude hook manifest must be installed"
assert_file_exists "$hook_project/.synapse/hooks/cursor.json" "Cursor hook manifest must be installed"
assert_file_exists "$hook_project/.synapse/hooks/gemini.json" "Gemini hook manifest must be installed"
assert_file_exists "$hook_project/.synapse/hooks/copilot.json" "Copilot hook manifest must be installed"

echo "[CI][rtk_full_release_gate][STREAMING] Checking streaming/proxy route previews"
check_route_preview "rust-cargo" proxy --route -- cargo test
check_route_preview "vcs-git" git --route status
check_route_preview "system-text" ls --route
check_route_preview "system-text" grep --route TODO src

echo "[CI][rtk_full_release_gate][ECOSYSTEM] Checking RTK ecosystem route previews"
check_route_preview "vcs-graphite" gt --route log
check_route_preview "infra-cli" docker --route ps
check_route_preview "language-tooling" dotnet --route test
check_route_preview "python-tooling" ruff --route check .
check_route_preview "vcs-hosting" gh --route run list
check_route_preview "python-pytest" pytest --route tests
check_route_preview "infra-cli" kubectl --route get pods

echo "[CI][rtk_full_release_gate][DISCOVER] Checking measured discover/learn diagnostics"
discover_output="$(run_syn discover --json --limit 3)"
assert_contains "$discover_output" "\"opportunities\"" "discover JSON must expose opportunities"
assert_contains "$discover_output" "\"history\"" "discover JSON must expose local history"
learn_output="$(run_syn learn --json)"
assert_contains "$learn_output" "\"mode\": \"measured-guidance\"" "learn must use measured guidance"
assert_contains "$learn_output" "\"signals\"" "learn JSON must expose tracking signals"

echo "[CI][rtk_full_release_gate][ECONOMICS] Checking measured adoption economics"
session_output="$(run_syn session --json)"
assert_contains "$session_output" "\"adoption\"" "session JSON must expose adoption analytics"
economics_output="$(run_syn cc-economics --format json)"
assert_contains "$economics_output" "\"local_only\": true" "economics must remain local-only"
assert_contains "$economics_output" "\"route_adoption_pct\"" "economics JSON must expose route adoption"

echo "[CI][rtk_full_release_gate][CLIPPY] Checking lint warnings"
(cd "$repo_root" && cargo clippy --all-targets --all-features -- -D warnings)

echo "[CI][rtk_full_release_gate][VERIFY] Running MyGRACE verification gate"
run_syn verify --ci

echo "[CI][rtk_full_release_gate][REVIEW] Running MyGRACE full review gate"
run_syn review --mode full --ci

echo "[CI][rtk_full_release_gate][PASS] Full RTK release gate passed"
# END_run_full_rtk_release_gate
