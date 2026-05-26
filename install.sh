#!/bin/sh
# MODULE_CONTRACT
# MODULE_ID: M-INSTALL
# PURPOSE: Installer script — installs Synapse from GitHub release artifacts or cargo source fallback
# SCOPE: Supported Linux x86_64/aarch64 and macOS x86_64/arm64 platform detection, safe release tarball download, SHA256 verification, local binary install, cargo tag/source-ref/main fallback, dry-run mapping, precise support diagnostics, and post-install smoke check
# DEPENDS: M-BUILD
# LINKS: install.sh, .github/workflows/release.yml

# START_MODULE_MAP
# cleanup — Removes installer temporary directory
# usage — Prints installer usage
# parse_args — Parses version, dry-run, diagnose, and help flags
# detect_platform — Maps uname output to release artifact architecture and OS suffix
# resolve_install_dir — Selects the install directory used by binary installation
# print_command_status — Reports whether a support diagnostic command is available
# print_diagnostics — Prints a Linux/macOS support diagnostic report
# checksum_tool — Selects an available SHA256 verifier
# verify_release_checksum — Verifies the release tarball against SHA256SUMS
# install_binary — Copies a built or extracted syn binary to the target install directory
# install_from_release — Downloads and installs a matching GitHub release artifact
# remote_tag_available — Checks whether the selected release tag exists upstream
# install_from_source — Builds Synapse from the selected Git tag, CI source ref, or default main fallback and installs the binary
# verify_install — Confirms the installed binary executes
# main — Detects platform and installs syn
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v2.28.0 - Restored Intel macOS prebuilt artifact mapping]
# END_CHANGE_SUMMARY

set -eu

DEFAULT_VERSION="v2.6.6"
VERSION="$DEFAULT_VERSION"
VERSION_EXPLICIT="0"
MODE="install"
ARCH="x86_64"
OS="unknown-linux-gnu"
TMP_DIR=""
INSTALLED_BIN=""
GIT_REQUIRED_MESSAGE="git not installed; required for cargo install --git. Run: sh install.sh --diagnose"

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
Usage: install.sh [version] [--dry-run] [--diagnose] [--help] [--version]

Environment:
  SYN_INSTALL_DIR       Install directory override, for example $HOME/.local/bin
  SYN_INSTALL_UNAME_S   Test override for uname -s mapping
  SYN_INSTALL_UNAME_M   Test override for uname -m mapping
  SYN_INSTALL_SOURCE_REF CI/test override for source fallback commit ref

Supported prebuilt artifacts:
  syn-x86_64-unknown-linux-gnu.tar.gz
  syn-aarch64-unknown-linux-gnu.tar.gz
  syn-x86_64-apple-darwin.tar.gz
  syn-aarch64-apple-darwin.tar.gz

Diagnostics:
  sh install.sh --diagnose
EOF
}
# END_usage

# START_CONTRACT_parse_args
# PURPOSE: Parse installer flags while preserving optional release tag input.
# INPUTS: { "$@": shell arguments }
# OUTPUTS: { VERSION, VERSION_EXPLICIT, and MODE variables }
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
            --diagnose)
                MODE="diagnose"
                ;;
            v*)
                VERSION="$1"
                VERSION_EXPLICIT="1"
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
# PURPOSE: Map supported Linux/macOS uname output to release artifact naming fields
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
            echo "Unsupported OS '${uname_s}'. Supported prebuilt hosts: Linux x86_64/aarch64 and macOS x86_64/arm64. Windows packaging is deferred."
            echo "Run: sh install.sh --diagnose"
            exit 1
            ;;
    esac

    case "$uname_m" in
        x86_64|amd64) ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        *)
            echo "Unsupported architecture '${uname_m}'. Supported prebuilt hosts: Linux x86_64/aarch64 and macOS x86_64/arm64."
            echo "Run: sh install.sh --diagnose"
            exit 1
            ;;
    esac
}
# END_detect_platform

