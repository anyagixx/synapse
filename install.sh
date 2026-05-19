#!/bin/sh
# MODULE_CONTRACT
# MODULE_ID: M-INSTALL
# PURPOSE: Installer script — installs Synapse from GitHub release artifacts or cargo source fallback
# SCOPE: Platform detection, release tarball download, local binary install, cargo fallback
# DEPENDS: M-BUILD
# LINKS: install.sh, .github/workflows/release.yml

# START_MODULE_MAP
# main — Detects platform and installs syn
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v2.6.0 — Aligned default release tag with Cargo package version]
# END_CHANGE_SUMMARY

# START_CONTRACT_main
# PURPOSE: Install Synapse for the detected platform
# INPUTS: { $1: version — optional release tag }
# OUTPUTS: { installed syn binary or nonzero exit }
# SIDE_EFFECTS: downloads artifacts, copies binary, may run cargo install
# START_main
set -e

VERSION="${1:-v2.3.1}"
ARCH="x86_64"
OS="unknown-linux-gnu"

case "$(uname -s)" in
    Linux)  OS="unknown-linux-gnu" ;;
    Darwin) OS="apple-darwin" ;;
esac

case "$(uname -m)" in
    x86_64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
esac

echo "Installing Synapse ${VERSION} for ${ARCH}-${OS}..."

# Try GitHub release download
TARBALL="syn-${ARCH}-${OS}.tar.gz"
URL="https://github.com/anyagixx/synapse/releases/download/${VERSION}/${TARBALL}"

if curl -fsSL "$URL" -o /tmp/syn.tar.gz 2>/dev/null; then
    tar -xzf /tmp/syn.tar.gz -C /tmp
    sudo cp /tmp/syn /usr/local/bin/syn 2>/dev/null || cp /tmp/syn ~/.local/bin/syn 2>/dev/null || {
        mkdir -p ~/.local/bin
        cp /tmp/syn ~/.local/bin/syn
    }
    chmod +x ~/.local/bin/syn 2>/dev/null || true
    rm /tmp/syn.tar.gz /tmp/syn 2>/dev/null
    echo "Synapse ${VERSION} installed successfully."
else
    echo "No pre-built binary for ${ARCH}-${OS}. Building from source..."
    command -v cargo >/dev/null 2>&1 || { echo "Rust not installed. Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; exit 1; }
    cargo install --git https://github.com/anyagixx/synapse --tag "${VERSION}"
fi

echo ""
echo "Run 'syn --help' to get started."
echo "Quickstart: mkdir my-project && cd my-project && syn init && opencode"
# END_main
