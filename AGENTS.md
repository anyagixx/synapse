# GRACE Methodology — Project Constitution

> **These rules are mandatory. Architecture first. Code second. Never the other way around.**

---

## Phase 0 — Architecture Artifacts (MANDATORY BEFORE ANY CODE)

**You are in Phase 0 until ALL 5 files exist in `docs/`. You MAY NOT write source code in Phase 0.**

Create files in this exact order:

### 1. `docs/requirements.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<REQUIREMENTS>
  <META><PROJECT>name</PROJECT><DESCRIPTION>what it does</DESCRIPTION><LANGUAGE>rust|python|ts|go</LANGUAGE></META>
  <UC-1><Actor>User</Actor><Action>create note</Action><Goal>persist text with title</Goal></UC-1>
  <NonGoals>what is explicitly OUT of scope</NonGoals>
  <Risks>what could go wrong</Risks>
  <OpenQuestions>what needs clarification</OpenQuestions>
</REQUIREMENTS>
```

### 2. `docs/technology.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<TECHNOLOGY>
  <STACK><LANGUAGE>rust</LANGUAGE><FRAMEWORK></FRAMEWORK><DATABASE></DATABASE></STACK>
  <TOOLS><TOOL purpose="build">cargo</TOOL><TOOL purpose="test">cargo test</TOOL></TOOLS>
</TECHNOLOGY>
```

### 3. `docs/development-plan.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<DEVELOPMENT_PLAN>
  <META><GENERATED_BY>LLM</GENERATED_BY></META>
  <ArchitectureNotes>key design decisions</ArchitectureNotes>
  <PHASES>
    <Phase-1 name="Foundation" status="pending">
      <MODULES>
        <M-CORE name="Core" type="CORE_LOGIC" status="planned">
          <PURPOSE>Core application logic</PURPOSE>
          <contract><inputs></inputs><outputs></outputs><errors></errors></contract>
          <FILES><FILE>src/core.rs</FILE></FILES>
          <verification-ref>V-M-CORE</verification-ref>
        </M-CORE>
      </MODULES>
    </Phase-1>
  </PHASES>
  <DataFlows>
    <DF-1 name="CreateNote" trigger="user submits form">
      <step-1 module="M-CORE">validate input</step-1>
      <step-2 module="M-STORAGE">persist to DB</step-2>
      <evidence>log: [Core][create_note] saved id={}</evidence>
    </DF-1>
  </DataFlows>
  <ImplementationOrder>
    <Phase-1><step-1 module="M-CORE">implement core types and create_note</step-1></Phase-1>
  </ImplementationOrder>
  <ExecutionPolicy><controller>main agent</controller><worker_per_module>1</worker_per_module></ExecutionPolicy>
  <DEPENDENCIES><DEP from="M-CORE" to="M-STORAGE"/></DEPENDENCIES>
</DEVELOPMENT_PLAN>
```

### 4. `docs/verification-plan.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<VERIFICATION_PLAN>
  <GlobalPolicy>
    <deterministic-first>true</deterministic-first>
    <log-format>[Module][function][BLOCK_NAME] message</log-format>
    <redaction>no secrets in logs</redaction>
  </GlobalPolicy>
  <ModuleVerification>
    <V-M-CORE MODULE="M-CORE" PRIORITY="critical">
      <unit-tests>test_core</unit-tests>
      <required-log-markers><marker>[Core][create_note][CREATE]</marker></required-log-markers>
      <required-trace-assertions><assert>CREATE log appears exactly once per call</assert></required-trace-assertions>
      <failure-packet>
        <scenario>create_note with empty title</scenario>
        <expected>error returned, no log emitted</expected>
        <observed>check actual behavior</observed>
        <suggested>validate before persist, add early return</suggested>
      </failure-packet>
    </V-M-CORE>
  </ModuleVerification>
  <PhaseGates>
    <Gate-Phase-1><requires>all V-M-* module checks PASS</requires><command>syn verify</command></Gate-Phase-1>
  </PhaseGates>
</VERIFICATION_PLAN>
```

### 5. `docs/knowledge-graph.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<KNOWLEDGE_GRAPH>
  <NODES>
    <M-CORE TYPE="CORE_LOGIC" STATUS="implemented">
      <NAME>Core Module</NAME><PATH>src/core.rs</PATH>
      <exports><fn-create_note>creates and persists a note</fn-create_note></exports>
      <verification-ref>V-M-CORE</verification-ref>
    </M-CORE>
  </NODES>
  <CrossLinks><CrossLink from="M-CORE" to="M-STORAGE" relation="depends_on"/></CrossLinks>
</KNOWLEDGE_GRAPH>
```

### Phase 0 STOP Gates
- If `docs/requirements.xml` is missing → **STOP. Ask user what to build.**
- If `docs/technology.xml` is missing → **STOP. Define the tech stack.**
- If `docs/development-plan.xml` is missing → **STOP. Design architecture with M-xxx modules, Phase-N gates, DF-xxx dataflows.**
- If `docs/verification-plan.xml` is missing → **STOP. Define V-M-xxx verification per module.**
- If ANY Phase 0 file is missing → **DO NOT write source code.**
- Only when ALL 5 exist → proceed to Phase 1.

---

## Phase 1+ — Implementation

### Every source file MUST have this structure:

```
// MODULE_CONTRACT
// MODULE_ID: M-XXX
// PURPOSE: [one sentence — what this module does]
// SCOPE: [what operations are included]
// DEPENDS: [module dependencies]
// LINKS: [knowledge graph references]

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

### Every function MUST have a contract:

```
// START_CONTRACT_create_note
// PURPOSE: Create a new note and persist to storage
// INPUTS: { title: String — note title }, { content: String — note body }
// OUTPUTS: { Note — the created note with id assigned }
// SIDE_EFFECTS: writes to database, emits [Core][create_note][CREATE] log
// LINKS: M-STORAGE, V-M-CORE
// START_create_note
pub fn create_note(title: &str, content: &str) -> Note { ... }
// END_create_note
```

### Semantic Markup Rules

1. **500-token granularity**: blocks should be ~500 TOKENS (not lines). If larger, split into sub-blocks.
2. **Unique block names**: every START_X/END_X pair must have a unique name within the file.
3. **Names describe WHAT, not HOW**: use `VALIDATE_INPUT` not `checkIfNullAndTrim`.
4. **Paired markers**: every START_X must have END_X. No orphans.
5. **Test files too**: substantial test files use the same structure (MODULE_CONTRACT, MODULE_MAP, blocks, CHANGE_SUMMARY).
6. **Log format**: `[ModuleName][functionName][BLOCK_NAME] descriptive message` — structured, stable fields, redacted secrets.

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
  2. Write code with contracts, MODULE_MAP, CHANGE_SUMMARY, semantic blocks
  3. Call verify_project — if FAIL: STOP and fix
  4. Call review_code (scoped) — fix critical issues
  5. Update CHANGE_SUMMARY
  6. Update knowledge-graph.xml
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
| `suggest_contract` | Generate MODULE_CONTRACT template for new modules |
| `project_status` | Overall health: contracts, markup, verification, token economy |
| `token_savings` | Cost tracking — commands, tokens saved, estimated $ saved |
| `compress_text` | Compress verbose text before adding to context (3 levels) |
| `lsp_hover` | Get type/signature at a code position |
| `lsp_references` | Find all usages of a symbol |

Shell commands (git, cargo, npm, etc.) are auto-proxied through `syn proxy` for token savings.
