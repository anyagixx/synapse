# F3-F6 — CLI, Runtime, Storage, Performance

# F3 — CLI surface cleanup

## `/home/truffle/Загрузки/synapse/src/cli.rs`
- **Фаза:** F3, then F6 refactor split
- **Зачем менять:** monolithic CLI file; ghost commands; public surface drift.
- **Что менять в F3:**
  - провести полную классификацию commands;
  - убрать из stable enum то, что не shipped;
  - пометить experimental commands;
  - убрать fake stable UX with `Command not yet implemented` where inappropriate;
  - выровнять help text under actual support policy.
- **Что менять в F6:**
  - split file into schema/dispatch/domain handlers.
- **Результат:** CLI surface honest and maintainable.
- **Зависимости:** docs truth matrix.

## `/home/truffle/Загрузки/synapse/src/main.rs`
- **Фаза:** F3, F4
- **Зачем менять:** entrypoint must consume new CLI surface and config semantics.
- **Что менять:**
  - adapt dispatch after CLI cleanup;
  - use pure config load or explicit init path;
  - improve top-level error reporting.
- **Результат:** main path stable and explicit.
- **Зависимости:** cli/config changes.

## `/home/truffle/Загрузки/synapse/src/lib.rs`
- **Фаза:** F3, F6
- **Зачем менять:** module exports may need adjustment after CLI split or command module introduction.
- **Что менять:**
  - expose new `cli` submodules if split;
  - remove dead module exposure if any.
- **Результат:** crate root matches architecture.
- **Зависимости:** cli refactor.

## `/home/truffle/Загрузки/synapse/docs/COMMANDS.md`
- **Фаза:** F3
- **Зачем менять:** command list must match enum after cleanup.
- **Что менять:** regenerate or manually sync command table.
- **Результат:** no ghost commands in docs.
- **Зависимости:** `src/cli.rs` command decision.

## `/home/truffle/Загрузки/synapse/docs/QUICKSTART.md`
- **Фаза:** F3
- **Зачем менять:** examples must target stable commands only.
- **Что менять:** refresh onboarding scripts.
- **Результат:** quickstart valid.
- **Зависимости:** command cleanup.

---

# F4 — Config/error/runtime hardening

## `/home/truffle/Загрузки/synapse/src/config.rs`
- **Фаза:** F4
- **Зачем менять:** `Config::load()` has hidden write side effects; config semantics unclear.
- **Что менять:**
  - split read-only load vs explicit init/create;
  - add error contexts;
  - make missing/invalid config behavior explicit;
  - document and expose mode cleanly.
- **Результат:** predictable config API.
- **Зависимости:** main/mcp/dashboard callers update.

## `/home/truffle/Загрузки/synapse/src/tracking/mod.rs`
- **Фаза:** F4, F5
- **Зачем менять:** silent `.ok()` swallowing DB failures; wrong project identity semantics; no degraded visibility.
- **Что менять в F4:**
  - replace silent `.ok()` with proper logging/result handling;
  - expose degraded tracking state.
- **Что менять в F5:**
  - store canonical project identity/hash instead of basename only;
  - add schema migration/versioning if needed.
- **Результат:** tracking reliable and accurate.
- **Зависимости:** config/runtime error model.

## `/home/truffle/Загрузки/synapse/src/dashboard.rs`
- **Фаза:** F4, F7
- **Зачем менять:** current handlers rely on hidden config behaviors, defaults, and local-only assumptions without enough signaling.
- **Что менять в F4:**
  - use explicit config load behavior;
  - better error mapping for API responses;
  - surface degraded subsystem state.
- **Что менять в F7:**
  - warn on non-loopback bind;
  - optional auth/rate-limit/CORS policy if exposing beyond localhost;
  - document local-only threat model.
- **Результат:** dashboard safer and more honest.
- **Зависимости:** config/status changes.

## `/home/truffle/Загрузки/synapse/src/mcp/server.rs`
- **Фаза:** F4, F7
- **Зачем менять:** unwrap/lock/runtime risk, HTTP mode ambiguity, feature claims may exceed behavior.
- **Что менять в F4:**
  - remove risky unwraps in runtime paths;
  - improve error propagation and concurrency guards;
  - make handler failures explicit.
