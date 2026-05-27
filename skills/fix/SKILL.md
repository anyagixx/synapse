---
name: grace-fix
description: "Debug via knowledge graph navigation — find, analyze, fix, verify"
---

# grace-fix

Debug issues through knowledge graph navigation. No random code reading.

## Process

1. Parse bug description
2. Search knowledge graph for related modules
3. Read MODULE_CONTRACT of target module
4. Navigate to specific START_BLOCK/END_BLOCK
5. Analyze code vs contract
6. Identify mismatch
7. Apply targeted fix
8. Run module-local verification
9. Report: what was wrong, what was fixed, what was verified