# START_CONTRACT_resolve_install_dir
# PURPOSE: Select the install directory used by binary installation.
# OUTPUTS: { stdout - target install directory }
# SIDE_EFFECTS: none
# START_resolve_install_dir
resolve_install_dir() {
    if [ -n "${SYN_INSTALL_DIR:-}" ]; then
        echo "$SYN_INSTALL_DIR"
    elif [ -w /usr/local/bin ] || command -v sudo >/dev/null 2>&1; then
        echo "/usr/local/bin"
    else
        echo "$HOME/.local/bin"
    fi
}
# END_resolve_install_dir

# START_CONTRACT_print_command_status
# PURPOSE: Report whether a command needed by release or source install is available.
# INPUTS: { $1: command name }
# OUTPUTS: { stdout status line }
# SIDE_EFFECTS: none
# START_print_command_status
print_command_status() {
    cmd="$1"
    if command -v "$cmd" >/dev/null 2>&1; then
        echo "tool.${cmd}=ok"
    else
        echo "tool.${cmd}=missing"
    fi
}
# END_print_command_status

# START_CONTRACT_print_diagnostics
# PURPOSE: Print a no-write supported Linux/macOS support report for install troubleshooting.
# OUTPUTS: { stdout key-value diagnostic report }
# SIDE_EFFECTS: reads uname, PATH, and local tool availability
# START_print_diagnostics
print_diagnostics() {
    uname_s="${SYN_INSTALL_UNAME_S:-$(uname -s)}"
    uname_m="${SYN_INSTALL_UNAME_M:-$(uname -m)}"
    diag_os="unsupported"
    diag_arch="unsupported"
    os_status="ok"
    arch_status="ok"
    platform_status="ok"

    case "$uname_s" in
        Linux) diag_os="unknown-linux-gnu" ;;
        Darwin) diag_os="apple-darwin" ;;
        *)
            os_status="unsupported"
            platform_status="unsupported-os"
            ;;
    esac

    case "$uname_m" in
        x86_64|amd64) diag_arch="x86_64" ;;
        aarch64|arm64) diag_arch="aarch64" ;;
        *)
            arch_status="unsupported"
            if [ "$platform_status" = "unsupported-os" ]; then
                platform_status="unsupported-os-and-arch"
            else
                platform_status="unsupported-arch"
            fi
            ;;
    esac

    install_dir="$(resolve_install_dir)"
    install_parent="$(dirname "$install_dir")"
    if [ -d "$install_dir" ] && [ -w "$install_dir" ]; then
        install_dir_status="writable"
    elif [ -d "$install_parent" ] && [ -w "$install_parent" ]; then
        install_dir_status="creatable"
    elif command -v sudo >/dev/null 2>&1; then
        install_dir_status="sudo"
    else
        install_dir_status="not-writable"
    fi

    echo "mode=diagnose"
    echo "version=${VERSION}"
    echo "host_os=${uname_s}"
    echo "host_arch=${uname_m}"
    echo "os_status=${os_status}"
    echo "arch_status=${arch_status}"
    echo "platform_status=${platform_status}"
    if [ "$platform_status" = "ok" ]; then
        echo "artifact=syn-${diag_arch}-${diag_os}.tar.gz"
    else
        echo "artifact=none"
    fi
    echo "checksum=SHA256SUMS"
    echo "install_dir=${install_dir}"
    echo "install_dir_status=${install_dir_status}"
    print_command_status curl
    print_command_status tar
    print_command_status cargo
    print_command_status git
    print_command_status sudo
    if checksum_tool >/dev/null 2>&1; then
        echo "tool.sha256=ok"
    else
        echo "tool.sha256=missing"
    fi
    echo "macos_intel_packaging=supported"
    echo "windows_packaging=deferred"
    echo "support_hint=Use SYN_INSTALL_DIR for a writable install path, install curl/tar for release downloads, or install Rust and git for source fallback."
}
# END_print_diagnostics

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
    install_dir="$(resolve_install_dir)"

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
        echo "Run: sh install.sh --diagnose"
        exit 1
    fi

    INSTALLED_BIN="$target_bin"
    echo "Installed syn to ${INSTALLED_BIN}"
}
# END_install_binary

