# FILE-BY-FILE REMEDIATION PLAN

Цель: жесткий пофайловый план изменений для устранения всех найденных проблем: drift, reliability, architecture, storage, testing, packaging, CI, governance.

---

## Как читать план

Для каждого файла указано:
- **Фаза** — когда менять
- **Зачем менять** — причина
- **Что менять** — конкретный объем правок
- **Результат** — что должно получиться
- **Зависимости** — что должно быть сделано раньше

Обозначения фаз:
- **F1** Truth inventory + docs/code alignment
- **F2** Governance + verification credibility
- **F3** CLI surface cleanup
- **F4** Config/error/runtime hardening
- **F5** Storage/index correctness
- **F6** Performance/index evolution
- **F7** MCP/dashboard/security truth repair
- **F8** Packaging/release/install
- **F9** Testing + CI parity gates
- **F10** Repo hygiene + final seal

---

# F1 — Truth inventory + docs/code alignment

## `/home/truffle/Загрузки/synapse/README.md`
- **Фаза:** F1
- **Зачем менять:** README overclaims shipped surface, metrics, self-verify state, Phase 0 certainty.
- **Что менять:**
  - переписать секции про 12 MCP tools, 17 CLI commands, self-verify, Phase 0 requirement так, чтобы совпадало с реальным code state;
  - убрать или пометить как experimental команды/flows, которых нет в stable CLI;
  - обновить install/release/platform messaging;
  - убрать неподтвержденные метрики или сделать их derived.
- **Результат:** README не обещает больше, чем реально shipped.
- **Зависимости:** truth matrix по CLI/MCP/release.

## `/home/truffle/Загрузки/synapse/INSTALL.md`
- **Фаза:** F1, затем F8
- **Зачем менять:** URL/repo/release drift, platform support drift.
- **Что менять:**
  - выровнять repo URLs, install snippets, artifact names;
  - переписать platform section под реальный release matrix;
  - позже в F8 добавить checksums/signatures instructions.
- **Результат:** install doc соответствует release workflow и install.sh.
- **Зависимости:** F8 release policy.

## `/home/truffle/Загрузки/synapse/GUIDE.md`
- **Фаза:** F1
- **Зачем менять:** может ссылаться на несуществующие flows/commands.
- **Что менять:**
  - пройти все command examples;
  - убрать ghost commands;
  - выровнять wording around GRACE strictness and supported flows.
- **Результат:** guide не ведет пользователя в dead paths.
- **Зависимости:** CLI classification.

## `/home/truffle/Загрузки/synapse/CHANGELOG.md`
- **Фаза:** F1, F10
- **Зачем менять:** должен честно зафиксировать corrective release.
- **Что менять:**
  - добавить release notes для remediation waves;
  - фиксировать breaking changes: command visibility, config semantics, installer behavior.
- **Результат:** прозрачная история изменений.
- **Зависимости:** финальный scope изменений.

## `/home/truffle/Загрузки/synapse/PLAN.md`
- **Фаза:** F1
- **Зачем менять:** текущий план устарел и содержит speculative items без связи с actual remediation order.
- **Что менять:**
  - либо архивировать как historical plan;
  - либо пометить superseded and link to new remediation plan;
  - убрать misleading targets if kept as active plan.
- **Результат:** нет конкурирующих планов с разным truth state.
- **Зависимости:** новый remediation plan готов.

## `/home/truffle/Загрузки/synapse/AGENTS.md`
- **Фаза:** F1, F2
- **Зачем менять:** содержит strict Phase 0/verification rules, которые не совпадают с self-repo reality и behavior инструментов.
- **Что менять:**
  - разделить policy для generated user project vs self-repo;
  - уточнить strict/non-strict mode;
  - привести verify/review semantics к реальности;
  - убрать формулировки, которые конфликтуют с actual repo docs state.
- **Результат:** governance честное и исполнимое.
- **Зависимости:** решение по strict mode.

## `/home/truffle/Загрузки/synapse/docs/COMMANDS.md`
- **Фаза:** F1, F3
- **Зачем менять:** docs mentions commands not present in parser enum.
- **Что менять:**
  - сверить каждую documented command с `src/cli.rs`;
  - удалить/переместить команды в experimental section;
  - позже генерировать или validate against capability manifest.
