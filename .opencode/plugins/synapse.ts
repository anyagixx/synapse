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
      message: "Synapse loaded — MCP tools + proxy active",
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
      output.system.push(`## Synapse Tools (via MCP)
You have 8 MCP tools from Synapse:
- semantic_search — find code by meaning
- view_signatures — view file structure  
- graphrag_query — explore code knowledge graph
- verify_project — run verification checks
- review_code — code integrity review
- project_status — full health report
- token_savings — token economy analytics
- compress_text — compress text for context

Shell commands are auto-proxied for token savings.
Use these tools proactively — they help you understand and improve the codebase.`)
    },
  }
}

export const server = SynapsePlugin
