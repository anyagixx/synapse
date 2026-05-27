// MODULE_CONTRACT
// MODULE_ID: M-PLUGIN
// PURPOSE: OpenCode plugin — injects Synapse proxy behavior and GRACE system context
// SCOPE: Proxy command detection, OpenCode tool.execute.before hook, system prompt transform
// DEPENDS: M-PROXY, M-MCP, M-GRACE
// LINKS: opencode-plugin/plugin.json, opencode.jsonc

// START_MODULE_MAP
// shouldProxy — Detects shell commands that should be routed through syn proxy
// SynapsePlugin — OpenCode plugin factory that registers proxy and system transforms
// server — OpenCode plugin export alias
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.5.0 — Added MyGRACE contract]
// END_CHANGE_SUMMARY

import type { Plugin } from "@opencode-ai/plugin"

const PROXY_COMMANDS = [
  "git ", "cargo ", "npm ", "npx ", "pnpm ", "yarn ",
  "ls", "cat ", "find ", "grep ", "tree ", "docker ",
  "make ", "go build", "go test", "pwd", "which ", "du ", "wc ",
]

// START_CONTRACT_shouldProxy
// PURPOSE: Decide whether a shell command should be proxied through Synapse
// INPUTS: { cmd: string — shell command }
// OUTPUTS: { boolean }
// START_shouldProxy
function shouldProxy(cmd: string): boolean {
  return PROXY_COMMANDS.some((p) => cmd.startsWith(p) || cmd.includes(` ${p.trim()}`))
}
// END_shouldProxy

// START_CONTRACT_SynapsePlugin
// PURPOSE: Register Synapse OpenCode hooks for command proxying and GRACE prompt context
// INPUTS: { client: OpenCode client }, { $: runtime helper }, { directory: string }
// OUTPUTS: { Plugin hook object }
// SIDE_EFFECTS: logs plugin load and mutates eligible bash commands
// START_SynapsePlugin
export const SynapsePlugin: Plugin = async ({ client, $, directory }) => {
  await client.app.log({
    body: {
      service: "synapse",
      level: "info",
      message: "Synapse loaded — GRACE Phase-0 enforcement active",
      extra: { directory },
    },
  })

  return {
    "tool.execute.before": async (input, output) => {
      if (input.tool !== "bash") return
      const cmd: string | undefined = output.args?.command
      if (!cmd || !shouldProxy(cmd)) return
      output.args.command = `syn proxy -- ${cmd}`
    },

    "experimental.chat.system.transform": async (_input, output) => {
      output.system.push(`## GRACE Methodology — SHARDED PHASE GATES ENFORCED

You are developing under GRACE. These rules are HARD GATES — not suggestions.

### PHASE 0 — ARCHITECTURE (BEFORE ANY CODE)
Before writing ANY source file, sharded architecture artifacts MUST exist:
- docs/graph-index.xml
- docs/plan-index.xml
- docs/verification-index.xml
- docs/modules/
- docs/phases/
- docs/verification/

Compatibility docs may also exist under docs/*.xml, but sharded indexes are primary source of truth.

If primary sharded artifacts are missing:
→ STOP immediately
→ Ask user what they want to build
→ Create missing artifacts BEFORE writing any code
→ DO NOT create source files during Phase 0

### PHASE 1+ — IMPLEMENTATION
- Every source file STARTS with MODULE_CONTRACT
- Every function wrapped in START_/END_ blocks
- After each module: call verify_project
- After each phase: call review_code
- Use grace_* tools for workflow-level planning, execution, lint, review, refresh, and status

### MCP Tools
Core: semantic_search | view_signatures | graphrag_query | verify_project | review_code | refresh_project | suggest_contract | project_status | token_savings | compress_text | lsp_hover | lsp_references
GRACE: grace_init | grace_plan | grace_verification | grace_execute | grace_multiagent_execute | grace_reviewer | grace_refresh | grace_refactor | grace_fix | grace_status | grace_ask | grace_explainer | grace_cli | grace_setup_subagents | grace_lint

Shell commands auto-proxied for token savings.`)
    },
  }
}
// END_SynapsePlugin

export const server = SynapsePlugin
