---
name: grace-explainer
description: "Complete GRACE methodology reference — how all 4 systems interlock"
---

# grace-explainer

Complete reference for the GRACE methodology — how contracts, verification, semantic markup, and knowledge graph form an integrated system.

## The Four Systems

### 1. Contracts (CDD)
**"Never write code without a contract."**
- MODULE_CONTRACT: WHAT the module does (PURPOSE, SCOPE, DEPENDS, LINKS)
- MODULE_MAP: WHAT it exports
- CHANGE_SUMMARY: WHAT changed and when
- Function contracts: INPUTS → OUTPUTS with SIDE_EFFECTS

### 2. Verification (VDD)
**"Logs are evidence, not decoration."**
- Module-local: unit tests, contract validation, block integrity
- Wave-level: cross-module integration, trace assertions
- Phase-level: full regression, graph consistency, file limits
- Failure packets: structured scenario → expected → observed → suggested

### 3. Semantic Markup
**"Markers are load-bearing structure."**
- START_X / END_X blocks: ~500 tokens each
- Unique names within file, WHAT not HOW naming
- Paired always, split when growing
- Same structure for test files

### 4. Knowledge Graph
**"Always current. Never drift."**
- M-XXX unique node per module with type, status, exports
- CrossLinks with relationship types (depends_on, imports, calls)
- Verification refs (V-M-XXX) per module
- Synced via `syn refresh`

## The Interlock

```
MODULE_CONTRACT (WHAT)
    ↕ defines
Development Plan (modules, phases, dataflows)
    ↕ verified by
Verification Plan (V-M-XXX per module)
    ↕ mapped in
Knowledge Graph (M-XXX nodes, CrossLinks)
    ↕ anchored by
Semantic Markup (START/END blocks)
```

## Development Workflow

```
1. Phase 0: Create 5 XML docs (architecture first)
2. Phase 1+: For each module:
   a. Read MODULE_CONTRACT
   b. Write code → MODULE_MAP → CHANGE_SUMMARY
   c. Add semantic blocks and function contracts
   d. verify_project → fix → re-verify
   e. review_code (scoped)
   f. Update knowledge graph
3. Phase Gate: verify (phase) + review (wave-audit) + refresh
```

## PCAM (Governed Autonomy)

- **Purpose**: MODULE_CONTRACT defines WHAT
- **Constraints**: Development plan + knowledge graph define BOUNDARIES
- **Autonomy**: You choose HOW within those boundaries
- **Metrics**: Contract OUTPUTS + verification evidence = DONE

## Available Tools

| MCP Tool | Use For |
|----------|---------|
| `semantic_search` | Find code patterns |
| `view_signatures` | Inspect file structure |
| `graphrag_query` | Navigate module relationships |
| `verify_project` | Check quality (3 levels) |
| `review_code` | Integrity review |
| `refresh_project` | Detect drift |
| `project_status` | Health overview |
| `token_savings` | Cost tracking |
| `compress_text` | Compress for context |

## CLI Commands

| Command | Use For |
|---------|---------|
| `syn init` | Install everything |
| `syn index` | Index codebase |
| `syn doctor` | 10-point diagnostic |
| `syn status` | Full health report |
| `syn refresh` | Sync artifacts with code |
| `syn verify` | Run verification |
| `syn review` | Code review |
| `syn proxy -- cmd` | Token-saving proxy |
