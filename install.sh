#!/bin/sh
# MODULE_CONTRACT
# MODULE_ID: M-INSTALL
# PURPOSE: Installer script — installs Synapse from GitHub release artifacts or cargo source fallback
# SCOPE: Linux/macOS platform detection, safe release tarball download, SHA256 verification, local binary install, cargo fallback, dry-run mapping, and post-install smoke check
# DEPENDS: M-BUILD
# LINKS: install.sh, .github/workflows/release.yml

# START_MODULE_MAP
# cleanup — Removes installer temporary directory
# usage — Prints installer usage
# parse_args — Parses version and dry-run/help flags
# detect_platform — Maps uname output to release artifact architecture and OS suffix
# checksum_tool — Selects an available SHA256 verifier
# verify_release_checksum — Verifies the release tarball against SHA256SUMS
# install_binary — Copies a built or extracted syn binary to the target install directory
# install_from_release — Downloads and installs a matching GitHub release artifact
# install_from_source — Builds Synapse from the selected Git tag and installs the binary
# verify_install — Confirms the installed binary executes
# main — Detects platform and installs syn
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v2.13.0 - Bumped default release tag to v2.3.5 for explicit-repo release artifact download validation]
# END_CHANGE_SUMMARY

set -eu

VERSION="v2.3.5"
MODE="install"
ARCH="x86_64"
OS="unknown-linux-gnu"
TMP_DIR=""
INSTALLED_BIN=""

# START_CONTRACT_cleanup
# PURPOSE: Remove temporary installer files created during release or source installation
# OUTPUTS: { none }
# SIDE_EFFECTS: removes the TMP_DIR directory when it exists
# START_cleanup
cleanup() {
    if [ -n "${TMP_DIR:-}" ] && [ -d "$TMP_DIR" ]; then
        rm -rf "$TMP_DIR"
    fi
}
# END_cleanup

# START_CONTRACT_usage
# PURPOSE: Print supported installer flags and environment overrides
# OUTPUTS: { stdout usage text }
# SIDE_EFFECTS: writes to stdout
# START_usage
usage() {
    cat <<'EOF'
Usage: install.sh [version] [--dry-run] [--help] [--version]

Environment:
  SYN_INSTALL_DIR       Install directory override, for example $HOME/.local/bin
  SYN_INSTALL_UNAME_S   Test override for uname -s mapping
  SYN_INSTALL_UNAME_M   Test override for uname -m mapping

Supported prebuilt artifacts:
  syn-x86_64-unknown-linux-gnu.tar.gz
  syn-aarch64-unknown-linux-gnu.tar.gz
  syn-x86_64-apple-darwin.tar.gz
  syn-aarch64-apple-darwin.tar.gz
EOF
}
# END_usage

# START_CONTRACT_parse_args
# PURPOSE: Parse installer flags while preserving optional release tag input
# INPUTS: { "$@": shell arguments }
# OUTPUTS: { VERSION and MODE variables }
# SIDE_EFFECTS: may print usage/version and exit
# START_parse_args
parse_args() {
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --help|-h)
                usage
                exit 0
                ;;
            --version)
                echo "$VERSION"
                exit 0
                ;;
            --dry-run)
                MODE="dry-run"
                ;;
            v*)
                VERSION="$1"
                ;;
            *)
                echo "Unknown argument: $1"
                usage
                exit 2
                ;;
        esac
        shift
    done
}
# END_parse_args

# START_CONTRACT_detect_platform
# PURPOSE: Map Linux/macOS uname output to release artifact naming fields
# OUTPUTS: { ARCH and OS variables for release artifact selection }
# SIDE_EFFECTS: exits on unsupported operating systems or architectures
# START_detect_platform
detect_platform() {
    uname_s="${SYN_INSTALL_UNAME_S:-$(uname -s)}"
    uname_m="${SYN_INSTALL_UNAME_M:-$(uname -m)}"

    case "$uname_s" in
        Linux)  OS="unknown-linux-gnu" ;;
        Darwin) OS="apple-darwin" ;;
        *)
            echo "Unsupported OS '${uname_s}'. Linux and macOS are supported; Windows support is planned later."
            exit 1
            ;;
    esac

    case "$uname_m" in
        x86_64|amd64) ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        *)
            echo "Unsupported architecture '${uname_m}'. Supported: x86_64, aarch64/arm64."
            exit 1
            ;;
    esac
}
# END_detect_platform

