// MODULE_CONTRACT
// MODULE_ID: M-RUNNER
// PURPOSE: Run action queue - plans and executes bounded run control actions with replayable evidence, pre-commit gates, phase advance handoff, and self-heal handoff
// SCOPE: Action model, durable action plan persistence, next-action execution, bounded loops, pre-commit verification before final completion, optional phase advancement, self-heal handoff, retry handoff, and replay action results
// DEPENDS: M-RUNNER, M-RUNNER-PRECOMMIT, M-RUNNER-PHASE-ENGINE, M-RUNNER-SELF-HEAL, M-TRACKING
// LINKS:
//   -> V-M-RUNNER (verified_by) - action queue and executor tests
//   -> UC-002 (implements) - bounded run control from objective to action/replay loop

// START_MODULE_MAP
// RunActionKind - Stable action kinds for bounded run control
// RunActionStatus - Execution state for one planned action
// RunAction - One queued run action
// RunActionPlan - Durable action queue snapshot for a run
// RunActionExecution - Result of applying one action
// RunActionLoopResult - Result of bounded action loop execution
// plan_run_actions - Build and persist next actions for a run
// execute_next_action - Apply exactly one safe next action
// pre_commit action - Verify gates before final completion
// advance_phase action - Optional phase advancement for completed runs
// self_heal action - Diagnose verification failures through bounded self-heal runtime
// run_action_loop - Apply safe actions until terminal, blocked, or budget exhausted
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.3.0 - Simplified phase auto-advance guard for clippy-clean integration]
// END_CHANGE_SUMMARY

use super::{
    default_steps, write_provenance_event, RunManager, RunRecord, RunReplay, RunReviewStatus,
    RunStatus,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// START_public_api

// START_RunActionKind
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunActionKind {
    Start,
    PreCommitVerify,
    CompleteStep,
    AwaitReview,
    Resume,
    AdvancePhase,
    SelfHeal,
    RetryRecovery,
    Replay,
}
// END_RunActionKind

// START_RunActionStatus
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunActionStatus {
    Pending,
    Applied,
    Blocked,
    Failed,
    Skipped,
}
// END_RunActionStatus

// START_RunAction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunAction {
    pub id: String,
    pub kind: RunActionKind,
    pub status: RunActionStatus,
    pub description: String,
    pub evidence_ref: Option<String>,
    pub reason: Option<String>,
    pub created_at: String,
    pub applied_at: Option<String>,
    pub error: Option<String>,
}
// END_RunAction

// START_RunActionPlan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunActionPlan {
    pub run_id: String,
    pub objective: String,
    pub run_status: String,
    pub current_step: usize,
    pub actions: Vec<RunAction>,
    pub created_at: String,
    pub updated_at: String,
}
// END_RunActionPlan

// START_RunActionExecution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunActionExecution {
    pub action: RunAction,
    pub run: RunRecord,
    pub replay: RunReplay,
    pub next_plan: RunActionPlan,
    pub blocked: bool,
    pub message: String,
}
// END_RunActionExecution

// START_RunActionLoopResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunActionLoopResult {
    pub run_id: String,
    pub executions: Vec<RunActionExecution>,
    pub final_run: RunRecord,
    pub final_replay: RunReplay,
    pub stopped_reason: String,
}
// END_RunActionLoopResult

impl RunManager {
    // START_CONTRACT_RunManager::action_plan_path
    // PURPOSE: Resolve durable action-plan path for one run
    // INPUTS: { run_id: &str }
    // OUTPUTS: { PathBuf }
    // START_run_manager_action_plan_path
    pub fn action_plan_path(&self, run_id: &str) -> PathBuf {
        self.runs_dir().join(format!("{}.actions.json", run_id))
    }
    // END_run_manager_action_plan_path

