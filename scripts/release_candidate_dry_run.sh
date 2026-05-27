#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-CI
# PURPOSE: Release candidate dry-run gate validates release metadata, installer truth, and full RTK parity before publishing.
# SCOPE: Tag/version and release-context freshness guards, checked-out candidate SHA evidence, generated release notes policy, checksum aggregation policy, supported Linux/macOS x86_64/aarch64 installer dry-run mapping, full RTK release gate, GitHub step-summary evidence, and optional local release smoke.
# DEPENDS: M-CI-RELEASE-SMOKE, M-RTK-FULL-PARITY, M-INSTALL, M-TESTS-PARITY
# LINKS: .github/workflows/release-candidate.yml, .github/workflows/release.yml, scripts/release_version_guard.sh, scripts/release_install_smoke.sh, scripts/rtk_full_release_gate.sh

# START_MODULE_MAP
# read_cargo_version - Extracts package.version from Cargo.toml
# require_file_contains - Fails when a release-policy marker is missing
# check_release_workflow_truth - Validates release notes, checksums, and artifact declarations
# check_installer_dry_run_case - Validates one installer platform mapping
# check_installer_matrix - Validates supported Linux x86_64/aarch64 and macOS x86_64/arm64 installer mappings
# write_step_summary - Writes release-candidate evidence to GitHub Step Summary when available
# run_release_candidate_dry_run - Executes the full dry-run gate
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.8.0 - Restored Intel macOS prebuilt release matrix]
# END_CHANGE_SUMMARY

# START_CONTRACT_run_release_candidate_dry_run
# PURPOSE: Validate release candidate policy without creating or mutating a GitHub release.
# INPUTS: { SYN_RELEASE_TAG: optional tag override }, { SYN_RC_SKIP_SMOKE: optional flag to skip local release smoke }, { SYN_RC_SKIP_FULL_RTK: optional flag to skip full RTK in non-release CI }, { SYN_RC_SKIP_FRESHNESS: optional flag to skip freshness in non-release CI }, { SYNAPSE_RTK_SOURCE: optional rtk-develop source path }, { SYNAPSE_RTK_SOURCE_REF: optional pinned RTK source ref }
# OUTPUTS: { exit code 0 - candidate checks pass }
# SIDE_EFFECTS: invokes release_version_guard.sh, install.sh --dry-run, rtk_full_release_gate.sh unless explicitly skipped, optionally release_install_smoke.sh, and may append to GITHUB_STEP_SUMMARY
# LINKS:
#   -> M-CI (depends) - CI release candidate gate
#   -> M-INSTALL (depends) - installer dry-run mapping
#   -> M-CI-RELEASE-SMOKE (depends) - release smoke policy
#   -> M-RTK-FULL-PARITY (depends) - release-blocking full RTK parity
# START_run_release_candidate_dry_run
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
release_workflow="$repo_root/.github/workflows/release.yml"
install_script="$repo_root/install.sh"

# START_CONTRACT_read_cargo_version
# PURPOSE: Extract package.version from Cargo.toml for release tag validation.
# OUTPUTS: { stdout - Cargo package version }
# SIDE_EFFECTS: reads Cargo.toml
# START_read_cargo_version
read_cargo_version() {
    awk -F '"' '/^version =/ {print $2; exit}' "$repo_root/Cargo.toml"
}
# END_read_cargo_version

# START_CONTRACT_require_file_contains
# PURPOSE: Fail the candidate gate when a required release-policy marker is missing.
# INPUTS: { $1: file path }, { $2: literal marker }, { $3: failure message }
# OUTPUTS: { exit code 0 - marker exists }
# SIDE_EFFECTS: reads the selected file and may write failure logs
# START_require_file_contains
require_file_contains() {
    file="$1"
    marker="$2"
    message="$3"
    if ! grep -F "$marker" "$file" >/dev/null 2>&1; then
        echo "[CI][release_candidate][FAIL] ${message}: ${marker}"
        exit 1
    fi
}
# END_require_file_contains

# START_CONTRACT_check_release_workflow_truth
# PURPOSE: Validate release workflow metadata policies used by the real publishing path.
# OUTPUTS: { exit code 0 - release workflow declares notes, checksums, and all supported Linux/macOS artifacts }
# SIDE_EFFECTS: reads .github/workflows/release.yml
# START_check_release_workflow_truth
check_release_workflow_truth() {
    require_file_contains "$release_workflow" "generate_release_notes: true" "release notes must be generated"
    require_file_contains "$release_workflow" "sort -k2 > dist/SHA256SUMS" "checksums must be aggregated deterministically"
    require_file_contains "$release_workflow" "sha256sum -c SHA256SUMS" "checksums must be verified before upload"

    for artifact in \
        syn-x86_64-unknown-linux-gnu.tar.gz \
        syn-aarch64-unknown-linux-gnu.tar.gz \
        syn-x86_64-apple-darwin.tar.gz \
        syn-aarch64-apple-darwin.tar.gz
    do
        require_file_contains "$release_workflow" "$artifact" "release workflow must declare artifact"
    done
}
# END_check_release_workflow_truth