# START_CONTRACT_checksum_tool
# PURPOSE: Select an available SHA256 verification command
# OUTPUTS: { sha256sum|shasum - command name supporting SHA256 checks }
# SIDE_EFFECTS: none
# START_checksum_tool
checksum_tool() {
    if command -v sha256sum >/dev/null 2>&1; then
        echo "sha256sum"
        return 0
    fi
    if command -v shasum >/dev/null 2>&1; then
        echo "shasum"
        return 0
    fi
    return 1
}
# END_checksum_tool

# START_CONTRACT_verify_release_checksum
# PURPOSE: Verify the downloaded release tarball against the release SHA256SUMS file
# INPUTS: { $1: checksum_file - SHA256SUMS path }, { $2: artifact_path - downloaded tarball }, { $3: artifact_name - tarball filename }
# OUTPUTS: { exit code 0 - checksum matches, nonzero - checksum unavailable, hard exit - checksum mismatch }
# SIDE_EFFECTS: reads checksum and artifact files
# START_verify_release_checksum
verify_release_checksum() {
    checksum_file="$1"
    artifact_path="$2"
    artifact_name="$3"
    verifier="$(checksum_tool)" || {
        echo "No SHA256 verifier found; trying source build."
        return 1
    }

    checksum_line="$(grep "  ${artifact_name}$" "$checksum_file" || true)"
    if [ -z "$checksum_line" ]; then
        echo "Checksum for ${artifact_name} missing; trying source build."
        return 1
    fi

    expected_hash="${checksum_line%% *}"
    if [ "$verifier" = "sha256sum" ]; then
        actual_line="$(sha256sum "$artifact_path")"
    else
        actual_line="$(shasum -a 256 "$artifact_path")"
    fi
    actual_hash="${actual_line%% *}"

    if [ "$expected_hash" != "$actual_hash" ]; then
        echo "Checksum mismatch for ${artifact_name}."
        exit 1
    fi

    echo "Verified SHA256 checksum for ${artifact_name}."
}
# END_verify_release_checksum

# START_CONTRACT_install_binary
# PURPOSE: Copy a syn binary into the selected install directory
# INPUTS: { $1: source_bin - extracted or compiled syn executable }
# OUTPUTS: { INSTALLED_BIN - absolute path to installed syn binary }
# SIDE_EFFECTS: creates install directory, may invoke sudo, writes syn binary
# START_install_binary
install_binary() {
    source_bin="$1"
    install_dir="${SYN_INSTALL_DIR:-}"

    if [ -z "$install_dir" ]; then
        if [ -w /usr/local/bin ] || command -v sudo >/dev/null 2>&1; then
            install_dir="/usr/local/bin"
        else
            install_dir="$HOME/.local/bin"
        fi
    fi

    target_bin="$install_dir/syn"
    if mkdir -p "$install_dir" 2>/dev/null && [ -w "$install_dir" ]; then
        cp "$source_bin" "$target_bin"
        chmod +x "$target_bin"
    elif command -v sudo >/dev/null 2>&1; then
        sudo mkdir -p "$install_dir"
        sudo cp "$source_bin" "$target_bin"
        sudo chmod +x "$target_bin"
    else
        echo "Cannot write to ${install_dir}. Set SYN_INSTALL_DIR to a writable directory."
        exit 1
    fi

    INSTALLED_BIN="$target_bin"
    echo "Installed syn to ${INSTALLED_BIN}"
}
# END_install_binary

