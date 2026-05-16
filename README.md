# Synapse

> **AI Agent Engineering Platform — works transparently through OpenCode CLI**
> Code intelligence + Token proxy + Compression + GRACE methodology
> *8 MCP tools for LLMs. Zero cognitive overhead for humans.*

---

## Как это работает

```
Ты → OpenCode CLI → LLM (принимает решения)
                        │
          ┌─────────────┼─────────────┐
          ▼             ▼             ▼
     MCP Tools       Plugin         Rules
   (8 инструментов)  (прозрачный    (инструкции
                     proxy)         для LLM)
          │
          ▼
      Synapse (фоновый движок)
```

Ты просто общаешься с AI через `opencode`. Synapse невидимо:
- Даёт LLM 8 MCP-инструментов для поиска и проверки кода
- Автоматически фильтрует вывод shell-команд (экономия 60-90% токенов)
- Сжимает ответы (экономия 65-75% токенов)

**Тебе не нужно знать команды Synapse. LLM сама решает когда их вызывать.**

---

## Установка

```bash
# Установи Rust если нет
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Установи OpenCode
curl -fsSL https://opencode.ai/install.sh | sh

# Установи Synapse
git clone https://github.com/anyagixx/synapse.git
cd synapse && make install
```

---

## Быстрый старт

```bash
# 1. Создай пустую папку
mkdir my-project && cd my-project

# 2. Установи хуки Synapse (один раз)
syn init

# 3. Запусти AI
opencode

# Всё! LLM сама спросит что делать, спроектирует, напишет код.
# Synapse работает невидимо — даёт LLM 8 MCP инструментов.
```

---

## 8 MCP Tools (LLM вызывает сама)

| Инструмент | Что делает |
|-----------|-----------|
| `semantic_search` | Поиск по коду с BM25-релевантностью (14 языков) |
| `view_signatures` | Сигнатуры функций и классов в файле |
| `graphrag_query` | Граф знаний — узлы, связи, пути между модулями |
| `verify_project` | 3 уровня проверки (контракты, структура, TODO) |
| `review_code` | Ревью — контракты, нейминг, секреты |
| `project_status` | Полный health-отчёт проекта |
| `token_savings` | Статистика экономии токенов |
| `compress_text` | Сжатие текста для AI-контекста (3 уровня) |

---

## CLI команды (для диагностики)

| Команда | Назначение |
|---------|-----------|
| `syn init --interactive` | Одноразовая настройка проекта |
| `syn index` | Переиндексация кодовой базы |
| `syn doctor` | Диагностика всех компонентов |
| `syn status` | Health-отчёт в терминале |
| `syn proxy -- <cmd>` | Запуск команды через фильтр |
| `syn gain` | Статистика экономии |
| `syn config` | Управление конфигурацией |

---

## Что под капотом

| Компонент | Описание |
|-----------|----------|
| **Octocode** | AST-индексация (tree-sitter: 5 языков) + fallback (14 языков), BM25 поиск с токенизацией |
| **RTK Proxy** | 30+ TOML-фильтров shell-команд с 6-стадийным пайплайном |
| **Caveman** | 3 уровня сжатия текста (lite/full/ultra) |
| **GRACE** | Методология: контракты, 3 уровня верификации, граф знаний (15 типов отношений) |

---

## Метрики

| Метрика | Значение |
|---------|----------|
| Бинарник | ~13 MB release |
| Зависимости | 0 (всё включено) |
| MCP инструментов | 8 |
| CLI команд | 15 |
| Языков индексации | 5 (AST) + 9 (fallback) = 14 |
| Тестов | 20 |
| Clippy warnings | 0 |

## License

Apache 2.0
