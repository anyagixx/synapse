# GRACE Methodology — Project Constitution

> **These rules are mandatory. Architecture first. Code second. Never the other way around.**

---

## Phase 0 — Architecture Artifacts (MANDATORY BEFORE ANY CODE)

**You are in Phase 0 until primary sharded architecture artifacts exist in `docs/`. You MAY NOT write source code in Phase 0.**

Create files in this exact order:

### 1. `docs/graph-index.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<GRAPH_INDEX>
  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY></META>
  <MODULES><MODULE id="M-CORE" path="docs/modules/M-CORE.xml" status="planned" /></MODULES>
  <RELATIONSHIPS></RELATIONSHIPS>
</GRAPH_INDEX>
```

### 2. `docs/plan-index.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<PLAN_INDEX>
  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY><ACTIVE_PHASE>Phase-0</ACTIVE_PHASE></META>
  <PHASES><PHASE id="Phase-0" path="docs/phases/Phase-0.xml" status="active" /></PHASES>
</PLAN_INDEX>
```

### 3. `docs/verification-index.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<VERIFICATION_INDEX>
  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY></META>
  <VERIFICATIONS><VERIFICATION id="V-M-CORE" module="M-CORE" path="docs/verification/V-M-CORE.xml" priority="critical" status="planned" /></VERIFICATIONS>
</VERIFICATION_INDEX>
```

### 4. `docs/modules/M-XXX.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<MODULE id="M-CORE" type="CORE_LOGIC" status="planned">
  <NAME>Core Module</NAME>
  <PURPOSE>Core application logic</PURPOSE>
  <FILES><FILE>src/core.rs</FILE></FILES>
  <VERIFICATION_REF>V-M-CORE</VERIFICATION_REF>
</MODULE>
```

### 5. `docs/phases/Phase-N.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<PHASE id="Phase-1" status="planned">
  <NAME>Foundation</NAME>
  <GOAL>Implement initial core modules with contracts and verification</GOAL>
  <MODULE_REFS><MODULE_REF id="M-CORE" /></MODULE_REFS>
</PHASE>
```

### 6. `docs/verification/V-M-XXX.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<VERIFICATION id="V-M-CORE" module="M-CORE" priority="critical" status="planned">
  <UNIT_TESTS></UNIT_TESTS>
  <REQUIRED_LOG_MARKERS></REQUIRED_LOG_MARKERS>
  <TRACE_ASSERTIONS></TRACE_ASSERTIONS>
  <PHASE_GATE>Phase-1</PHASE_GATE>
