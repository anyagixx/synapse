// MODULE_CONTRACT
// MODULE_ID: M-WORKSPACE-RUN
// PURPOSE: Run crate root — autonomous bounded-run runtime with phase gates, pre-commit checks, self-heal cycles, and structured handoffs
// DEPENDS: M-WORKSPACE-CORE, M-WORKSPACE-ENGINE
// LINKS:
//   → Phase-97 (implements) — workspace split
//   ← V-M-WORKSPACE-RUN (verified_by) — verification shard

#![allow(clippy::if_same_then_else, clippy::new_without_default)]

// START_MODULE_MAP
// run — Run state machine, step/gate/review models, action queue, phase/pre-commit gates, self-heal, agent context, handoffs, scenarios
// RunManager, RunRecord, RunStatus, RunGate, RunGateStatus — Re-exported core types
// agent_context, scenario, actions, self_heal, handoff, phase, pre_commit — Re-exported sub-modules
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Extracted from synapse-agent monolith into syn-run workspace crate]
// END_CHANGE_SUMMARY

pub mod run;

// START_CONTRACT_public_api
// PURPOSE: Re-export core run types for downstream consumers
// OUTPUTS: { RunManager, RunRecord, RunReplayEvent, RunStatus, RunGate, RunGateStatus, agent_context, scenario, actions, self_heal, handoff, phase, pre_commit }
// LINKS:
//   → NFR-002 (traces_to) — type exports enable reliable downstream usage
// START_public_api
pub use run::RunManager;
pub use run::RunRecord;
pub use run::RunReplayEvent;
pub use run::RunStatus;
pub use run::RunGate;
pub use run::RunGateStatus;
pub use run::agent_context;
pub use run::scenario;
pub use run::actions;
pub use run::self_heal;
pub use run::handoff;
pub use run::phase;
pub use run::pre_commit;
// END_public_api
