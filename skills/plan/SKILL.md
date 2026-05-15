---
name: grace-plan
description: "Design architecture from requirements — modules, contracts, data flows, knowledge graph"
---

# grace-plan

Architecture planning phase. Reads `requirements.xml` and `technology.xml`,
generates `development-plan.xml` and `knowledge-graph.xml`.

## Process

1. Read requirements.xml → extract use cases
2. Read technology.xml → identify stack constraints
3. Decompose into modules (ENTRY_POINT, CORE_LOGIC, DATA_LAYER, UI_COMPONENT, UTILITY, INTEGRATION)
4. Create MODULE_CONTRACT for each module
5. Define data flows between modules
6. Create knowledge-graph.xml with CrossLinks
7. Assign verification references to each module

## Rules

- Every module must have MODULE_CONTRACT before any code
- Modules ≤500 lines in contract
- knowledge-graph.xml ≤500 lines
- Unique tag convention: `<M-AUTH>` not `<Module ID="M-AUTH">`