- **Результат:** docs commands == shipped commands.
- **Зависимости:** command classification.

## `/home/truffle/Загрузки/synapse/docs/QUICKSTART.md`
- **Фаза:** F1
- **Зачем менять:** onboarding может вести в unsupported flows.
- **Что менять:**
  - убрать usage paths for missing commands;
  - обновить examples под shipped CLI only;
  - корректно описать init/index/search/verify/review/status path.
- **Результат:** quickstart проходит end-to-end.
- **Зависимости:** CLI truth matrix.

## `/home/truffle/Загрузки/synapse/docs/WORKFLOW.md`
- **Фаза:** F1, F2
- **Зачем менять:** описывает `syn plan`, `syn execute`, strict 500-line/500-token semantics, которые не совпадают с runtime/support reality.
- **Что менять:**
  - разделить shipped workflow vs aspirational workflow;
  - поправить strict mode wording;
  - синхронизировать verify/review/status semantics.
- **Результат:** workflow doc соответствует инструменту.
- **Зависимости:** governance decision.

## `/home/truffle/Загрузки/synapse/docs/FAQ.md`
- **Фаза:** F1
- **Зачем менять:** potential security scan wording drift, unsupported guidance.
- **Что менять:**
  - проверить все claims на соответствие;
  - исправить спорные ответы про verify/review/install/platforms.
- **Результат:** FAQ безопасен и точен.
- **Зависимости:** truth matrix.

## `/home/truffle/Загрузки/synapse/docs/references/semantic-markup.md`
- **Фаза:** F2
- **Зачем менять:** semantic rules должны совпадать с actual validators.
- **Что менять:**
  - проверить 500-token/500-line/unique block semantics;
  - убрать правила, которые verify не enforce либо пометить advisory.
- **Результат:** reference doc совпадает с validators.
- **Зависимости:** F2 verify/review redesign.

## `/home/truffle/Загрузки/synapse/docs/references/knowledge-graph.md`
- **Фаза:** F2
- **Зачем менять:** docs must reflect whether knowledge graph artifacts required on self-repo.
- **Что менять:**
  - уточнить strict mode behavior;
  - описать missing-artifact handling.
- **Результат:** no knowledge-graph policy drift.
- **Зависимости:** F2 governance.

## `/home/truffle/Загрузки/synapse/docs/references/contract-driven-dev.md`
- **Фаза:** F2
- **Зачем менять:** contract rules and enforcement need exactness.
- **Что менять:**
  - выровнять requirements with actual contract validator;
  - отделить must/should/can.
- **Результат:** contract rules measurable.
- **Зависимости:** F2 validator alignment.

## `/home/truffle/Загрузки/synapse/docs/references/verification-driven-dev.md`
- **Фаза:** F2
- **Зачем менять:** current verify claims stronger than actual checks.
- **Что менять:**
  - описать exact verification scopes;
  - отличить hard checks from heuristic checks;
  - убрать ambiguous “ALL PASS” language where needed.
- **Результат:** verification docs credible.
- **Зависимости:** F2.

## `/home/truffle/Загрузки/synapse/docs/references/unique-tag-convention.md`
- **Фаза:** F2
- **Зачем менять:** tags reference must mirror parser behavior.
- **Что менять:**
  - сверить parsing expectations with implementation in semantic extractor.
- **Результат:** naming/tag docs == parser rules.
- **Зависимости:** semantic extractor review.

---

# F2 — Governance + verification credibility

## `/home/truffle/Загрузки/synapse/src/grace/verify.rs`
- **Фаза:** F2
- **Зачем менять:** verification messaging and actual checks drift; 500-token rule mismatch; strictness ambiguity.
- **Что менять:**
  - ввести explicit strict/non-strict/self-repo policy;
  - синхронизировать artifact requirements with repo mode;
  - исправить naming and logic around 500-token or 500-line rule;
  - улучшить trace-assertion semantics;
  - ясно маркировать hard fail vs warning.
