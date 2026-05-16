# GRACE Methodology — Project Constitution

> **These rules are mandatory. They define HOW this project is built.**
> The LLM has freedom in HOW to implement, but not in WHAT to build.
> Contracts, verification plans, and the knowledge graph define WHAT.

---

## 6 Core Principles

### 1. Never Write Code Without a Contract
Before generating or editing any module, create or update its MODULE_CONTRACT with
PURPOSE, SCOPE, DEPENDS, and LINKS. The contract is the source of truth.
Code implements the contract, not the other way around.

```
// MODULE_CONTRACT
// MODULE_ID: M-XXX
// PURPOSE: [what this module does — one sentence]
// SCOPE: [what operations are included]
// DEPENDS: [module dependencies]
// LINKS: [knowledge graph references]
```

### 2. Semantic Markup Is Load-Bearing Structure
Every logical block of code MUST be wrapped in START_/END_ anchors.
These are NOT comments — they are structural anchors for verification tools.

```
// START_public_api
pub fn create_note(...) -> Note { ... }
// END_public_api

// START_search
pub fn search_notes(query: &str) -> Vec<Note> { ... }
// END_search
```

### 3. Knowledge Graph Is Always Current
After adding or removing modules, update `docs/knowledge-graph.xml`.
After changing module dependencies, update MODULE_CONTRACT.LINKS.
Never let the graph drift from reality.

### 4. Verification Is a First-Class Artifact
Testing, traces, and log anchors are designed BEFORE large implementation waves.
`docs/verification-plan.xml` is part of the architecture, not an afterthought.
After every change: call `verify_project` MCP tool. Never skip on failure.

### 5. Top-Down Synthesis
Code generation follows:
```
Requirements → Architecture → Contracts → Code → Tests → Verify → Review
```
Never jump straight to code when requirements or architecture are unclear.

### 6. Governed Autonomy
Agents have freedom in HOW to implement, but not in WHAT to build.
Contracts, plans, graph references, and verification requirements define the allowed space.
If a contract seems wrong — propose a change, don't silently deviate.

---

## Rules for Modifications

1. **Read the MODULE_CONTRACT** before editing any file.
2. **After editing source files**, update MODULE_MAP if exports changed.
3. **After adding/removing modules**, update `docs/knowledge-graph.xml`.
4. **After changing tests or commands**, update `docs/verification-plan.xml`.
5. **After fixing bugs**, add a CHANGE_SUMMARY and strengthen nearby verification.
6. **Never remove semantic markup anchors** unless the structure is intentionally replaced.

---

## MCP Tools (use proactively)

| Tool | When to use |
|------|------------|
| `semantic_search` | Before writing code — find existing patterns |
| `view_signatures` | Understand a file's structure |
| `graphrag_query` | Navigate module relationships |
| `verify_project` | After every change — check contracts, structure |
| `review_code` | Before declaring done — check quality |
| `project_status` | Show overall health |
| `token_savings` | Show cost savings |
| `compress_text` | Compress long text |

---

## Stop and Ask Gates

- If requirements are unclear: **STOP and ASK the user**
- If verification fails: **STOP and fix before continuing**
- If architectural drift detected: **STOP and propose a plan revision**
- If contract seems wrong: **STOP and ask for approval to change it**
- If module dependency missing: **STOP and update the plan first**

---

## PCAM (Purpose, Constraints, Autonomy, Metrics)

- **Purpose**: Defined by the contract. You know WHAT to build.
- **Constraints**: Defined by the plan and knowledge graph. You know the BOUNDARIES.
- **Autonomy**: You choose HOW to implement within those boundaries.
- **Metrics**: The contract's OUTPUTS plus verification evidence tell you if you're done.
