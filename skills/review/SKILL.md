---
name: grace-review
description: "GRACE integrity validation — contracts, markup, graph, security"
---

# grace-review

Validate GRACE integrity across the project.

## Modes

| Mode | Scope | When |
|------|-------|------|
| `scoped` | Single module | During execution |
| `wave-audit` | Wave modules | After wave completion |
| `full` | Entire project | Phase boundaries |

## Checks

1. Semantic markup integrity — all START/END pairs match
2. Contract compliance — code implements what contract promises
3. Verification integrity — tests exist for all claims
4. Graph consistency — knowledge-graph.xml matches actual modules
5. Naming conventions — files, functions, variables follow standards
6. Security — no API keys, passwords, tokens committed
7. 500-line rule — no artifact exceeds limit
