# Installation

Synapse currently ships prebuilt release archives for Linux and macOS. Windows packaging is deferred.

## Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/v2.5.2/install.sh | sh
```

Without sudo:

```bash
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/v2.5.2/install.sh | SYN_INSTALL_DIR="$HOME/.local/bin" sh
```

The installer downloads a matching release tarball when available, verifies `SHA256SUMS`, installs `syn`, and then runs `syn --version`. If a matching archive is unavailable, it falls back to a locked Cargo install from the selected Git tag. During the short pre-tag release-candidate window, the default installer can build the repository `main` branch instead of failing on a not-yet-published tag.

## Diagnostics

Before opening an install issue, run the installer diagnostic mode on the same machine:

```bash
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/v2.5.2/install.sh -o /tmp/synapse-install.sh
sh /tmp/synapse-install.sh --diagnose
```

The report shows OS and architecture support status, the detected Linux/macOS artifact, install directory status, required tools, checksum verifier availability, and the source fallback prerequisites. Use `SYN_INSTALL_DIR="$HOME/.local/bin"` when the default install path is not writable.

## Supported Prebuilt Archives

| Platform | Release archive |
|----------|-----------------|
| Linux x86_64 | `syn-x86_64-unknown-linux-gnu.tar.gz` |
| Linux aarch64 | `syn-aarch64-unknown-linux-gnu.tar.gz` |
| macOS x86_64 | `syn-x86_64-apple-darwin.tar.gz` |
| macOS arm64 | `syn-aarch64-apple-darwin.tar.gz` |

## Source Install

For development checkouts:

```bash
git clone https://github.com/anyagixx/synapse.git
cd synapse
make install
```

For the latest published release tag:

```bash
cargo install --locked --git https://github.com/anyagixx/synapse --tag v2.5.2
```

The Cargo package name is `synapse-agent`, but the installed binary remains `syn`.

## Verify

```bash
syn --version
syn --help
```

## Next Steps

```bash
cd my-project
syn init
syn index
opencode
```