- **Что менять в F7:**
  - decide HTTP mode: implement fully, hide, or mark experimental;
  - align exposed tool registry and docs;
  - improve security wording and transport messaging.
- **Результат:** MCP server stable and truth-aligned.
- **Зависимости:** CLI/docs classification.

## `/home/truffle/Загрузки/synapse/src/mcp/lsp.rs`
- **Фаза:** F4, F7
- **Зачем менять:** runtime unwrap risk; LSP lifecycle assumptions may be brittle.
- **Что менять:**
  - replace runtime unwraps;
  - improve failure reporting;
  - decide whether client remains one-shot or becomes persistent later.
- **Результат:** LSP bridge degrades visibly, not catastrophically.
- **Зависимости:** MCP server error model.

## `/home/truffle/Загрузки/synapse/src/utils/mod.rs`
- **Фаза:** F4
- **Зачем менять:** runtime helper unwrap patterns and token estimation semantics need audit.
- **Что менять:**
  - remove avoidable panics;
  - ensure helpers used by verify/reporting have documented limitations.
- **Результат:** utility layer safer.
- **Зависимости:** none.

## `/home/truffle/Загрузки/synapse/src/hooks/mod.rs`
- **Фаза:** F4
- **Зачем менять:** hook install/status paths likely drift; status may check wrong config path.
- **Что менять:**
  - align install path and status path detection;
  - improve diagnostics;
  - ensure command behavior matches docs.
- **Результат:** `syn hooks status` trustworthy.
- **Зависимости:** init/hook policy.

---

# F5 — Storage/index correctness

## `/home/truffle/Загрузки/synapse/src/indexer/storage.rs`
- **Фаза:** F5, then F6
- **Зачем менять:** JSON storage rewrite per file, silent corruption fallback, weak path fallback, query-time overhead.
- **Что менять в F5:**
  - atomic write via temp file + rename;
  - explicit corruption detection and recovery path;
  - stop silently defaulting to empty on malformed data;
  - improve index path resolution semantics;
  - introduce format version/checksum if feasible.
- **Что менять в F6:**
  - reduce cloning;
  - cache/precompute token/vector features;
  - prepare backend abstraction if migrating later.
- **Результат:** index storage durable, diagnosable, faster.
- **Зависимости:** runtime error model.

## `/home/truffle/Загрузки/synapse/src/indexer/mod.rs`
- **Фаза:** F5, F6
- **Зачем менять:** current indexing loop likely flushes too often; serial architecture; lock usage risk.
- **Что менять в F5:**
  - batch persistence once per run;
  - adjust index/update flow to support atomic storage commit;
  - improve error surfacing.
- **Что менять в F6:**
  - incremental indexing design;
  - changed-files watch mode;
  - optional parallel parse stage.
- **Результат:** indexer scalable and less I/O-heavy.
- **Зависимости:** storage changes.

## `/home/truffle/Загрузки/synapse/src/indexer/walker.rs`
- **Фаза:** F5, F6, F10
- **Зачем менять:** walker may need repo hygiene improvements and incremental support.
- **Что менять:**
  - improve exclusion policy for heavy/generated dirs;
  - support changed-file targeting;
  - possibly honor additional ignore patterns for vendored assets.
- **Результат:** less search/index noise, faster walks.
- **Зависимости:** repo hygiene policy.

## `/home/truffle/Загрузки/synapse/src/indexer/parser.rs`
- **Фаза:** F6
- **Зачем менять:** parser stage may benefit from parallelization and better fallback clarity.
- **Что менять:**
  - optimize parse flow;
  - review tree-sitter fallback heuristics;
  - make parser errors measurable for status/doctor.
- **Результат:** parser faster and more observable.
- **Зависимости:** indexer redesign.

## `/home/truffle/Загрузки/synapse/src/indexer/languages/mod.rs`
- **Фаза:** F6
- **Зачем менять:** language coverage docs and parser reality should match; language detection may feed capability docs.
- **Что менять:**
  - validate documented language support;
  - expose machine-readable list for parity tests/docs generation.
