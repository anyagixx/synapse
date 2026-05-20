# Changelog

## 2.4.0 (2026-05-20)

- Prepared first public Linux/macOS release candidate after Phase 11 hardening
- Bumped release line beyond `v2.3.5` so post-release fixes are not hidden behind a stale installer tag
- Added release freshness guard for stale tag detection in local, CI, release-candidate, and release workflows
- Added commit-pinned fresh-install smoke and a default main source fallback for the short pre-tag release-candidate window
- Kept package metadata publishable by using Cargo package `synapse-agent` while preserving the `syn` binary and library crate
- Removed unimplemented public MCP HTTP/LSP flags until those transports are real
- Clarified privacy and token-savings claims in public documentation

## 2.3.0 (2026-05-16)

- Capabilities registry (`src/capabilities.rs`) — single source of truth
- Docs parity tests (4 automated checks against capabilities)
- 32 total tests (24 unit + 4 integration + 4 parity)

## 2.2.0 (2026-05-16)

- **F1-F5 Remediation**: truth inventory, CLI cleanup, config hardening, atomic storage
- Removed 7 ghost CLI commands (PlanCmd, ExecuteCmd, FixCmd, ExplainCmd, LogsCmd, TelemetryCmd, McpProxyCmd)
- Config::load() now read-only; Config::load_or_default() + Config::init_default() added
- Atomic storage writes (temp file + rename)
- Corruption detection with warning logs

## 2.1.1 (2026-05-16)

- 28 audit issues fixed
- lsp_references MCP tool implemented (12 tools total)
- Cargo.toml: version 2.1.0, correct repo URL
- Dockerfile: binary path syn
- install.sh: v2.1.0 default, correct naming
- build.rs: removed dead code
- AGENTS.md created at project root

## 2.1.0 (2026-05-16)

- All 3 verification levels PASS on self
- 28 tests (24 unit + 4 integration)
- Smart contract suggestions MCP tool
- LSP hover + references
- JSON structured logging, graceful shutdown, web dashboard HTML

## 2.0.0 (2026-05-16)

- Self-referential quality: MODULE_CONTRACT in all 33 source files
- 264 semantic blocks, 0 unclosed, 0 duplicates
- Mutex → RwLock for concurrent reads
- DefaultHasher → SHA-256 for stable indexes
- Integration tests (4 e2e)

## 1.0.0 (2026-05-16)

- Vector search (trigram n-gram + cosine similarity)
- LSP integration (hover, go-to-def, references)
- Web dashboard (axum: /health, /api/status, /api/graph, /api/tokens)
- 10 MCP tools, 17 CLI commands, 24 tests

## 0.9.0 (2026-05-16)

- grace-ask + grace-explainer skills
- Agent adapters (Claude Code, Cursor, Copilot)
- Git history search (`syn history`)
- Multi-repo MCP proxy detection

## 0.8.0 (2026-05-16)

- 5 methodology reference docs
- Trace assertions in verify_project
- Failure packets with suggested fixes
- Per-project token economy

## 0.7.0 (2026-05-16)

- grace-refresh (drift detection + sync)
- Grace-status upgrade (MODULE_MAP, CHANGE_SUMMARY, function contracts)
- Grace-multiagent-execute skill
- Sub-agent roles (implementer, contract-reviewer, verification-reviewer, fixer)

## 0.6.0 (2026-05-16)

- Full GRACE methodology: MODULE_MAP, CHANGE_SUMMARY, function contracts
- Wave-audit review mode
- 500-token rule, unique block names

## 0.5.0-0.2.0 (2026-05-16)

- GRACE enforcement via AGENTS.md + plugin
- Phase 0 gate (docs before code)
- OpenCode compatibility fixes
- CLI commands working (search, view, verify, review)

## 0.1.0 (2026-05-15)

- Initial release
- Project scaffold with CLI dispatch
- Octocode, RTK, Caveman, GRACE modules
