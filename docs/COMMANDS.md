# Synapse CLI Commands

## Project Commands

| Command | Description |
|---------|-------------|
| `syn init` | Bootstrap project |
| `syn index` | Index codebase |
| `syn status` | Project health report |

## GRACE Workflow Commands

| Command | Description |
|---------|-------------|
| `syn plan` | Generate architecture plan from requirements |
| `syn execute` | Execute development plan (write code) |
| `syn verify` | Run verification suite (3 levels) |
| `syn review` | GRACE integrity review |
| `syn fix` | Debug via knowledge graph navigation |
| `syn explain` | Explain code using indexed context |

## Search Commands

| Command | Description |
|---------|-------------|
| `syn search <query>` | Semantic code search |
| `syn view <files>` | View file signatures (functions, classes) |
| `syn grep <pattern>` | AST structural search |

## Proxy Commands

| Command | Description |
|---------|-------------|
| `syn proxy -- <cmd>` | Run command through token-saving proxy |
| `syn gain` | View token savings analytics |

## Compress Commands

| Command | Description |
|---------|-------------|
| `syn compress <file>` | Compress file for AI context |

## MCP Commands

| Command | Description |
|---------|-------------|
| `syn mcp` | Start MCP server (stdio) |
| `syn mcp --http` | Start MCP server (HTTP) |
| `syn mcp-proxy` | Start multi-repo MCP proxy |

## Utility Commands

| Command | Description |
|---------|-------------|
| `syn config` | View/edit configuration |
| `syn logs` | View MCP server logs |
| `syn telemetry` | Manage telemetry consent |
| `syn completion <shell>` | Generate shell completion |

## Global Flags

```
syn --help       Show help
syn --version    Show version
syn --quiet      Suppress output
syn --verbose    Verbose output
syn --json       JSON output format
```