- **Результат:** language support claims derived from code.
- **Зависимости:** capability manifest plan.

---

# F6 — Performance/index evolution + CLI modularization

## `/home/truffle/Загрузки/synapse/src/cli/mod.rs`
- **Фаза:** F6 new file if refactor chooses module tree
- **Зачем менять:** replace monolithic `src/cli.rs` architecture.
- **Что менять:** create module root for CLI system.
- **Результат:** modular CLI foundation.
- **Зависимости:** F3 decisions.

## `/home/truffle/Загрузки/synapse/src/cli/schema.rs`
- **Фаза:** F6 new file
- **Зачем менять:** separate clap schema from handlers.
- **Что менять:** move Parser/Subcommand structs here.
- **Результат:** cleaner schema layer.
- **Зависимости:** F3 decisions.

## `/home/truffle/Загрузки/synapse/src/cli/dispatch.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate command routing.
- **Что менять:** dispatch logic only.
- **Результат:** simpler testing.
- **Зависимости:** schema split.

## `/home/truffle/Загрузки/synapse/src/cli/commands/init.rs`
- **Фаза:** F6 new file
- **Зачем менять:** split init logic out of monolith.
- **Что менять:** move `InitCmd::run` and helpers.
- **Результат:** init isolated.
- **Зависимости:** cli split.

## `/home/truffle/Загрузки/synapse/src/cli/commands/index.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate indexing command behavior.
- **Что менять:** move index/watch flow.
- **Результат:** command-level maintainability.
- **Зависимости:** cli split, indexer APIs stable.

## `/home/truffle/Загрузки/synapse/src/cli/commands/search.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate search command.
- **Что менять:** move search handling.
- **Результат:** cleaner command boundaries.
- **Зависимости:** cli split.

## `/home/truffle/Загрузки/synapse/src/cli/commands/view.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate view handling.
- **Что менять:** move signature viewing logic.
- **Результат:** same.
- **Зависимости:** cli split.

## `/home/truffle/Загрузки/synapse/src/cli/commands/verify.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate verification command.
- **Что менять:** move verify command handling.
- **Результат:** better testing around governance.
- **Зависимости:** grace API stable.

## `/home/truffle/Загрузки/synapse/src/cli/commands/review.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate review command.
- **Что менять:** move review logic.
- **Результат:** same.
- **Зависимости:** grace review stable.

## `/home/truffle/Загрузки/synapse/src/cli/commands/status.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate status command.
- **Что менять:** move status handling and json/plain output.
- **Результат:** same.
- **Зависимости:** status API stable.

## `/home/truffle/Загрузки/synapse/src/cli/commands/proxy.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate proxy command.
- **Что менять:** move proxy invocation.
- **Результат:** clearer command behavior.
- **Зависимости:** proxy API stable.

## `/home/truffle/Загрузки/synapse/src/cli/commands/gain.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate tracking analytics command.
- **Что менять:** move gain output.
- **Результат:** same.
- **Зависимости:** tracking schema update.

## `/home/truffle/Загрузки/synapse/src/cli/commands/compress.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate compression command.
- **Что менять:** move compress/restore handling.
- **Результат:** same.
- **Зависимости:** none.

## `/home/truffle/Загрузки/synapse/src/cli/commands/mcp.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate MCP startup and mode policy.
- **Что менять:** move MCP command behavior.
- **Результат:** easier transport policy maintenance.
- **Зависимости:** MCP mode decision.

## `/home/truffle/Загрузки/synapse/src/cli/commands/config.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate config command semantics.
- **Что менять:** move config command handling.
- **Результат:** same.
- **Зависимости:** config API redesign.

## `/home/truffle/Загрузки/synapse/src/cli/commands/graphrag.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate graph query command.
- **Что менять:** move graph command handling.
- **Результат:** same.
- **Зависимости:** graphrag stable.

