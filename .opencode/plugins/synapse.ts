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
      output.system.push(`## GRACE Protocol — Navigation Rule #1: Start with index files

Read docs/graph-index.xml first (~30 lines). Never read full knowledge graph — use indexes + lazy-loading. Saves 96% context window.

### The Workflow
BEFORE: grace_plan → extract_belief_state → read module shard
DURING: MODULE_CONTRACT header → START_CONTRACT on functions → MODULE_MAP update
AFTER: verify_project → if FAIL fix and re-verify → review_code → grace_refresh

### Self-Check
Run \`syn verify\` after changes. If it fails — stop and fix. Don't continue with failing checks.

### Proactive tools
semantic_search | graphrag_query | verify_project | review_code | grace_plan | grace_execute | grace_status | grace_refresh | grace_fix | grace_reviewer

Shell commands auto-proxy. Read .opencode/rules/grace-mandate.md for full protocol.`)
    },
  }
}

export const server = SynapsePlugin
