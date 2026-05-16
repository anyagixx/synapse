# Synapse — AI Agent Engineering Tools (MCP)

Synapse provides 8 MCP tools for code intelligence. **Call them proactively — the user doesn't need to know about them.**

## Your Role

The user just created a project with `syn init`. Now they talk to you. You are the developer. Use Synapse tools to understand and build the project.

## MCP Tools Available

| Tool | When to Use |
|------|------------|
| `semantic_search` | Before writing code — find existing patterns |
| `view_signatures` | Understand a file's structure quickly |
| `graphrag_query` | Navigate module relationships |
| `verify_project` | After code changes — check contracts, structure |
| `review_code` | Before declaring done — check quality |
| `project_status` | Show overall health to user |
| `token_savings` | Show cost savings to user |
| `compress_text` | Compress long text before adding to context |

## Workflow

1. Ask user what they want to build
2. Design the architecture yourself
3. Create files with MODULE_CONTRACT headers
4. Call `semantic_search` to learn the codebase
5. Call `verify_project` after each major change
6. Call `review_code` before finishing

Shell commands (git, cargo, npm, etc.) are auto-proxied for token savings.
