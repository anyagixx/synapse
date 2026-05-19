# MODULE_CONTRACT
# MODULE_ID: M-HOOK-CURSOR
# PURPOSE: Cursor shell hook — aliases common developer commands through syn proxy when Cursor session is active
# SCOPE: Cursor shell alias setup for token-saving command proxying
# DEPENDS: M-PROXY
# LINKS: hooks/adapters/README.md

# START_MODULE_MAP
# main — Installs aliases when CURSOR_SESSION is present
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v2.5.0 — Added MyGRACE contract]
# END_CHANGE_SUMMARY

# Synapse hook for Cursor IDE
# Place in: .cursor/rules/ or configure as shell hook
# Cursor uses shell commands directly — this hook auto-proxies them

# To enable in Cursor:
# 1. Add to .cursor/settings.json:
#    "cursor.cpp.enableShellHooks": true
# 2. Source this file in your shell profile

# START_CONTRACT_main
# PURPOSE: Configure Cursor shell command aliases for Synapse proxying
# OUTPUTS: { shell aliases when CURSOR_SESSION is set }
# SIDE_EFFECTS: defines aliases in current shell
# START_main
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
# END_main
