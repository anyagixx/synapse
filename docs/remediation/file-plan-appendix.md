# Appendix — Cross-file additions and final state

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
