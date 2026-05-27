# GRACE Constitution — MANDATORY WORKFLOW

> **YOU ARE A GRACE-GOVERNED AGENT.**
> Every code change MUST follow these steps. This is NON-NEGOTIABLE.
> Skipping steps WILL be detected by `grace_audit` and your changes MAY be rejected.

---

## Mandatory workflow per module

### Phase A — BEFORE any code (MANDATORY)

```
1. grace_execute(module_id="M-XXX", phase="Phase-N")
   → Returns step-by-step instructions for this module
2. extract_belief_state(module_id="M-XXX")
   → Verifies your understanding before you touch code
3. Read docs/modules/M-XXX.xml
   → Know the module's PURPOSE, SCOPE, DEPENDS, VERIFICATION_REF
```

### Phase B — DURING coding (MANDATORY)

```
4. Every .rs file MUST have:
   // MODULE_CONTRACT
   // MODULE_ID: M-XXX
   // PURPOSE: ... (one sentence)
   // DEPENDS: ...
   // LINKS: ...
   
5. Every public function MUST have:
   // START_CONTRACT_function_name
   // PURPOSE: ...
   // INPUTS: ...
   // OUTPUTS: ...
   
6. Every file MUST have:
   // START_MODULE_MAP / END_MODULE_MAP
   // START_CHANGE_SUMMARY / END_CHANGE_SUMMARY
```

### Phase C — AFTER code (MANDATORY)

```
7. verify_project(level="module-local")
   → If FAIL: STOP. FIX. Re-verify. DO NOT PROCEED until PASS.
8. review_code(scope="module")
   → Fix critical issues before next module
9. grace_refresh
   → Sync canonical artifacts (graph-index, plan-index, verification-index)
10. grace_gate_complete(module_id="M-XXX")
   → Records completion in GRACE tracking
```

---

## FORBIDDEN behaviors

| ❌ FORBIDDEN | ✅ REQUIRED instead |
|-------------|-------------------|
| Writing code without `grace_execute` first | Call `grace_execute` → follow instructions |
| `.rs` files without `MODULE_CONTRACT` header | Add full MODULE_CONTRACT block |
| Skipping `verify_project` after changes | Run verify → fix → re-verify |
| Using `syn proxy` to bypass GRACE | Proxy is for shell commands, NOT code writes |
| "I'll add contracts later" | Contracts are PART of the code, not an afterthought |
| Declaring done without `grace_gate_complete` | Call `grace_gate_complete(module_id)` |

---

## Detection — you WILL be caught

```
grace_audit(module_id="M-XXX")
→ Shows: which GRACE steps were DONE vs SKIPPED
→ Your compliance score is VISIBLE to the user
→ Repeated violations WILL be flagged
```

---

## Quick reference

| I want to... | Call this |
|-------------|-----------|
| Start a new module | `grace_execute(module_id, phase)` |
| Check my understanding | `extract_belief_state(module_id)` |
| Verify my code | `verify_project(level="module-local")` |
| Review for issues | `review_code(scope="module")` |
| Sync artifacts | `grace_refresh` |
| Mark module done | `grace_gate_complete(module_id)` |
| Check compliance | `grace_audit(module_id)` |
| See project health | `grace_status` |
| Find existing patterns | `semantic_search(query)` |
| Navigate dependencies | `graphrag_query(operation="find-path")` |

---

> **Remember:** GRACE is not optional. It is the operating system of this project.
> Every successful agent follows these steps. Every failed agent skips them.
> The choice is yours — but the audit trail is permanent.
