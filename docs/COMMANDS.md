# Synapse CLI Commands

This page lists shipped CLI commands only.

## Setup

| Command | Description |
|---------|-------------|
| `syn init` | Install OpenCode MCP config, plugin, rules, and MyGRACE starter artifacts |
| `syn doctor` | Run local setup diagnostics |
| `syn doctor --deps` | Include Python/pip and requirements dependency diagnostics |
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
| `syn verify --profile lite` | Run MyGRACE verification with lightweight function-contract requirements |
| `syn review` | Run MyGRACE integrity review |
| `syn review --profile balanced` | Run MyGRACE review with balanced contract strictness |
| `syn refresh` | Report canonical artifact drift |
| `syn refresh --fix` | Rewrite canonical MyGRACE artifacts from source contracts |
| `syn status` | Show project health |
| `syn ci verify` | CI-friendly verification output |
| `syn ci review` | CI-friendly review output |
| `syn ci status` | CI-friendly status output |

## Runtime Utilities

| Command | Description |
|---------|-------------|
| `syn read <file>` | Read files through the token-saving proxy |
| `syn ls [args...]` | List directory contents through the token-saving proxy |
| `syn tree [args...]` | Show directory tree through the token-saving proxy |
| `syn find [args...]` | Find files through the token-saving proxy |
| `syn rg <pattern> [path...]` | Search with ripgrep through the token-saving proxy |
| `syn grep <pattern> [path...]` | Search with grep through the token-saving proxy |
| `syn git [args...]` | Run git through the token-saving proxy |
| `syn cargo [args...]` | Run cargo through the token-saving proxy |
| `syn npm [args...]` | Run npm through the token-saving proxy |
| `syn pnpm [args...]` | Run pnpm through the token-saving proxy |
| `syn npx [args...]` | Run npx through the token-saving proxy |
| `syn pytest [args...]` | Run pytest through the token-saving proxy |
| `syn json <file>` | Inspect JSON with compact values |
| `syn json --keys-only <file>` | Inspect JSON structure without printing values |
| `syn deps [path]` | Summarize dependency manifests without dumping full files |
| `syn env --filter <name>` | Show filtered environment variables with secrets masked |
| `syn wc <file>` | Count text locally with compact wc-style output |
| `syn rewrite <cmd>` | Print the hook rewrite for routeable commands, safe command chains, and pipeline left edges without executing it |
| `syn proxy -- <cmd>` | Run a shell command through the token-saving proxy |
| `syn proxy --route -- <cmd>` | Preview the selected token-saving adapter without executing the command |
| `syn gain` | Show token savings analytics |
| `syn gain --graph` | Show token savings analytics with ASCII bars |
| `syn gain --sessions --adapters` | Show session-level and adapter-level token economics |
| `syn compress <path>` | Compress files for AI context |
| `syn mcp` | Start the MCP server over stdio |
| `syn config` | Print current configuration |
| `syn config path` | Print the config file path |
| `syn config edit` | Open the config file in `$EDITOR` |
| `syn serve` | Start the local dashboard with health, token, belief-state, MentalTest, traceability, and cascade views |

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
