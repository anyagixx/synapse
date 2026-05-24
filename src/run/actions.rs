// MODULE_CONTRACT
// MODULE_ID: M-RUNNER
// PURPOSE: Run action queue - plans and executes bounded run control actions with replayable evidence
// SCOPE: Action model, durable action plan persistence, next-action execution, bounded loops, retry handoff, and replay action results
// DEPENDS: M-RUNNER, M-TRACKING
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
// run_action_loop - Apply safe actions until terminal, blocked, or budget exhausted
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added durable run action queue executor]
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
    CompleteStep,
    AwaitReview,
    Resume,
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

        let applied_run = match action.kind {
            RunActionKind::Start => self.start_run(run_id)?,
            RunActionKind::CompleteStep => {
                self.complete_current_step(run_id, action.evidence_ref.as_deref())?
            }
            RunActionKind::Resume => self.resume_run(run_id)?,
            RunActionKind::RetryRecovery => {
                self.attempt_recovery(run_id)?;
                self.load(run_id)?
            }
            RunActionKind::Replay => self.load(run_id)?,
            RunActionKind::AwaitReview => unreachable!(),
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
            if matches!(
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
        RunStatus::Failed => RunAction::pending(
            "retry-recovery",
            RunActionKind::RetryRecovery,
            "Attempt bounded recovery if retry budget remains",
            Some("action://retry-recovery".into()),
            record.blocked_reason.clone(),
            now,
        ),
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
    use crate::run::{RunReviewStatus, RunStepStatus};

    fn ready_run() -> RunRecord {
        let mut record = RunRecord::new(
            "goal".into(),
            "Phase-23".into(),
            "M-RUNNER".into(),
            "drive action queue".into(),
        );
        record.steps = default_steps("M-RUNNER");
        record.status = RunStatus::Ready;
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
        assert_eq!(result.executions.len(), 5);
        assert!(result
            .final_run
            .steps
            .iter()
            .all(|step| matches!(step.status, RunStepStatus::Passed)));
        assert!(result.final_run.evidence_refs.len() >= 4);
        assert!(manager.action_plan_path(&run_id).exists());
    }

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
}