    // START_CONTRACT_RunManager::save_action_plan
    // PURPOSE: Persist action queue snapshot for a bounded run
    // INPUTS: { plan: &RunActionPlan }
    // OUTPUTS: { anyhow::Result<()> }
    // START_run_manager_save_action_plan
    pub fn save_action_plan(&self, plan: &RunActionPlan) -> anyhow::Result<()> {
        self.ensure_runs_dir()?;
        let path = self.action_plan_path(&plan.run_id);
        let tmp = path.with_extension("actions.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(plan)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
    // END_run_manager_save_action_plan

    // START_CONTRACT_RunManager::load_action_plan
    // PURPOSE: Load a persisted action queue snapshot for a run
    // INPUTS: { run_id: &str }
    // OUTPUTS: { anyhow::Result<RunActionPlan> }
    // START_run_manager_load_action_plan
    pub fn load_action_plan(&self, run_id: &str) -> anyhow::Result<RunActionPlan> {
        let data = std::fs::read(self.action_plan_path(run_id))?;
        Ok(serde_json::from_slice(&data)?)
    }
    // END_run_manager_load_action_plan

    // START_CONTRACT_RunManager::plan_run_actions
    // PURPOSE: Build and persist the next safe action queue for a run from current durable state
    // INPUTS: { run_id: &str }
    // OUTPUTS: { anyhow::Result<RunActionPlan> }
    // LINKS:
    //   -> UC-002 (implements) - exposes explicit next actions instead of hidden autonomous jumps
    // START_run_manager_plan_run_actions
    pub fn plan_run_actions(&self, run_id: &str) -> anyhow::Result<RunActionPlan> {
        let mut record = self.load(run_id)?;
        if record.steps.is_empty() {
            record.steps = default_steps(&record.module_id);
            self.save(&record)?;
        }
        let now = chrono::Utc::now().to_rfc3339();
        let actions = plan_actions_for_record(&record, &now);
        let plan = RunActionPlan {
            run_id: record.run_id.clone(),
            objective: record.objective.clone(),
            run_status: format!("{:?}", record.status),
            current_step: record.current_step,
            actions,
            created_at: now.clone(),
            updated_at: now,
        };
        self.save_action_plan(&plan)?;
        Ok(plan)
    }
    // END_run_manager_plan_run_actions

    // START_CONTRACT_RunManager::execute_next_action
    // PURPOSE: Apply one safe queued run action and return updated run, replay, and next plan
    // INPUTS: { run_id: &str }
    // OUTPUTS: { anyhow::Result<RunActionExecution> }
    // LINKS:
    //   -> NFR-002 (traces_to) - blocked review gates must stop instead of auto-approving
    // START_run_manager_execute_next_action
    pub fn execute_next_action(&self, run_id: &str) -> anyhow::Result<RunActionExecution> {
        let mut plan = self.plan_run_actions(run_id)?;
        let mut action = plan
            .actions
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("run {} has no planned actions", run_id))?;
        if matches!(action.kind, RunActionKind::AwaitReview) {
            action.status = RunActionStatus::Blocked;
            action.applied_at = Some(chrono::Utc::now().to_rfc3339());
            plan.actions[0] = action.clone();
            plan.updated_at = chrono::Utc::now().to_rfc3339();
            self.save_action_plan(&plan)?;
            let run = self.load(run_id)?;
            return Ok(RunActionExecution {
                action,
                replay: run.replay(),
                next_plan: plan,
                blocked: true,
                message: "blocked run requires explicit approval or rejection".into(),
                run,
            });
        }
        if matches!(action.kind, RunActionKind::PreCommitVerify) {
            let report = self.pre_commit_verify(run_id)?;
            let run = self.load(run_id)?;
            let blocked = !report.passed;
            action.status = if blocked {
                RunActionStatus::Blocked
            } else {
                RunActionStatus::Applied
            };
            action.applied_at = Some(chrono::Utc::now().to_rfc3339());
            let next_plan = self.plan_run_actions(run_id)?;
            return Ok(RunActionExecution {
                action,
                replay: run.replay(),
                next_plan,
                blocked,
                message: if blocked {
                    "pre-commit verification failed; run completion blocked".into()
                } else {
                    "pre-commit verification passed".into()
                },
                run,
            });
        }
        if matches!(action.kind, RunActionKind::AdvancePhase) {
            let report = self.advance_phase(false)?;
            let mut run = self.load(run_id)?;
            run.metadata
                .insert("phase_advance_checked".into(), report.passed.to_string());
            run.metadata
                .insert("phase_advance_done".into(), report.advanced.to_string());
            if !report.passed {
                run.status = RunStatus::Blocked;
                run.blocked_reason = Some("phase gates blocked automatic advance".into());
            }
            run.touch();
            self.save(&run)?;
            let blocked = !report.passed;
            action.status = if blocked {
                RunActionStatus::Blocked
            } else {
                RunActionStatus::Applied
            };
            action.applied_at = Some(chrono::Utc::now().to_rfc3339());
            let next_plan = self.plan_run_actions(run_id)?;
            return Ok(RunActionExecution {
                action,
                replay: run.replay(),
                next_plan,
                blocked,
                message: if blocked {
                    "phase gates blocked automatic advance".into()
                } else {
                    "phase advanced".into()
                },
                run,
            });
        }

        let applied_run = match action.kind {
            RunActionKind::Start => self.start_run(run_id)?,
            RunActionKind::CompleteStep => {
                self.complete_current_step(run_id, action.evidence_ref.as_deref())?
            }
            RunActionKind::Resume => self.resume_run(run_id)?,
            RunActionKind::SelfHeal => {
                let record = self.load(run_id)?;
                let profile = record
                    .metadata
                    .get("verify_profile")
                    .map(String::as_str)
                    .unwrap_or("strict")
                    .to_string();
                self.execute_self_heal(run_id, &profile)?;
                self.load(run_id)?
            }
            RunActionKind::RetryRecovery => {
                self.attempt_recovery(run_id)?;
                self.load(run_id)?
            }
            RunActionKind::Replay => self.load(run_id)?,
            RunActionKind::PreCommitVerify
            | RunActionKind::AdvancePhase
            | RunActionKind::AwaitReview => {
                anyhow::bail!("Action kind {:?} must be executed through the phase/pre-commit/review gate flow", action.kind);
            }
        };
        action.status = RunActionStatus::Applied;
        action.applied_at = Some(chrono::Utc::now().to_rfc3339());
        write_provenance_event(
            &applied_run.run_id,
            &applied_run.module_id,
            &applied_run.phase,
            &applied_run.status,
            "execute_run_action",
            &format!("action={} kind={:?}", action.id, action.kind),
        );
        let next_plan = self.plan_run_actions(run_id)?;
        Ok(RunActionExecution {
            action,
            replay: applied_run.replay(),
            next_plan,
            blocked: false,
            message: "action applied".into(),
            run: applied_run,
        })
    }
    // END_run_manager_execute_next_action

    // START_CONTRACT_RunManager::run_action_loop
    // PURPOSE: Apply safe queued actions until the run is terminal, blocked, or action budget is exhausted
    // INPUTS: { run_id: &str }, { max_actions: usize }
    // OUTPUTS: { anyhow::Result<RunActionLoopResult> }
    // LINKS:
    //   -> NFR-002 (traces_to) - loop is bounded by explicit action budget
    // START_run_manager_run_action_loop
    pub fn run_action_loop(
        &self,
        run_id: &str,
        max_actions: usize,
    ) -> anyhow::Result<RunActionLoopResult> {
        let budget = max_actions.max(1);
        let mut executions = Vec::new();
        let mut stopped_reason = format!("action budget exhausted: {}", budget);
        for _ in 0..budget {
            let execution = self.execute_next_action(run_id)?;
            let terminal = matches!(
                execution.run.status,
                RunStatus::Completed | RunStatus::Failed | RunStatus::Escalated
            ) || matches!(execution.action.kind, RunActionKind::Replay);
            let blocked = execution.blocked;
            if blocked {
                stopped_reason = execution.message.clone();
            } else if terminal {
                stopped_reason = format!("terminal status: {:?}", execution.run.status);
            }
            executions.push(execution);
            if blocked || terminal {
                break;
            }
        }
        let final_run = self.load(run_id)?;
        Ok(RunActionLoopResult {
            run_id: run_id.to_string(),
            executions,
            final_replay: final_run.replay(),
            final_run,
            stopped_reason,
        })
    }
    // END_run_manager_run_action_loop
}

fn plan_actions_for_record(record: &RunRecord, now: &str) -> Vec<RunAction> {
    let action = match record.status {
        RunStatus::Planned | RunStatus::Ready => RunAction::pending(
            "start-run",
            RunActionKind::Start,
            "Start run after gate policy has allowed execution",
            None,
            None,
            now,
        ),
        RunStatus::Running => {
            if should_self_heal(record) {
                return vec![RunAction::pending(
                    "self-heal",
                    RunActionKind::SelfHeal,
                    "Run bounded self-heal for failing verification context",
                    Some("action://self-heal".into()),
                    record.blocked_reason.clone(),
                    now,
                )];
            }
            if is_final_step(record) && !pre_commit_verified(record) {
                return vec![RunAction::pending(
                    "pre-commit-verify",
                    RunActionKind::PreCommitVerify,
                    "Run pre-commit verification before final completion",
                    Some("pre-commit://verify".into()),
                    None,
                    now,
                )];
            }
            let step = record.steps.get(record.current_step);
            let name = step
                .map(|step| step.name.as_str())
                .unwrap_or("unknown-step");
            RunAction::pending(
                &format!("complete-step-{}", record.current_step + 1),
                RunActionKind::CompleteStep,
                &format!("Complete current run step {}", name),
                Some(format!("action://{}", name.replace('-', "_"))),
                None,
                now,
            )
        }
        RunStatus::Blocked => {
            if should_self_heal(record) {
                RunAction::pending(
                    "self-heal",
                    RunActionKind::SelfHeal,
                    "Run bounded self-heal for blocked verification failure",
                    Some("action://self-heal".into()),
                    record.blocked_reason.clone(),
                    now,
                )
            } else if matches!(
                record.latest_review_status(),
                Some(RunReviewStatus::Approved)
            ) {
                RunAction::pending(
                    "resume-approved-run",
                    RunActionKind::Resume,
                    "Resume blocked run after explicit approval",
                    Some("action://resume-approved".into()),
                    None,
                    now,
                )
            } else {
                RunAction::pending(
                    "await-review",
                    RunActionKind::AwaitReview,
                    "Await explicit approve/reject decision before continuing",
                    None,
                    record.blocked_reason.clone(),
                    now,
                )
            }
        }
        RunStatus::Failed => {
            if should_self_heal(record) {
                RunAction::pending(
                    "self-heal",
                    RunActionKind::SelfHeal,
                    "Run bounded self-heal for failed verification run",
                    Some("action://self-heal".into()),
                    record.blocked_reason.clone(),
                    now,
                )
            } else {
                RunAction::pending(
                    "retry-recovery",
                    RunActionKind::RetryRecovery,
                    "Attempt bounded recovery if retry budget remains",
                    Some("action://retry-recovery".into()),
                    record.blocked_reason.clone(),
                    now,
                )
            }
        }
        RunStatus::Completed
            if record
                .metadata
                .get("phase_auto_advance")
                .map(String::as_str)
                == Some("true")
                && record
                    .metadata
                    .get("phase_advance_done")
                    .map(String::as_str)
                    != Some("true") =>
        {
            RunAction::pending(
                "advance-phase",
                RunActionKind::AdvancePhase,
                "Advance active MyGRACE phase after completed run",
                Some("phase://advance".into()),
                None,
                now,
            )
        }
        RunStatus::Completed | RunStatus::Escalated => RunAction::pending(
            "replay-run",
            RunActionKind::Replay,
            "Replay durable run timeline",
            None,
            None,
            now,
        ),
    };
    vec![action]
}

// START_CONTRACT_is_final_step
// PURPOSE: Detect whether a running record is about to complete its last bounded step
// INPUTS: { record: &RunRecord }
// OUTPUTS: { bool }
// START_is_final_step
fn is_final_step(record: &RunRecord) -> bool {
    !record.steps.is_empty() && record.current_step >= record.steps.len().saturating_sub(1)
}
// END_is_final_step

// START_CONTRACT_pre_commit_verified
// PURPOSE: Detect whether current run already passed pre-commit verification
// INPUTS: { record: &RunRecord }
// OUTPUTS: { bool }
// START_pre_commit_verified
fn pre_commit_verified(record: &RunRecord) -> bool {
    record
        .metadata
        .get("pre_commit_verified")
        .is_some_and(|value| value == "true")
}
// END_pre_commit_verified

// START_CONTRACT_should_self_heal
// PURPOSE: Decide whether the next safe run action should enter bounded self-heal
// INPUTS: { record: &RunRecord }
// OUTPUTS: { bool }
// START_should_self_heal
fn should_self_heal(record: &RunRecord) -> bool {
    if record
        .metadata
        .get("self_heal_diagnosis_ready")
        .is_some_and(|value| value == "true")
    {
        return false;
    }
    if record
        .metadata
        .get("self_heal_requested")
        .is_some_and(|value| value == "true")
        || record
            .metadata
            .get("suggested_action")
            .is_some_and(|value| value == "self_heal")
    {
        return true;
    }
    record
        .blocked_reason
        .as_ref()
        .or_else(|| {
            record
                .outcome
                .as_ref()
                .and_then(|outcome| outcome.blocked_reason.as_ref())
        })
        .is_some_and(|reason| reason.to_ascii_lowercase().contains("verification"))
}
// END_should_self_heal

impl RunAction {
    fn pending(
        id: &str,
        kind: RunActionKind,
        description: &str,
        evidence_ref: Option<String>,
        reason: Option<String>,
        now: &str,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            status: RunActionStatus::Pending,
            description: description.into(),
            evidence_ref,
            reason,
            created_at: now.into(),
            applied_at: None,
            error: None,
        }
    }
}

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{RunGate, RunGateStatus, RunReviewStatus, RunStepStatus};

    fn ready_run() -> RunRecord {
        let mut record = RunRecord::new(
            "goal".into(),
            "Phase-23".into(),
            "M-RUNNER".into(),
            "drive action queue".into(),
        );
        record.steps = default_steps("M-RUNNER");
        record.status = RunStatus::Ready;
        record.required_gates.push(RunGate {
            id: "gate-verification".into(),
            name: "verification".into(),
            required: true,
            status: RunGateStatus::Passed,
            reason: None,
            evidence_refs: vec!["syn verify".into()],
        });
        record
    }

    #[test]
    fn action_loop_completes_ready_run_with_evidence() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let record = ready_run();
        let run_id = record.run_id.clone();
        manager.save(&record).unwrap();

        let result = manager.run_action_loop(&run_id, 8).unwrap();

        assert_eq!(result.final_run.status, RunStatus::Completed);
        assert_eq!(result.executions.len(), 6);
        assert!(result
            .final_run
            .steps
            .iter()
            .all(|step| matches!(step.status, RunStepStatus::Passed)));
        assert!(result.final_run.evidence_refs.len() >= 4);
        assert!(manager.action_plan_path(&run_id).exists());
    }

    // START_CONTRACT_test_pre_commit_blocks_final_completion
    // PURPOSE: Verify PreCommitVerify blocks a final CompleteStep when required gates failed.
    // START_test_pre_commit_blocks_final_completion
    #[test]
    fn test_pre_commit_blocks_final_completion() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let mut record = ready_run();
        record.status = RunStatus::Running;
        record.current_step = record.steps.len().saturating_sub(1);
        record.steps[record.current_step].status = RunStepStatus::Running;
        record.required_gates[0].status = RunGateStatus::Failed;
        record.required_gates[0].reason = Some("verification failed".into());
        manager.save(&record).unwrap();

        let result = manager.execute_next_action(&record.run_id).unwrap();
        let loaded = manager.load(&record.run_id).unwrap();

        assert!(result.blocked);
        assert_eq!(result.action.kind, RunActionKind::PreCommitVerify);
        assert_eq!(loaded.status, RunStatus::Blocked);
        assert_ne!(
            loaded.outcome.as_ref().map(|outcome| &outcome.kind),
            Some(&crate::run::RunOutcomeKind::Success)
        );
    }
    // END_test_pre_commit_blocks_final_completion

    #[test]
    fn blocked_run_without_review_stops_at_await_review_action() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let mut record = ready_run();
        record.status = RunStatus::Blocked;
        record.blocked_reason = Some("human review required".into());
        manager.save(&record).unwrap();

        let result = manager.execute_next_action(&record.run_id).unwrap();

        assert!(result.blocked);
        assert_eq!(result.action.kind, RunActionKind::AwaitReview);
        assert_eq!(result.action.status, RunActionStatus::Blocked);
        assert_eq!(result.run.status, RunStatus::Blocked);
    }

    #[test]
    fn approved_blocked_run_resumes_then_loop_completes() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let mut record = ready_run();
        record.status = RunStatus::Ready;
        manager.save(&record).unwrap();
        manager.start_run(&record.run_id).unwrap();
        manager
            .block_run(
                &record.run_id,
                "review checkpoint",
                Some("action://blocked"),
            )
            .unwrap();
        let reviewed = manager
            .approve_run(
                &record.run_id,
                "unit-test",
                "approved bounded resume",
                Some("action://approval"),
            )
            .unwrap();
        assert_eq!(
            reviewed.latest_review_status(),
            Some(RunReviewStatus::Approved)
        );

        let result = manager.run_action_loop(&record.run_id, 8).unwrap();

        assert_eq!(result.final_run.status, RunStatus::Completed);
        assert!(result
            .executions
            .iter()
            .any(|execution| execution.action.kind == RunActionKind::Resume));
    }

    #[test]
    // START_CONTRACT_verification_blocked_run_plans_self_heal_action
    // PURPOSE: Verify blocked verification runs enter SelfHeal before human-review waiting
    // START_verification_blocked_run_plans_self_heal_action
    fn verification_blocked_run_plans_self_heal_action() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let mut record = ready_run();
        record.status = RunStatus::Blocked;
        record.blocked_reason = Some("verification gate blocked".into());
        manager.save(&record).unwrap();

        let plan = manager.plan_run_actions(&record.run_id).unwrap();

        assert_eq!(plan.actions[0].kind, RunActionKind::SelfHeal);
    }
    // END_verification_blocked_run_plans_self_heal_action

    #[test]
    // START_CONTRACT_action_plan_persists_for_replay
    // PURPOSE: Verify action plan serialization round-trips for replay debugging.
    // START_action_plan_persists_for_replay
    fn action_plan_persists_for_replay() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let record = ready_run();
        let run_id = record.run_id.clone();
        manager.save(&record).unwrap();

        let plan = manager.plan_run_actions(&run_id).unwrap();
        manager.save_action_plan(&plan).unwrap();
        let restored = manager.load_action_plan(&run_id).unwrap();

        assert_eq!(plan.actions.len(), restored.actions.len());
        assert_eq!(plan.run_id, restored.run_id);
    }
    // END_action_plan_persists_for_replay

}
