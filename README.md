# Synapse

> **AI Agent Engineering Platform — works transparently through OpenCode CLI**
> *12 MCP tools. Phase 0 gate. Self-verified. Zero overhead for humans.*

---

## Как это работает

```
Ты → OpenCode CLI → LLM
                       │
         ┌─────────────┼─────────────┐
         ▼             ▼             ▼
    MCP Tools       Plugin       AGENTS.md
  (12 инструментов)  (proxy,      (GRACE
                     GRACE)       конституция)
         │
         ▼
     Synapse (фоновый движок)
```

Ты общаешься с AI через `opencode`. Synapse невидимо:
- Даёт LLM **12 MCP-инструментов** для поиска, проверки и генерации кода
- Авто-фильтрует вывод shell-команд (экономия 60-90% токенов)
- **Принуждает GRACE методологию**: Phase 0, контракты, верификация, ревью
- **Сам проходит собственные проверки**: `syn verify` → ALL PASS

---

## Установка

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# OpenCode
curl -fsSL https://opencode.ai/install.sh | sh

# Synapse
git clone https://github.com/anyagixx/synapse.git
cd synapse && make install
```

---

## Быстрый старт

```bash
mkdir my-project && cd my-project
syn init          # 1 сек: интеграция с OpenCode
opencode          # LLM видит 12 MCP инструментов + Phase 0 gate
# LLM: "Что ты хочешь построить?"
# Ты:  "Приложение для заметок с поиском"
```

---

## Phase 0 — Архитектура перед кодом

LLM **не может писать код** пока не созданы 5 файлов в `docs/`:

| Файл | Что содержит |
|------|-------------|
| `docs/requirements.xml` | Что строим |
| `docs/technology.xml` | На чём строим |
| `docs/development-plan.xml` | Модули, фазы, зависимости |
| `docs/verification-plan.xml` | Как проверяем |
| `docs/knowledge-graph.xml` | Связи между модулями |

AGENTS.md содержит STOP-правило: «You MAY NOT write source code in Phase 0.»

---

## 12 MCP Tools

| Инструмент | Назначение |
|-----------|-----------|
| `semantic_search` | BM25 + векторный поиск (14 языков) |
| `view_signatures` | Сигнатуры функций и классов |
| `graphrag_query` | Граф знаний — узлы, связи, пути |
| `verify_project` | 3 уровня проверки (11 проверок) |
| `review_code` | 3 режима ревью (scoped/wave-audit/full) |
| `project_status` | Полный health-отчёт |
| `token_savings` | Статистика экономии токенов |
| `compress_text` | Сжатие текста (3 уровня) |
| `refresh_project` | Синхронизация графа и плана с кодом |
| `suggest_contract` | Генерация MODULE_CONTRACT шаблона |
| `lsp_hover` | Тип/сигнатура через LSP |
| `lsp_references` | Поиск использований символа |

---

## Что `syn init` создаёт

```
my-project/
├── AGENTS.md                     ← GRACE конституция
├── opencode.jsonc                ← MCP авто-старт
├── docs/                         ← Phase 0: 5 XML-шаблонов
│   ├── requirements.xml
│   ├── technology.xml
│   ├── development-plan.xml
│   ├── verification-plan.xml
│   └── knowledge-graph.xml
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

---

## Что под капотом

| Компонент | Описание |
|-----------|----------|
| **Octocode** | AST-индексация (5 языков) + fallback (14), BM25 + векторный + гибридный поиск |
| **RTK Proxy** | 30+ TOML-фильтров, 8-стадийный пайплайн, авто-прокси через плагин |
| **Caveman** | 3 уровня сжатия (lite/full/ultra) |
| **GRACE** | Phase 0 gate, MODULE_CONTRACT/MAP/CHANGE_SUMMARY, 11 проверок, 3 режима ревью |

---

## Метрики

| Метрика | Значение |
|---------|----------|
| Бинарник | ~13 MB release |
| Зависимости | 0 внешних системных (всё статически слинковано) |
| MCP инструментов | **12** |
| CLI команд | 18 |
| Проверок verify | 11 |
| Режимов review | 3 |
| Doctor проверок | 10 |
| Языков индексации | 14 |
| Тестов | **32** (24 unit + 4 integration + 4 parity) |
| Контрактов в своём коде | **34/34** |
| self-verify | **ALL PASS** |

## License

Apache 2.0
