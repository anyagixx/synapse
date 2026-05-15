# Synapse

> **Unified AI Agent Engineering Platform**
> Code intelligence + Token proxy + Communication compression + GRACE methodology

Synapse объединяет четыре мощных инструмента в один CLI-бинарник для AI-assisted разработки:

- **Octocode** — AST-индексация кода, семантический поиск, GraphRAG, MCP сервер
- **RTK** — прокси-фильтрация вывода shell-команд (экономия 60-90% токенов)
- **Caveman** — сжатие ответов AI и input-файлов (экономия 65-75% токенов)
- **GRACE** — строгая методология: контракты, верификация, knowledge graph, multi-agent execution

## Quick Start

```bash
# Install
curl -fsSL https://synapse.dev/install.sh | sh

# Bootstrap project
mkdir my-app && cd my-app
syn init

# Index codebase
syn index

# Start MCP server for OpenCode
syn mcp

# Check savings
syn gain
```

## Documentation

| File | Description |
|------|-------------|
| `docs/QUICKSTART.md` | Onboarding for non-developers |
| `docs/COMMANDS.md` | Full CLI reference |
| `docs/WORKFLOW.md` | GRACE development workflow |
| `docs/CONTRACTS.md` | Module contract specification |
| `docs/SEMANTIC_MARKUP.md` | Semantic markup reference |
| `docs/PROXY.md` | Proxy filter reference |
| `docs/COMPRESS.md` | Compression reference |
| `docs/FAQ.md` | Troubleshooting |

## Architecture

```
┌──────────────────────────────────────────────────┐
│ GRACE Methodology (contracts, verification, graph)│
├──────────────────────────────────────────────────┤
│ Caveman (AI output/input compression)             │
├──────────────────────────────────────────────────┤
│ RTK Proxy (shell command filtering)               │
├──────────────────────────────────────────────────┤
│ Octocode (AST indexing, GraphRAG, search, MCP)    │
├──────────────────────────────────────────────────┤
│ Rust Core (single binary, tokio, LanceDB, SQLite) │
└──────────────────────────────────────────────────┘
```

## License

Apache 2.0
