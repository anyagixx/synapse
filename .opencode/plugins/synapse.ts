import type { Plugin } from "@opencode-ai/plugin"

function shellQuote(value: string): string {
  return `'${value.replace(/'/g, `'\\''`)}'`
}

export const SynapsePlugin: Plugin = async ({ client, $, directory }) => {
  let canRewrite = true
  try {
    await $`which syn`.quiet()
  } catch {
    console.warn("[synapse] syn binary not found in PATH — plugin proxy rewrite disabled")
    canRewrite = false
  }

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
      if (!cmd || !canRewrite) return
      try {
        const result = await $`syn rewrite ${cmd}`.quiet().nothrow()
        const rewritten = String(result.stdout).trim()
        if (rewritten && rewritten !== cmd) {
          output.args.command = `SYNAPSE_SESSION_ID=${shellQuote(sessionId)} ${rewritten}`
        }
      } catch {
        // syn rewrite failed; pass through unchanged.
      }
    },

    "experimental.chat.system.transform": async (_input, output) => {
      output.system.unshift(`## Navigation Rule #1 (CRITICAL): ALWAYS start with index files

Read docs/graph-index.xml first (~30 lines), then plan-index.xml, then module shard. Never read full knowledge graph — indexes + lazy-loading save 96% context window.

## GRACE Protocol — How We Build Software Here

### 1. Never Write Code Without a Contract
Every .rs file starts with // MODULE_CONTRACT. Know PURPOSE, SCOPE, DEPENDS before writing.

### 2. Semantic Markup Is Load-Bearing
// START_CONTRACT_fnName and // START_MODULE_MAP blocks let future agents navigate. Keep them paired.

### 3. Knowledge Graph Is Always Current
Update docs/graph-index.xml when adding modules or changing dependencies.

### 4. Verify Before Declaring Done
After every module: verify_project → if FAIL, fix and re-verify. Don't continue with failing checks.

### 5. Top-Down: Plan → Execute → Verify → Review → Refresh
grace_plan → grace_execute → code with contracts → verify_project → review_code → grace_refresh

### 6. Governed Autonomy
You choose HOW to implement. You do NOT choose to skip contracts, skip verification, or skip graph updates.

## Self-Check
Run syn verify. If it fails — stop and fix. Read .opencode/rules/grace-mandate.md for full protocol.`)
    },
  }
}

export const server = SynapsePlugin
