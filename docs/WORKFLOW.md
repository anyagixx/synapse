# MyGRACE Development Workflow

## Overview

```
IDEA -> GRACE PLAN -> IMPLEMENT -> VERIFY -> REVIEW -> MERGE -> STATUS
                                  ^                    |
                                  +---- GRACE FIX -----+
```

Synapse exposes planning and fixing as MyGRACE MCP/skill workflows, while the shipped CLI provides local gates and project operations.

## 1. Idea To Requirements

Tell the AI what you want to build in OpenCode. The AI uses `grace_init`, `grace_plan`, and `grace_verification` to create or update sharded artifacts under `docs/`.

Primary artifacts:
- `docs/graph-index.xml`
- `docs/plan-index.xml`
- `docs/verification-index.xml`
- `docs/modules/`
- `docs/phases/`
- `docs/verification/`

## 2. Plan To Architecture

The AI reads the indexes first, then loads only the relevant phase or module shard. Each governed source file must carry `MODULE_CONTRACT`, `MODULE_MAP`, `CHANGE_SUMMARY`, function contracts, and semantic blocks.

## 3. Implementation

The AI uses `grace_execute` for bounded implementation guidance. Local project indexing and navigation use:

```bash
syn index
syn search "auth contract"
syn view src/main.rs
```

## 4. Verification

Run verification before declaring work done:

```bash
syn verify
syn ci verify
```

Verification checks contracts, semantic block structure, shard integrity, trace assertions, and canonical MyGRACE drift.

## 5. Review

Run integrity review after verification:

```bash
syn review
syn review --mode full
syn ci review
```

Review checks contract consistency, semantic markup, verification integrity, and artifact health.

## 6. Fix Flow

For failures, use the MyGRACE fix workflow through OpenCode or locally:

```bash
syn skills run grace_fix issue="verification failed for M-AUTH"
```

Then patch the smallest affected module and re-run `syn verify` plus `syn review`.

## 7. Status And Drift

Use status and refresh to keep project truth current:

```bash
syn status
syn refresh
syn refresh --fix
```

## Artifact Size Rule

AI-facing source and test files should stay within the repository's verification size target. When a file grows too large, split it by module responsibility and update the MyGRACE shards.
