#!/usr/bin/env bash
# Synapse OpenCode hook — call: synapse-rewrite "$CMD"
set -euo pipefail

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
