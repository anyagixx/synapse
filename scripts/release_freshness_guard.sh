#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-CI-RELEASE-SMOKE
# PURPOSE: Release freshness guard prevents shipping a Cargo version whose Git tag already points at an older commit.
# SCOPE: Read Cargo.toml package version, derive v<version>, detect stale local tags, and require exact tag checkout in release context.
# DEPENDS: M-BUILD, M-INSTALL
# LINKS: Cargo.toml, .github/workflows/ci.yml, .github/workflows/release.yml

# START_MODULE_MAP
# read_cargo_version - Extracts package.version from Cargo.toml
# current_release_tag - Resolves explicit or GitHub-provided release tag
# is_github_tag_ref - Detects whether the current job was triggered from a GitHub tag ref
# run_release_freshness_guard - Validates tag freshness against HEAD
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.1.0 - Migrated semantic LINKS to typed format]
# END_CHANGE_SUMMARY

# START_CONTRACT_run_release_freshness_guard
# PURPOSE: Fail when the current Cargo version maps to a tag that exists but is not the current HEAD.
# INPUTS: { SYN_RELEASE_TAG: optional release tag override }, { GITHUB_REF_NAME/GITHUB_REF_TYPE/GITHUB_REF: GitHub release context }
# OUTPUTS: { exit code 0 - tag is fresh or not yet published, nonzero - stale or wrong release tag }
# SIDE_EFFECTS: reads Cargo.toml and git metadata, writes CI log markers
# LINKS:
#   -> M-CI-RELEASE-SMOKE (depends) - release freshness policy
# START_run_release_freshness_guard
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# START_CONTRACT_read_cargo_version
# PURPOSE: Extract package.version from the repository Cargo.toml.
# OUTPUTS: { stdout - Cargo package version }
# SIDE_EFFECTS: reads Cargo.toml
# START_read_cargo_version
read_cargo_version() {
    awk -F '"' '/^version =/ {print $2; exit}' "$repo_root/Cargo.toml"
}
# END_read_cargo_version

# START_CONTRACT_current_release_tag
# PURPOSE: Resolve the release tag requested by CI or GitHub tag context.
# OUTPUTS: { stdout - tag name or empty string }
# SIDE_EFFECTS: reads environment variables
# START_current_release_tag
current_release_tag() {
    if [[ -n "${SYN_RELEASE_TAG:-}" ]]; then
        printf '%s\n' "$SYN_RELEASE_TAG"
    elif [[ "${GITHUB_REF_TYPE:-}" == "tag" && -n "${GITHUB_REF_NAME:-}" ]]; then
        printf '%s\n' "$GITHUB_REF_NAME"
    elif [[ "${GITHUB_REF:-}" == refs/tags/* ]]; then
        printf '%s\n' "${GITHUB_REF#refs/tags/}"
    fi
}
# END_current_release_tag

cargo_version="$(read_cargo_version)"
expected_tag="v${cargo_version}"
release_tag="$(current_release_tag)"
head_commit="$(git -C "$repo_root" rev-parse HEAD)"

# START_CONTRACT_is_github_tag_ref
# PURPOSE: Detect whether CI is running from a real GitHub tag ref rather than a local dry-run tag override.
# OUTPUTS: { exit code 0 - GitHub tag ref, nonzero - branch/local context }
# SIDE_EFFECTS: reads environment variables
# START_is_github_tag_ref
is_github_tag_ref() {
    [[ "${GITHUB_REF_TYPE:-}" == "tag" || "${GITHUB_REF:-}" == refs/tags/* ]]
}
# END_is_github_tag_ref

echo "[CI][release_freshness_guard][CHECK] expected=${expected_tag} head=${head_commit}"

if git -C "$repo_root" rev-parse -q --verify "refs/tags/${expected_tag}^{commit}" >/dev/null; then
    tag_commit="$(git -C "$repo_root" rev-list -n 1 "$expected_tag")"
    if [[ "$tag_commit" != "$head_commit" ]]; then
        echo "[CI][release_freshness_guard][FAIL] ${expected_tag} points to ${tag_commit}, not HEAD ${head_commit}"
        echo "[CI][release_freshness_guard][FAIL] Bump Cargo.toml/install.sh version before release work continues."
        exit 1
    fi
    echo "[CI][release_freshness_guard][TAG] ${expected_tag} points at HEAD"
else
    echo "[CI][release_freshness_guard][TAG] ${expected_tag} is not published yet"
fi

if [[ -n "$release_tag" ]]; then
    if [[ "$release_tag" != "$expected_tag" ]]; then
        echo "[CI][release_freshness_guard][FAIL] Expected release tag ${expected_tag}, got ${release_tag}"
        exit 1
    fi
fi

if is_github_tag_ref; then
    exact_tag="$(git -C "$repo_root" describe --tags --exact-match HEAD 2>/dev/null || true)"
    if [[ "$exact_tag" != "$expected_tag" ]]; then
        echo "[CI][release_freshness_guard][FAIL] Release context must checkout ${expected_tag} exactly; got ${exact_tag:-no exact tag}"
        exit 1
    fi
fi

echo "[CI][release_freshness_guard][PASS] Release tag freshness validated"
# END_run_release_freshness_guard
