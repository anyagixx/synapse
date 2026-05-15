---
name: grace-verify
description: "Three-level verification — module-local, wave, phase"
---

# grace-verify

Run three levels of verification on the project.

## Level 1: Module-Local

- Unit tests pass
- Contract compliance: PURPOSE matches implementation
- Semantic markup: all START_BLOCK/END_BLOCK pairs match
- No TODO/FIXME in production code
- All GRACE artifacts ≤500 lines

## Level 2: Wave-Level

- Integration tests pass
- Cross-module contracts satisfied
- Data flow works end-to-end
- Knowledge graph matches actual dependencies

## Level 3: Phase-Level

- Full regression tests pass
- Security audit (no secrets, no unsafe patterns)
- Performance benchmarks
- Documentation is current

## Exit Codes

- 0: All levels pass
- 1: Module-local failures
- 2: Wave-level failures
- 3: Phase-level failures
