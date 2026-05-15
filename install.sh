#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-v0.1.0}"
REPO="anyagixx/synapse"
BIN_NAME="syn"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s)" in
    Linux)  OS="unknown-linux-musl" ;;
    Darwin) OS="apple-darwin" ;;
    *)      echo "Unsupported OS"; exit 1 ;;
esac

case "$(uname -m)" in
    x86_64)  ARCH="x86_64" ;;
    aarch64) ARCH="aarch64" ;;
    arm64)   ARCH="aarch64" ;;
    *)       echo "Unsupported arch"; exit 1 ;;
esac

if [ "$VERSION" = "latest" ]; then
    VERSION="v0.1.0"
fi

TAR="synapse-${ARCH}-${OS}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${TAR}"

echo "Downloading Synapse ${VERSION} for ${ARCH}-${OS}..."
mkdir -p "$INSTALL_DIR"

if curl -fsSL "$URL" -o "/tmp/${TAR}"; then
    tar xzf "/tmp/${TAR}" -C "$INSTALL_DIR"
    chmod +x "${INSTALL_DIR}/${BIN_NAME}"
    rm -f "/tmp/${TAR}"
    echo "Installed to ${INSTALL_DIR}/${BIN_NAME}"
    echo ""
    echo "Run 'synapse --help' to get started."
    echo "For non-developers: run 'opencode' and describe what you want to build."
else
    echo "No pre-built binary for your platform at:"
    echo "  $URL"
    echo ""
    echo "Building from source instead..."
    if command -v cargo &> /dev/null; then
        echo "Installing package 'synapse' with binary 'syn'..."
        cargo install --git "https://github.com/${REPO}" --tag "$VERSION" --bin syn
        # Create convenience symlink
        if [ -f "$HOME/.cargo/bin/syn" ]; then
            BIN_NAME="syn"
            INSTALL_DIR="$HOME/.cargo/bin"
        fi
    else
        echo "Rust is required to build from source."
        echo "Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "Then re-run this script."
        exit 1
    fi
fi
