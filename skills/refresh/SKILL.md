---
name: grace-refresh
description: "Sync knowledge graph and verification plan with code"
---

# grace-refresh

Detect and report drift between code and GRACE artifacts.

## Purpose

After code changes, the knowledge graph and verification plan can drift from reality.
`grace-refresh` reconciles them by scanning actual source files and comparing with
`docs/knowledge-graph.xml` and `docs/verification-plan.xml`.

## Process

### Step 1: Scan code
- Walk all source files
- Extract MODULE_CONTRACT from each file
- Collect module IDs, dependencies, function contracts

### Step 2: Compare with knowledge-graph.xml
- Check every code module has an M-xxx node in knowledge-graph.xml
- Check every M-xxx node in graph has corresponding code module
- Report: missing nodes, stale nodes, dependency mismatches

### Step 3: Compare with verification-plan.xml
- Check every code module has V-M-xxx entry in verification plan
- Check every V-M-xxx entry has corresponding code module
- Report: missing entries, stale entries

### Step 4: Phase 0 check
- Verify all 5 docs/ files exist
- Report missing artifacts

### Step 5: Contract quality
- Report MODULE_MAP, CHANGE_SUMMARY, function contract issues per module

## Suggested Actions

For each drift detected, provide a concrete fix:
- "Add M-MODULE to knowledge-graph.xml"
- "Remove stale M-OLD from knowledge-graph.xml"
- "Add V-M-MODULE to verification-plan.xml"
- "Add MODULE_MAP to src/module.rs"

## Scope

Default to **targeted** — only changed files since last sync.
Escalate to **full** when targeted reveals broader drift.
