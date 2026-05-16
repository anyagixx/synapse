# Synapse

> **AI Agent Engineering Platform — works transparently through OpenCode CLI**
> Code intelligence + Token proxy + Compression + GRACE methodology
> *8 MCP tools. Phase 0 gate. Zero cognitive overhead for humans.*

---

## Как это работает

```
Ты → OpenCode CLI → LLM (принимает решения)
                        │
          ┌─────────────┼─────────────┐
          ▼             ▼             ▼
     MCP Tools       Plugin       AGENTS.md
   (8 инструментов)  (proxy,      (GRACE
                     GRACE)       конституция)
          │
          ▼
      Synapse (фоновый движок)
```

Ты просто общаешься с AI через `opencode`. Synapse невидимо:
- Даёт LLM 8 MCP-инструментов для поиска и проверки кода
- Автоматически фильтрует вывод shell-команд (экономия 60-90% токенов)
- **Принуждает GRACE методологию: Phase 0 (docs ДО кода), контракты, верификация**

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
syn init          # 1 сек: AGENTS.md + 5 XML-шаблонов + MCP + плагин
opencode          # LLM видит Phase 0 gate
# LLM: "Вижу пустой requirements.xml — что делаем?"
# Ты:  "Приложение для заметок с поиском"
# LLM: заполняет 5 docs → пишет код с MODULE_CONTRACT → verify → review
```

---

## Phase 0 — Архитектура перед кодом (GRACE enforcement)

LLM **не может писать код** пока не созданы 5 файлов в `docs/`:

| Файл | Что содержит |
|------|-------------|
| `docs/requirements.xml` | Что строим |
| `docs/technology.xml` | На чём строим |
| `docs/development-plan.xml` | Модули, фазы, зависимости |
| `docs/verification-plan.xml` | Как проверяем |
| `docs/knowledge-graph.xml` | Связи между модулями |

AGENTS.md содержит `STOP`-правило: «You are in Phase 0 until ALL 5 files exist. You MAY NOT write source code.»

---

## 8 MCP Tools (LLM вызывает сама)

| Инструмент | Что делает |
|-----------|-----------|
| `semantic_search` | Поиск по коду с BM25 (14 языков) |
| `view_signatures` | Сигнатуры функций и классов |
| `graphrag_query` | Граф знаний — узлы, связи, пути |
| `verify_project` | 3 уровня проверки (контракты, структура, TODO) |
| `review_code` | Ревью — контракты, нейминг, секреты |
| `project_status` | Полный health-отчёт |
| `token_savings` | Статистика экономии токенов |
| `compress_text` | Сжатие текста (3 уровня) |

---

## Что `syn init` создаёт

```
my-project/
├── AGENTS.md                     ← GRACE конституция (читается каждую сессию)
├── opencode.jsonc                ← MCP авто-старт
├── docs/                         ← Phase 0: 5 XML-шаблонов
│   ├── requirements.xml
│   ├── technology.xml
│   ├── development-plan.xml
│   ├── verification-plan.xml
│   └── knowledge-graph.xml
└── .opencode/
    ├── plugins/synapse.ts        ← авто-прокси + GRACE-контекст
    ├── rules/synapse.md          ← инструкции для LLM
    └── package.json              ← зависимости плагина
```

---

## CLI команды (диагностика)

| Команда | Назначение |
|---------|-----------|
| `syn init` | Установка всех хуков + шаблонов |
| `syn index` | Индексация кодовой базы |
| `syn index --watch` | Авто-переиндексация при изменениях |
| `syn doctor` | Диагностика: 10 проверок |
| `syn status` | Health-отчёт |
| `syn proxy -- <cmd>` | Ручной запуск через фильтр |
| `syn gain` | Статистика экономии |

---

## Что под капотом

| Компонент | Описание |
|-----------|----------|
| **Octocode** | AST-индексация (tree-sitter: 5 языков) + fallback (14 языков), BM25 поиск |
| **RTK Proxy** | 30+ TOML-фильтров, 8-стадийный пайплайн, авто-прокси через плагин |
| **Caveman** | 3 уровня сжатия текста (lite/full/ultra) |
| **GRACE** | Phase 0 gate, контракты, 3 уровня верификации, граф знаний (15 типов) |

---

## Метрики

| Метрика | Значение |
|---------|----------|
| Бинарник | ~13 MB release |
| Зависимости | 0 (всё включено) |
| MCP инструментов | 8 |
| CLI команд | 15 |
| Doctor проверок | 10 |
| Языков индексации | 5 (AST) + 9 (fallback) = 14 |
| Тестов | 20 |
| Clippy warnings | 0 |

## License

Apache 2.0
