# F1-F2 — Truth and Governance

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

