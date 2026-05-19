# Synapse

> **AI Agent Engineering Platform — works transparently through OpenCode CLI**
> *27 MCP tools. Sharded Phase 0 gate. Self-verified. Zero overhead for humans.*

---

## Как это работает

```
Ты → OpenCode CLI → LLM
                       │
         ┌─────────────┼─────────────┐
         ▼             ▼             ▼
    MCP Tools       Plugin       AGENTS.md
  (27 инструментов)  (proxy,      (GRACE
                     GRACE)       конституция)
         │
         ▼
     Synapse (фоновый движок)
```

Ты общаешься с AI через `opencode`. Synapse невидимо:
- Даёт LLM **27 MCP-инструментов** для поиска, проверки и генерации кода
- Авто-фильтрует вывод shell-команд (экономия 60-90% токенов)
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
- даёт OpenCode **27 MCP tools**
- даёт **15 workflow tools** для init/plan/execute/review/fix/status
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

# Synapse (prebuilt Linux release or Cargo source fallback)
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/main/install.sh | sh

# Custom install directory without sudo
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/main/install.sh | SYN_INSTALL_DIR="$HOME/.local/bin" sh

# Source install for development checkouts
git clone https://github.com/anyagixx/synapse.git
cd synapse && make install
```

---

## Быстрый старт

```bash
mkdir my-project && cd my-project
syn init          # 1 сек: интеграция с OpenCode
opencode          # LLM видит 27 MCP инструментов + sharded Phase 0 gate
# LLM: "Что ты хочешь построить?"
# Ты:  "Приложение для заметок с поиском"
```

---

## Phase 0 — Архитектура перед кодом

LLM **не может писать код** пока не созданы shard indexes и dirs в `docs/`:

| Файл | Что содержит |
|------|-------------|
| `docs/graph-index.xml` | Граф модулей |
| `docs/plan-index.xml` | Фазы и порядок |
| `docs/verification-index.xml` | Проверки модулей |
| `docs/modules/` | Sharded module docs |
| `docs/phases/` | Sharded phase docs |
| `docs/verification/` | Sharded verification docs |

AGENTS.md содержит STOP-правило: «You MAY NOT write source code in Phase 0.»

---

## 27 MCP Tools

### 12 Core tools

| Инструмент | Назначение |
|-----------|-----------|
| `semantic_search` | BM25 + векторный поиск (14 языков) |
| `view_signatures` | Сигнатуры функций и классов |
| `graphrag_query` | Граф знаний — узлы, связи, пути |
| `verify_project` | 3 уровня проверки (sharded model) |
| `review_code` | 3 режима ревью (scoped/wave-audit/full) |
| `project_status` | Полный health-отчёт |
| `token_savings` | Статистика экономии токенов |
| `compress_text` | Сжатие текста (3 уровня) |
| `refresh_project` | Синхронизация графа и плана с кодом |
| `suggest_contract` | Генерация MODULE_CONTRACT шаблона |
| `lsp_hover` | Тип/сигнатура через LSP |
| `lsp_references` | Поиск использований символа |

### 15 GRACE workflow tools

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
│   ├── requirements.xml          ← compatibility layer
│   ├── technology.xml            ← compatibility layer
│   ├── development-plan.xml      ← compatibility layer
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
| `syn serve` | Web дашборд |
| `syn proxy -- <cmd>` | Ручной прокси |
| `syn gain` | Статистика экономии |
| `syn skills list|show|run` | Локальный запуск и отладка 15 GRACE skills |
| `syn ci verify|review|status` | CI-friendly strict outputs |

---

## Что под капотом

| Компонент | Описание |
|-----------|----------|
| **Octocode** | AST-индексация (5 языков) + fallback (14), BM25 + векторный + гибридный поиск |
| **RTK Proxy** | 30+ TOML-фильтров, 8-стадийный пайплайн, авто-прокси через плагин |
| **Caveman** | 3 уровня сжатия (lite/full/ultra) |
| **GRACE** | Sharded Phase 0 gate, MODULE_CONTRACT/MAP/CHANGE_SUMMARY, 27 MCP tools, 15 workflow tools, 3 режима ревью |

---

## Метрики

| Метрика | Значение |
|---------|----------|
| Бинарник | ~13 MB release |
| Зависимости | 0 внешних системных (всё статически слинковано) |
| MCP инструментов | **27** |
| CLI команд | 19 |
| Проверок verify | 11 |
| GRACE workflow tools | 15 |
| Режимов review | 3 |
| Doctor проверок | 10 |
| Языков индексации | 14 |
| Тестов | **36** (24 unit + 5 integration + 4 parity + 3 MCP/skills) |
| Контрактов в своём коде | **40/40** |
| self-verify | **ALL PASS** |

## License

Apache 2.0
