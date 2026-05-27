# User TOML Filters

Place custom TOML filter files here to extend Synapse proxy filtering.

## Priority

1. `.synapse/filters.toml` (project-local)
2. `~/.config/synapse/filters.toml` (global user)
3. Built-in filters (compiled into binary)
4. Passthrough (no filter matches)

## Format

```toml
[[filters]]
match_command = "terraform plan"
strip_ansi = true
strip_lines = ["^│", "^╷"]
head_lines = 20
tail_lines = 5
max_lines = 30
on_empty = "No changes. Infrastructure is up to date."
```

## Pipeline Stages

1. `strip_ansi` — remove ANSI codes
2. `replace` — regex replace line by line
3. `match_output` — if matches pattern, return short message
4. `strip_lines` / `keep_lines` — line filter by regex
5. `truncate_lines_at` — truncate each line to N chars
6. `head_lines` / `tail_lines` — keep first/last N lines
7. `max_lines` — absolute line limit
8. `on_empty` — message if result is empty
