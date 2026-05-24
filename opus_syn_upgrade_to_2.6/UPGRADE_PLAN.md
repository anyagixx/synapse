# Synapse Upgrade Plan to 2.6

Status: done
Mode: balanced
Primary goal: make Synapse better for autonomous agents
Checkpoint owner: OpenCode
Last updated: 2026-05-24

## Current checkpoint
- Active program: Synapse Upgrade Plan to 2.6
- Current subphase: Closed
- Last completed activity: added durable lessons memory in `M-MEMORY`, exposed it through the crate root, and verified the full plan closure with format, full tests, clippy, MyGRACE verify, and full review
- Next recommended activity: treat future integrations, richer write tools, and dashboard polish as post-2.6 backlog, not as open scope in this plan

## North Star
Turn Synapse from strong governance/verification toolkit into controlled autonomous agent platform.

Target behavior:
1. accept bounded objective
2. create machine-readable run
3. enforce gates automatically
4. execute bounded workflow safely
5. collect evidence and provenance
6. stop or escalate on failure
7. resume from durable state

## Strategic principles
- small diffs
- bounded autonomy
- verification after every meaningful change
- review after every meaningful change
- no large rewrites first
- single-run runtime before multi-agent runtime
- machine-readable policy before rich UI
- provenance before analytics

---

## Full roadmap

### Phase 1 — Autonomous Agents Core
Goal: make Synapse execute controlled autonomous workflows, not only describe them.

#### Subphase 1.1 — Run Model
Goal: define formal runtime object model.

Deliverables:
- Run model
- Task model
- Step model
- Gate model
- Outcome model
- Escalation model
- durable run identifiers
- current status lifecycle

Minimum fields for Run:
- run_id
- goal
- phase
- module_id
- objective
- status
- created_at
- updated_at
- current_step
- required_gates
- evidence_refs
- blocked_reason
- escalation_reason

Why:
- current skills mostly return text guidance in `src/skills/engine.rs`
- autonomous execution needs machine state, not prose only

Success criteria:
- system can represent one bounded autonomous run in stable schema
- run state can be serialized and restored
- state model supports blocked, failed, completed, and escalated outcomes

#### Subphase 1.2 — Gate Policy Engine
Goal: convert process rules into machine-enforced gate policy.

Inputs today:
- `AGENTS.md`
- `docs/development-plan.xml`
- `src/grace/status.rs`
- `src/grace/mental_test.rs`
- `src/grace/traceability.rs`

Deliverables:
- gate policy schema
- preconditions model
- blocking severity model
- required checks per stage
- pass/fail criteria
- fallback/escalation behavior

Examples:
- before code step: artifacts complete, target known, mental-test requirement resolved, belief state available
- before completion: verify pass, review pass, traceability pass, no drift

Success criteria:
- runtime can ask one source of truth whether next step is allowed
- blocked state includes exact machine-readable reason

#### Subphase 1.3 — Bounded Workflow Runner
Goal: orchestrate GRACE tools in one safe execution loop.

Needed operations:
- plan_run
- start_run
- resume_run
- complete_run
- fail_run
- escalate_run

Base execution loop:
1. load run context
2. read relevant artifacts
3. determine gates
4. execute gate checks
5. stop if blocked
6. continue one bounded step if pass
7. store evidence
8. rerun required verification
9. complete or escalate

Success criteria:
- one run can progress through deterministic steps
- one failing gate halts progress
- run can resume after interruption

#### Subphase 1.4 — Provenance Ledger
Goal: track autonomous behavior, not only token counts.

Current limitation:
- `src/tracking/mod.rs` stores token economics only

Deliverables:
- run ledger storage
- tool invocation records
- gate pass/fail snapshots
- artifact output refs
- retry counters
- human handoff markers
- escalation records

Success criteria:
- every run has replayable history
- dashboard or CLI can inspect lifecycle and evidence

#### Subphase 1.5 — Autonomous Skill Upgrade
Goal: make core skills emit machine-usable execution outputs.

