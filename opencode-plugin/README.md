# Synapse OpenCode Plugin

Integrates Synapse into OpenCode CLI.

## Features

- **before_tool_call** — proxies shell commands through `syn proxy --`
- **after_tool_call** — compresses AI responses through Caveman
- **on_project_open** — starts MCP server + triggers indexing

## Install

```bash
opencode plugins add synapse
```

## Config

```json
{
  "synapse": {
    "proxy_enabled": true,
    "compress_output": "full",
    "mcp_port": 3100
  }
}
```
