---
name: grace-ask
description: "Grounded Q&A — answer questions about the project using GRACE artifacts"
---

# grace-ask

Answer questions about the project using all GRACE artifacts as ground truth.

## Process

### Step 1: Load All Artifacts
Read all 5 Phase 0 documents:
- `docs/requirements.xml` — what are we building?
- `docs/technology.xml` — what stack?
- `docs/development-plan.xml` — modules, phases, dataflows
- `docs/verification-plan.xml` — how do we verify?
- `docs/knowledge-graph.xml` — how do modules relate?

### Step 2: Understand the Question
Classify the question:
- **Code question** — "How does X work?" → search code via `semantic_search`
- **Architecture question** — "Why is M-A connected to M-B?" → consult knowledge graph
- **Status question** — "What's done?" → consult development plan + `project_status`
- **Verification question** — "Is X tested?" → consult verification plan

### Step 3: Find Relevant Modules
- Use `semantic_search` with the question as query
- Use `graphrag_query` to find related modules
- Use `view_signatures` to inspect specific files

### Step 4: Navigate to Specific Blocks
- Read the relevant source file
- Navigate to the semantic block matching the question
- Read the MODULE_CONTRACT and function contracts

### Step 5: Answer with Citations
- Cite exact file:line locations
- Reference MODULE_CONTRACT.PURPOSE where relevant
- Reference knowledge graph CrossLinks for dependency questions
- If answer comes from code — cite the semantic block name

## Rules
- Always ground answers in actual artifacts (XML docs or source code)
- Never guess architecture — if unclear, consult the knowledge graph
- If artifacts are stale, run `syn refresh` and report drift
