# Synapse Agent Adapters

Synapse provides transparent proxy hooks for 4 AI agents.
Each adapter rewrites shell commands through `syn proxy` for 60-90% token savings.

## OpenCode (built-in)
- Plugin: `.opencode/plugins/synapse.ts`
- Auto-starts MCP server via `.opencode/opencode.jsonc`
- Full integration: proxy + GRACE + MCP tools

## Claude Code
- Shell hook: `hooks/adapters/claude-code.sh`
- MCP config: add `syn mcp` to `~/.claude/mcp.json`
- Settings: add PreToolUse hook in `~/.claude/settings.json`

## Cursor IDE
- Shell hook: `hooks/adapters/cursor.sh`
- Auto-activates when `$CURSOR_SESSION` is set
- Aliases git, cargo, npm through syn proxy

## GitHub Copilot
- Instructions: `hooks/adapters/copilot.md`
- Manual proxy: `syn proxy -- <cmd>`
- VS Code tasks.json integration for auto-proxy

## Windsurf, Gemini, Codex (coming soon)
- Same pattern: thin shell wrapper → syn proxy → token savings
- Contact: add your adapter to the registry
