# Installation

## Quick Install (macOS, Linux, Windows)

```bash
curl -fsSL https://synapse.dev/install.sh | sh
```

## Manual Install

### Linux (x86_64 / aarch64)
```bash
# Download release
curl -fsSL https://github.com/synapse-ai/synapse/releases/latest/download/syn-x86_64-unknown-linux-musl.tar.gz | tar xz
# Move to PATH
sudo mv syn /usr/local/bin/
```

### macOS (Intel / Apple Silicon)
```bash
curl -fsSL https://github.com/synapse-ai/synapse/releases/latest/download/syn-x86_64-apple-darwin.tar.gz | tar xz
sudo mv syn /usr/local/bin/
```

### Windows
Download `syn-x86_64-pc-windows-msvc.zip` from releases, extract, add to PATH.

### Via Cargo
```bash
cargo install --git https://github.com/synapse-ai/synapse
```

### Via Homebrew
```bash
brew install synapse-ai/tap/synapse
```

## Verify

```bash
syn --version
syn --help
```

## Next Steps

```bash
# Bootstrap your project
cd my-project
syn init

# Index codebase for AI search
syn index

# Start MCP server for OpenCode
syn mcp

# Check token savings
syn gain
```
