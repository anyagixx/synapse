#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-latest}"
REPO="synapse-ai/synapse"
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

TAR="${ARCH}-${OS}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${BIN_NAME}-${TAR}"

echo "Downloading Synapse ${VERSION} for ${ARCH}-${OS}..."
mkdir -p "$INSTALL_DIR"
curl -fsSL "$URL" | tar xz -C "$INSTALL_DIR" "$BIN_NAME"
chmod +x "${INSTALL_DIR}/${BIN_NAME}"

echo "Installed to ${INSTALL_DIR}/${BIN_NAME}"
echo "Make sure ${INSTALL_DIR} is in your PATH"
