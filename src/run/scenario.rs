// MODULE_CONTRACT
// MODULE_ID: M-RUNNER
// PURPOSE: Autonomous run scenario harness - drives bounded runs through gates, evidence, review, completion, and replay
// SCOPE: Scenario request/result model, happy-path execution, blocked-review execution, and deterministic replay evidence
// DEPENDS: M-RUNNER, M-GRACE-STATUS, M-TRACKING
// LINKS:
//   -> V-M-RUNNER (verified_by) - autonomous E2E scenario tests
//   -> UC-002 (implements) - bounded objective to gate/action/evidence/replay workflow

// START_MODULE_MAP
// RunScenarioMode - Selects happy-path or blocked-review scenario behavior
// RunScenarioRequest - Bounded scenario input contract
// RunScenarioResult - Persisted scenario outcome and replay evidence
// run_scenario_from_report - Build gates from project status and drive the scenario
// run_scenario_with_policy - Drive the scenario from a supplied gate policy for deterministic tests
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added autonomous E2E run scenario harness]
// END_CHANGE_SUMMARY

use super::{
    default_steps, write_provenance_event, RunGateDecision, RunGatePolicy, RunManager, RunRecord,
    RunReplay, RunStatus,
};
use serde::{Deserialize, Serialize};

// START_public_api

// START_RunScenarioMode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunScenarioMode {
    HappyPath,
    BlockedReview,
}
// END_RunScenarioMode

impl RunScenarioMode {
    // START_CONTRACT_RunScenarioMode::parse
    // PURPOSE: Parse a stable CLI scenario name into a mode
    // INPUTS: { value: &str }
    // OUTPUTS: { anyhow::Result<RunScenarioMode> }
    // START_run_scenario_mode_parse
    pub fn parse(value: &str) -> anyhow::Result<Self> {
        let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
        match normalized.as_str() {
            "" | "happy" | "happy-path" | "e2e" => Ok(Self::HappyPath),
            "blocked" | "blocked-review" | "review" => Ok(Self::BlockedReview),
            other => anyhow::bail!(
                "unknown run scenario '{}'; expected happy or blocked-review",
                other
            ),
        }
    }
    // END_run_scenario_mode_parse

    // START_CONTRACT_RunScenarioMode::as_str
    // PURPOSE: Return a stable metadata label for a scenario mode
    // OUTPUTS: { &'static str }
    // START_run_scenario_mode_as_str
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HappyPath => "happy",
            Self::BlockedReview => "blocked-review",
        }
    }
    // END_run_scenario_mode_as_str
}

// START_RunScenarioRequest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunScenarioRequest {
    pub goal: String,
    pub phase: String,
    pub module_id: String,
    pub objective: String,
    pub mode: RunScenarioMode,
}
// END_RunScenarioRequest

impl RunScenarioRequest {
    // START_CONTRACT_RunScenarioRequest::new
    // PURPOSE: Create a bounded autonomous scenario request with explicit scope fields
    // INPUTS: { goal: impl Into<String> }, { phase: impl Into<String> }, { module_id: impl Into<String> }, { objective: impl Into<String> }, { mode: RunScenarioMode }
    // OUTPUTS: { RunScenarioRequest }
    // START_run_scenario_request_new
    pub fn new(
        goal: impl Into<String>,
        phase: impl Into<String>,
        module_id: impl Into<String>,
        objective: impl Into<String>,
        mode: RunScenarioMode,
    ) -> Self {
        Self {
            goal: goal.into(),
            phase: phase.into(),
            module_id: module_id.into(),
            objective: objective.into(),
            mode,
        }
    }
    // END_run_scenario_request_new
}

// START_RunScenarioResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunScenarioResult {
    pub mode: RunScenarioMode,
    pub decision: RunGateDecision,
    pub run: RunRecord,
    pub replay: RunReplay,
    pub evidence_refs: Vec<String>,
    pub blocked_before_review: bool,
}
// END_RunScenarioResult

