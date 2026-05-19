#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-CI-RELEASE-SMOKE
# PURPOSE: Release version guard rejects GitHub release tags that do not match Cargo.toml.
# SCOPE: Read Cargo.toml package version, compare it to SYN_RELEASE_TAG/GITHUB_REF_NAME, and fail release jobs on mismatch.
# DEPENDS: M-BUILD, M-INSTALL
# LINKS: .github/workflows/release.yml, Cargo.toml

# START_MODULE_MAP
# read_cargo_version - Extracts package.version from Cargo.toml
# run_release_version_guard - Compares release tag to v<package.version>
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.0.0 - Added release tag versus Cargo.toml version guard]
# END_CHANGE_SUMMARY

# START_CONTRACT_run_release_version_guard
# PURPOSE: Ensure a release tag exactly matches the Cargo package version.
# INPUTS: { SYN_RELEASE_TAG: optional release tag override }, { GITHUB_REF_NAME: GitHub tag name fallback }
# OUTPUTS: { exit code 0 - tag matches or no tag is available, nonzero - tag mismatch }
# SIDE_EFFECTS: reads Cargo.toml and writes CI log markers
# LINKS: M-CI-RELEASE-SMOKE
# START_run_release_version_guard
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
release_tag="${SYN_RELEASE_TAG:-${GITHUB_REF_NAME:-}}"

# START_CONTRACT_read_cargo_version
# PURPOSE: Extract package.version from the repository Cargo.toml.
# OUTPUTS: { stdout - Cargo package version }
# SIDE_EFFECTS: reads Cargo.toml
# START_read_cargo_version
read_cargo_version() {
    awk -F '"' '/^version =/ {print $2; exit}' "$repo_root/Cargo.toml"
}
# END_read_cargo_version

if [[ -z "$release_tag" ]]; then
    echo "[CI][release_version_guard][SKIP] No release tag in environment"
    exit 0
fi

cargo_version="$(read_cargo_version)"
expected_tag="v${cargo_version}"

echo "[CI][release_version_guard][VALIDATE] tag=${release_tag} cargo=${cargo_version}"
if [[ "$release_tag" != "$expected_tag" ]]; then
    echo "[CI][release_version_guard][FAIL] Expected ${expected_tag}, got ${release_tag}"
    exit 1
fi

echo "[CI][release_version_guard][PASS] Release tag matches Cargo.toml"
# END_run_release_version_guard