- **Результат:** verify output meaning exact, reproducible, honest.
- **Зависимости:** governance decision from AGENTS/docs.

## `/home/truffle/Загрузки/synapse/src/grace/review.rs`
- **Фаза:** F2, F7
- **Зачем менять:** current review reports missing artifacts and potential secrets, but semantics too heuristic and partly inconsistent with verify.
- **Что менять:**
  - выровнять artifact policy with verify;
  - разделить critical/warning/info more rigorously;
  - пересмотреть secret scan wording and implementation;
  - добавить explicit heuristic labels.
- **Результат:** review trustworthy, less misleading.
- **Зависимости:** verify semantics.

## `/home/truffle/Загрузки/synapse/src/grace/status.rs`
- **Фаза:** F2, F4
- **Зачем менять:** status should reflect degraded subsystems and same truth model as verify/review.
- **Что менять:**
  - унифицировать health semantics;
  - показывать warnings for degraded runtime components;
  - выводить strict/non-strict mode and artifact policy.
- **Результат:** status becomes authoritative dashboard of truth.
- **Зависимости:** F2 verify/review alignment.

## `/home/truffle/Загрузки/synapse/src/grace/contract.rs`
- **Фаза:** F2
- **Зачем менять:** contract docs and validator behavior may differ; suspicious test contract parsing examples inside file need review.
- **Что менять:**
  - проверить validator criteria;
  - убрать accidental low-quality internal examples if misleading;
  - align with reference docs.
- **Результат:** contract validation exact and explainable.
- **Зависимости:** docs reference alignment.

## `/home/truffle/Загрузки/synapse/src/grace/semantic.rs`
- **Фаза:** F2
- **Зачем менять:** semantic rules must match docs and verification outputs.
- **Что менять:**
  - audit block extraction edge cases;
  - clarify duplicate/closure handling;
  - expose machine-readable counts used by status/review/verify.
- **Результат:** semantic layer consistent across tools.
- **Зависимости:** docs/reference review.

## `/home/truffle/Загрузки/synapse/src/grace/refresh.rs`
- **Фаза:** F2
- **Зачем менять:** drift detection should know strict/self-repo artifact policy.
- **Что менять:**
  - support mode-aware artifact expectations;
  - emit actionable drift reasons.
- **Результат:** refresh no longer flags unavoidable repo-state ambiguities incorrectly.
- **Зависимости:** governance decision.

## `/home/truffle/Загрузки/synapse/src/grace/mod.rs`
- **Фаза:** F2
- **Зачем менять:** facade must expose updated semantics cleanly.
- **Что менять:**
  - wire new verify/review/status/refresh mode structs or settings.
- **Результат:** grace engine API coherent.
- **Зависимости:** submodule changes.

## `/home/truffle/Загрузки/synapse/docs/requirements.xml`
- **Фаза:** F2 only if self-repo decides to include Phase 0 artifacts
- **Зачем менять:** satisfy strict Phase 0 policy in-repo.
- **Что менять:** create real project requirements if team chooses strict self-host mode.
- **Результат:** artifact exists and maintained.
- **Зависимости:** policy decision.

## `/home/truffle/Загрузки/synapse/docs/technology.xml`
- **Фаза:** F2 only if chosen
- **Зачем менять:** same as above.
- **Что менять:** create true stack file for current repo.
- **Результат:** artifact exists.
- **Зависимости:** policy decision.

## `/home/truffle/Загрузки/synapse/docs/development-plan.xml`
- **Фаза:** F2 only if chosen
- **Зачем менять:** same.
- **Что менять:** encode actual module architecture, phases, dependencies.
- **Результат:** maintained dev-plan artifact.
- **Зависимости:** policy decision.

## `/home/truffle/Загрузки/synapse/docs/verification-plan.xml`
- **Фаза:** F2 only if chosen
- **Зачем менять:** currently missing; review flags it.
- **Что менять:** define verification refs matching actual modules.
- **Результат:** review/verify artifact consistency.
- **Зависимости:** policy decision.

## `/home/truffle/Загрузки/synapse/docs/knowledge-graph.xml`
- **Фаза:** F2 only if chosen
- **Зачем менять:** currently missing; review flags it.
- **Что менять:** represent actual modules and links.
- **Результат:** self-repo graph artifact.
- **Зависимости:** policy decision.

