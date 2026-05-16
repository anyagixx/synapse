# Synapse — Как построить своё приложение с помощью AI

> **Ты никогда не писал код? Это руководство для тебя.**
> Synapse + OpenCode = твой персональный AI-разработчик.

---

## Что такое Synapse?

Synapse это инструмент, который:
- **Индексирует код** — AI понимает структуру проекта
- **Экономит токены** — фильтрует вывод команд (Git, Cargo, NPM) на 60-90%
- **Сжимает текст** — экономит 65-75% токенов в ответах AI
- **Проверяет качество** — контракты, верификация, ревью
- **Строит граф знаний** — AI видит связи между файлами

Ты описываешь что хочещь — Synapse даёт AI контекст и инструменты чтобы написать код.

---

## Установка (2 минуты)

### Шаг 1: Установи Rust и OpenCode

```bash
# Rust (если ещё нет)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Перезагрузи терминал

# OpenCode — AI-кодинг ассистент
curl -fsSL https://opencode.ai/install.sh | sh
```

### Шаг 2: Установи Synapse

```bash
# Собери из исходников
git clone https://github.com/anyagixx/synapse.git
cd synapse
make install

# Проверь
syn --version
syn doctor
```

---

## Твой первый проект (5 минут)

### 1. Создай папку

```bash
mkdir my-notes-app
cd my-notes-app
```

### 2. Запусти мастер

```bash
syn init --interactive
```

Ответь на вопросы:
- **Название проекта**: `my-notes-app`
- **Что хочешь сделать?**: `Приложение для заметок с поиском`
- **Язык**: `Rust` (или Python/TypeScript/Go)

Всё! Synapse создал:
- `.opencode/rules/synapse.md` — AI знает все команды
- `.opencode/opencode.jsonc` — MCP сервер авто-стартует
- `.opencode/plugins/synapse.ts` — плагин прокси и компрессии
- `docs/requirements.xml` — твои требования
- `docs/` — шаблоны для планирования

### 3. Сгенерируй план

```bash
syn plan
```

Synapse прочитает `docs/requirements.xml` и создаст `docs/development-plan.xml` — архитектуру твоего приложения с модулями.

### 4. Создай заготовки файлов

```bash
syn execute
```

Создает файлы `.rs` с MODULE_CONTRACT заголовками в папке `src/`.

### 5. Запусти AI

```bash
opencode
```

Теперь просто скажи AI:
```
"Реализуй модули из docs/development-plan.xml. 
Начни с M-NOTES — создание и хранение заметок."
```

---

## Основные команды

### Поиск по коду
```bash
syn index                      # Индексирует проект (обязательно первым делом!)
syn search "как сохранить"     # Находит релевантный код
syn view src/main.rs           # Сигнатуры функций в файле
syn grep "struct"              # AST-поиск по структурам
```

### Проверка качества
```bash
syn verify                     # 3 уровня проверки
syn review                     # Полное ревью (контракты, секреты, стиль)
syn status                     # Отчёт о здоровье проекта
syn explain "как работает поиск"  # Объяснение кода
```

### Экономия токенов
```bash
syn proxy -- cargo build       # Прокси для любой команды
syn gain                       # Сколько токенов сэкономлено
syn compress README.md         # Сжать файл (создаёт бекап .original.md)
```

### Диагностика
```bash
syn doctor                     # Проверяет всё ли настроено
syn hooks status               # Статус интеграции с OpenCode
syn hooks install opencode     # Установить хуки
```

---

## Типичный рабочий цикл

```
1. syn index                   # Индексация
2. opencode                    # Запуск AI
   AI: "Что делаем?"
   Ты: "Добавь тёмную тему"
   
3. AI: пишет код, запускает тесты
   syn proxy -- cargo test      # Тесты через прокси (экономия токенов!)
   
4. syn verify                   # Проверка контрактов и структуры
5. syn status                   # Здоровье проекта
6. syn gain                     # Экономия за сессию

7. Повторить с шага 3
```

---

## Если что-то не работает

### AI не видит Synapse команды
```bash
syn doctor                     # Покажет что не так
syn hooks install opencode     # Переустановит хуки
syn init                       # Пересоздаст настройки
```

### Поиск ничего не находит
```bash
syn index                      # Переиндексируй проект
syn status                     # Проверь что в индексе
```

### Ошибка при запуске
```bash
syn doctor                     # Диагностика всего
cargo build                    # Проверь что проект компилируется
```

### Нет MCP инструментов в OpenCode
```bash
# Проверь что .opencode/opencode.jsonc существует
cat .opencode/opencode.jsonc
# Должен содержать:
# "mcpServers": { "synapse": { "command": "syn", "args": ["mcp"] } }
```

---

## Где что лежит

| Файл | Для чего |
|------|----------|
| `docs/requirements.xml` | Твои требования к проекту |
| `docs/development-plan.xml` | Архитектура и модули |
| `docs/verification-plan.xml` | План тестирования |
| `docs/knowledge-graph.xml` | Граф знаний |
| `.opencode/rules/synapse.md` | Инструкции для AI |
| `.opencode/opencode.jsonc` | Авто-старт MCP |
| `.opencode/plugins/synapse.ts` | Плагин |
| `~/.config/synapse/synapsec.toml` | Глобальная конфигурация |

---

## Советы

1. **Всегда индексируй**: `syn index` после клонирования или крупных изменений
2. **Используй proxy**: `syn proxy -- <команда>` для любой shell-команды — экономит токены
3. **Проверяй контракты**: `syn verify` перед коммитом
4. **Следи за экономией**: `syn gain` покажет сколько денег сэкономлено
5. **Авто-индексация**: `syn index --watch` следит за изменениями и переиндексирует

---

## Для продвинутых

```bash
# Инкрементальная индексация (следит за файлами)
syn index --watch

# AST структурный grep
syn grep "fn \w+" --rewrite "$1"

# Поиск только в Python файлах
syn search "database" --language python

# Статус в JSON для скриптов
syn status --json

# Сжать все .md файлы
syn compress docs/*.md

# Восстановить сжатые файлы
syn compress docs/*.md --restore

# Запрос к графу знаний
syn graphrag search "login"
syn graphrag overview

# MCP сервер вручную
syn mcp   # stdio режим
```

---

## Нужна помощь?

- `/help` внутри OpenCode
- `syn doctor` — диагностика проблем
- `syn explain "как работает X"` — объяснение кода
- Документация проекта: `docs/`
