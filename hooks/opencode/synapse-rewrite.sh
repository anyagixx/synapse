#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-HOOK-OPENCODE-REWRITE
# PURPOSE: OpenCode shell rewrite hook — delegates command rewrite decisions to syn rewrite
# SCOPE: syn rewrite delegation and exec handoff for supported command families
# DEPENDS: M-CLI-RTK-COMMANDS, M-PROXY
# LINKS: hooks/opencode/README.md

# START_MODULE_MAP
# main — Rewrites eligible command invocations for OpenCode
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v2.6.0 — Delegated rewrite decisions to syn rewrite]
# END_CHANGE_SUMMARY

# Synapse OpenCode hook — call: synapse-rewrite "$CMD"
set -euo pipefail

# START_CONTRACT_main
# PURPOSE: Proxy eligible OpenCode shell commands using syn rewrite and skip unknown commands
# INPUTS: { $@: command and args }
# OUTPUTS: { proxied command output or exit 1 to skip }
# SIDE_EFFECTS: execs rewritten syn proxy command for known command families
# START_main
REWRITTEN="$(syn rewrite "$@" 2>/dev/null)" || exit 1
if [[ -z "$REWRITTEN" || "$REWRITTEN" == "$*" ]]; then
    exit 1
fi
exec bash -lc "$REWRITTEN"
# END_main
