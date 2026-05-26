# Synapse

> **AI Agent Engineering Platform — works transparently through OpenCode CLI**
> *48 MCP tools. Sharded Phase 0 gate. Self-verified. Zero overhead for humans.*

---

## Как это работает

```
Ты → OpenCode CLI → LLM
                       │
         ┌─────────────┼─────────────┐
         ▼             ▼             ▼
    MCP Tools       Plugin       AGENTS.md
  (48 инструментов)  (proxy,      (GRACE
                     GRACE)       конституция)
         │
         ▼
     Synapse (фоновый движок)
```

Ты общаешься с AI через `opencode`. Synapse невидимо:
- Даёт LLM **48 MCP-инструментов** для поиска, проверки и генерации кода
- Авто-фильтрует шумный вывод shell-команд; фактическую экономию показывает `syn gain`
- **Принуждает GRACE методологию**: Phase 0, контракты, верификация, ревью
- **Сам проходит собственные проверки**: `syn verify` → ALL PASS

## Почему это полезно

Обычно AI coding flow ломается на 4 местах:
- архитектура живёт в голове и теряется между сессиями
- shell output слишком шумный и дорогой по токенам
- у LLM нет стабильных project-native tools
- verification и review происходят слишком поздно

Synapse решает это так:
- хранит архитектуру в **sharded GRACE artifacts**
- даёт OpenCode **48 MCP tools**
- даёт **16 workflow tools** для init/plan/execute/review/fix/status/run-history
- режет shell noise через proxy
- навязывает verify/review discipline прямо в цикле работы

---

## Что можно делать сразу

```bash
syn init
syn index
opencode
```

Внутри OpenCode дальше доступны сценарии:
- `grace_init` — создать/восстановить архитектурный каркас
- `grace_plan` — разложить работу на модули и фазы
- `grace_execute` — вести bounded implementation step
- `grace_reviewer` — прогонять integrity review
- `grace_status` — смотреть phase/module health
- `grace_fix` — разбирать баг через код + артефакты

---

## Локальная отладка skills

```bash
syn skills list
syn skills show grace_status
syn skills run grace_status detail_level=summary
```

---

## Установка

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# OpenCode
curl -fsSL https://opencode.ai/install.sh | sh

# Synapse (latest published Linux/macOS release)
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/v2.6.7/install.sh | sh

# Custom install directory without sudo
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/v2.6.7/install.sh | SYN_INSTALL_DIR="$HOME/.local/bin" sh

# Source install for development checkouts
git clone https://github.com/anyagixx/synapse.git
cd synapse && make install
```

---

## Быстрый старт

```bash
mkdir my-project && cd my-project
syn init          # 1 сек: интеграция с OpenCode
syn hooks install all
syn hooks audit all --json
opencode          # LLM видит 48 MCP инструментов + sharded Phase 0 gate
# LLM: "Что ты хочешь построить?"
# Ты:  "Приложение для заметок с поиском"
```

Для небольших Python/SQL ботов не нужно вручную ломать синтаксис комментариев: `suggest_contract` и валидатор используют `#` для Python/shell, `--` для SQL и `//` для Rust/TS/JS. Если проект маленький и function contracts на каждый helper создают шум, запускай проверки с профилем:

```bash
syn verify --profile lite
syn review --profile balanced
syn doctor --deps
```

`MODULE_ID` должен быть одним значением вроде `M-BOT`; связанные модули перечисляются в `DEPENDS` или typed `LINKS`.
Новый формат `LINKS` поддерживает направление и тип связи, например `→ M-STORAGE (depends) — persistence` или `← V-M-BOT (verified_by) — tests`.
Старый `LINKS: M-STORAGE, V-M-BOT` остаётся совместимым и трактуется как legacy `depends`.
Semantic anchors теперь можно писать и в GRACE XML-like стиле (`// <BLOCK name="validate"> ... // </BLOCK>`); старый `START_/END_` синтаксис остаётся совместимым, а `anchor-syntax-consistent` показывает смешанные файлы как warning.
`syn verify` также проверяет non-human programming patterns: explicit typing, explicit flow, explicit null handling, отсутствие magic values и deterministic iteration. В `lite/balanced` это помогает без шума мигрировать маленькие проекты, в `strict` блокируются только реально рискованные паттерны.

---

## Phase 0 — Архитектура перед кодом

LLM **не может писать код** пока не созданы shard indexes и dirs в `docs/`:

