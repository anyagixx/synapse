# GRACE Protocol — Synapse v2.7.1

## Navigation Rule #1 (CRITICAL)
**ALWAYS start with index files.** Read `docs/graph-index.xml` (maps all modules), then `docs/plan-index.xml` (current phase), then the specific module shard you're working on. Never read the full knowledge graph — index files are ~30 lines. Full files waste tokens.

## Token Budget Reality
Reading full module files directly: ~5000 tokens. Reading via indexes + lazy-loading: ~200 tokens. **You save 96% of context window.** Spend saved tokens on code quality, not navigation.

---

## Six Core Principles

### 1. Never Write Code Without a Contract
Every `.rs` file starts with `// MODULE_CONTRACT`. Before generating code, know the module's PURPOSE, SCOPE, and DEPENDS. The contract is the source of truth.

### 2. Semantic Markup Is Load-Bearing Structure
`// START_CONTRACT_fnName`, `// START_MODULE_MAP`, `// START_CHANGE_SUMMARY` — these anchors let future agents (and you) navigate deterministically. Keep them paired, unique, and proportional to a working window.

### 3. Knowledge Graph Is Always Current
When you add a module, move exports, or rename dependencies — update `docs/graph-index.xml`. It's the project map. Stale maps waste everyone's tokens.

### 4. Verify Before You Declare Done
After every module change: `verify_project(level="module-local")`. If it fails — fix and re-verify. Don't continue to next module with failing checks.

### 5. Top-Down Synthesis
`grace_plan → grace_execute → code with contracts → verify_project → review_code → grace_refresh`. Never jump to code when planning or verification are unclear.

### 6. Governed Autonomy
You have freedom in HOW to implement. You do NOT have freedom to skip contracts, skip verification, or skip graph updates. Those are the rails.

---

## The Workflow (call this sequence)

**Before code:**
1. `grace_plan` or `grace_execute` — get phase/module instructions
2. `extract_belief_state(module_id)` — verify your understanding
3. Read `docs/modules/M-XXX.xml` — know the contract

**During code:**
4. Write file with `// MODULE_CONTRACT` header
5. Every public function gets `// START_CONTRACT_fnName` block
6. Update `// START_MODULE_MAP` with new exports
7. Add `// START_CHANGE_SUMMARY` entry

**After code:**
8. `verify_project(level="module-local")` — if FAIL, fix and re-verify
9. `review_code(scope="module")` — catch issues
10. `grace_refresh` — sync artifacts

---

## File Structure (← ALWAYS read first)
```
docs/
  graph-index.xml          ← maps all modules (~30 lines)
  plan-index.xml           ← current active phase
  modules/M-XXX.xml        ← per-module contracts
  phases/Phase-N.xml       ← per-phase goals
  verification/V-M-XXX.xml ← per-module checks
src/
  ... code with MODULE_CONTRACT + semantic markup ...
```

---

## Self-Check (run after changes)
```bash
syn verify                # all gates: module-local, wave, phase
syn lint                  # structural integrity
syn status                # project health
```

If `syn verify` fails — STOP and fix. Don't push broken verification.

---

## MCP Tools (call proactively)

| Tool | When |
|------|------|
| `semantic_search` | Find existing patterns before writing |
| `graphrag_query` | Navigate module relationships |
| `verify_project` | After every code change |
| `review_code` | Before declaring module done |
| `grace_execute` | Before starting any module |
| `grace_plan` | When planning new feature |
| `grace_status` | Check project health |
| `grace_refresh` | Sync canonical artifacts |

Shell commands auto-proxy through `syn proxy` for token savings.
