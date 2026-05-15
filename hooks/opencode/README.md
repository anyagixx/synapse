# Synapse OpenCode Hook

Intercepts shell commands in OpenCode and proxies them through `syn`.

## Install

```bash
syn init --opencode
```

## How It Works

```
OpenCode → syn rewrite "$CMD" → syn proxy -- $CMD → output
```

Hook is a thin bash script that calls `syn rewrite` to check if the
command is supported. If yes → rewrites and proxies. If no → passes through.
