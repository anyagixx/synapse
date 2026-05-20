# Synapse CLI Commands

This page lists shipped CLI commands only.

## Setup

| Command | Description |
|---------|-------------|
| `syn init` | Install OpenCode MCP config, plugin, rules, and MyGRACE starter artifacts |
| `syn doctor` | Run local setup diagnostics |
| `syn hooks install` | Install Synapse hooks for supported agents |
| `syn hooks status` | Show hook installation status |

## Code Navigation

| Command | Description |
|---------|-------------|
| `syn index` | Index the current codebase |
| `syn index --watch` | Re-index when files change |
| `syn index --no-git` | Include files normally ignored by `.gitignore` |
| `syn search <query>` | Search indexed code |
| `syn view <path>` | Show indexed signatures for one or more files |
| `syn graphrag` | Build and summarize the code graph |
| `syn graphrag search <query>` | Search graph nodes |
| `syn history <query>` | Search recent git history summaries |

## MyGRACE Gates

| Command | Description |
|---------|-------------|
| `syn verify` | Run MyGRACE verification |
| `syn review` | Run MyGRACE integrity review |
| `syn refresh` | Report canonical artifact drift |
| `syn refresh --fix` | Rewrite canonical MyGRACE artifacts from source contracts |
| `syn status` | Show project health |
| `syn ci verify` | CI-friendly verification output |
| `syn ci review` | CI-friendly review output |
| `syn ci status` | CI-friendly status output |

## Runtime Utilities

| Command | Description |
|---------|-------------|
| `syn proxy -- <cmd>` | Run a shell command through the token-saving proxy |
| `syn gain` | Show token savings analytics |
| `syn gain --graph` | Show token savings analytics with ASCII bars |
| `syn compress <path>` | Compress files for AI context |
| `syn mcp` | Start the MCP server over stdio |
| `syn config` | Print current configuration |
| `syn config path` | Print the config file path |
| `syn config edit` | Open the config file in `$EDITOR` |
| `syn serve` | Start the local dashboard |

## Skills

| Command | Description |
|---------|-------------|
| `syn skills list` | List local MyGRACE workflow skills |
| `syn skills show <name>` | Show one skill definition |
| `syn skills run <name> key=value` | Run a local skill helper |

## Global Flags

```bash
syn --help
syn --version
```