| Файл | Что содержит |
|------|-------------|
| `docs/graph-index.xml` | Граф модулей |
| `docs/plan-index.xml` | Фазы и порядок |
| `docs/verification-index.xml` | Проверки модулей |
| `docs/traceability-index.xml` | Матрица requirement/use-case -> code/log |
| `docs/modules/` | Sharded module docs |
| `docs/phases/` | Sharded phase docs |
| `docs/verification/` | Sharded verification docs |

AGENTS.md содержит STOP-правило: «You MAY NOT write source code in Phase 0.»

---

## 48 MCP Tools

### 32 Core tools

| Инструмент | Назначение |
|-----------|-----------|
| `semantic_search` | BM25 + векторный поиск (14 языков) |
| `view_signatures` | Сигнатуры функций и классов |
| `graphrag_query` | Граф знаний — узлы, typed LINKS, связи, пути |
| `verify_project` | 3 уровня проверки (sharded model) |
| `review_code` | 3 режима ревью (scoped/wave-audit/full) |
| `project_status` | Полный health-отчёт |
| `analyze_logs` | LDD-анализ structured LOG trajectory/anomaly/compare |
| `extract_belief_state` | Создание и валидация docs/belief-states перед кодогенерацией |
| `generate_requirements` | Создание и валидация полного `docs/requirements.xml` с AAG use cases |
| `generate_technology` | Создание и валидация полного `docs/technology.xml` с exact versions и compatibility matrix |
| `generate_development_plan` | Создание и валидация `docs/development-plan.xml` с DataFlows, GenerationOrder и MentalTests |
| `mental_test_run` | Запуск MentalTest перед кодогенерацией и запись trace в `docs/mental-tests/` |
| `traceability_report` | End-to-end matrix: requirements/use cases -> modules/functions/LOG evidence |
| `cascade_impact` | Preview downstream impact for changed requirements/contracts/interfaces/code |
| `cascade_execute` | Execute cached cascade preview and write proposal/changelog artifacts |
| `run_test_guide` | Запуск natural-language testing guide и запись tester-agent summary/failure artifacts |
| `submit_test_report` | Передача XML failure report разработчику с подсветкой LOG evidence refs |
| `self_heal` | Один bounded self-heal цикл для persisted autonomous run |
| `advance_phase` | Проверка phase gates и dry-run перехода к следующей фазе |
| `pre_commit_check` | Pre-commit gate для persisted bounded run |
| `token_savings` | Статистика экономии токенов |
| `compress_text` | Сжатие текста (3 уровня) |
| `refresh_project` | Синхронизация графа и плана с кодом |
| `diagnose_failure` | Разбор tester-agent failure evidence и bounded fix diagnosis |
| `repair_contract` | Безопасный MODULE_CONTRACT repair preview/apply |
| `suggest_contract` | Генерация MODULE_CONTRACT шаблона |
| `lsp_hover` | Тип/сигнатура через LSP |
| `lsp_references` | Поиск использований символа |
| `tools/recommend` | Подбор компактного набора MCP tools под текущий контекст |
| `compact_evidence` | Сжатие evidence refs для persisted run |
| `check_budget` | Проверка session token budget и estimated-token affordability |
| `context_pressure` | Оценка заполнения context window и рекомендация нового session |

### 16 GRACE workflow tools

| Tool | Purpose |
|------|---------|
| `grace_init` | Инициализация sharded GRACE layout |
| `grace_plan` | Планирование модулей и фаз |
| `grace_verification` | Планирование проверки модулей |
| `grace_execute` | Bounded execution guidance |
| `grace_multiagent_execute` | Разбивка работ по ролям/модулям |
| `grace_reviewer` | Workflow review orchestration |
| `grace_refresh` | Artifact refresh orchestration |
| `grace_refactor` | Refactor planning under GRACE |
| `grace_fix` | Диагностика и bounded fix flow |
| `grace_status` | Статус по артефактам и фазам |
| `grace_run_history` | История bounded autonomous runs и provenance events |
| `grace_ask` | Вопросы по артефактам и коду |
| `grace_explainer` | Объяснение кода и архитектуры |
| `grace_cli` | Помощь по Synapse/OpenCode CLI |
| `grace_setup_subagents` | Настройка planner/implementer/reviewer/verifier/fixer |
| `grace_lint` | Проверка shard integrity |

---

## Что `syn init` создаёт

