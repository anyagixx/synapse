// MODULE_CONTRACT
// MODULE_ID: M-RUNNER
// PURPOSE: Run replay timeline — converts persisted run state into deterministic replay events
// SCOPE: Replay model, state/gate/step/review/outcome event assembly, evidence reference preservation
// DEPENDS: M-RUNNER
// LINKS:
//   → V-M-RUNNER (verified_by) - replay event ordering and evidence tests

// START_MODULE_MAP
// RunReplay — Replayable bounded-run timeline
// RunReplayEvent — One replay timeline event
// replay — Assemble replay events from persisted run state
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Added deterministic run replay model]
// END_CHANGE_SUMMARY

use super::RunRecord;
use serde::{Deserialize, Serialize};

// START_public_api

// START_RunReplayEvent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunReplayEvent {
    pub kind: String,
    pub status: String,
    pub detail: String,
    pub evidence_refs: Vec<String>,
}
// END_RunReplayEvent

// START_RunReplay
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunReplay {
    pub run_id: String,
    pub status: String,
    pub events: Vec<RunReplayEvent>,
}
// END_RunReplay

impl RunRecord {
    // START_CONTRACT_RunRecord::replay
    // PURPOSE: Assemble a deterministic replay timeline from run state, gates, steps, reviews, and outcome
    // OUTPUTS: { RunReplay }
    // LINKS:
    //   → UC-002 (implements) - replay gives maintainers evidence for bounded autonomous decisions
    //   → NFR-003 (traces_to) - replayable state reduces context recovery overhead
    // START_run_record_replay
    pub fn replay(&self) -> RunReplay {
        let mut events = vec![RunReplayEvent {
            kind: "state".into(),
            status: format!("{:?}", self.status),
            detail: self
                .blocked_reason
                .clone()
                .unwrap_or_else(|| self.objective.clone()),
            evidence_refs: self.evidence_refs.clone(),
        }];
        events.extend(self.required_gates.iter().map(|gate| RunReplayEvent {
            kind: "gate".into(),
            status: format!("{:?}", gate.status),
            detail: gate.reason.clone().unwrap_or_else(|| gate.name.clone()),
            evidence_refs: gate.evidence_refs.clone(),
        }));
        events.extend(self.steps.iter().map(|step| {
            RunReplayEvent {
                kind: "step".into(),
                status: format!("{:?}", step.status),
                detail: step
                    .error
                    .clone()
                    .unwrap_or_else(|| step.description.clone()),
                evidence_refs: step.evidence_refs.clone(),
            }
        }));
        events.extend(self.review_decisions.iter().map(|decision| RunReplayEvent {
            kind: "review".into(),
            status: format!("{:?}", decision.status),
            detail: format!("{}: {}", decision.reviewer, decision.reason),
            evidence_refs: decision.evidence_refs.clone(),
        }));
        if let Some(outcome) = &self.outcome {
            events.push(RunReplayEvent {
                kind: "outcome".into(),
                status: format!("{:?}", outcome.kind),
                detail: outcome.summary.clone(),
                evidence_refs: Vec::new(),
            });
        }
        RunReplay {
            run_id: self.run_id.clone(),
            status: format!("{:?}", self.status),
            events,
        }
    }
    // END_run_record_replay
}

// END_public_api