# START_CONTRACT_remote_tag_available
# PURPOSE: Check whether the selected release tag exists upstream before using tagged source fallback.
# INPUTS: { $1: tag - release tag such as v2.5.1 }
# OUTPUTS: { exit code 0 - tag exists, nonzero - tag unavailable }
# SIDE_EFFECTS: reads remote git refs
# START_remote_tag_available
remote_tag_available() {
    tag="$1"
    git ls-remote --exit-code --tags https://github.com/anyagixx/synapse "refs/tags/${tag}" >/dev/null 2>&1
}
# END_remote_tag_available

# START_CONTRACT_install_from_release
# PURPOSE: Download and install the release artifact for the detected platform
# OUTPUTS: { exit code 0 - release artifact installed, nonzero - release path unavailable }
# SIDE_EFFECTS: downloads tarball and extracts it under TMP_DIR
# LINKS: .github/workflows/release.yml
# START_install_from_release
install_from_release() {
    command -v curl >/dev/null 2>&1 || { echo "curl not found; trying source build. Run: sh install.sh --diagnose"; return 1; }
    command -v tar >/dev/null 2>&1 || { echo "tar not found; trying source build. Run: sh install.sh --diagnose"; return 1; }

    TARBALL="syn-${ARCH}-${OS}.tar.gz"
    URL="https://github.com/anyagixx/synapse/releases/download/${VERSION}/${TARBALL}"
    CHECKSUM_URL="https://github.com/anyagixx/synapse/releases/download/${VERSION}/SHA256SUMS"
    checksum_file="$TMP_DIR/SHA256SUMS"
    release_dir="$TMP_DIR/release"
    mkdir -p "$release_dir"

    if curl -fsSL "$URL" -o "$TMP_DIR/$TARBALL" 2>/dev/null; then
        if ! curl -fsSL "$CHECKSUM_URL" -o "$checksum_file" 2>/dev/null; then
            echo "Release checksum file unavailable for ${VERSION}; trying source build."
            echo "Run: sh install.sh --diagnose"
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

    echo "Release artifact ${TARBALL} unavailable for ${VERSION}; trying source build."
    echo "Run: sh install.sh --diagnose"
    return 1
}
# END_install_from_release

# START_CONTRACT_install_from_source
# PURPOSE: Build and install Synapse from the selected Git tag, CI source ref, or default main fallback when no release artifact is available
# OUTPUTS: { installed syn binary or nonzero exit }
# SIDE_EFFECTS: invokes cargo install under TMP_DIR and copies resulting binary
# START_install_from_source
install_from_source() {
    command -v cargo >/dev/null 2>&1 || {
        echo "Rust not installed. Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "Run: sh install.sh --diagnose"
        exit 1
    }
    command -v git >/dev/null 2>&1 || { echo "$GIT_REQUIRED_MESSAGE"; exit 1; }

    if [ -n "${SYN_INSTALL_SOURCE_REF:-}" ]; then
        echo "Building Synapse from source ref ${SYN_INSTALL_SOURCE_REF}..."
        cargo install --locked --git https://github.com/anyagixx/synapse --rev "${SYN_INSTALL_SOURCE_REF}" --root "$TMP_DIR/cargo-root"
    elif remote_tag_available "$VERSION"; then
        echo "Building Synapse from release tag ${VERSION}..."
        cargo install --locked --git https://github.com/anyagixx/synapse --tag "${VERSION}" --root "$TMP_DIR/cargo-root"
    elif [ "$VERSION_EXPLICIT" = "0" ]; then
        echo "Release tag ${VERSION} is not published yet; building default main branch."
        cargo install --locked --git https://github.com/anyagixx/synapse --branch main --root "$TMP_DIR/cargo-root"
    else
        echo "Requested release tag ${VERSION} is unavailable. Choose a published tag or omit the version to install the default branch."
        echo "Run: sh install.sh --diagnose"
        exit 1
    fi
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
        echo "Run: sh install.sh --diagnose"
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
if [ "$MODE" = "diagnose" ]; then
    print_diagnostics
    exit 0
fi

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