# START_CONTRACT_install_from_release
# PURPOSE: Download and install the release artifact for the detected platform
# OUTPUTS: { exit code 0 - release artifact installed, nonzero - release path unavailable }
# SIDE_EFFECTS: downloads tarball and extracts it under TMP_DIR
# LINKS: .github/workflows/release.yml
# START_install_from_release
install_from_release() {
    command -v curl >/dev/null 2>&1 || { echo "curl not found; trying source build."; return 1; }
    command -v tar >/dev/null 2>&1 || { echo "tar not found; trying source build."; return 1; }

    TARBALL="syn-${ARCH}-${OS}.tar.gz"
    URL="https://github.com/anyagixx/synapse/releases/download/${VERSION}/${TARBALL}"
    CHECKSUM_URL="https://github.com/anyagixx/synapse/releases/download/${VERSION}/SHA256SUMS"
    checksum_file="$TMP_DIR/SHA256SUMS"
    release_dir="$TMP_DIR/release"
    mkdir -p "$release_dir"

    if curl -fsSL "$URL" -o "$TMP_DIR/$TARBALL" 2>/dev/null; then
        if ! curl -fsSL "$CHECKSUM_URL" -o "$checksum_file" 2>/dev/null; then
            echo "Release checksum file unavailable; trying source build."
            return 1
        fi
        verify_release_checksum "$checksum_file" "$TMP_DIR/$TARBALL" "$TARBALL" || return 1
        tar -xzf "$TMP_DIR/$TARBALL" -C "$release_dir"
        if [ ! -x "$release_dir/syn" ]; then
            echo "Release artifact did not contain an executable syn binary."
            return 1
        fi
        install_binary "$release_dir/syn"
        return 0
    fi

    return 1
}
# END_install_from_release

# START_CONTRACT_install_from_source
# PURPOSE: Build and install Synapse from the selected Git tag when no release artifact is available
# OUTPUTS: { installed syn binary or nonzero exit }
# SIDE_EFFECTS: invokes cargo install under TMP_DIR and copies resulting binary
# START_install_from_source
install_from_source() {
    command -v cargo >/dev/null 2>&1 || {
        echo "Rust not installed. Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    }
    command -v git >/dev/null 2>&1 || { echo "git not installed; required for cargo install --git."; exit 1; }

    cargo install --git https://github.com/anyagixx/synapse --tag "${VERSION}" --root "$TMP_DIR/cargo-root"
    install_binary "$TMP_DIR/cargo-root/bin/syn"
}
# END_install_from_source

# START_CONTRACT_verify_install
# PURPOSE: Confirm the installed syn binary starts successfully
# OUTPUTS: { exit code 0 - installed binary reports version }
# SIDE_EFFECTS: executes syn --version
# START_verify_install
verify_install() {
    if [ -z "$INSTALLED_BIN" ] || [ ! -x "$INSTALLED_BIN" ]; then
        echo "Installed binary is missing or not executable."
        exit 1
    fi

    "$INSTALLED_BIN" --version >/dev/null
}
# END_verify_install

# START_CONTRACT_main
# PURPOSE: Install Synapse for the detected platform
# INPUTS: { $1: version - optional release tag }, { SYN_INSTALL_DIR: path - optional target directory }
# OUTPUTS: { installed syn binary or nonzero exit }
# SIDE_EFFECTS: downloads artifacts, copies binary, may run cargo install
# START_main
parse_args "$@"
detect_platform
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/synapse-install.XXXXXX")"
trap cleanup EXIT HUP INT TERM

echo "Installing Synapse ${VERSION} for ${ARCH}-${OS}..."

if [ "$MODE" = "dry-run" ]; then
    echo "artifact=syn-${ARCH}-${OS}.tar.gz"
    echo "checksum=SHA256SUMS"
    echo "install_dir=${SYN_INSTALL_DIR:-auto}"
    echo "mode=dry-run"
    exit 0
fi

if install_from_release; then
    echo "Synapse ${VERSION} installed from release artifact."
else
    echo "No pre-built binary for ${ARCH}-${OS}. Building from source..."
    install_from_source
fi

verify_install

echo ""
echo "Run 'syn --help' to get started."
echo "Quickstart: mkdir my-project && cd my-project && syn init && opencode"
# END_main
