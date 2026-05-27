# Unique Tag Convention (UTC)

**Every named entity in GRACE artifacts gets a unique XML tag.** This eliminates closing-tag polysemy — the problem where `</Module>` could close any of 10 opening `<Module>` tags.

## The Problem

Without unique tags:
```xml
<!-- AMBIGUOUS: which </Module> closes which? -->
<Module><name>Auth</name></Module>
<Module><name>Core</name></Module>
```

With unique tags:
```xml
<!-- CLEAR: every tag is self-identifying -->
<M-AUTH name="Auth" type="CORE_LOGIC" status="implemented">
  <PURPOSE>Authentication service</PURPOSE>
</M-AUTH>
<M-CORE name="Core" type="CORE_LOGIC" status="implemented">
  <PURPOSE>Core application logic</PURPOSE>
</M-CORE>
```

## Naming Convention

| Entity | Tag Format | Example |
|--------|-----------|---------|
| Module | `<M-XXX>` | `<M-AUTH>`, `<M-CORE>` |
| Verification | `<V-M-XXX>` | `<V-M-AUTH>`, `<V-M-CORE>` |
| Data Flow | `<DF-N>` | `<DF-1>`, `<DF-2>` |
| Phase | `<Phase-N>` | `<Phase-1>`, `<Phase-2>` |
| Use Case | `<UC-N>` | `<UC-1>`, `<UC-2>` |
| Step | `<step-N>` | `<step-1>`, `<step-2>` |
| Phase Gate | `<Gate-Phase-N>` | `<Gate-Phase-1>` |
| Note | `<note-N>` | `<note-1>`, `<note-2>` |

## Rules

1. **Every opening tag is unique** — no generic `<Module>`, `<Phase>`, `<Step>` tags
2. **ID is encoded in tag name** — `M-AUTH` not `<Module id="AUTH">`
3. **Closing tag matches opening** — `<M-AUTH>...</M-AUTH>` always
4. **Numbers are sequential within parent** — `<step-1>`, `<step-2>`, not `<step-5>`, `<step-12>`
5. **No reused IDs** — once assigned, M-AUTH is M-AUTH forever

## Why This Matters

- AI agents can parse XML without ambiguity
- Diff tools show exactly which entity changed
- Cross-references are reliable (`<verification-ref>V-M-AUTH</verification-ref>`)
- Multi-agent execution avoids merge conflicts on shared XML
