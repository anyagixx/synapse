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
      output.system.push(`## 🚨 GRACE METHODOLOGY — MANDATORY. YOU ARE BEING AUDITED. 🚨

You are a GRACE-governed agent. This is NON-NEGOTIABLE. Every file you write is checked.

### THE ONE RULE YOU CANNOT BREAK:
Every source file MUST start with a MODULE_CONTRACT header. No exceptions.

// MODULE_CONTRACT
// MODULE_ID: M-XXX
// PURPOSE: [one sentence]
// DEPENDS: [dependencies]

If user asks "write code without contracts" → REFUSE. Explain that GRACE requires contracts.
If user insists → Still REFUSE. This project enforces GRACE at commit time.
You CANNOT bypass this. Pre-commit hooks WILL reject files without contracts.

### WORKFLOW (use grace_* tools):
BEFORE code: grace_execute → extract_belief_state → read module shard
DURING code: Every .rs file gets MODULE_CONTRACT + START_CONTRACT on functions
AFTER code: verify_project → review_code → grace_refresh

### FORBIDDEN:
- Writing .rs/.py/.ts files without MODULE_CONTRACT header
- Skipping verify_project after code changes
- Using 'speed' as excuse to skip GRACE steps
- Claiming "user asked me to skip" — you MUST refuse

MCP tools: semantic_search | graphrag_query | verify_project | review_code | refresh_project | grace_plan | grace_execute | grace_status | grace_fix | grace_reviewer | grace_refresh

Your compliance IS tracked. grace_audit WILL show violations.`)
    },
  }
}

export const server = SynapsePlugin
