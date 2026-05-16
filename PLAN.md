# Synapse v2.0 — Implementation Plan

## Цель
Довести Synapse до уровня когда AI-агент с его помощью справляется с разработкой быстрее и качественнее чем команда архитекторов + senior разработчиков.

---

## Неделя 1: Self-Referential Quality
_Инструмент проходит собственные проверки_

### 1.1 MODULE_CONTRACT во все 30 .rs файлов
Каждый файл получает:
```
// MODULE_CONTRACT
// MODULE_ID: M-XXX
// PURPOSE: [one sentence]
// SCOPE: [operations]
// DEPENDS: [dependencies]
// LINKS: [knowledge graph references]
```

### 1.2 MODULE_MAP + CHANGE_SUMMARY
Каждый файл получает:
```
// START_MODULE_MAP
// fn_name — description
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY
```

### 1.3 START/END блоки + function contracts
Каждая pub fn получает:
```
// START_CONTRACT_fnName
// PURPOSE: ...
// INPUTS: ...
// OUTPUTS: ...
// START_fnName
pub fn fnName(...) -> ... { ... }
// END_fnName
```

### 1.4 syn verify → ALL PASS
### 1.5 syn review → 0 critical

---

## Неделя 2: Production Engineering

### 2.1 Split cli.rs → src/cmd/*.rs
- src/cmd/init.rs, index.rs, search.rs, view.rs, verify.rs, review.rs
- src/cmd/status.rs, proxy.rs, gain.rs, compress.rs, mcp.rs
- src/cmd/config.rs, graphrag.rs, hooks.rs, doctor.rs, refresh.rs
- src/cmd/history.rs, serve.rs

### 2.2 Concurrency fixes
- Mutex → RwLock in Indexer.storage
- MCP server: tokio::spawn for concurrent tool calls

### 2.3 Fix error handling
- All .unwrap() → graceful handling
- All .ok() → proper logging
- Add error contexts with anyhow::Context

### 2.4 Storage fixes
- DefaultHasher → SHA-256 for stable index paths
- SQLite FTS5 for text search
- Incremental file watching (re-index changed files only)

### 2.5 Integration tests
- tests/integration_test.rs: full end-to-end
- Test MCP server startup + tool calls
- Test init + index + search + verify + review flow
- Test proxy filtering + token tracking

### 2.6 Binary releases
- GitHub Actions: build linux-x86_64 + macos-arm64
- Upload .tar.gz to GitHub Release
- Update install.sh

---

## Неделя 3: Intelligence Layer

### 3.1 Smart contract suggestions
- MCP tool: suggest_contract(description) → MODULE_CONTRACT template

### 3.2 Automated drift repair
- syn refresh --fix: auto-fix knowledge-graph.xml and verification-plan.xml

### 3.3 Parallel indexing
- rayon for file walking + parsing

### 3.4 Incremental indexing
- Watch mode: re-index only changed files

### 3.5 Cache layer
- GraphRAG CodeGraph cached to disk
- Verification results cached with TTL

### 3.6 LSP integration
- Connect mcp/lsp.rs to MCP server tools
- Persistent LSP client (not one-shot)

### 3.7 Plugin system
- skills/ as loadable modules with trait-based registry

---

## Неделя 4: Enterprise & Observability

### 4.1 Prometheus metrics
- /metrics endpoint: request count, latency, errors
- Token savings counter, verify pass/fail gauge

### 4.2 Structured logging
- JSON format via tracing-subscriber
- Request IDs, span tracing

### 4.3 Graceful shutdown
- SIGTERM handler for MCP + dashboard

### 4.4 Web dashboard HTML UI
- Full HTML page with graph visualization
- Status cards, token charts

### 4.5 Streaming proxy
- Line-by-line output for long-running commands

### 4.6 Authentication
- API key for MCP HTTP transport
- Token-based for dashboard

---

## Final Metrics Target

| Metric | v1.0 | v2.0 Target |
|--------|------|-------------|
| MODULE_CONTRACT coverage | 0% | 100% |
| syn verify on self | FAIL | ALL PASS |
| Tests | 24 unit | 24 unit + 15 integration |
| .unwrap() in production | 21 | 0 |
| Silent errors (.ok()) | 8 | 0 |
| MCP concurrency | 1 req | N req |
| Binary releases | 0 | linux + macos |
| Dashboard | JSON API | HTML UI |
| Parallel indexing | No | Yes (rayon) |
| Incremental watch | Full re-index | Changed files only |
| Cache layer | None | GraphRAG + verify |
| LSP tools | Dead code | Active MCP tools |
| Prometheus metrics | 0 | /metrics |
