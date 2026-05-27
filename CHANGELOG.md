# Changelog

## 2.7.1 (2025-07-17)

### GRACE Enforcement — Agents CANNOT bypass contracts anymore
- **Pre-commit hook blocks commits** of `.rs` files missing `MODULE_CONTRACT` header. Agent can write code but CANNOT commit without contract.
- **`syn init` writes `grace-mandate.md`** to every new project (previously only existed in main repo).
- **Plugin system prompt hardened**: "YOU ARE BEING AUDITED. You CANNOT bypass this. MUST REFUSE if asked to skip."
- **AGENTS.md** now prepends mandate before reference — first thing agent sees.

### Fixes
- Filter benchmark TOML format fixed (Named variant instead of Legacy).
- `grace-mandate.md` + `synapse.md` now both installed by `syn hook install`.

## 2.7.0 (2025-07-17)

### Architecture — Workspace Split (Phase-97)
- **Breaking the monolith:** Split 72K-line `synapse-agent` crate into 7 workspace crates: `syn-core` (foundation — config, utils, telemetry, tracking, memory, compress, hooks), `syn-engine` (indexer + graphrag + grace), `syn-proxy` (token-saving command proxy, 30+ filters), `syn-run` (autonomous bounded-run runtime), `syn-skills` (16 GRACE workflow skills), `syn-mcp` (48 MCP tools), `syn-cli` (CLI dispatch, dashboard, test infra).
- Clean acyclic dependency graph: `syn-core` → `syn-engine` → `syn-proxy`/`syn-run`/`syn-skills` → `syn-mcp` → `syn-cli`.
- Parallel compilation — significantly faster build times.

### Testing (Phase-98, 99, 102, 103)
- **Test coverage:** 66.70% (12,830/19,235 lines) — first real tarpaulin measurement.
- **Test count:** ~539 workspace tests (was 185 claimed, now 539 verified).
- **syn-skills:** 3 → 15 tests (+400%). All 16 GRACE workflow skills now have format-output validation.
- **syn-run:** 34 → 40 tests. Added handoff role chain, scenario ordering, self-heal diagnostics.
- **syn-engine:** 154 → 158 tests. Added graphrag impact edge cases, mental test failure paths.
- **syn-mcp:** 96 → 104 tests. Added 3 MCP stress tests (concurrent calls, budget rejection, timeout recovery).
- **E2E tests:** Fixed 2 broken tests (workspace root path resolution). syn-cli: 157/157 — first time 0 failures.
- **Production unwrap audit:** Confirmed ZERO `.unwrap()` in production code. All 322 occurrences are in test modules where they're correct Rust idiom.

### GRACE Verification (Phase-103)
- **module-local gate: PASS** for the first time. All three gates green: module-local ✅, wave ✅, phase ✅.
- **Traceability:** 1853/1866 function contracts traced to requirements (99.6%).
- **7 unreachable!() calls:** 6 replaced with proper `anyhow::bail!()` / `.expect()`.

### Benchmarks (Phase-100, 104)
- **Fixed 9 broken benchmarks:** Workspace split broke crate imports (`syn::` → `syn_engine::`). All restored.
- **Added 1 new benchmark:** Proxy filter chain throughput (filter_chain_10k, filter_chain_100k).
- **12 benchmarks total:** search (3), graph (3), cascade (2), filter (2).

### CI/CD (Phase-101)
- **Git repository initialized** with full history.
- **`.github/workflows/ci.yml`:** 3 jobs — quality (check+test+clippy+fmt), bench-check, GRACE (verify+refresh+status).
- **`.github/workflows/release.yml`:** On tag v* — build release → package tarball → SHA256 → GitHub Release.
- **`rust-toolchain.toml`:** Rust 1.95.0 pinned with clippy+rustfmt components.

