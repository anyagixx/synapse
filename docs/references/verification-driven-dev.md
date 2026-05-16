# Verification-Driven Development (VDD)

**Logs are evidence, not decoration.** Verification is a first-class architectural artifact.

## Three Verification Levels

### Level 1: Module-Local (deterministic)
- Unit tests with exact assertions
- Contract INPUT → OUTPUT mapping tests
- SIDE_EFFECTS validation (log emissions, state changes)
- Semantic block integrity (paired, unique, properly closed)

### Level 2: Wave-Level (trace)
- Cross-module integration tests
- Trace assertions: [Module][function][BLOCK_NAME] markers appear as expected
- Shared interface contract validation

### Level 3: Phase-Level (full)
- Full regression suite
- Knowledge graph consistency with code
- Verification plan completeness
- TODO/FIXME detection
- File size limits

## Log Format

All logs MUST use:
```
[ModuleName][functionName][BLOCK_NAME] descriptive message
```

Rules:
- Structured with stable fields
- Safe to retain and inspect (no secrets)
- Precise enough to navigate back to source block
- Redacted: never log credentials, tokens, PII

## Trace Assertions

Tests verify that log markers appear:
```rust
#[test]
fn test_create_note_logs() {
    create_note("test", "body");
    // Assert: [Core][create_note][CREATE] appears in log output
    assert!(log_contains("[Core][create_note][CREATE]"));
}
```

## Failure Packets

When verification fails, provide structured packet:
```
Scenario: create_note with empty title
Expected:  error returned, no log emitted
Observed:  panic in validate_input()
First divergent block: START_CONTRACT_create_note → validate_input
Suggested: add early return before validation, add test for empty title
```

## Deterministic-First Policy

1. Prefer exact assertions over fuzzy matching
2. Allow trace checks only when exact equality insufficient
3. Treat weak observability as a real verification defect
4. Do not accept verbose logs as substitute for actionable traces

## If Verification Fails

1. Read the failure packet
2. Navigate to first divergent block
3. Make minimal fix within scope
4. Strengthen nearby tests to prevent regression
5. Re-verify — if still failing: STOP and report
