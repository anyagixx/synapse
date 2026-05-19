# FILE-BY-FILE REMEDIATION PLAN

Этот файл теперь является индексом sharded remediation-плана. Полный исходный план сохранен без смыслового сокращения в `docs/remediation/`, чтобы каждый artifact оставался ниже Phase 2 maintainability target.

## Как читать план

Для каждого файла указано:
- **Фаза** — когда менять
- **Зачем менять** — причина
- **Что менять** — конкретный объем правок
- **Результат** — что должно получиться
- **Зависимости** — что должно быть сделано раньше

## Shards

- `docs/remediation/file-plan-F1-F2.md` — truth inventory, docs/code alignment, governance credibility.
- `docs/remediation/file-plan-F3-F6.md` — CLI surface cleanup, runtime hardening, storage/index correctness, performance/index evolution.
- `docs/remediation/file-plan-F7-F10.md` — MCP/dashboard/security, packaging/release/install, testing/parity, repo hygiene.
- `docs/remediation/file-plan-appendix.md` — cross-file additions, execution clusters, planning-file policy, final done state.

## Phase Legend

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

## Current Phase 2 Usage

Phase 2 uses this plan as historical remediation context, not as a competing active plan. The active execution source of truth remains `docs/plan-index.xml` and `docs/phases/Phase-2.xml`.
