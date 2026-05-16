# Synapse hook for GitHub Copilot
# Works with Copilot CLI and Copilot Chat
# Place in: copilot-instructions.md or ~/.config/copilot/

# Copilot can use syn proxy via shell integration
# Add to your copilot instructions:

## Synapse Integration
# When running shell commands, prefix with `syn proxy --` for token savings:
# - git commands: syn proxy -- git status
# - cargo commands: syn proxy -- cargo build
# - npm commands: syn proxy -- npm test
# - Other: syn proxy -- <any command>

# For auto-proxying, add to shell profile:
# if [ -n "$COPILOT_SESSION" ]; then
#     alias git='syn proxy -- git'
#     alias cargo='syn proxy -- cargo'
#     alias npm='syn proxy -- npm'
# fi

# For VS Code tasks.json integration:
# {
#   "version": "2.0.0",
#   "tasks": [{
#     "label": "build (syn proxy)",
#     "type": "shell",
#     "command": "syn proxy -- cargo build"
#   }]
# }
