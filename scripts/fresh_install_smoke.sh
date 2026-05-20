#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-CI
# PURPOSE: Fresh install smoke gate validates the public installer path on Linux/macOS runners.
# SCOPE: Download installer from a configured URL, install into an isolated SYN_INSTALL_DIR, and verify syn --version/--help.
# DEPENDS: M-INSTALL
# LINKS: install.sh, .github/workflows/release-candidate.yml

# START_MODULE_MAP
# run_fresh_install_smoke - Downloads install.sh, installs syn into a temp dir, and verifies CLI startup
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.0.0 - Added Phase 11 fresh-machine installer evidence gate]
# END_CHANGE_SUMMARY

# START_CONTRACT_run_fresh_install_smoke
# PURPOSE: Validate user-facing curl installer flow without writing outside a temporary install directory.
# INPUTS: { SYN_INSTALL_SCRIPT_URL: installer URL }, { SYN_RELEASE_TAG: optional release tag }
# OUTPUTS: { exit code 0 - installed syn runs --version and --help }
# SIDE_EFFECTS: downloads install.sh, writes temporary install directory, executes installed syn
# START_run_fresh_install_smoke
set -euo pipefail

script_url="${SYN_INSTALL_SCRIPT_URL:-https://raw.githubusercontent.com/anyagixx/synapse/main/install.sh}"
release_tag="${SYN_RELEASE_TAG:-}"
tmp_dir="$(mktemp -d)"
install_dir="$tmp_dir/bin"
installer="$tmp_dir/install.sh"
trap 'rm -rf "$tmp_dir"' EXIT

echo "[CI][fresh_install_smoke][DOWNLOAD] ${script_url}"
curl -fsSL "$script_url" -o "$installer"

echo "[CI][fresh_install_smoke][INSTALL] SYN_INSTALL_DIR=${install_dir}"
if [[ -n "$release_tag" ]]; then
    SYN_INSTALL_DIR="$install_dir" sh "$installer" "$release_tag"
else
    SYN_INSTALL_DIR="$install_dir" sh "$installer"
fi

echo "[CI][fresh_install_smoke][VERIFY] syn --version"
"$install_dir/syn" --version

echo "[CI][fresh_install_smoke][VERIFY] syn --help"
"$install_dir/syn" --help >/dev/null

echo "[CI][fresh_install_smoke][PASS] Fresh install smoke passed"
# END_run_fresh_install_smoke
