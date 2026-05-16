# Contract-Driven Development (CDD)

**Never write code without a contract.** The contract is the source of truth. Code implements the contract, not the other way around.

## PCAM Framework

| Component | Defined By | Question Answered |
|-----------|-----------|-------------------|
| **Purpose** | MODULE_CONTRACT | WHAT to build |
| **Constraints** | Development plan + knowledge graph | BOUNDARIES |
| **Autonomy** | Implementer's judgment | HOW to implement |
| **Metrics** | CONTRACT OUTPUTS + verification evidence | AM I DONE? |

## MODULE_CONTRACT

Every source file starts with:
```
// MODULE_CONTRACT
// MODULE_ID: M-XXX
// PURPOSE: [one sentence]
// SCOPE: [operations included]
// DEPENDS: [module dependencies]
// LINKS: [knowledge graph references]
```

## MODULE_MAP

Lists every public export with a one-line description:
```
// START_MODULE_MAP
// create_note — Creates and persists a new note
// search_notes — Returns notes matching query text
// END_MODULE_MAP
```

## CHANGE_SUMMARY

Tracks evolution of the module:
```
// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Initial implementation]
// END_CHANGE_SUMMARY
```

## Function Contracts

Every function gets a contract with typed inputs/outputs:
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

## Development Flow

```
Phase 0: Architecture → 5 XML docs (M-xxx, V-M-xxx)
Phase 1+: Per module:
  1. Read MODULE_CONTRACT
  2. Write code → MODULE_MAP → CHANGE_SUMMARY
  3. verify_project → fix → re-verify
  4. review_code (scoped)
  5. Update knowledge graph
```

## Rules

1. If a contract seems wrong — PROPOSE a change, don't silently deviate
2. Never change a contract without user approval
3. If DEPENDS doesn't match actual imports — STOP and update the plan
4. Contract completeness: PURPOSE + SCOPE + DEPENDS + LINKS minimum