## `/home/truffle/Загрузки/synapse/src/cli/commands/hooks.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate hook install/status commands.
- **Что менять:** move hooks behavior.
- **Результат:** clearer hook surface.
- **Зависимости:** hook fix complete.

## `/home/truffle/Загрузки/synapse/src/cli/commands/doctor.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate diagnostics path.
- **Что менять:** move doctor logic.
- **Результат:** same.
- **Зависимости:** status/health semantics stable.

## `/home/truffle/Загрузки/synapse/src/cli/commands/refresh.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate refresh command.
- **Что менять:** move artifact drift sync handling.
- **Результат:** same.
- **Зависимости:** grace refresh stable.

## `/home/truffle/Загрузки/synapse/src/cli/commands/history.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate history query command.
- **Что менять:** move history behavior.
- **Результат:** same.
- **Зависимости:** none.

## `/home/truffle/Загрузки/synapse/src/cli/commands/serve.rs`
- **Фаза:** F6 new file
- **Зачем менять:** isolate dashboard startup command.
- **Что менять:** move serve handling and bind checks.
- **Результат:** same.
- **Зависимости:** dashboard hardening.

## `/home/truffle/Загрузки/synapse/src/proxy/mod.rs`
- **Фаза:** F6
- **Зачем менять:** proxy may need better streaming/degraded-mode integration and tracking accuracy.
- **Что менять:**
  - align error handling with tracking/runtime policy;
  - later support better streaming/reporting if kept in scope.
- **Результат:** proxy truthful and observable.
- **Зависимости:** tracking/runtime policy.

## `/home/truffle/Загрузки/synapse/src/proxy/runner.rs`
- **Фаза:** F6
- **Зачем менять:** command execution may need better streaming/error surfaces.
- **Что менять:**
  - improve execution result model;
  - support long-running output handling if roadmap keeps it.
- **Результат:** runner clearer and safer.
- **Зависимости:** proxy redesign.

## `/home/truffle/Загрузки/synapse/src/proxy/toml_filter.rs`
- **Фаза:** F6
- **Зачем менять:** filter system may need observability and better error surfaces, maybe docs parity for built-in filters.
- **Что менять:**
  - improve parse/apply errors;
  - possibly expose built-in filter inventory for docs/tests.
- **Результат:** filter engine less opaque.
- **Зависимости:** proxy observability goals.

## `/home/truffle/Загрузки/synapse/src/graphrag/mod.rs`
- **Фаза:** F6
- **Зачем менять:** graph overview/tooling claims should be machine-readable and cache-aware later.
- **Что менять:**
  - expose stable capability/overview metadata;
  - prepare for optional caching if accepted.
- **Результат:** graph API more usable for docs/status.
- **Зависимости:** capability manifest optional.

## `/home/truffle/Загрузки/synapse/src/graphrag/builder.rs`
- **Фаза:** F6
- **Зачем менять:** builder may need performance and import-parsing improvements.
- **Что менять:**
  - optimize graph build path;
  - improve failure visibility.
- **Результат:** graph build more scalable.
- **Зависимости:** indexer evolution.

## `/home/truffle/Загрузки/synapse/src/graphrag/types.rs`
- **Фаза:** F6
- **Зачем менять:** may need machine-readable export stability for status/docs.
- **Что менять:**
  - add metadata fields if capability/reporting requires them.
- **Результат:** stable graph schemas.
- **Зависимости:** graph/reporting goals.

## `/home/truffle/Загрузки/synapse/src/compress/mod.rs`
- **Фаза:** F6 optional
- **Зачем менять:** expose limitations/metrics more clearly, maybe improve restore safety.
- **Что менять:**
  - audit restore path and reporting;
  - expose compression stats if needed.
- **Результат:** compression feature clearer.
- **Зависимости:** none.

## `/home/truffle/Загрузки/synapse/src/skills/mod.rs`
- **Фаза:** F6 optional / planned
- **Зачем менять:** currently stub; either hide from claims or implement registry later.
- **Что менять:**
  - classify as internal stub or build actual plugin registry later;
  - avoid overclaim in docs meanwhile.
- **Результат:** no misleading skill-system claim.
- **Зависимости:** product decision.

---