impl RunManager {
    // START_CONTRACT_RunManager::run_scenario_from_report
    // PURPOSE: Build gate policy from project health and drive a bounded objective through execution or an explicit block
    // INPUTS: { request: RunScenarioRequest }, { report: &crate::grace::status::StatusReport }
    // OUTPUTS: { anyhow::Result<RunScenarioResult> }
    // LINKS:
    //   -> UC-002 (implements) - objective to run/gates/action/evidence/replay scenario
    //   -> NFR-002 (traces_to) - scenario stops on failed gates with persisted reason
    // START_run_manager_run_scenario_from_report
    pub fn run_scenario_from_report(
        &self,
        request: RunScenarioRequest,
        report: &crate::grace::status::StatusReport,
    ) -> anyhow::Result<RunScenarioResult> {
        let RunScenarioRequest {
            goal,
            phase,
            module_id,
            objective,
            mode,
        } = request;
        let (record, decision) = self.create_run(&goal, &phase, &module_id, &objective, report)?;
        self.drive_scenario(mode, record, decision)
    }
    // END_run_manager_run_scenario_from_report

    // START_CONTRACT_RunManager::run_scenario_with_policy
    // PURPOSE: Drive a bounded scenario using a supplied gate policy for deterministic verification
    // INPUTS: { request: RunScenarioRequest }, { policy: RunGatePolicy }
    // OUTPUTS: { anyhow::Result<RunScenarioResult> }
    // LINKS:
    //   -> V-M-RUNNER (verified_by) - unit tests avoid full project status collection
    // START_run_manager_run_scenario_with_policy
    pub fn run_scenario_with_policy(
        &self,
        request: RunScenarioRequest,
        policy: RunGatePolicy,
    ) -> anyhow::Result<RunScenarioResult> {
        let RunScenarioRequest {
            goal,
            phase,
            module_id,
            objective,
            mode,
        } = request;
        let mut record = RunRecord::new(goal, phase, module_id.clone(), objective);
        record.steps = default_steps(&module_id);
        let decision = self.attach_gate_policy(&mut record, policy);
        self.save(&record)?;
        write_provenance_event(
            &record.run_id,
            &record.module_id,
            &record.phase,
            &record.status,
            "run_scenario_create",
            &format!("mode={} blocked={}", mode.as_str(), decision.blocked),
        );
        self.drive_scenario(mode, record, decision)
    }
    // END_run_manager_run_scenario_with_policy

    // START_CONTRACT_RunManager::drive_scenario
    // PURPOSE: Execute a created scenario run through start, optional review block, completion, and replay
    // INPUTS: { mode: RunScenarioMode }, { record: RunRecord }, { decision: RunGateDecision }
    // OUTPUTS: { anyhow::Result<RunScenarioResult> }
    // START_run_manager_drive_scenario
    fn drive_scenario(
        &self,
        mode: RunScenarioMode,
        mut record: RunRecord,
        decision: RunGateDecision,
    ) -> anyhow::Result<RunScenarioResult> {
        record
            .metadata
            .insert("scenario_mode".into(), mode.as_str().into());
        record.metadata.insert(
            "scenario_contract".into(),
            "bounded_objective_to_replay".into(),
        );
        record.touch();
        self.save(&record)?;
        if decision.blocked {
            return Ok(scenario_result(mode, decision, record, false));
        }

        let run_id = record.run_id.clone();
        let mut current = self.start_run(&run_id)?;
        let blocked_before_review = if matches!(mode, RunScenarioMode::BlockedReview) {
            let stepped = self.complete_current_step(&run_id, Some("scenario://read-artifacts"))?;
            let blocked = self.block_run(
                &stepped.run_id,
                "scenario review checkpoint before bounded change",
                Some("scenario://blocked-review"),
            )?;
            let reviewed = self.approve_run(
                &blocked.run_id,
                "scenario",
                "bounded review approved for deterministic replay",
                Some("scenario://approval"),
            )?;
            current = self.resume_run(&reviewed.run_id)?;
            true
        } else {
            false
        };
        current = self.complete_remaining_scenario_steps(&run_id, current)?;
        write_provenance_event(
            &current.run_id,
            &current.module_id,
            &current.phase,
            &current.status,
            "run_scenario_complete",
            &format!(
                "mode={} evidence={}",
                mode.as_str(),
                current.evidence_refs.len()
            ),
        );
        Ok(scenario_result(
            mode,
            decision,
            current,
            blocked_before_review,
        ))
    }
    // END_run_manager_drive_scenario

