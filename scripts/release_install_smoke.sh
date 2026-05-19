#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-CI-RELEASE-SMOKE
# PURPOSE: Release/install smoke gate validates the packaged syn binary before release.
# SCOPE: Linux/macOS release build, tarball packaging, SHA256 checksum verification, extraction, executable check, and Cargo.toml version smoke.
# DEPENDS: M-BUILD, M-INSTALL
# LINKS: .github/workflows/ci.yml, .github/workflows/release.yml, install.sh

# START_MODULE_MAP
# checksum_generate - Writes a SHA256 checksum sidecar for a release artifact
# checksum_verify - Verifies a SHA256 checksum sidecar for a release artifact
# run_release_install_smoke - Builds, packages, checksums, extracts, and executes a Linux/macOS release artifact
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.3.0 - Added packaged syn --version assertion against Cargo.toml]
# END_CHANGE_SUMMARY

# START_CONTRACT_run_release_install_smoke
# PURPOSE: Validate that the release tarball checksum and layout can be verified and executed by installer consumers
# OUTPUTS: { exit code 0 - packaged syn binary runs --version }
# SIDE_EFFECTS: invokes cargo build --release --locked and writes temporary package files
# LINKS: M-BUILD, M-INSTALL
# START_run_release_install_smoke
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
host_os="$(uname -s)"
host_arch="$(uname -m)"

case "${host_os}:${host_arch}" in
    Linux:x86_64|Linux:amd64) default_target="x86_64-unknown-linux-gnu" ;;
    Linux:aarch64|Linux:arm64) default_target="aarch64-unknown-linux-gnu" ;;
    Darwin:x86_64) default_target="x86_64-apple-darwin" ;;
    Darwin:arm64|Darwin:aarch64) default_target="aarch64-apple-darwin" ;;
    *) echo "[CI][release_install_smoke][SKIP] Unsupported release smoke host ${host_os}/${host_arch}"; exit 0 ;;
esac

release_target="${SYN_RELEASE_TARGET:-$default_target}"
artifact_name="${SYN_RELEASE_ARTIFACT:-syn-${release_target}.tar.gz}"
output_dir="${SYN_RELEASE_OUTPUT_DIR:-}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

dist_dir="$tmp_dir/dist"
unpack_dir="$tmp_dir/unpack"
if [[ -z "$output_dir" ]]; then
    output_dir="$tmp_dir"
fi
mkdir -p "$output_dir"
artifact_path="$output_dir/$artifact_name"
checksum_file="$output_dir/${artifact_name}.sha256"
binary_path="$repo_root/target/$release_target/release/syn"
expected_version="$(awk -F '"' '/^version =/ {print $2; exit}' "$repo_root/Cargo.toml")"

# START_CONTRACT_checksum_generate
# PURPOSE: Generate a SHA256 checksum sidecar for a packaged release artifact
# INPUTS: { artifact_path - release artifact path }, { checksum_file - sidecar path }
# OUTPUTS: { checksum sidecar file }
# SIDE_EFFECTS: writes checksum_file
# START_checksum_generate
checksum_generate() {
    artifact_dir="$(dirname "$artifact_path")"
    artifact_base="$(basename "$artifact_path")"
    checksum_base="$(basename "$checksum_file")"
    if command -v sha256sum >/dev/null 2>&1; then
        (cd "$artifact_dir" && sha256sum "$artifact_base" > "$checksum_base")
    else
        (cd "$artifact_dir" && shasum -a 256 "$artifact_base" > "$checksum_base")
    fi
}
# END_checksum_generate

# START_CONTRACT_checksum_verify
# PURPOSE: Verify a SHA256 checksum sidecar for a packaged release artifact
# INPUTS: { checksum_file - sidecar path }
# OUTPUTS: { exit code 0 - checksum matches }
# SIDE_EFFECTS: reads checksum_file and release artifact
# START_checksum_verify
checksum_verify() {
    artifact_dir="$(dirname "$artifact_path")"
    checksum_base="$(basename "$checksum_file")"
    if command -v sha256sum >/dev/null 2>&1; then
        (cd "$artifact_dir" && sha256sum -c "$checksum_base")
    else
        (cd "$artifact_dir" && shasum -a 256 -c "$checksum_base")
    fi
}
# END_checksum_verify

echo "[CI][release_install_smoke][BUILD] Building ${release_target}"
(cd "$repo_root" && rustup target add "$release_target" >/dev/null && cargo build --release --locked --target "$release_target")

echo "[CI][release_install_smoke][PACKAGE] Packaging ${artifact_name}"
mkdir -p "$dist_dir" "$unpack_dir"
cp "$binary_path" "$dist_dir/"
cp "$repo_root/README.md" "$dist_dir/"
[[ -f "$repo_root/GUIDE.md" ]] && cp "$repo_root/GUIDE.md" "$dist_dir/" || true
[[ -f "$repo_root/LICENSE" ]] && cp "$repo_root/LICENSE" "$dist_dir/" || true
tar -czf "$artifact_path" -C "$dist_dir" .

echo "[CI][release_install_smoke][CHECKSUM] Verifying ${artifact_name}"
checksum_generate
checksum_verify

echo "[CI][release_install_smoke][EXTRACT] Extracting ${artifact_name}"
tar -xzf "$artifact_path" -C "$unpack_dir"
test -x "$unpack_dir/syn"

echo "[CI][release_install_smoke][RUN] Checking packaged binary"
if [[ "${SYN_RELEASE_SKIP_RUN:-0}" != "1" ]]; then
    version_output="$("$unpack_dir/syn" --version)"
    expected_output="syn ${expected_version}"
    if [[ "$version_output" != "$expected_output" ]]; then
        echo "[CI][release_install_smoke][FAIL] Expected '${expected_output}', got '${version_output}'"
        exit 1
    fi
fi
echo "[CI][release_install_smoke][PASS] Release/install smoke passed"
# END_run_release_install_smoke
