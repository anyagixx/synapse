import { definePlugin, HookContext } from "opencode/sdk";

export default definePlugin({
  name: "synapse",
  version: "0.1.0",

  hooks: {
    async beforeToolCall(ctx: HookContext) {
      const cmd = ctx.toolCall?.command;
      if (!cmd || !ctx.config.proxy_enabled) return;

      if (["git", "cargo", "npm", "npx", "pnpm", "ls", "cat", "find"].some(
        (p) => cmd.startsWith(p),
      )) {
        const proxyCmd = `syn proxy -- ${cmd}`;
        const { stdout } = await ctx.exec(proxyCmd, { timeout: 30_000 });
        ctx.toolCall.result = stdout;
      }
    },

    async afterToolCall(ctx: HookContext) {
      const output = ctx.toolCall?.result;
      if (!output || typeof output !== "string") return;

      // Caveman compression of AI responses
      ctx.toolCall.result = await ctx.exec(
        `syn compress --level ${ctx.config.compress_output || "full"}`,
        { input: output },
      );
    },

    async onProjectOpen(ctx: HookContext) {
      const projectDir = ctx.project?.path;
      if (!projectDir) return;

      // Start MCP server in background
      ctx.exec("syn mcp", {
        background: true,
        env: { PROJECT_DIR: projectDir },
      });

      // Check if indexing is needed
      const { stdout } = await ctx.exec("syn status --quick", { timeout: 5_000 });
      if (stdout.includes("not indexed")) {
        ctx.exec("syn index", { background: true });
      }
    },
  },
});