</VERIFICATION>
```

Compatibility docs may also exist under `docs/*.xml`, but sharded indexes are primary source of truth.
`docs/requirements.xml` must be a complete `<RequirementsAnalysis>` artifact with Goals, DomainModel entities, Actors, AAG UseCases (`Actor` + `Action` + `Goal`), NonFunctionalRequirements, Constraints, and Glossary before implementation starts. Use `generate_requirements` when it is missing or still a stub.
`docs/technology.xml` must be a complete `<Technology>` artifact with exact Language/Runtime/PackageManager versions, versioned dependencies, DependencyMatrix compatibility checks, KnownIssues, and DevOps notes. Use `generate_technology` when versions are missing, blank, `latest`, wildcard, or range-only.
`docs/development-plan.xml` must be a complete `<DevelopmentPlan>` artifact with ArchitectureGraph, DataFlows, GenerationOrder, MentalTests, NonHumanPatterns, and ContractGuidelines. Use `generate_development_plan` when DataFlows, GenerationOrder, MentalTests, or NonHumanPatterns are missing. Use `mental_test_run` before code generation for critical or complex modules.
Agent-based testing uses natural-language guides under `docs/tests/guides/` and tester-agent reports under `docs/tests/results/`. Testing guides are Markdown, not code: `## Test: ...`, `### Steps`, `### Expected Behavior`, and `### Data to Capture`. Use `run_test_guide` after a module is code-complete and `submit_test_report` when a failure XML contains LOG evidence for the developer agent.

### Phase 0 STOP Gates
- If `docs/graph-index.xml` is missing → **STOP. Ask user what to build.**
- If `docs/plan-index.xml` is missing → **STOP. Define phases and execution order.**
- If `docs/verification-index.xml` is missing → **STOP. Define verification structure.**
- If `docs/modules/` has no module shard → **STOP. Define module architecture.**
- If `docs/phases/` has no phase shard → **STOP. Define delivery phases.**
- If `docs/verification/` has no verification shard → **STOP. Define per-module verification.**
- If ANY primary sharded artifact is missing → **DO NOT write source code.**
- Only when sharded model exists → proceed to Phase 1.

---

## Phase 1+ — Implementation

### Every source file MUST have this structure:

Use the file's native comment syntax: Rust/TypeScript/JavaScript `//`, Python/shell `#`, SQL `--`.
`MODULE_ID` is exactly one id (`M-XXX`); put related modules in `DEPENDS` or `LINKS`, never as comma-separated `MODULE_ID` values.

```
// MODULE_CONTRACT
// MODULE_ID: M-XXX
// PURPOSE: [one sentence — what this module does]
// SCOPE: [what operations are included]
// DEPENDS: [module dependencies]
// LINKS:
//   → M-STORAGE (depends) — runtime dependency
//   ← V-M-XXX (verified_by) — verification shard

// START_MODULE_MAP
// create_note — Creates and persists a new note
// search_notes — Returns notes matching query text
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Initial implementation]
// END_CHANGE_SUMMARY

// START_public_api
... module-level code ...
// END_public_api
```

### Function contracts

Strict profile requires function contracts for functions. For small bots, scripts, migrations, and helpers, use `profile=lite` or `profile=balanced`; then module-level contracts are enough for small/non-critical files and function contracts are reserved for public or risky behavior.

Rust/TypeScript example:
```
// START_CONTRACT_create_note
// PURPOSE: Create a new note and persist to storage
// INPUTS: { title: String — note title }, { content: String — note body }
// OUTPUTS: { Note — the created note with id assigned }
// SIDE_EFFECTS: writes to database, emits [Core][create_note][CREATE] log
// LINKS:
//   → M-STORAGE (uses) — note persistence
//   ← V-M-CORE (verified_by) — core verification
// START_create_note
pub fn create_note(title: &str, content: &str) -> Note { ... }
// END_create_note
```

### Typed LINKS

Prefer directional typed LINKS:

```
// LINKS:
//   → M-STORAGE (depends) — user data persistence
//   ← V-M-CORE (verified_by) — verification evidence
//   → UC-001 (implements) — user-facing use case
//   → Entity:Order (manages) — domain entity lifecycle
```

Allowed types: `implements`, `depends`, `refines`, `traces_to`, `verified_by`, `manages`, `uses`.
Old comma-separated `LINKS: M-STORAGE, V-M-CORE` remains supported and is parsed as legacy `depends`, but new work should use the typed format.

Python and SQL use the same markers with native comments:

```python
# START_CONTRACT_parse_date
# PURPOSE: Parse a user-provided date
def parse_date(value: str): ...
```

```sql
-- START_CONTRACT_create_users
-- PURPOSE: Create the users table
CREATE TABLE users (...);
```

### Semantic Markup Styles

Both styles are valid. For new GRACE-heavy modules, prefer XML-like anchors because paired tags carry stronger structure in long contexts. Keep one style per file when practical; `anchor-syntax-consistent` reports mixed files as a warning.

XML-like:

```
// <MODULE name="OrderService">
//   <BLOCK name="stock-validation">
//     ... code ...
//   </BLOCK>
// </MODULE>
```

Legacy START/END:

```
// START_stock-validation
// ... code ...
// END_stock-validation
```

### Semantic Markup Rules

1. **500-token granularity**: blocks should be ~500 TOKENS when they grow. Do not split tiny files or 20-line helpers just to satisfy a number.
2. **Unique block names**: every START_X/END_X pair must have a unique name within the file.
3. **Names describe WHAT, not HOW**: use `VALIDATE_INPUT` not `checkIfNullAndTrim`.
4. **Paired markers**: every START_X must have END_X. No orphans.
5. **Test files too**: substantial test files use the same structure (MODULE_CONTRACT, MODULE_MAP, blocks, CHANGE_SUMMARY).
6. **Log format**: use structured `<LOG>` entries at semantic transition points; legacy `[ModuleName][functionName][BLOCK_NAME]` trace markers remain readable but do not carry LDD expectations.

### Structured Log Format

Every semantic transition point SHOULD emit a structured LOG entry:

```
// <LOG id="module-001" level="INFO" ref="block-name" module="M-XXX" contract="functionName">
//   EVENT: event_name
//   CONTEXT: key=value
//   STATE: variable=value
//   DECISION: Why this path was chosen
//   EXPECTATION: What should happen next
//   RESULT: success|failure|warning|blocked
//   TRACEABILITY: useCase=UC-001, mentalTest=MT-001
// </LOG>
```

`EXPECTATION` plus `RESULT` enables Log Driven Development. After collecting runtime logs, call `analyze_logs` with `mode=trajectory|anomaly|compare`.

### Observable Belief State

Before substantial module code generation, call `extract_belief_state` for the target module and inspect/refine the generated `docs/belief-states/M-XXX.xml` artifact. New source files may also embed the same block near the module contract:

```
// <BELIEF_STATE module="M-XXX" version="1.0">
//   UNDERSTANDING:
//     Module responsibility: Restate the module purpose in your own words
//     Key data flows:
//       - INPUT: what enters this module and from where
//       - PROCESSING: key transformations and decisions
//       - OUTPUT: what leaves the module and side effects
//     Critical invariants:
//       - what must always remain true
//   IMPLEMENTATION_STRATEGY:
//     Approach: high-level strategy
//     Complexity_areas: parsing, compatibility, verification
//     Patterns_applied: explicit flow, typed links, bounded blocks
//   VERIFICATION_INTENT:
//     I expect the following to be true after my code runs:
//     - expected behavior or invariant
//   RISKS_ACKNOWLEDGED:
//     - known risk before coding
// </BELIEF_STATE>
```

`belief-state-exists` validates discovered belief state structure and reports coverage in `syn status`, `verify_project`, and `review_code`.

### PCAM (Purpose, Constraints, Autonomy, Metrics)

- **Purpose**: Defined by MODULE_CONTRACT — WHAT to build.
- **Constraints**: Defined by development plan and knowledge graph — BOUNDARIES.
- **Autonomy**: You choose HOW to implement within boundaries.
- **Metrics**: CONTRACT OUTCOMES + verification evidence = DONE criteria.

### Development Workflow

```
Phase 0:   Architecture → 5 XML docs (M-xxx, V-M-xxx, DF-xxx, Phase-N)
Phase 1-N: Per module:
  1. Read MODULE_CONTRACT + knowledge graph
  2. Call extract_belief_state for the target module and refine the belief if needed
  3. Write code with contracts, MODULE_MAP, CHANGE_SUMMARY, semantic blocks
  4. Call verify_project — if FAIL: STOP and fix
  5. Call review_code (scoped) — fix critical issues
  6. Update CHANGE_SUMMARY
  7. Update knowledge-graph.xml
Phase Gate: Call verify_project (phase level) + review_code (full)
```

### Stop Gates

- Requirements unclear → **STOP and ASK user**
- Phase 0 incomplete → **STOP, do not write code**
- verify_project FAIL → **STOP, fix, re-verify**
- review_code finds critical → **STOP, fix before next module**
- Architectural drift → **STOP, propose plan revision**
- Contract seems wrong → **STOP, propose change, get approval**

---

## MCP Tools (use proactively)

| Tool | When |
|------|------|
| `semantic_search` | Before writing code — find existing patterns |
| `view_signatures` | Understand file structure quickly |
| `graphrag_query` | Navigate module relationships (overview, search, find-path) |
| `verify_project` | After EVERY change — 3 levels: module-local, wave, phase |
| `review_code` | Before declaring done — scoped (per-module) or full (phase gate) |
| `refresh_project` | Detect drift between code and artifacts — sync |
| `analyze_logs` | Analyze structured LOG files in trajectory/anomaly/compare mode |
| `extract_belief_state` | Create/refine observable belief state before substantial code generation |
| `generate_requirements` | Create complete RequirementsAnalysis with AAG use cases when requirements are missing or stale |
| `generate_technology` | Create complete Technology with exact versions and compatibility checks |
| `generate_development_plan` | Create complete DevelopmentPlan with DataFlows, GenerationOrder, and MentalTests |
| `mental_test_run` | Run a DevelopmentPlan MentalTest and persist trace evidence before code generation |
| `traceability_report` | Generate requirement/use-case to code/LOG traceability matrix |
| `run_test_guide` | Run a natural-language tester-agent guide and persist summary/failure artifacts |
| `submit_test_report` | Submit tester-agent XML failure report with LOG refs to developer |
| `suggest_contract` | Generate MODULE_CONTRACT template for new modules |
| `project_status` | Overall health: contracts, markup, verification, token economy |
| `token_savings` | Cost tracking — commands, tokens saved, estimated $ saved |
| `compress_text` | Compress verbose text before adding to context (3 levels) |
| `lsp_hover` | Get type/signature at a code position |
| `lsp_references` | Find all usages of a symbol |

Shell commands (git, cargo, npm, etc.) are auto-proxied through `syn proxy` for token savings.