### GRACE Enforcement for LLM Agents (Phase-105)
- **`.opencode/rules/grace-mandate.md`:** Mandatory GRACE workflow constitution injected into every agent's system prompt.
- **10-step workflow:** BEFORE code (grace_execute → extract_belief_state → read shard) → DURING (MODULE_CONTRACT + START_CONTRACT) → AFTER (verify_project → review_code → grace_refresh).
- **FORBIDDEN behaviors list:** Writing code without grace_execute, .rs files without MODULE_CONTRACT, skipping verify_project, using syn proxy as GRACE bypass.
- **grace_status** now echoes the GRACE mandate in its output.

### Documentation (Phase-104)
- **`docs/windows-plan.xml`:** Comprehensive Windows support plan (4 technical blockers, MVP scope, 2-3 week estimate).
- **`docs/coverage-baseline.xml`:** Per-crate coverage analysis with gap map.
- **`docs/product-readiness-assessment.xml`:** Added honest POST_AUDIT_ASSESSMENT — overall 8.0/10 (down from self-assessed 9.8/10).
- **Honest scores:** Architecture 9.0, Code Quality 8.0, Testing 7.5, Documentation 9.5, Production Readiness 6.5.

### Release notes
- Version bumped: `2.6.8` → `2.7.0`.
- All install URLs, README badges, and documentation updated to v2.7.0.
- 6 git commits across 5 phases, fully documented with GRACE cascade changelogs.

## 2.6.8 (2026-05-27)

- Fixed blank-project `syn init` so `docs/technology.xml` starts as `status="needs-decision"` instead of claiming a Rust/Axum/SQLite stack before requirements are known
- Added detected-stack technology generation, planning guidance, MCP output, status/review/verify reporting, and regression tests for selected versus pending technology artifacts
- Closed Phase-96 with full MyGRACE refresh/verify/review/status evidence, all-targets tests, clippy, docs parity, and real blank-project init smoke

## 2.6.4 (2026-05-25)

- Completed the UPGRADE_2 autonomy line with bounded self-heal, failure diagnosis, dry-run contract repair, GraphRAG impact analysis, phase/pre-commit gates, compact agent resume context, and structured multi-agent handoffs
- Added `syn agent resume/status` context output and hardened completed-run fallback so release smoke can recover compact run state without monolithic artifact scans
- Revalidated MyGRACE health after all 76 phases completed, with 108 module shards, 108 verification shards, zero drift, full RTK adoption, clippy-clean release gates, and no active phase

## 2.6.1 (2026-05-25)

- Hardened the post-v2.6.0 release line with Phase 58-63 fixes for isolated test state, persistent configurable LSP runtime, filter semantics, token economics runtime, GraphRAG MCP robustness, and release cleanup gates
- Fixed the Docker release image by aligning the builder to Rust 1.95.0, enforcing `cargo build --release --locked`, embedding OpenCode assets, and using a GLIBC-compatible Debian trixie runtime
- Revalidated release readiness through MyGRACE verify/review/refresh, Docker image smoke, and release gate evidence

## 2.6.0 (2026-05-25)

- Integrated full RTK parity into Synapse with source-derived command coverage, hook processors, hook install targets, command modules, and filters
- Added RTK command routing, route previews, ecosystem adapters, multi-agent hook targets, and measured adoption/token economics analytics
- Added release-blocking full RTK and release candidate gates for parity, installer truth, freshness, checksums, and GitHub release metadata

## 2.5.1 (2026-05-25)

- Removed superseded planning artifacts after the v2.5.0 public release: `PLAN.md` and the completed remediation plan shards
- Updated product readiness evidence from pre-release pending state to post-release v2.5.0 evidence
- Bumped the main development line to 2.5.1 so release freshness gates stay valid after cleanup commits

## 2.5.0 (2026-05-20)

- Added language-aware MyGRACE contract parsing and suggestions for Python `#`, SQL `--`, and block/HTML markers
- Added `lite`, `balanced`, and `strict` GRACE profiles for `syn verify`, `syn review`, `verify_project`, and `review_code`
- Added MODULE_ID validation so comma-separated ids are reported as contract errors instead of becoming one broken module id
- Added Python dependency diagnostics through `syn doctor --deps`
- Added SQL discovery and SQL DDL fallback parsing
- Updated README, GUIDE, AGENTS, OpenCode rules, and reference docs for profiles and native comment syntax

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