---

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

# F7 — MCP/dashboard/security truth repair

## `/home/truffle/Загрузки/synapse/src/mcp/mod.rs`
- **Фаза:** F7
- **Зачем менять:** module docs and exports should reflect actual MCP modes and LSP support.
- **Что менять:** align exports/comments/contracts with final MCP policy.
- **Результат:** module truth aligned.
- **Зависимости:** `mcp/server.rs`, `mcp/lsp.rs`.

## `/home/truffle/Загрузки/synapse/src/dashboard.rs`
- **Фаза:** F7
- **Зачем менять:** local-only security posture needs explicit guardrails.
- **Что менять:** add bind checks, optional auth/token path if chosen, clearer warnings.
- **Результат:** safer HTTP surface.
- **Зависимости:** product security decision.

## `/home/truffle/Загрузки/synapse/src/grace/review.rs`
- **Фаза:** F7
- **Зачем менять:** secret scan feature must be downgraded or improved.
- **Что менять:**
  - rename report labels to heuristic/basic if keeping current approach;
  - or integrate stronger pattern logic.
- **Результат:** security review not oversold.
- **Зависимости:** CI security tool choice.

## `/home/truffle/Загрузки/synapse/docs/FAQ.md`
- **Фаза:** F7
- **Зачем менять:** security/runtime questions must reflect real protections.
- **Что менять:** document localhost-only expectations, heuristic secret scanning, HTTP mode support state.
- **Результат:** users warned correctly.
- **Зависимости:** runtime policy.

## `/home/truffle/Загрузки/synapse/README.md`
- **Фаза:** F7 follow-up
- **Зачем менять:** update security posture messaging after MCP/dashboard decisions.
- **Что менять:** revise marketed claims.
- **Результат:** no false safety implication.
- **Зависимости:** F7 decisions.

---

# F8 — Packaging/release/install

## `/home/truffle/Загрузки/synapse/Cargo.toml`
- **Фаза:** F8
- **Зачем менять:** package metadata/repo/homepage must match project truth and release story.
- **Что менять:**
  - verify `repository`, `homepage`, description;
  - possibly add features or metadata for release tooling;
  - keep dependency list honest if release matrix changes.
- **Результат:** cargo metadata aligned with public docs.
- **Зависимости:** canonical repo/homepage decision.

## `/home/truffle/Загрузки/synapse/install.sh`
- **Фаза:** F8
- **Зачем менять:** installer uses fragile temp paths and may mismatch release artifacts/URLs.
- **Что менять:**
  - use `mktemp`;
  - validate URLs and artifact names;
  - add checksum/signature verification if available;
  - improve cleanup and error handling;
  - align supported platforms with release outputs.
- **Результат:** safer, accurate installer.
- **Зависимости:** release workflow updates.

## `/home/truffle/Загрузки/synapse/Dockerfile`
- **Фаза:** F8 optional
- **Зачем менять:** if docs mention container usage, Docker build should align with release/runtime changes.
- **Что менять:**
  - verify build path after CLI/module changes;
  - optionally add smoke command.
- **Результат:** container path remains valid.
- **Зависимости:** build/release changes.

## `/home/truffle/Загрузки/synapse/.github/workflows/release.yml`
- **Фаза:** F8
- **Зачем менять:** current release builds Linux-only, while docs imply broader support.
- **Что менять:**
  - add matrix for supported platforms;
  - standardize artifact names;
  - generate checksums;
  - optionally sign artifacts;
  - upload all outputs consistently.
- **Результат:** release artifacts == documented support.
- **Зависимости:** platform support decision.

## `/home/truffle/Загрузки/synapse/.github/workflows/ci.yml`
- **Фаза:** F8, F9
- **Зачем менять:** CI lacks release-smoke parity and docs drift gates.
- **Что менять:**
  - in F8 add release build smoke if needed;
  - in F9 add docs parity, self-host verify, installer tests, security scan.
- **Результат:** CI catches drift earlier.
- **Зависимости:** test scripts ready.

