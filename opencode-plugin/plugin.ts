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
      message: "Synapse loaded — proxy compression GRACE active",
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
      output.system.push(`## Synapse Platform

Available CLI tools (use via bash):
- syn index — index codebase
- syn search <q> — semantic search  
- syn view <file> — view signatures
- syn verify — 3-level verification
- syn review — integrity review
- syn fix <bug> — debug with knowledge graph
- syn status — project health
- syn proxy -- <cmd> — token-saving proxy
- syn compress <file> — compress for context
- syn gain — token savings report
- syn graphrag overview — knowledge graph overview

Shell commands (git, cargo, npm, etc.) are auto-proxied for token savings.`)
    },
  }
}

export const server = SynapsePlugin
