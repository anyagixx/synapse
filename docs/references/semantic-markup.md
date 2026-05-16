# Semantic Markup

**Markers are not comments — they are load-bearing structure.** Verification tools and navigation depend on them.

## Module-Level Markup

```
// MODULE_CONTRACT        ← source of truth for module
// MODULE_ID: M-XXX
// PURPOSE: [one sentence]
// SCOPE: [operations]
// DEPENDS: [dependencies]
// LINKS: [knowledge graph refs]

// START_MODULE_MAP       ← all public exports
// export_name — description
// END_MODULE_MAP

// START_CHANGE_SUMMARY    ← versioned change tracking
// LAST_CHANGE: [v1.0.0 — what changed]
// END_CHANGE_SUMMARY

// START_public_api        ← module body wrapper
... code ...
// END_public_api
```

## Function-Level Markup

```
// START_CONTRACT_fnName   ← function contract
// PURPOSE: [what it does]
// INPUTS: { name: Type — desc }
// OUTPUTS: { Type — desc }
// SIDE_EFFECTS: [effects]
// LINKS: [related artifact IDs]

// START_fnName            ← function body wrapper
pub fn fnName(...) { ... }
// END_fnName
```

## Granularity Rules

1. **~500 tokens per block** (chars/4 estimate). Not lines — tokens.
2. **Unique names per file** — no duplicate START_X/END_X within a file.
3. **Names describe WHAT, not HOW** — `VALIDATE_INPUT` not `checkIfNullAndTrim`.
4. **Paired always** — every START_X must have END_X. No orphans.
5. **Split when growing** — if a block exceeds ~500 tokens, split into sub-blocks.

## Block Types

| Marker | Purpose |
|--------|---------|
| `START_public_api` | Module body wrapper |
| `START_CONTRACT_name` | Function contract |
| `START_name` | Logic block / function body |
| `START_MODULE_MAP` | Export listing |
| `START_CHANGE_SUMMARY` | Change history |

## Test Files

Substantial test files use the same structure:
- MODULE_CONTRACT at top
- MODULE_MAP listing test functions
- CHANGE_SUMMARY tracking test evolution
- Semantic blocks for test groups

## Log Integration

Semantic blocks serve as log anchors:
```
// START_create_note
pub fn create_note(...) -> Note {
    log::info!("[Core][create_note][CREATE] title={}", title);
    ...
}
// END_create_note
```

The log marker `[Core][create_note][CREATE]` ties the log entry to the CREATE block.
