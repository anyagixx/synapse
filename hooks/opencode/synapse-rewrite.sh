#!/usr/bin/env bash
# MODULE_CONTRACT
# MODULE_ID: M-HOOK-OPENCODE-REWRITE
# PURPOSE: OpenCode shell rewrite hook — proxies known tool commands through syn proxy
# SCOPE: Command family matching and exec handoff to syn proxy
# DEPENDS: M-PROXY
# LINKS: hooks/opencode/README.md

# START_MODULE_MAP
# main — Rewrites eligible command invocations for OpenCode
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v2.5.0 — Added MyGRACE contract]
# END_CHANGE_SUMMARY

# Synapse OpenCode hook — call: synapse-rewrite "$CMD"
set -euo pipefail

# START_CONTRACT_main
# PURPOSE: Proxy eligible OpenCode shell commands and skip unknown commands
# INPUTS: { $@: command and args }
# OUTPUTS: { proxied command output or exit 1 to skip }
# SIDE_EFFECTS: execs syn proxy for known command families
# START_main
CMD="$*"

# Only rewrite known tool commands
case "$CMD" in
    git\ *|cargo\ *|npm\ *|npx\ *|pnpm\ *|ls\ *|cat\ *|find\ *|grep\ *|docker\ *|aws\ *)
        exec syn proxy -- "$@"
        ;;
    *)
        # Unknown command → pass through (exit 1 tells hook to skip)
        exit 1
        ;;
esac
# END_main
