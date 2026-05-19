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

**What platforms are supported?**
Prebuilt release artifact: Linux x86_64.
Source fallback via Cargo: Linux/macOS on x86_64 or aarch64.
Windows: build/test support exists in CI; install from source with Cargo.

## GRACE

**What is GRACE?**
Graph-RAG Anchored Code Engineering — a methodology where all code is written against strict contracts, semantically marked up for AI navigation, and verified at three levels before merging.

**What happens if I skip GRACE?**
Synapse works without GRACE (search, proxy, compression still function). But strict mode enforces the full methodology.

## Token Economy

**How many tokens does Synapse save?**
- Proxy: 60-90% on shell commands
- Caveman output: 65-75% on AI responses
- Caveman input: ~46% on context files
- Total: typically 70-85% overall

**How do I check my savings?**
```bash
syn gain
syn gain --graph
```

## Privacy

**Is my code sent anywhere?**
No. Everything runs locally. Embedding/LLM calls only go to external APIs if you configure them (Voyage, OpenAI, etc.). No code is sent to Synapse servers.

**What does telemetry collect?**
Nothing by default (opt-in). When enabled: anonymous device hash, version, command counts, aggregate savings. No code, no file paths, no secrets.

## Troubleshooting

**`syn: command not found`**
Add `~/.local/bin` to your PATH or restart your terminal.

**Search returns no results**
Run `syn index` first.

**Proxy slows down commands**
Proxy overhead is ~5-10ms per command. If you notice real slowdowns, report with `syn logs`.
