# Synapse hook for Cursor IDE
# Place in: .cursor/rules/ or configure as shell hook
# Cursor uses shell commands directly — this hook auto-proxies them

# To enable in Cursor:
# 1. Add to .cursor/settings.json:
#    "cursor.cpp.enableShellHooks": true
# 2. Source this file in your shell profile

# Wrapper function for common dev commands
if [ -n "$CURSOR_SESSION" ]; then
    alias git='syn proxy -- git'
    alias cargo='syn proxy -- cargo'
    alias npm='syn proxy -- npm'
    alias npx='syn proxy -- npx'
    alias pnpm='syn proxy -- pnpm'
    alias make='syn proxy -- make'
    alias go='syn proxy -- go'
fi
