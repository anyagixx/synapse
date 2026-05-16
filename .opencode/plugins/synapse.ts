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
      message: "Synapse loaded — GRACE methodology active",
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
      output.system.push(`## GRACE Methodology — MANDATORY DEVELOPMENT RULES

You are developing under the GRACE methodology. These rules are NOT optional.

### 1. NEVER Write Code Without a Contract
Before creating or editing any source file, it MUST have a MODULE_CONTRACT:
\`\`\`
// MODULE_CONTRACT
// MODULE_ID: M-XXX
// PURPOSE: [one sentence — what this module does]
// SCOPE: [what operations are included]
// DEPENDS: [module dependencies, comma-separated]
// LINKS: [knowledge graph references]

// START_public_api
... code ...
// END_public_api
\`\`\`
Code implements the contract. The contract is the source of truth.

### 2. Semantic Markup Is Load-Bearing
Every function/struct/class MUST be wrapped in START_/END_ blocks:
\`\`\`
// START_create_note
pub fn create_note(...) { ... }
// END_create_note
\`\`\`
These are NOT comments — they are structural anchors for verification tools.

### 3. ALWAYS Verify After Changes
After every code change, call \`verify_project\` MCP tool.
If verification FAILS: STOP and fix before continuing.
NEVER skip a failing verification step.

### 4. Review Before Committing
Before declaring work done, call \`review_code\` MCP tool (mode: scoped).
Fix all critical issues before proceeding.

### 5. Search Before Writing
Before writing new code, call \`semantic_search\` to find existing patterns.
Understand the codebase before modifying it.

### 6. Knowledge Graph Awareness
Call \`graphrag_query\` (overview) to understand module relationships.
New modules must link to existing ones in MODULE_CONTRACT.LINKS.

### 7. Top-Down Synthesis
Architecture → Contracts → Code → Tests → Verify → Review.
NEVER jump straight to code.

### 8. Governed Autonomy
You choose HOW to implement. Contracts, plans and verification define WHAT.
If a contract seems wrong — PROPOSE a change, don't silently deviate.

### 9. Stop If Unsure
If requirements are unclear: STOP and ASK the user.
If verification fails repeatedly: STOP and report what's blocking.
If architectural drift is detected: STOP and propose a plan revision.

### Available MCP Tools (use them proactively)
- semantic_search — find code by meaning
- view_signatures — understand file structure
- graphrag_query — navigate module relationships
- verify_project — run 3-level verification
- review_code — integrity review (contracts, secrets, naming)
- project_status — full health report
- token_savings — token economy analytics
- compress_text — compress text for context efficiency`)
    },
  }
}

export const server = SynapsePlugin
