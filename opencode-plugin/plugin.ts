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
      message: "Synapse v0.2 loaded — MCP tools + auto-proxy active",
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
      output.system.push(`## Synapse (code intelligence platform)
Available MCP tools: semantic_search, view_signatures, graphrag_query, verify_project, review_code, project_status, token_savings, compress_text
Shell commands are auto-proxied for token savings.`)
    },
  }
}

export const server = SynapsePlugin