    // START_CONTRACT_RunManager::complete_remaining_scenario_steps
    // PURPOSE: Complete all remaining scenario steps with deterministic evidence references
    // INPUTS: { run_id: &str }, { record: RunRecord }
    // OUTPUTS: { anyhow::Result<RunRecord> }
    // START_run_manager_complete_remaining_scenario_steps
    fn complete_remaining_scenario_steps(
        &self,
        run_id: &str,
        mut record: RunRecord,
    ) -> anyhow::Result<RunRecord> {
        while matches!(record.status, RunStatus::Running | RunStatus::Ready) {
            let step_name = record
                .steps
                .get(record.current_step)
                .map(|step| step.name.replace('-', "_"))
                .unwrap_or_else(|| "unknown_step".into());
            let evidence = format!("scenario://{}", step_name);
            record = self.complete_current_step(run_id, Some(&evidence))?;
        }
        if !matches!(record.status, RunStatus::Completed) {
            anyhow::bail!(
                "scenario run {} ended as {:?}, expected Completed",
                record.run_id,
                record.status
            );
        }
        Ok(record)
    }
    // END_run_manager_complete_remaining_scenario_steps
}

fn scenario_result(
    mode: RunScenarioMode,
    decision: RunGateDecision,
    run: RunRecord,
    blocked_before_review: bool,
) -> RunScenarioResult {
    let replay = run.replay();
    let evidence_refs = run.evidence_refs.clone();
    RunScenarioResult {
        mode,
        decision,
        run,
        replay,
        evidence_refs,
        blocked_before_review,
    }
}

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{RunGate, RunGateStatus, RunReviewStatus};

    fn policy(status: RunGateStatus) -> RunGatePolicy {
        RunGatePolicy {
            phase: "Phase-22".into(),
            module_id: "M-RUNNER".into(),
            required_gates: vec![RunGate {
                id: "gate-scenario".into(),
                name: "scenario".into(),
                required: true,
                status,
                reason: Some("scenario gate blocked".into()),
                evidence_refs: vec!["scenario://gate".into()],
            }],
            stop_on_block: true,
            retry_budget: 1,
            escalation_target: "human-review".into(),
        }
    }

    fn request(mode: RunScenarioMode) -> RunScenarioRequest {
        RunScenarioRequest::new(
            "prove autonomous runtime",
            "Phase-22",
            "M-RUNNER",
            "bounded objective to replay",
            mode,
        )
    }

    #[test]
    fn happy_path_scenario_completes_with_replay_evidence() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());

        let result = manager
            .run_scenario_with_policy(
                request(RunScenarioMode::HappyPath),
                policy(RunGateStatus::Passed),
            )
            .unwrap();

        assert_eq!(result.run.status, RunStatus::Completed);
        assert!(result.decision.passed);
        assert!(!result.blocked_before_review);
        assert_eq!(result.run.steps.len(), 4);
        assert!(result
            .run
            .steps
            .iter()
            .all(|step| matches!(step.status, super::super::RunStepStatus::Passed)));
        assert!(result
            .replay
            .events
            .iter()
            .any(|event| event.kind == "outcome"));
        assert!(result.evidence_refs.len() >= 4);
    }

    #[test]
    fn blocked_review_scenario_requires_approval_then_completes() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());

        let result = manager
            .run_scenario_with_policy(
                request(RunScenarioMode::BlockedReview),
                policy(RunGateStatus::Passed),
            )
            .unwrap();

        assert_eq!(result.run.status, RunStatus::Completed);
        assert!(result.blocked_before_review);
        assert_eq!(
            result.run.latest_review_status(),
            Some(RunReviewStatus::Approved)
        );
        assert!(result
            .replay
            .events
            .iter()
            .any(|event| event.kind == "review"));
        assert!(result
            .evidence_refs
            .iter()
            .any(|evidence| evidence == "scenario://blocked-review"));
    }

    #[test]
    fn scenario_stops_before_action_when_gate_is_blocked() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());

        let result = manager
            .run_scenario_with_policy(
                request(RunScenarioMode::HappyPath),
                policy(RunGateStatus::Blocked),
            )
            .unwrap();

        assert_eq!(result.run.status, RunStatus::Blocked);
        assert!(result.decision.blocked);
        assert!(!result.blocked_before_review);
        assert!(result
            .replay
            .events
            .iter()
            .any(|event| event.kind == "gate" && event.status == "Blocked"));
        assert!(result
            .run
            .steps
            .iter()
            .all(|step| matches!(step.status, super::super::RunStepStatus::Pending)));
    }
}