Priority skills:
- grace_execute
- grace_multiagent_execute
- grace_fix
- grace_status

Deliverables:
- machine-readable next-step output
- required gates output
- blocked reason output
- structured plan output
- compatibility with existing text guidance where useful

Success criteria:
- agent can request executable next action instead of narrative only

#### Subphase 1.6 — Mental-Test-First Autonomy
Goal: enforce mental tests automatically for critical or complex work.

Current state:
- mental tests exist in `docs/development-plan.xml`
- runtime guidance references them
- enforcement is not yet autonomous-first

Deliverables:
- rule linking complexity/criticality to mental test requirement
- auto-run before implementation stage
- block when missing or failed
- persist evidence into run ledger

Success criteria:
- critical module run cannot start implementation without passing mental-test gate

#### Subphase 1.7 — Traceability-Driven Planning
Goal: turn traceability results into planning inputs.

Current base:
- strong traceability engine
- strict traceability index in `docs/traceability-index.xml`

Deliverables:
- convert trace gaps into tasks
- prioritize unresolved trace debt before risky implementation
- suggest missing LINKS / verification / log anchors
- integrate trace closure into completion criteria

Success criteria:
- runtime can surface traceability debt as actionable work

#### Subphase 1.8 — Self-Healing Failure Loop
Goal: introduce bounded recovery path for failed runs.

Deliverables:
- failure classifier
- localized recovery planner
- retry budget
- rerun gate subset
- escalation packet for human handoff

Success criteria:
- runtime retries safely inside scope limits
- repeated failure escalates with clear evidence bundle

### Phase 2 — Retrieval and Graph Intelligence
Goal: improve agent understanding of large codebases.

Planned work:
- richer GraphRAG edge types
- symbol/reference/call graph support
- better ranking and path explanation
- stronger cross-file retrieval
- later semantic retrieval improvements

Priority ideas:
- call/reference edges
- test-to-code edges
- log/runtime edges
- graph ranking based on typed LINKS and usage

### Phase 3 — Real Test Harness
Goal: prove behavior with stronger evidence loops.

Planned work:
- richer tester execution
- fixture/environment model
- evidence bundles
- flaky tracking
- traceability-aware test completion

### Phase 4 — Dashboard as Agent Cockpit
Goal: turn dashboard into operational control plane.

Planned work:
- runs page
- run detail page
- queue view
- blocked/failure board
- review/approve flow
- session replay

### Phase 5 — Wider Integrations and Write Tools
Goal: expand platform reach and edit capability.

Planned work:
- broader host integrations
- richer MCP write/refactor tools
- stronger LSP actions
- durable project memory / lessons store

---

## Recommended implementation order
1. Subphase 1.1 — Run Model
2. Subphase 1.2 — Gate Policy Engine
3. Subphase 1.3 — Bounded Workflow Runner
4. Subphase 1.4 — Provenance Ledger
5. Subphase 1.5 — Autonomous Skill Upgrade
6. Subphase 1.6 — Mental-Test-First Autonomy
7. Subphase 1.7 — Traceability-Driven Planning
8. Subphase 1.8 — Self-Healing Failure Loop
9. Phase 2
10. Phase 3
11. Phase 4
12. Phase 5

## Current stop point
Stopped after full plan closure.
Phase 1 run model through bounded recovery, Phase 2 graph/search improvements, Phase 3 tester evidence additions, Phase 4 cockpit queue/review/replay surfaces, and Phase 5 durable lessons memory now exist in code.

## Next exact step
No remaining step in this plan.
Post-2.6 backlog candidates:
- add CLI/MCP commands over `M-MEMORY`
- add dashboard lesson browser
- extend review approvals with signed reviewer identity
- split remaining near-limit large files before adding major runtime features

## Notes for future sessions
When resuming:
1. read this file first
2. confirm current git status
3. confirm docs/graph-index.xml, docs/plan-index.xml, docs/verification-index.xml still valid
4. continue from `Next exact step`
5. keep scope on autonomous-agent runtime before dashboard or integrations
