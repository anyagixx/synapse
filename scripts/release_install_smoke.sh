#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-CI-RELEASE-SMOKE
# PURPOSE: Release/install smoke gate validates the packaged syn binary before release.
# SCOPE: Release build, Linux tarball packaging, SHA256 checksum verification, extraction, executable check, and version smoke.
# DEPENDS: M-BUILD, M-INSTALL
# LINKS: .github/workflows/ci.yml, .github/workflows/release.yml, install.sh

# START_MODULE_MAP
# run_release_install_smoke - Builds, packages, checksums, extracts, and executes the Linux release artifact
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.1.0 - Added release artifact SHA256 checksum verification]
# END_CHANGE_SUMMARY

# START_CONTRACT_run_release_install_smoke
# PURPOSE: Validate that the release tarball checksum and layout can be verified and executed by installer consumers
# OUTPUTS: { exit code 0 - packaged syn binary runs --version }
# SIDE_EFFECTS: invokes cargo build --release --locked and writes temporary package files
# LINKS: M-BUILD, M-INSTALL
# START_run_release_install_smoke
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
artifact_name="syn-x86_64-unknown-linux-gnu.tar.gz"

if [[ "$(uname -s)" != "Linux" ]]; then
    echo "[CI][release_install_smoke][SKIP] Linux release artifact smoke only runs on Linux"
    exit 0
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

dist_dir="$tmp_dir/dist"
unpack_dir="$tmp_dir/unpack"
artifact_path="$tmp_dir/$artifact_name"
checksum_file="$tmp_dir/SHA256SUMS"

echo "[CI][release_install_smoke][BUILD] Building release binary"
(cd "$repo_root" && cargo build --release --locked)

echo "[CI][release_install_smoke][PACKAGE] Packaging ${artifact_name}"
mkdir -p "$dist_dir" "$unpack_dir"
cp "$repo_root/target/release/syn" "$dist_dir/"
cp "$repo_root/README.md" "$dist_dir/"
[[ -f "$repo_root/GUIDE.md" ]] && cp "$repo_root/GUIDE.md" "$dist_dir/" || true
[[ -f "$repo_root/LICENSE" ]] && cp "$repo_root/LICENSE" "$dist_dir/" || true
tar -czf "$artifact_path" -C "$dist_dir" .

echo "[CI][release_install_smoke][CHECKSUM] Verifying ${artifact_name}"
(cd "$tmp_dir" && sha256sum "$artifact_name" > "$checksum_file")
(cd "$tmp_dir" && sha256sum -c "$checksum_file")

echo "[CI][release_install_smoke][EXTRACT] Extracting ${artifact_name}"
tar -xzf "$artifact_path" -C "$unpack_dir"
test -x "$unpack_dir/syn"

echo "[CI][release_install_smoke][RUN] Checking packaged binary"
"$unpack_dir/syn" --version >/dev/null
echo "[CI][release_install_smoke][PASS] Release/install smoke passed"
# END_run_release_install_smoke