# START_CONTRACT_check_installer_dry_run_case
# PURPOSE: Validate one installer uname mapping against a release artifact name.
# INPUTS: { $1: uname -s value }, { $2: uname -m value }, { $3: expected artifact name }
# OUTPUTS: { exit code 0 - dry-run output maps to the expected artifact }
# SIDE_EFFECTS: executes install.sh --dry-run
# START_check_installer_dry_run_case
check_installer_dry_run_case() {
    system="$1"
    machine="$2"
    expected_artifact="$3"
    output="$(SYN_INSTALL_UNAME_S="$system" SYN_INSTALL_UNAME_M="$machine" sh "$install_script" --dry-run)"
    if ! printf '%s\n' "$output" | grep -F "artifact=${expected_artifact}" >/dev/null 2>&1; then
        echo "[CI][release_candidate][FAIL] ${system}/${machine} did not map to ${expected_artifact}"
        echo "$output"
        exit 1
    fi
}
# END_check_installer_dry_run_case

# START_CONTRACT_check_installer_matrix
# PURPOSE: Validate installer dry-run mapping for the supported Linux/macOS release matrix.
# OUTPUTS: { exit code 0 - all supported installer dry-run mappings match release artifacts }
# SIDE_EFFECTS: executes install.sh --dry-run for each supported release host
# START_check_installer_matrix
check_installer_matrix() {
    check_installer_dry_run_case Linux x86_64 syn-x86_64-unknown-linux-gnu.tar.gz
    check_installer_dry_run_case Linux aarch64 syn-aarch64-unknown-linux-gnu.tar.gz
    check_installer_dry_run_case Darwin x86_64 syn-x86_64-apple-darwin.tar.gz
    check_installer_dry_run_case Darwin arm64 syn-aarch64-apple-darwin.tar.gz
}
# END_check_installer_matrix

# START_CONTRACT_write_step_summary
# PURPOSE: Write release-candidate evidence to GitHub Step Summary for maintainers.
# INPUTS: { $1: smoke_status - skipped or executed }, { $2: full_rtk_status - skipped or executed }
# OUTPUTS: { markdown summary when GITHUB_STEP_SUMMARY is set }
# SIDE_EFFECTS: appends to GITHUB_STEP_SUMMARY and writes a CI log marker
# START_write_step_summary
write_step_summary() {
    smoke_status="$1"
    full_rtk_status="$2"
    if [[ -z "${GITHUB_STEP_SUMMARY:-}" ]]; then
        return 0
    fi

    {
        echo "## Release candidate dry-run"
        echo ""
        echo "- Tag: \`${release_tag}\`"
        echo "- Commit: \`${candidate_commit}\`"
        echo "- Cargo version: \`${cargo_version}\`"
        echo "- Release notes: generated by publishing workflow"
        echo "- Checksums: SHA256SUMS aggregation and verification checked"
        echo "- Installer matrix: Linux x86_64, Linux aarch64, macOS x86_64, macOS arm64"
        echo "- Full RTK gate: ${full_rtk_status}"
        echo "- Smoke: ${smoke_status}"
        echo "- Publishing: not performed by this dry-run"
    } >> "$GITHUB_STEP_SUMMARY"
    echo "[CI][release_candidate][SUMMARY] Wrote GitHub step summary"
}
# END_write_step_summary

cargo_version="$(read_cargo_version)"
release_tag="${SYN_RELEASE_TAG:-v${cargo_version}}"
candidate_commit="$(git -C "$repo_root" rev-parse HEAD 2>/dev/null || echo unknown)"

echo "[CI][release_candidate][TAG] Validating ${release_tag}"
(cd "$repo_root" && SYN_RELEASE_TAG="$release_tag" bash scripts/release_version_guard.sh)

echo "[CI][release_candidate][FRESHNESS] Validating ${release_tag}"
if [[ "${SYN_RC_SKIP_FRESHNESS:-0}" = "1" ]]; then
    echo "[CI][release_candidate][FRESHNESS] Skipping freshness guard; release workflows must not set SYN_RC_SKIP_FRESHNESS"
else
    (cd "$repo_root" && SYN_RELEASE_TAG="$release_tag" bash scripts/release_freshness_guard.sh)
fi

echo "[CI][release_candidate][WORKFLOW] Checking release metadata policy"
check_release_workflow_truth

echo "[CI][release_candidate][INSTALLER] Checking installer dry-run matrix"
check_installer_matrix

if [[ "${SYN_RC_SKIP_FULL_RTK:-0}" = "1" ]]; then
    echo "[CI][release_candidate][RTK_FULL] Skipping full RTK release gate; release workflows must not set SYN_RC_SKIP_FULL_RTK"
    full_rtk_status="skipped by SYN_RC_SKIP_FULL_RTK"
else
    echo "[CI][release_candidate][RTK_FULL] Running full RTK release gate"
    (cd "$repo_root" && bash scripts/rtk_full_release_gate.sh)
    full_rtk_status="executed"
fi

if [[ "${SYN_RC_SKIP_SMOKE:-0}" = "1" ]]; then
    echo "[CI][release_candidate][SMOKE] Skipping local release smoke; matrix smoke is expected elsewhere"
    smoke_status="skipped locally; matrix smoke expected elsewhere"
else
    echo "[CI][release_candidate][SMOKE] Running local release/install smoke"
    (cd "$repo_root" && bash scripts/release_install_smoke.sh)
    smoke_status="executed locally"
fi

write_step_summary "$smoke_status" "$full_rtk_status"
echo "[CI][release_candidate][PASS] Release candidate dry-run passed"
# END_run_release_candidate_dry_run
