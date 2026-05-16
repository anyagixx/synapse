# Synapse hook for Claude Code
# Place in: ~/.claude/settings.json → "hooks": {"PreToolUse": [...]}
# Or source this file in shell: source hooks/adapters/claude-code.sh

# Claude Code sees Synapse MCP tools via its MCP config
# This hook adds transparent shell command proxying

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
