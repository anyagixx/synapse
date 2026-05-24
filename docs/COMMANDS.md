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
| `syn hooks audit --json` | Audit OpenCode hook trust markers and MCP/rewrite/session wiring |

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
| `syn gh [args...]` | Run GitHub CLI through the token-saving proxy |
| `syn glab [args...]` | Run GitLab CLI through the token-saving proxy |
| `syn aws [args...]` | Run AWS CLI through the token-saving proxy |
| `syn psql [args...]` | Run psql through the token-saving proxy |
| `syn curl [args...]` | Run curl through the token-saving proxy |
| `syn wget [args...]` | Run wget through the token-saving proxy |
| `syn jq [args...]` | Run jq through the token-saving proxy |
| `syn go [args...]` | Run Go tooling through the token-saving proxy |
| `syn golangci [args...]` | Run golangci-lint through the token-saving proxy |
| `syn dotnet [args...]` | Run dotnet through the token-saving proxy |
| `syn rake [args...]` | Run rake through the token-saving proxy |
| `syn rspec [args...]` | Run rspec through the token-saving proxy |
| `syn rubocop [args...]` | Run rubocop through the token-saving proxy |
| `syn gradle [args...]` | Run Gradle through the token-saving proxy |
| `syn gradlew [args...]` | Run local Gradle wrapper through the token-saving proxy |
| `syn make [args...]` | Run make through the token-saving proxy |
| `syn just [args...]` | Run just through the token-saving proxy |
| `syn helm [args...]` | Run Helm through the token-saving proxy |
| `syn kubectl [args...]` | Run kubectl through the token-saving proxy |
| `syn docker [args...]` | Run Docker through the token-saving proxy |
| `syn podman [args...]` | Run Podman through the token-saving proxy |
| `syn ruff [args...]` | Run ruff through the token-saving proxy |
| `syn mypy [args...]` | Run mypy through the token-saving proxy |
| `syn basedpyright [args...]` | Run basedpyright through the token-saving proxy |
| `syn pip [args...]` | Run pip through the token-saving proxy |
| `syn uv [args...]` | Run uv through the token-saving proxy |
| `syn next [args...]` | Run Next.js tooling through the token-saving proxy |
| `syn playwright [args...]` | Run Playwright through the token-saving proxy |
| `syn prettier [args...]` | Run Prettier through the token-saving proxy |
| `syn prisma [args...]` | Run Prisma through the token-saving proxy |
| `syn tsc [args...]` | Run TypeScript compiler through the token-saving proxy |
| `syn vitest [args...]` | Run Vitest through the token-saving proxy |
| `syn json <file>` | Inspect JSON with compact values |
| `syn json --keys-only <file>` | Inspect JSON structure without printing values |
| `syn deps [path]` | Summarize dependency manifests without dumping full files |
| `syn env --filter <name>` | Show filtered environment variables with secrets masked |
| `syn wc <file>` | Count text locally with compact wc-style output |
| `syn pipe --filter <name>` | Filter stdin through Synapse RTK filters |
| `syn log <file>` | Deduplicate and summarize log output |
| `syn smart <file>` | Summarize source file structure without printing full code |
| `syn discover [cmd...]` | Discover routeable token-heavy commands and Synapse replacements |
| `syn discover --json --limit 10` | Render bounded discovery diagnostics as JSON |
| `syn learn` | Show bounded RTK learning guidance for recurring misses |
| `syn learn --json` | Render bounded RTK learning guidance as JSON |
| `syn rtk-parity --ci` | Check RTK parity inventory for CI |
| `syn rewrite <cmd>` | Print the hook rewrite for routeable commands, safe command chains, pipeline left edges, safe fd redirects, and shell prefixes without executing it |
| `syn proxy -- <cmd>` | Run a shell command through the token-saving proxy |
| `syn proxy --route -- <cmd>` | Preview the selected token-saving adapter without executing the command |
| `syn filters verify --filter <name>` | Run inline tests for project, user, or built-in RTK-style proxy filters |
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
