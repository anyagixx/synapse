---
name: grace-multiagent-execute
description: "Execute development plan in parallel waves with multiple agents"
---

# grace-multiagent-execute

Execute the development plan in parallel-safe waves with controller/worker architecture.

## Prerequisites
- `docs/development-plan.xml` with Phase-N and step-N definitions
- `docs/verification-plan.xml` with V-M-xxx per module
- `docs/knowledge-graph.xml` up to date
- `skills/roles/` sub-agent definitions exist

## Execution Profiles

| Profile | Parallel Modules | Risk | When to use |
|---------|-----------------|------|------------|
| `safe` | No shared dependencies | Zero | First wave of a new project |
| `balanced` | Independent sub-trees only | Low | Well-defined interfaces |
| `fast` | All modules, merge conflicts expected | Medium | Mature project, strong tests |

## Process

### Step 1: Controller Loads Artifacts
Parse development-plan.xml, knowledge-graph.xml, verification-plan.xml once.
Build execution queue with dependency ordering.

### Step 2: Controller Creates Execution Packets
For each module in the current wave:
- MODULE_CONTRACT block (from plan)
- MODULE_MAP skeleton
- Target file path
- Verification ref (V-M-xxx)
- Assigned worker ID
- Execution profile constraints

### Step 3: Workers Execute in Parallel
Each worker:
- Receives one execution packet per module
- Implements exactly what the contract requires
- Preserves MODULE_CONTRACT, MODULE_MAP, semantic blocks
- Runs module-local verification
- Reports graph delta + verification delta
- Commits result

### Step 4: Controller Synchronizes
- Collect all worker deltas
- Merge knowledge-graph.xml updates (batch, no conflicts)
- Merge verification-plan.xml updates
- Run wave-level verification
- Commit wave as atomic unit

### Step 5: Phase Gate
After all waves in a phase complete:
- Run phase-level verification
- Run wave-audit review
- Update development-plan.xml statuses
- Report to user

## Worker Constraints (MANDATORY)

- Do not invent new modules or architecture
- Do not edit shared planning artifacts directly
- If contract is unclear: STOP and ask controller
- If new dependency required: STOP and ask controller to revise plan
- If verification fails: fix in scope, re-verify, report delta

## Controller Ownership

Controller OWNS:
- docs/development-plan.xml
- docs/knowledge-graph.xml
- docs/verification-plan.xml

Workers MAY NOT modify these files. Workers report deltas to controller.
