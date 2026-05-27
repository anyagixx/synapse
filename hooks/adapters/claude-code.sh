# MODULE_CONTRACT
# MODULE_ID: M-HOOK-CLAUDE
# PURPOSE: Claude Code shell hook — routes common development commands through syn proxy
# SCOPE: syn_claude_proxy wrapper and Claude Code hook example
# DEPENDS: M-PROXY
# LINKS: hooks/adapters/README.md

# START_MODULE_MAP
# syn_claude_proxy — Proxies known command families and falls back to raw execution
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v2.5.0 — Added MyGRACE contract]
# END_CHANGE_SUMMARY

# Synapse hook for Claude Code
# Place in: ~/.claude/settings.json → "hooks": {"PreToolUse": [...]}
# Or source this file in shell: source hooks/adapters/claude-code.sh

# Claude Code sees Synapse MCP tools via its MCP config
# This hook adds transparent shell command proxying

# START_CONTRACT_syn_claude_proxy
# PURPOSE: Proxy known command families through Synapse while preserving fallback execution
# INPUTS: { $@: command and args }
# OUTPUTS: { command output }
# SIDE_EFFECTS: executes external command
# START_syn_claude_proxy
syn_claude_proxy() {
    local cmd="$1"
    case "$cmd" in
        git|git\ *|cargo|cargo\ *|npm|npm\ *|npx|npx\ *|ls|ls\ *|\
        cat|cat\ *|find|find\ *|grep|grep\ *|tree|tree\ *|docker|docker\ *|\
        make|make\ *|go\ build|go\ test|pwd|which|du|wc)
            syn proxy -- "$@" 2>/dev/null || "$@"
            ;;
        *)
            "$@"
            ;;
    esac
}
# END_syn_claude_proxy

# To use with Claude Code settings.json:
# {
#   "hooks": {
#     "PreToolUse": [
#       {
#         "matcher": "Bash",
#         "hooks": [{
#           "type": "command",
#           "command": "syn proxy -- $CLAUDE_TOOL_INPUT"
#         }]
#       }
#     ]
#   }
# }
