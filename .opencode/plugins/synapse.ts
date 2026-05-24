import type { Plugin } from "@opencode-ai/plugin"

const PROXY_COMMANDS = [
  "git ", "cargo ", "npm ", "npx ", "pnpm ", "yarn ",
  "bun ", "deno ", "uv ", "pytest", "python -m pytest",
  "ruff ", "mypy ", "basedpyright ", "pip ", "pip3 ",
  "ls", "cat ", "find ", "grep ", "rg ", "tree ", "docker ",
  "kubectl ", "helm ", "terraform ", "tofu ", "gh ", "glab ",
  "make ", "just ", "go build", "go test", "golangci-lint ",
  "gradle ", "gradlew ", "./gradlew ", "dotnet ", "rake ", "rspec ",
  "pwd", "which ", "du ", "wc ", "journalctl ", "systemctl ",
]

function shouldProxy(cmd: string): boolean {
  const trimmed = cmd.trim()
  if (!trimmed || trimmed.includes("syn proxy --") || trimmed.startsWith("rtk ")) return false
  return PROXY_COMMANDS.some((p) => trimmed.startsWith(p) || trimmed.includes(` ${p.trim()}`))
}

function shellQuote(value: string): string {
  return `'${value.replace(/'/g, `'\\''`)}'`
}

export const SynapsePlugin: Plugin = async ({ client, $, directory }) => {
  const sessionId = process.env.SYNAPSE_SESSION_ID
    ?? process.env.OPENCODE_SESSION_ID
    ?? `opencode-${Date.now()}`

  await client.app.log({
    body: {
      service: "synapse",
      level: "info",
      message: "Synapse loaded — GRACE Phase-0 enforcement and RTK-style proxy routing active",
      extra: { directory, sessionId },
    },
  })

  return {
    "tool.execute.before": async (input, output) => {
      if (input.tool !== "bash") return
      const cmd: string | undefined = output.args?.command
      if (!cmd || !shouldProxy(cmd)) return
      output.args.command = `SYNAPSE_SESSION_ID=${shellQuote(sessionId)} syn proxy -- ${cmd}`
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
