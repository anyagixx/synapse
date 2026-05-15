# Synapse

> **Unified AI Agent Engineering Platform**
> Code intelligence + Token proxy + Communication compression + GRACE methodology
> *Для разработчиков и тех, кто никогда не писал код.*

---

## Non-Developer Quickstart (ты никогда не писал код — это для тебя)

Synapse + OpenCode превращают твою идею в готовое приложение. Ты просто говоришь, что хочешь — AI пишет код, тестирует, чинит ошибки.

### 1. Установи

```bash
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/main/install.sh | sh
syn --version
```

### 2. Создай проект и запусти AI

```bash
mkdir my-pet-project
cd my-pet-project
syn init
opencode
```

### 3. Начни разработку — просто говори AI что хочешь

Внутри OpenCode:

```
Ты: "Я хочу сделать приложение для заметок.
     Нужно: создавать, редактировать, удалять, искать по тексту."

AI: "Я спроектирую архитектуру... Модули: M-NOTES, M-SEARCH, M-STORAGE.
     Начинаем?"

Ты: "Да"
```

AI сам:
- Спроектирует архитектуру (`syn plan`)
- Напишет код с контрактами (`syn execute`)
- Протестирует (`syn verify`)
- Исправит ошибки (`syn fix` если что-то пошло не так)

### 4. Следи за прогрессом

```bash
syn status    # Сколько модулей готово, тесты, здоровье
syn gain      # Сколько токенов сэкономлено
syn explain   # "как работает поиск?" — AI объяснит
```

### Что ты НИКОГДА не делаешь

| ❌ Никогда | ✅ Вместо этого |
|-----------|----------------|
| Писать код | Опиши что хочешь |
| Читать документацию | Спроси AI |
| Дебажить ошибки | Скажи "почини: заметки не сохраняются" |
| Запускать тесты | `syn verify` сделает сам |
| Настраивать сборку | `syn init` настроит всё |
| Отслеживать прогресс | `syn status` покажет |

---

## Для разработчиков

Synapse объединяет четыре мощных инструмента в один CLI-бинарник:

- **Octocode** — AST-индексация, семантический поиск, GraphRAG, MCP сервер
- **RTK** — прокси-фильтрация вывода shell-команд (экономия 60-90% токенов)
- **Caveman** — сжатие ответов AI и input-файлов (экономия 65-75% токенов)
- **GRACE** — строгая методология: контракты, верификация, knowledge graph

### Установка

```bash
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/main/install.sh | sh

# Или через Cargo:
cargo install --git https://github.com/anyagixx/synapse
```

### Быстрый старт

```bash
syn init              # Создать проект
syn index             # Проиндексировать код
syn mcp               # Запустить MCP для OpenCode
syn proxy -- cargo check  # Запустить через экономитель токенов
syn compress README.md    # Сжать файл для AI
syn gain              # Статистика экономии
```

### Архитектура

```
┌──────────────────────────────────────────────────┐
│ GRACE Methodology (contracts, verification, graph)│
├──────────────────────────────────────────────────┤
│ Caveman (AI output/input compression)             │
├──────────────────────────────────────────────────┤
│ RTK Proxy (shell command filtering)               │
├──────────────────────────────────────────────────┤
│ Octocode (AST indexing, GraphRAG, search, MCP)    │
├──────────────────────────────────────────────────┤
│ Rust Core (single binary — 13MB, 0 зависимостей)  │
└──────────────────────────────────────────────────┘
```

### 21 CLI команд

| Команда | Назначение |
|---------|-----------|
| `init` | Создать проект |
| `index` | Индексировать кодбазу |
| `search` | Семантический поиск |
| `view` | Сигнатуры функций |
| `plan` | Спроектировать архитектуру |
| `execute` | Написать код по плану |
| `verify` | 3 уровня проверок |
| `review` | GRACE integrity review |
| `fix` | Поиск и исправление багов |
| `status` | Health report |
| `explain` | Q&A по коду |
| `proxy` | Запуск команды через экономитель |
| `gain` | Статистика экономии токенов |
| `compress` | Сжатие файлов |
| `graphrag` | Запросы к графу знаний |
| `mcp` | MCP сервер |
| `config` | Настройки |
| `logs` | Логи MCP |
| `telemetry` | Телеметрия |
| `completion` | Автодополнение |

### 4 MCP Tools (для AI-агентов)

- `semantic_search` — поиск по коду
- `view_signatures` — сигнатуры функций
- `graphrag_query` — граф знаний (search, get-node, find-path, overview)
- `lsp_*` — LSP инструменты (go-to-def, hover, references)

### Документация

Каждый файл ≤500 строк — AI читает за один контекст.

| Файл | О чём |
|------|-------|
| `docs/QUICKSTART.md` | Для non-developer |
| `docs/COMMANDS.md` | Все команды |
| `docs/WORKFLOW.md` | GRACE workflow |
| `docs/FAQ.md` | Вопросы и ответы |

### GRACE Skills (6 скиллов)

`skills/plan`, `execute`, `verify`, `review`, `fix`, `status`

### Репозиторий

```bash
git clone https://github.com/anyagixx/synapse.git
cd synapse
cargo build --release
./target/release/synapse --help
```

### Интеграция с OpenCode

```bash
opencode plugins add ./opencode-plugin
# Или после публикации:
opencode plugins add synapse
```

### Метрики

| Метрика | Значение |
|---------|----------|
| Binary | 13MB release |
| Зависимости | 0 (Rust only) |
| CLI команд | 21 |
| MCP tools | 4 |
| GRACE skills | 6 |
| Warnings | 0 |
| Фильтров proxy | 30+ |
| Языков индексации | 5 (tree-sitter) + fallback |
| Типов GraphRAG | 15 |

## License

Apache 2.0
