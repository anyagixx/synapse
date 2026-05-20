import type { Plugin } from "@opencode-ai/plugin"

const PROXY_COMMANDS = [
  "git ", "cargo ", "npm ", "npx ", "pnpm ", "yarn ",
  "ls", "cat ", "find ", "grep ", "tree ", "docker ",
  "make ", "go build", "go test", "pwd", "which ", "du ", "wc ",
]

function shouldProxy(cmd: string): boolean {
  return PROXY_COMMANDS.some((p) => cmd.startsWith(p) || cmd.includes(` ${p.trim()}`))
}

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
- Use native comment syntax for markers: // for Rust/TS/JS, # for Python/shell, -- for SQL
- MODULE_ID is one id only; related modules belong in DEPENDS/LINKS
- Strict profile wraps functions in contracts; lite/balanced profiles reserve function contracts for public or risky behavior
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

export const server = SynapsePlugin
