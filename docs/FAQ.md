# FAQ

## General

**What is Synapse?**
A single binary that combines code intelligence (search + GraphRAG), token-saving proxy, AI response/file compression, and GRACE strict development methodology.

**Do I need to know programming?**
No. You describe what you want in plain language. AI handles everything.

**What AI agents does it work with?**
OpenCode, Claude Code, Cursor, Windsurf, Cline, Copilot, Gemini CLI, Codex, and any MCP-compatible agent.

## Installation

**How do I install?**
```bash
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/main/install.sh | sh
```
Without sudo:
```bash
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/main/install.sh | SYN_INSTALL_DIR="$HOME/.local/bin" sh
```

**What platforms are supported?**
Prebuilt release artifacts:
- Linux x86_64
- Linux aarch64
- macOS x86_64
- macOS arm64

Source fallback via Cargo remains available on Linux/macOS when a matching prebuilt artifact cannot be downloaded.
Windows packaging is planned later and is not part of the current Linux/macOS release matrix.

**How do I diagnose install failures?**
```bash
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/main/install.sh -o /tmp/synapse-install.sh
sh /tmp/synapse-install.sh --diagnose
```
The diagnostic report prints the detected artifact, install directory status, required tools, checksum support, and source fallback prerequisites.

## GRACE

**What is GRACE?**
Graph-RAG Anchored Code Engineering — a methodology where all code is written against strict contracts, semantically marked up for AI navigation, and verified at three levels before merging.

**What happens if I skip GRACE?**
Synapse works without GRACE (search, proxy, compression still function). But strict mode enforces the full methodology.

## Token Economy

**How many tokens does Synapse save?**
- Proxy: measured locally on proxied commands; noisy commands with built-in or project filters can save 60-90%
- Caveman output: 65-75% on AI responses
- Caveman input: ~46% on context files
- Total: depends on the command mix and configured filters

**How do I check my savings?**
```bash
syn gain
syn gain --graph
```

## Privacy

**Is my code sent anywhere?**
No. Everything runs locally. Embedding/LLM calls only go to external APIs if you configure them (Voyage, OpenAI, etc.). No code is sent to Synapse servers.

**What does telemetry collect?**
Synapse currently has no telemetry upload path. Token savings are tracked locally for `syn gain`.

## Troubleshooting

**`syn: command not found`**
Add `~/.local/bin` to your PATH or restart your terminal.

**Search returns no results**
Run `syn index` first.

**Proxy slows down commands**
Proxy overhead is ~5-10ms per command. If you notice real slowdowns, run `syn doctor` and include the command output in an issue report.