```text
my-project/
├── AGENTS.md                     ← GRACE конституция
├── opencode.jsonc                ← MCP авто-старт
├── docs/
│   ├── graph-index.xml           ← primary graph index
│   ├── plan-index.xml            ← primary phase/plan index
│   ├── verification-index.xml    ← primary verification index
│   ├── modules/
│   │   └── M-CORE.xml
│   ├── phases/
│   │   ├── Phase-0.xml
│   │   └── Phase-1.xml
│   ├── verification/
│   │   └── V-M-CORE.xml
│   ├── mental-tests/            ← MentalTest traces
│   ├── tests/                   ← testing guides + tester-agent results
│   ├── requirements.xml          ← RequirementsAnalysis + AAG use cases
│   ├── technology.xml            ← exact versions + compatibility matrix
│   ├── development-plan.xml      ← DataFlows + GenerationOrder + MentalTests + NonHumanPatterns
│   ├── verification-plan.xml     ← compatibility layer
│   └── knowledge-graph.xml       ← compatibility layer
└── .opencode/
    ├── plugins/synapse.ts        ← авто-прокси + GRACE
    ├── rules/synapse.md          ← инструкции для LLM
    └── package.json              ← зависимости
```

---

## CLI команды

| Команда | Назначение |
|---------|-----------|
| `syn init` | Установка интеграции |
| `syn index` / `--watch` | Индексация + авто-переиндексация |
| `syn search "q"` | Прямой поиск по коду |
| `syn verify` | 3 уровня проверки |
| `syn review` | Ревью кода |
| `syn refresh` | Синхронизация artifacts |
| `syn status` | Health-отчёт |
| `syn doctor` | Диагностика (10 проверок) |
| `syn history "q"` | Поиск по git-истории |
| `syn serve` | Web дашборд: health/status/tokens plus GRACE state views |
| `syn proxy -- <cmd>` | Ручной прокси |
| `syn proxy --route -- <cmd>` | Показать выбранный token-saving adapter без запуска команды |
| `syn gt <cmd>` | Graphite stacked-PR команды через token-saving proxy |
| `syn binlog <path>` | Краткая диагностика MSBuild binlog |
| `syn dotnet-format-report <path>` | Сводка dotnet format JSON report |
| `syn dotnet-trx <path-or-dir>` | Сводка TRX test results |
| `syn gain` | Статистика экономии |
| `syn gain --graph` | Статистика экономии с ASCII-графом |
| `syn gain --sessions --adapters` | Экономика по сессиям и command adapters |
| `syn session` | Экономика по tracked RTK sessions |
| `syn cc-economics --format json` | Локальная экономика токенов для Claude Code/Synapse sessions |
| `syn tools list|validate|new` | Локальные user-defined MCP tools из `~/.synapse/tools` |
| `syn skills list|show|run` | Локальный запуск и отладка 16 GRACE skills |
| `syn ci verify|review|status` | CI-friendly strict outputs |

Dashboard routes exposed by `syn serve`:

| Route | Назначение |
|-------|-----------|
| `/belief-states` | BeliefState coverage and module drill-down links |
| `/mental-tests` | DevelopmentPlan MentalTest status |
| `/traceability/{artifact_id}` | Artifact-scoped traceability chains |
| `/cascade/preview?artifact=...&change=...` | Cascade impact preview |
| `/cascade/history` | Cascade changelog history |
| `/api/belief-states`, `/api/mental-tests`, `/api/traceability`, `/api/cascade-impact`, `/api/cascade-history` | JSON API for automation |

---

## Что под капотом

| Компонент | Описание |
|-----------|----------|
| **Octocode** | AST-индексация (5 языков) + fallback (14), BM25 + векторный + гибридный поиск |
| **RTK Proxy** | 30+ TOML-фильтров, 8-стадийный пайплайн, авто-прокси через плагин |
| **Caveman** | 3 уровня сжатия (lite/full/ultra) |
| **GRACE** | Sharded Phase 0 gate, MODULE_CONTRACT/MAP/CHANGE_SUMMARY, 48 MCP tools, 16 workflow tools, 3 режима ревью |

---

## Метрики

| Метрика | Значение |
|---------|----------|
| Бинарник | ~13 MB release |
| Зависимости | 0 внешних системных (всё статически слинковано) |
| MCP инструментов | **48** |
| CLI команд | 19 |
| Проверок verify | 55 |
| GRACE workflow tools | 16 |
| Режимов review | 3 |
| Doctor проверок | 10 |
| Языков индексации | 14 |
| Тестов | **185** (cargo test --all-targets) |
| Контрактов в своём коде | **94/94** |
| Belief-state coverage | **85/85** |
| Traceability | **strict, 100.0%** |
| Non-human warnings | **0** |
| self-verify | **ALL PASS** |

## License

Apache 2.0
