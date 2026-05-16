# Knowledge Graph

**Always current. Never let the graph drift from reality.**

## Purpose

The knowledge graph is a living map of all modules, their exports, dependencies, and cross-links. It serves as navigation and dependency analysis for both humans and AI agents.

## XML Structure

```xml
<KNOWLEDGE_GRAPH>
  <NODES>
    <M-XXX TYPE="CORE_LOGIC|DATA_LAYER|UI|UTILITY|INTEGRATION|ENTRY_POINT" STATUS="planned|implemented|deprecated">
      <NAME>Module Name</NAME>
      <PATH>src/module.rs</PATH>
      <exports><fn-name>description</fn-name></exports>
      <verification-ref>V-M-XXX</verification-ref>
    </M-XXX>
  </NODES>
  <CrossLinks>
    <CrossLink from="M-A" to="M-B" relation="depends_on|imports|calls|configures"/>
  </CrossLinks>
</KNOWLEDGE_GRAPH>
```

## Module Types

| Type | Description |
|------|-------------|
| ENTRY_POINT | Application startup, main, routing |
| CORE_LOGIC | Business rules, domain logic |
| DATA_LAYER | Database, storage, persistence |
| UI | User interface components |
| UTILITY | Shared helpers, common functions |
| INTEGRATION | External API clients, adapters |

## Maintenance Rules

1. **After creating a new module** — add M-XXX node with STATUS="implemented"
2. **After adding exports** — update `<exports>` list
3. **After changing dependencies** — update CrossLinks
4. **After removing a module** — remove node, remove all CrossLinks referencing it
5. **After each module** in execution — verify graph synced via `syn refresh`

## CrossLinks

Bidirectional and consistent:
- Relationship types: `depends_on`, `imports`, `calls`, `configures`, `implements`, `extends`
- If M-A depends_on M-B, both nodes must reference each other
- No orphan CrossLinks — every `from` and `to` must match existing M-XXX nodes

## Verification Refs

Every module node SHOULD reference its verification entry:
```xml
<M-CORE TYPE="CORE_LOGIC" STATUS="implemented">
  <verification-ref>V-M-CORE</verification-ref>
</M-CORE>
```

## Drift Detection

Run `syn refresh` to detect:
- Modules in code but not in graph → add them
- Modules in graph but not in code → remove or mark deprecated
- CrossLinks pointing to missing nodes → fix or remove
- Missing verification-refs → add them