## `/home/truffle/Загрузки/synapse/INSTALL.md`
- **Фаза:** F8
- **Зачем менять:** final install docs must match new release pipeline.
- **Что менять:** update URLs, platform matrix, checksum verification instructions.
- **Результат:** doc-install parity.
- **Зависимости:** release/install changes.

---

# F9 — Testing + parity gates

## `/home/truffle/Загрузки/synapse/tests/integration_test.rs`
- **Фаза:** F9
- **Зачем менять:** current integration coverage mostly happy-path; misses docs parity, corrupted index, command drift, install/runtime edge cases.
- **Что менять:**
  - expand with negative cases;
  - add verify/review/status consistency checks;
  - add hooks status regression test;
  - add config load/init behavior tests;
  - add command visibility tests.
- **Результат:** broader regression net.
- **Зависимости:** core fixes landed.

## `/home/truffle/Загрузки/synapse/tests/fixtures/`
- **Фаза:** F9 new dir
- **Зачем менять:** need stable sample repos and broken cases.
- **Что менять:** create fixture projects:
  - minimal valid rust repo
  - mixed-language repo
  - broken-contract repo
  - corrupted-index repo
  - large-ish repo sample
- **Результат:** reproducible integration coverage.
- **Зависимости:** test strategy.

## `/home/truffle/Загрузки/synapse/tests/cli_help_snapshot.rs`
- **Фаза:** F9 new file
- **Зачем менять:** protect CLI help and command surface from drift.
- **Что менять:** snapshot top-level and subcommand help outputs.
- **Результат:** docs and help parity easier to maintain.
- **Зависимости:** CLI cleanup stable.

## `/home/truffle/Загрузки/synapse/tests/docs_parity.rs`
- **Фаза:** F9 new file
- **Зачем менять:** automatic check for README/docs claims vs code.
- **Что менять:** parse command list/tool count/platform claims/metrics or compare against generated manifest.
- **Результат:** drift caught in CI.
- **Зависимости:** capability source ready.

## `/home/truffle/Загрузки/synapse/tests/storage_corruption.rs`
- **Фаза:** F9 new file
- **Зачем менять:** ensure malformed index no longer silently disappears.
- **Что менять:** simulate truncated/invalid index and assert explicit behavior.
- **Результат:** storage hardening tested.
- **Зависимости:** storage fixes complete.

## `/home/truffle/Загрузки/synapse/tests/config_behavior.rs`
- **Фаза:** F9 new file
- **Зачем менять:** lock in pure load vs init semantics.
- **Что менять:** test missing config, invalid config, readonly paths, init path.
- **Результат:** config contract protected.
- **Зависимости:** config redesign complete.

## `/home/truffle/Загрузки/synapse/tests/tracking_identity.rs`
- **Фаза:** F9 new file
- **Зачем менять:** prevent basename collision regressions.
- **Что менять:** test two same-named directories under different parents.
- **Результат:** tracking correctness protected.
- **Зависимости:** tracking schema update.

## `/home/truffle/Загрузки/synapse/tests/mcp_runtime.rs`
- **Фаза:** F9 new file
- **Зачем менять:** validate stdio mode, HTTP mode policy, tool exposure.
- **Что менять:** add smoke/runtime behavior checks.
- **Результат:** MCP truth enforced.
- **Зависимости:** MCP policy stabilized.

## `/home/truffle/Загрузки/synapse/tests/dashboard_bind.rs`
- **Фаза:** F9 new file
- **Зачем менять:** validate loopback/non-loopback warning behavior.
- **Что менять:** add server startup policy tests where feasible.
- **Результат:** dashboard security posture enforced.
- **Зависимости:** dashboard hardening complete.

## `/home/truffle/Загрузки/synapse/tests/release_install_smoke.rs`
- **Фаза:** F9 optional/new file or script-based
- **Зачем менять:** protect install/release flow.
- **Что менять:** smoke-test artifact naming/install logic, possibly via shell-driven CI.
- **Результат:** installer drift caught before release.
- **Зависимости:** release workflow stable.

