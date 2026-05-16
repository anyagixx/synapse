# Synapse — AI Agent Engineering Tools

You have access to Synapse, a local CLI tool for code intelligence, token optimization, and GRACE methodology.

## Available Commands

Run these via bash tool:

| Command | Description |
|---------|-------------|
| `syn search <query>` | Semantic code search. Use to find relevant code by meaning. |
| `syn view <file>` | View function/class signatures in a file. |
| `syn explain <query>` | Explain code using indexed context. |
| `syn status` | Project health report — contracts, tests, tokens saved. |
| `syn verify` | 3-level verification: contracts, semantic markup, 500-line rule. |
| `syn review` | GRACE integrity review — checks markup, contracts, secrets. |
| `syn proxy -- <cmd>` | Run any shell command through token-saving proxy. |
| `syn compress <file>` | Compress files for AI context (creates .original.md backup). |
| `syn gain` | How many tokens Synapse saved. |
| `syn doctor` | Diagnose setup issues — checks all components |
| `syn hooks status` | Show hook installation status |
| `syn hooks install <agent>` | Install hooks for an AI agent |

## How to Use

1. Search code before writing: `syn search "login logic"`
2. Run commands through proxy: `syn proxy -- cargo test`
3. Before major changes, run verify: `syn verify`
4. When debugging, use: `syn fix "bug description"`
5. For code quality, run: `syn review`
6. After every few changes, check: `syn status`
