# GRACE Methodology — Project Constitution

> **These rules are mandatory. They define HOW this project is built.**
> Architecture first. Code second. Never the other way around.

---

## Phase 0 — Architecture Artifacts (MANDATORY BEFORE ANY CODE)

**You are in Phase 0 until ALL 5 files exist in `docs/`. You MAY NOT write source code in Phase 0.**

Create files in this exact order:

### 1. `docs/requirements.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<REQUIREMENTS>
  <META><PROJECT>name</PROJECT><DESCRIPTION>what it does</DESCRIPTION><LANGUAGE>rust|python|ts|go</LANGUAGE></META>
  <REQUIREMENT><NAME>Feature</NAME><PURPOSE>why needed</PURPOSE><DEPENDS>other features</DEPENDS></REQUIREMENT>
</REQUIREMENTS>
```

### 2. `docs/technology.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<TECHNOLOGY>
  <STACK><LANGUAGE>python</LANGUAGE><FRAMEWORK>fastapi</FRAMEWORK><DATABASE>sqlite</DATABASE></STACK>
  <TOOLS><TOOL purpose="testing">pytest</TOOL></TOOLS>
</TECHNOLOGY>
```

### 3. `docs/development-plan.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<DEVELOPMENT_PLAN>
  <PHASES>
    <PHASE id="1" name="Foundation">
      <MODULES>
        <M-1><ID>M-CORE</ID><NAME>Module Name</NAME><PURPOSE>What it does</PURPOSE><FILES><FILE>src/module.rs</FILE></FILES></M-1>
      </MODULES>
    </PHASE>
  </PHASES>
  <DEPENDENCIES><DEP from="M-CORE" to="M-STORAGE"/></DEPENDENCIES>
</DEVELOPMENT_PLAN>
```

### 4. `docs/verification-plan.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<VERIFICATION_PLAN>
  <GLOBAL_POLICY><DETERMINISTIC_FIRST>true</DETERMINISTIC_FIRST></GLOBAL_POLICY>
  <MODULE_VERIFICATION id="V-M-CORE"><MODULE_ID>M-CORE</MODULE_ID><UNIT_TESTS>test_module</UNIT_TESTS></MODULE_VERIFICATION>
</VERIFICATION_PLAN>
```

### 5. `docs/knowledge-graph.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<KNOWLEDGE_GRAPH>
  <NODES><NODE id="M-CORE"><NAME>Core</NAME><KIND>module</KIND><PATH>src/core.rs</PATH></NODE></NODES>
  <RELATIONSHIPS><REL from="M-CORE" to="M-STORAGE" type="depends_on"/></RELATIONSHIPS>
</KNOWLEDGE_GRAPH>
```

### Phase 0 STOP Gates
- If `docs/requirements.xml` is missing → **STOP. Ask user what to build. Create the file.**
- If `docs/development-plan.xml` is missing → **STOP. Design the architecture. Create the file.**
- If `docs/verification-plan.xml` is missing → **STOP. Plan verification. Create the file.**
- If ANY Phase 0 file is missing → **DO NOT write source code.**
- Only when ALL 5 exist → proceed to Phase 1.

---

## Phase 1+ — Implementation (only after Phase 0 complete)

### 1. Never Write Code Without a Contract
Before creating or editing any source file, it MUST have a MODULE_CONTRACT:
```
// MODULE_CONTRACT
// MODULE_ID: M-XXX
// PURPOSE: [one sentence — what this module does]
// SCOPE: [what operations are included]
// DEPENDS: [module dependencies]
// LINKS: [knowledge graph references]

// START_public_api
... code ...
// END_public_api
```
Code implements the contract. The contract is the source of truth.

### 2. Semantic Markup Is Load-Bearing Structure
Every function/struct/class MUST be wrapped in START_/END_ blocks:
```
// START_create_note
pub fn create_note(...) { ... }
// END_create_note
```
These are NOT comments — they are structural anchors for verification tools.

### 3. Follow the Development Plan EXACTLY
- Implement modules in Phase order (1, then 2, then 3...)
- Each module gets its MODULE_CONTRACT before code
- After each module: call `verify_project` MCP tool
- If verification fails: **STOP and fix before next module**
- Update `docs/knowledge-graph.xml` after each module

### 4. ALWAYS Verify After Changes
After every code change, call `verify_project` MCP tool.
If verification FAILS: STOP and fix before continuing.
NEVER skip a failing verification step.

### 5. Review Before Committing
Before declaring work done, call `review_code` MCP tool (mode: scoped).
Fix all critical issues before proceeding.

### 6. Search Before Writing
Before writing new code, call `semantic_search` to find existing patterns.

### 7. Top-Down Synthesis (NEVER VIOLATE)
```
Phase 0: Architecture → Docs (5 XML files)
Phase 1+: Contracts → Code → Tests → Verify → Review
```
NEVER jump to code before Phase 0 is complete.

### 8. Governed Autonomy
You choose HOW to implement. Contracts and plans define WHAT.
If a contract seems wrong — PROPOSE a change, don't silently deviate.

### 9. Stop If Unsure
- If requirements unclear: **STOP and ASK user**
- If verification fails repeatedly: **STOP and report**
- If architectural drift detected: **STOP and propose plan revision**

---

## MCP Tools (use proactively)

| Tool | When |
|------|------|
| `semantic_search` | Before writing code |
| `view_signatures` | Understand file structure |
| `graphrag_query` | Navigate module relationships |
| `verify_project` | After EVERY change |
| `review_code` | Before declaring done |
| `project_status` | Show health |