## `/home/truffle/Загрузки/synapse/.github/workflows/ci.yml`
- **Фаза:** F9
- **Зачем менять:** wire all parity tests and self-host checks.
- **Что менять:**
  - add `cargo build --release`;
  - add self-host `syn verify` check;
  - add docs parity test job;
  - add installer smoke job;
  - add external secret scan if chosen;
  - optional `cargo deny`/coverage.
- **Результат:** CI becomes anti-drift gatekeeper.
- **Зависимости:** tests and scripts exist.

## `/home/truffle/Загрузки/synapse/scripts/`
- **Фаза:** F9
- **Зачем менять:** may need helper scripts for docs parity, release smoke, checksum verify.
- **Что менять:**
  - add deterministic helper scripts only if needed;
  - keep them minimal and CI-oriented.
- **Результат:** repeatable automation.
- **Зависимости:** exact CI jobs chosen.

---

# F10 — Repo hygiene + final seal

## `/home/truffle/Загрузки/synapse/.gitignore`
- **Фаза:** F10
- **Зачем менять:** repo contains noisy/generated assets that may pollute search/test/audit.
- **Что менять:**
  - review ignored generated outputs;
  - ensure local artifacts, temp indexes, release outputs, fixtures behave correctly.
- **Результат:** cleaner repo behavior.
- **Зависимости:** release/test artifact policy.

## `/home/truffle/Загрузки/synapse/.opencode/`
- **Фаза:** F10
- **Зачем менять:** vendored dependencies/config may create search noise and drift.
- **Что менять:**
  - decide what should be committed;
  - reduce node_modules/vendor noise if possible;
  - align plugin/rules with init-generated outputs.
- **Результат:** lower audit/search noise.
- **Зависимости:** hook/install policy.

## `/home/truffle/Загрузки/synapse/opencode-plugin/plugin.ts`
- **Фаза:** F10
- **Зачем менять:** plugin file currently outside contract system and may drift from hooks/init docs.
- **Что менять:**
  - align plugin behavior with generated `.opencode/plugins/synapse.ts`;
  - decide source-of-truth between plugin template and committed plugin.
- **Результат:** plugin story consistent.
- **Зависимости:** hook/init packaging decisions.

## `/home/truffle/Загрузки/synapse/hooks/opencode/synapse-rewrite.sh`
- **Фаза:** F10
- **Зачем менять:** hook script should align with plugin/proxy/runtime expectations.
- **Что менять:**
  - audit path assumptions;
  - update behavior or docs;
  - decide whether to bring under same governance/testing.
- **Результат:** hook runtime consistent.
- **Зависимости:** proxy/hook policy.

## `/home/truffle/Загрузки/synapse/hooks/adapters/cursor.sh`
- **Фаза:** F10 optional
- **Зачем менять:** shell adapters currently outside contract/testing coverage.
- **Что менять:**
  - audit install/usefulness;
  - decide keep/remove/experimental.
- **Результат:** reduced dead surface.
- **Зависимости:** product scope decision.

## `/home/truffle/Загрузки/synapse/hooks/adapters/claude-code.sh`
- **Фаза:** F10 optional
- **Зачем менять:** same as above.
- **Что менять:** audit and classify.
- **Результат:** same.
- **Зависимости:** scope decision.

## `/home/truffle/Загрузки/synapse/build.rs`
- **Фаза:** F10
- **Зачем менять:** build script should track correct generated/config sources after refactors and packaging updates.
- **Что менять:**
  - audit rerun-if-changed list;
  - include new capability/generated docs files if required;
  - ensure no stale dependencies.
- **Результат:** reproducible builds.
- **Зависимости:** final file layout stabilized.

## `/home/truffle/Загрузки/synapse/Cargo.toml`
- **Фаза:** F10 follow-up
- **Зачем менять:** after refactors/tests/scripts, update package metadata, features, dev-dependencies.
- **Что менять:**
  - add/remove deps from remediation work;
  - keep manifest lean.
- **Результат:** dependency graph honest and minimal.
- **Зависимости:** implementation complete.

## `/home/truffle/Загрузки/synapse/Makefile`
- **Фаза:** F10
- **Зачем менять:** expose new validation flows and release/test helpers.
- **Что менять:**
  - add targets for parity, verify-self, release-smoke, install-smoke if useful.
