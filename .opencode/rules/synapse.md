# Synapse — AI Agent Engineering Tools

You have access to Synapse MCP tools for code intelligence, quality checks, and token optimization.

## MCP Tools (call via tool_use)

| Tool | Description |
|------|-------------|
| `semantic_search` | Search codebase by meaning. Pass `query` string and optional `max_results` (default 10). |
| `view_signatures` | View function/class signatures in a file. Pass `path`. |
| `graphrag_query` | Explore code knowledge graph. Operations: `overview`, `search`, `get-node`, `get-relationships`, `find-path`. |
| `verify_project` | Run GRACE verification. Levels: `module-local`, `wave`, `phase`, `all`. |
| `review_code` | Integrity review — contracts, naming, secrets. Modes: `scoped`, `full`. |
| `project_status` | Full project health report (contracts, verification, token economy). |
| `token_savings` | Token savings analytics — commands tracked, tokens saved, estimated cost. |
| `compress_text` | Compress text for AI context. Pass `text` and optional `level` (lite/full/ultra). |

## CLI Commands (run via bash for setup/diagnostics only)

| Command | Purpose |
|---------|---------|
| `syn init --interactive` | One-time project setup (MCP, plugin, rules, templates) |
| `syn index` | Re-index the codebase |
| `syn doctor` | Diagnose setup issues — checks all components |
| `syn status` | Full project health report in terminal |
| `syn proxy -- <cmd>` | Run shell command through token-saving proxy |
| `syn gain` | View token savings statistics |
| `syn config` | Manage configuration |

## Workflow

1. Call `semantic_search` before writing code to find relevant existing code
2. Use `view_signatures` to understand file structure
3. After major changes, call `verify_project` to check contracts and structure
4. Before commits, call `review_code` for code review
5. Call `project_status` to see overall health
6. Use `compress_text` to compress verbose text before sending to context
7. Shell commands through plugin are automatically proxied for token savings
