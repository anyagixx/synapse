#!/bin/sh
set -e

VERSION="${1:-v2.1.0}"
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
