# Support

## Installation Diagnostics

For supported Linux and macOS install issues, collect the installer diagnostic report before opening an issue:

```bash
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/v2.6.2/install.sh -o /tmp/synapse-install.sh
sh /tmp/synapse-install.sh --diagnose
```

The report is local and no-write. It shows OS and architecture support status, the selected release artifact when the host is supported, install directory status, required tools, checksum verifier availability, and source fallback prerequisites.

If the install directory is not writable, use:

```bash
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/v2.6.2/install.sh | SYN_INSTALL_DIR="$HOME/.local/bin" sh
```

macOS Intel and Windows packaging are deferred and are not part of the current release matrix.
