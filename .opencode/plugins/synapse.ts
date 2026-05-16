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
      message: "Synapse plugin loaded — proxy & code intelligence active",
      extra: { directory },
    },
  })

  return {
    "tool.execute.before": async (input, output) => {
      if (input.tool !== "bash") return
      const cmd: string | undefined = output.args?.command
      if (!cmd || !shouldProxy(cmd)) return

      // Rewrite command to pass through token-saving proxy
      output.args.command = `syn proxy -- ${cmd}`
    },

    "tool.execute.after": async (_input, output) => {
      const text = output.output
      if (!text || typeof text !== "string" || text.length < 200) return

      // Estimate compression potential
      const fillerRatio = (text.match(/\b(the|a|an|this|that|these|those|I think|I believe|Furthermore|Moreover|Additionally|However|Therefore|Thus|Consequently|In addition|In other words|It's worth noting|It should be noted|As you can see|Obviously|Essentially|Basically|Interestingly|Importantly|please note that|feel free to|don't hesitate to|let me know if|happy to help|you're welcome)\b/gi) || []).length
      const totalWords = text.split(/\s+/).length || 1
      if (fillerRatio / totalWords > 0.05) {
        output.metadata = { ...output.metadata, synapse: { compressible: true } }
      }
    },

    "experimental.chat.system.transform": async (_input, output) => {
      output.system.push(`## Synapse Development Platform

You have access to Synapse CLI for code intelligence and GRACE methodology.

### Essential Commands
| Command | Purpose |
|---------|---------|
| \`syn index\` | Index codebase for search (run first!) |
| \`syn search <q>\` | Semantic code search |
| \`syn view <file>\` | View function/class signatures |
| \`syn explain <q>\` | Explain code using indexed context |
| \`syn status\` | Project health: contracts, tests, tokens |
| \`syn verify\` | 3-level code verification |
| \`syn review\` | GRACE integrity review (contracts, secrets, naming) |
| \`syn fix <bug>\` | Debug via knowledge graph |
| \`syn proxy -- <cmd>\` | Run command through token-saving proxy |
| \`syn compress <file>\` | Compress files (creates .original.md backup) |
| \`syn gain\` | Token savings analytics |
| \`syn graphrag overview\` | Code knowledge graph overview |

### Workflow
1. Always run \`syn index\` on new projects
2. Search before coding: \`syn search "feature name"\`
3. Understand code: \`syn view path/to/file.rs\`
4. Verify before commits: \`syn verify\`
5. Review quality: \`syn review\`
6. Debug with context: \`syn fix "bug description"\`

Shell commands (git, cargo, npm, etc.) are automatically proxied through syn for token savings.`)
    },
  }
}

export const server = SynapsePlugin
