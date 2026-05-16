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
      output.system.push(`## GRACE Methodology — PHASE GATES ENFORCED

You are developing under GRACE. These rules are HARD GATES — not suggestions.

### PHASE 0 — ARCHITECTURE (BEFORE ANY CODE)
Before writing ANY source file, these 5 docs MUST exist:
1. docs/requirements.xml — what are we building?
2. docs/technology.xml — what stack?
3. docs/development-plan.xml — modules, phases, dependencies
4. docs/verification-plan.xml — how to verify?
5. docs/knowledge-graph.xml — module relationships

**If ANY of these 5 files is missing or empty:**
→ STOP immediately
→ Ask the user what they want to build
→ Create the missing files BEFORE writing any code
→ DO NOT create source files during Phase 0

**When ALL 5 exist:** move to Phase 1.

### PHASE 1+ — IMPLEMENTATION
- Every source file STARTS with MODULE_CONTRACT
- Every function wrapped in START_/END_ blocks
- After each module: call verify_project
- After each phase: call review_code

### MCP Tools (use proactively)
semantic_search | view_signatures | graphrag_query | verify_project | review_code | project_status | token_savings | compress_text

Shell commands auto-proxied for token savings.`)
    },
  }
}

export const server = SynapsePlugin
