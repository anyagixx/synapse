# GRACE Development Workflow

## Overview

```
IDEA → PLAN → EXECUTE → VERIFY → REVIEW → MERGE → STATUS
                         ↑                     |
                         └── FIX ←─────────────┘
```

## Phase 1: Idea → Requirements

You tell AI what you want. AI creates `requirements.xml` and `technology.xml`.
You approve or request changes.

## Phase 2: Plan → Architecture

AI runs `syn plan`. Generated:
- `development-plan.xml` — modules, dependencies, phases
- `knowledge-graph.xml` — module map with CrossLinks
- `MODULE_CONTRACT` in every file (PURPOSE, SCOPE, DEPENDS, LINKS)

**Rule:** No code without MODULE_CONTRACT.

## Phase 3: Execute → Implementation

AI runs `syn execute`. Per-module loop:
1. Read MODULE_CONTRACT
2. Implement with semantic `START_BLOCK`/`END_BLOCK`
3. Run module-local verification
4. Pass → next module. Fail → fix and retry

## Phase 4: Verify → Quality Gate

AI runs `syn verify`. Three levels:

| Level | Scope | Speed |
|-------|-------|-------|
| Module-local | Unit tests, contract compliance, block integrity | Fast |
| Wave-level | Integration tests, cross-module contracts | Medium |
| Phase-level | Full regression, security audit, perf benchmarks | Slow |

**Gate:** Wave-level must pass before merge. Phase-level for releases.

## Phase 5: Review → Integrity Check

AI runs `syn review`. Checks:
1. Semantic markup — all START/END pairs match
2. Contract compliance — code matches contract
3. Verification integrity — tests exist for all claims
4. Graph consistency — knowledge graph matches code
5. No secrets — no API keys, passwords committed

## Phase 6: Status → Health Report

AI runs `syn status`. Shows:
- Modules with/without contracts
- Test counts and pass rates
- Knowledge graph health
- Token savings from proxy + compression

## The 500-Line Rule

Every AI-facing artifact MUST be ≤500 lines:
- XML plans (requirements, technology, dev-plan, verification, knowledge-graph)
- MODULE_CONTRACT in source files
- Any generated .md file

If exceeded → automatically split. Enforced by `syn verify --strict`.

## Strict Mode

```bash
syn config set strictness true
```

- Contract required: YES
- Verification gate: Phase-level
- Semantic markup: REQUIRED
- Knowledge graph check: On every commit
- Review before merge: REQUIRED
- 500-line enforcement: STRICT (CI fails)