- **Результат:** repeatable maintainer workflows.
- **Зависимости:** CI/test scripts finalized.

---

# Cross-file additions strongly recommended

## `/home/truffle/Загрузки/synapse/docs/remediation/`
- **Фаза:** F1-F10 optional dir
- **Зачем менять:** keep remediation artifacts organized.
- **Что менять:**
  - place truth matrix
  - capability matrix
  - drift checklist
  - release support matrix
- **Результат:** management visibility without polluting root.
- **Зависимости:** if team wants permanent planning artifacts.

## `/home/truffle/Загрузки/synapse/src/capabilities.rs`
- **Фаза:** F3/F9 optional new file
- **Зачем менять:** single source of truth for shipped commands, MCP tools, platforms, experimental features.
- **Что менять:**
  - define machine-readable capability registry;
  - feed docs parity tests and status output.
- **Результат:** drift resistance by design.
- **Зависимости:** command/tool/platform classification.

## `/home/truffle/Загрузки/synapse/tests/capabilities_parity.rs`
- **Фаза:** F9 optional new file
- **Зачем менять:** enforce capability registry parity.
- **Что менять:** compare registry to docs/help/MCP exposure.
- **Результат:** one-source-of-truth enforcement.
- **Зависимости:** `src/capabilities.rs` exists.

---

# Execution order summary by file clusters

## Cluster 1 — first touch, mandatory
1. `README.md`
2. `INSTALL.md`
3. `docs/COMMANDS.md`
4. `docs/QUICKSTART.md`
5. `docs/WORKFLOW.md`
6. `AGENTS.md`
7. `src/cli.rs`
8. `src/grace/verify.rs`
9. `src/grace/review.rs`
10. `src/grace/status.rs`
11. `src/config.rs`
12. `src/indexer/storage.rs`
13. `src/indexer/mod.rs`
14. `src/tracking/mod.rs`
15. `src/mcp/server.rs`
16. `src/dashboard.rs`
17. `src/hooks/mod.rs`
18. `tests/integration_test.rs`
19. `.github/workflows/ci.yml`
20. `install.sh`
21. `.github/workflows/release.yml`

## Cluster 2 — second wave
1. `src/main.rs`
2. `src/lib.rs`
3. `src/mcp/lsp.rs`
4. `src/indexer/walker.rs`
5. `src/indexer/parser.rs`
6. `src/indexer/languages/mod.rs`
7. `src/proxy/*`
8. `src/graphrag/*`
9. `Cargo.toml`
10. `Makefile`
11. `build.rs`

## Cluster 3 — new files / modularization / parity
1. `src/cli/mod.rs`
2. `src/cli/schema.rs`
3. `src/cli/dispatch.rs`
4. `src/cli/commands/*.rs`
5. `tests/docs_parity.rs`
6. `tests/storage_corruption.rs`
7. `tests/config_behavior.rs`
8. `tests/tracking_identity.rs`
9. `tests/mcp_runtime.rs`
10. `tests/dashboard_bind.rs`
11. `tests/fixtures/*`
12. optional `src/capabilities.rs`

---

# Что делать со старыми и новыми planning files

## Existing
- `PLAN.md` — оставить как historical plan, но пометить superseded.

## New
- этот файл — operational remediation plan.
- если нужны доп. артефакты:
  - `docs/remediation/truth-matrix.md`
  - `docs/remediation/capability-matrix.md`
  - `docs/remediation/release-support-matrix.md`
  - `docs/remediation/drift-checklist.md`

---

# Final done state

После выполнения плана должны быть истинны все пункты:
1. docs == code == release == install == CI
2. shipped CLI surface полностью определен
3. ghost commands removed/hidden/labeled
4. verify/review/status согласованы семантически
5. config load не пишет на диск скрытно
6. index storage не теряет молча данные
7. tracking не смешивает проекты по basename
8. MCP/dashboard security posture честно описан
9. installer/release platform claims подтверждены artifacts
10. CI автоматически ловит drift
11. repo search/audit noise уменьшен
12. любой claim в README можно проверить автоматически или через code path
