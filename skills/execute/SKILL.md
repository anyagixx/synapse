---
name: grace-execute
description: "Sequential implementation of development plan with strict contracts"
---

# grace-execute

Execute development plan module by module. Each module is implemented
with full MODULE_CONTRACT, semantic markup, and tests.

## Process

1. Read development-plan.xml → get module list
2. For each module:
   a. Read MODULE_CONTRACT
   b. Read verification-plan.xml for this module
   c. Implement code with START_BLOCK/END_BLOCK markup
   d. Write tests per verification plan
   e. Run module-local verification
   f. Pass → commit. Fail → fix and retry
3. After module: run wave-level verification
4. Commit shared artifacts (knowledge graph, verification plan)

## Rules

- No code written before MODULE_CONTRACT
- Every logical section wrapped in START_BLOCK/END_BLOCK
- Test must exist for every verification entry
- Module-local verification must pass before next module
