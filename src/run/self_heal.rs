// MODULE_CONTRACT
// MODULE_ID: M-RUNNER-SELF-HEAL
// PURPOSE: Bounded self-heal runtime that verifies a run, diagnoses failures, persists repair context, creates fixer handoffs, delegates safe contract repairs, and escalates when retry budget is exhausted
// SCOPE: SelfHealPlan, SelfHealDiagnosis, SelfHealResult, metadata persistence helpers, periodic evidence compaction, verification execution, diagnosis capture, verifier-to-fixer handoff creation, dry-run repair action capture, and bounded escalation
// DEPENDS: M-RUNNER, M-RUNNER-HANDOFF, M-GRACE, M-GRACE-CONTRACT-GENERATOR, M-GRACE-FIX, M-GRACE-VERIFY-TYPES
// LINKS:
//   -> M-RUNNER (depends) - persists self-heal state inside durable run records
//   -> M-GRACE (depends) - runs profile-aware verification
//   -> M-GRACE-CONTRACT-GENERATOR (depends) - builds dry-run contract repair actions
//   -> M-GRACE-FIX (depends) - turns verification failures into indexed diagnoses
//   -> UC-002 (implements) - verifies and reviews bounded autonomous changes
//   -> NFR-002 (traces_to) - self-heal loops must stop with explicit failures
//   <- V-M-RUNNER-SELF-HEAL (verified_by) - self-heal runtime tests

// START_MODULE_MAP
// SelfHealPlan - Persisted bounded self-heal plan embedded in run metadata
// SelfHealDiagnosis - One diagnosed verification failure group with optional repair actions
// SelfHealResult - Tool/action result returned after one self-heal iteration
// RunManager::load_self_heal_plan - Reads self-heal metadata from a run record
// RunManager::save_self_heal_plan - Persists self-heal metadata to a run record
// RunManager::execute_self_heal - Runs one bounded verify/diagnose/escalate iteration
// should_compact_self_heal_evidence - Detects periodic evidence compaction checkpoints
// self-heal handoff - Creates verifier-to-fixer handoff when diagnoses remain unresolved
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.3.1 - Replaced manual modulo check for release clippy gate]
// END_CHANGE_SUMMARY

use super::handoff::HandoffRole;
use super::{write_provenance_event, RunManager, RunOutcome, RunOutcomeKind, RunRecord, RunStatus};
use crate::grace::contract_generator::{ContractGenerator, ContractRepairResult};
use crate::grace::verify::VerificationResult;
use crate::grace::{GraceEngine, GraceProfile};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SELF_HEAL_PLAN_KEY: &str = "self_heal_plan";
const SELF_HEAL_REQUESTED_KEY: &str = "self_heal_requested";
const SELF_HEAL_DIAGNOSIS_READY_KEY: &str = "self_heal_diagnosis_ready";
const SELF_HEAL_SUGGESTED_ACTION_KEY: &str = "suggested_action";
const SELF_HEAL_EVIDENCE_COMPACTION_INTERVAL: u32 = 5;

// START_public_api

// START_SelfHealPlan
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfHealPlan {
    pub run_id: String,
    pub profile: String,
    pub iteration: u32,
    pub max_iterations: u32,
    pub failed_groups: Vec<String>,
    pub diagnoses: Vec<SelfHealDiagnosis>,
    pub updated_at: String,
}
// END_SelfHealPlan

// START_SelfHealDiagnosis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfHealDiagnosis {
    pub failure_group: String,
    pub failed_checks: Vec<String>,
    pub related_modules: Vec<String>,
    pub suggested_blocks: Vec<String>,
    pub diagnosis: String,
    pub fix_applied: bool,
    pub fix_evidence_ref: Option<String>,
    #[serde(default)]
    pub repair_actions: Vec<ContractRepairResult>,
}
// END_SelfHealDiagnosis

// START_SelfHealResult
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfHealResult {
    pub run_id: String,
    pub profile: String,
    pub complete: bool,
    pub escalated: bool,
    pub iteration: u32,
    pub max_iterations: u32,
    pub failures_remaining: usize,
    pub failures_fixed: usize,
    pub diagnoses: Vec<SelfHealDiagnosis>,
    pub message: String,
}
// END_SelfHealResult

impl SelfHealPlan {
    // START_CONTRACT_SelfHealPlan::new
    // PURPOSE: Create an empty self-heal plan for a run and retry budget
    // INPUTS: { record: &RunRecord }, { profile: GraceProfile }, { max_iterations: u32 }
    // OUTPUTS: { SelfHealPlan }
    // START_self_heal_plan_new
    fn new(record: &RunRecord, profile: GraceProfile, max_iterations: u32) -> Self {
        Self {
            run_id: record.run_id.clone(),
            profile: profile.as_str().into(),
            iteration: 0,
            max_iterations,
            failed_groups: Vec::new(),
            diagnoses: Vec::new(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
    // END_self_heal_plan_new

    // START_CONTRACT_SelfHealPlan::touch
    // PURPOSE: Refresh the self-heal plan timestamp after a state change
    // OUTPUTS: { () }
    // START_self_heal_plan_touch
    fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
    // END_self_heal_plan_touch
}

impl RunManager {
    // START_CONTRACT_RunManager::load_self_heal_plan
    // PURPOSE: Load a persisted self-heal plan from run metadata
    // INPUTS: { record: &RunRecord }
    // OUTPUTS: { Option<SelfHealPlan> }
    // START_run_manager_load_self_heal_plan
    pub fn load_self_heal_plan(&self, record: &RunRecord) -> Option<SelfHealPlan> {
        record
            .metadata
            .get(SELF_HEAL_PLAN_KEY)
            .and_then(|raw| serde_json::from_str(raw).ok())
    }
    // END_run_manager_load_self_heal_plan

    // START_CONTRACT_RunManager::save_self_heal_plan
    // PURPOSE: Persist a self-heal plan inside run metadata and save the run record
    // INPUTS: { record: &mut RunRecord }, { plan: &SelfHealPlan }
    // OUTPUTS: { anyhow::Result<()> }
    // START_run_manager_save_self_heal_plan
    pub fn save_self_heal_plan(
        &self,
        record: &mut RunRecord,
        plan: &SelfHealPlan,
    ) -> anyhow::Result<()> {
        record
            .metadata
            .insert(SELF_HEAL_PLAN_KEY.into(), serde_json::to_string(plan)?);
        record
            .metadata
            .insert("self_heal_iterations".into(), plan.iteration.to_string());
        record.metadata.insert(
            "self_heal_remaining_budget".into(),
            plan.max_iterations
                .saturating_sub(plan.iteration)
                .to_string(),
        );
        record
            .metadata
            .insert("self_heal_profile".into(), plan.profile.clone());
        record.touch();
        if should_compact_self_heal_evidence(plan) {
            self.save_with_compaction(record).map(|_| ())
        } else {
            self.save(record)
        }
    }
    // END_run_manager_save_self_heal_plan

    // START_CONTRACT_RunManager::execute_self_heal
    // PURPOSE: Run one bounded self-heal iteration by verifying, diagnosing failures, and escalating when budget is exhausted
    // INPUTS: { run_id: &str }, { profile_name: &str }
    // OUTPUTS: { anyhow::Result<SelfHealResult> }
    // LINKS:
    //   -> UC-002 (implements) - bounded objective execution must preserve failure context
    //   -> NFR-002 (traces_to) - retry loops must stop with explicit escalation
    // START_run_manager_execute_self_heal
    pub fn execute_self_heal(
        &self,
        run_id: &str,
        profile_name: &str,
    ) -> anyhow::Result<SelfHealResult> {
        let profile = GraceProfile::from_name(profile_name).ok_or_else(|| {
            anyhow::anyhow!(
                "unsupported GRACE profile '{}'; use lite, balanced, or strict",
                profile_name
            )
        })?;
        let mut record = self.load(run_id)?;
        let max_iterations = self_heal_budget(&record);
        let mut plan = self
            .load_self_heal_plan(&record)
            .unwrap_or_else(|| SelfHealPlan::new(&record, profile, max_iterations));
        plan.profile = profile.as_str().into();
        plan.max_iterations = max_iterations;

        let verification = verify_project_sync(&self.root, profile)?;
        let failures = failing_results(&verification);

        if failures.is_empty() {
            plan.failed_groups.clear();
            plan.touch();
            record
                .metadata
                .insert("self_heal_complete".into(), "true".into());
            record.metadata.remove(SELF_HEAL_REQUESTED_KEY);
            record.metadata.remove(SELF_HEAL_DIAGNOSIS_READY_KEY);
            record.metadata.remove(SELF_HEAL_SUGGESTED_ACTION_KEY);
            record.blocked_reason = None;
            if matches!(
                record.status,
                RunStatus::Blocked | RunStatus::Failed | RunStatus::Escalated
            ) {
                record.status = RunStatus::Ready;
                record.outcome = None;
            }
            self.save_self_heal_plan(&mut record, &plan)?;
            write_provenance_event(
                &record.run_id,
                &record.module_id,
                &record.phase,
                &record.status,
                "self_heal_complete",
                "verification passed with no failing groups",
            );
            return Ok(SelfHealResult {
                run_id: record.run_id,
                profile: profile.as_str().into(),
                complete: true,
                escalated: false,
                iteration: plan.iteration,
                max_iterations: plan.max_iterations,
                failures_remaining: 0,
                failures_fixed: plan.diagnoses.iter().filter(|d| d.fix_applied).count(),
                diagnoses: plan.diagnoses,
                message: "verification passed; self-heal loop is complete".into(),
            });
        }

        if plan.iteration >= plan.max_iterations {
            plan.failed_groups = failures.iter().map(|result| result.level.clone()).collect();
            plan.touch();
            self.save_self_heal_plan(&mut record, &plan)?;
            let reason = format!(
                "self-heal budget exhausted after {} iterations; failing_groups={}",
                plan.iteration,
                plan.failed_groups.join(",")
            );
            let escalated = self.escalate_run(run_id, &reason)?;
            write_provenance_event(
                &escalated.run_id,
                &escalated.module_id,
                &escalated.phase,
                &escalated.status,
                "self_heal_exhausted",
                &reason,
            );
            return Ok(SelfHealResult {
                run_id: escalated.run_id,
                profile: profile.as_str().into(),
                complete: false,
                escalated: true,
                iteration: plan.iteration,
                max_iterations: plan.max_iterations,
                failures_remaining: failures.len(),
                failures_fixed: plan.diagnoses.iter().filter(|d| d.fix_applied).count(),
                diagnoses: plan.diagnoses,
                message: reason,
            });
        }

        plan.iteration += 1;
        plan.failed_groups = failures.iter().map(|result| result.level.clone()).collect();
        let new_diagnoses = diagnose_failures(&self.root, &failures);
        plan.diagnoses.extend(new_diagnoses);
        plan.touch();

        let message = format!(
            "self-heal diagnosis ready after iteration {}/{}; apply fixes and run self_heal again",
            plan.iteration, plan.max_iterations
        );
        record.status = RunStatus::Blocked;
        record.blocked_reason = Some(message.clone());
        record.outcome = Some(RunOutcome {
            kind: RunOutcomeKind::Blocked,
            summary: format!("run {} blocked by self-heal diagnosis", record.run_id),
            blocked_reason: Some(message.clone()),
            escalation_reason: None,
        });
        record
            .metadata
            .insert("self_heal_complete".into(), "false".into());
        record
            .metadata
            .insert(SELF_HEAL_DIAGNOSIS_READY_KEY.into(), "true".into());
        record.metadata.remove(SELF_HEAL_REQUESTED_KEY);
        record.metadata.remove(SELF_HEAL_SUGGESTED_ACTION_KEY);
        self.save_self_heal_plan(&mut record, &plan)?;
        self.create_handoff(
            &record.run_id,
            HandoffRole::Verifier,
            HandoffRole::Fixer,
            message.clone(),
            vec!["self-heal://diagnosis".into()],
            handoff_next_actions(&plan),
        )?;
        record = self.load(&record.run_id)?;
        write_provenance_event(
            &record.run_id,
            &record.module_id,
            &record.phase,
            &record.status,
            "self_heal_iteration",
            &format!(
                "iteration={}/{} failing_groups={}",
                plan.iteration,
                plan.max_iterations,
                plan.failed_groups.join(",")
            ),
        );

        Ok(SelfHealResult {
            run_id: record.run_id,
            profile: profile.as_str().into(),
            complete: false,
            escalated: false,
            iteration: plan.iteration,
            max_iterations: plan.max_iterations,
            failures_remaining: failures.len(),
            failures_fixed: plan.diagnoses.iter().filter(|d| d.fix_applied).count(),
            diagnoses: plan.diagnoses,
            message,
        })
    }
    // END_run_manager_execute_self_heal
}

// START_CONTRACT_self_heal_budget
// PURPOSE: Resolve bounded self-heal retry budget from run policy with a safe default
// INPUTS: { record: &RunRecord }
// OUTPUTS: { u32 }
// START_self_heal_budget
fn self_heal_budget(record: &RunRecord) -> u32 {
    record
        .policy
        .as_ref()
        .map(|policy| policy.retry_budget)
        .filter(|budget| *budget > 0)
        .unwrap_or(3)
}
// END_self_heal_budget

// START_CONTRACT_should_compact_self_heal_evidence
// PURPOSE: Return true at periodic self-heal checkpoints where evidence refs should be compacted.
// INPUTS: { plan: &SelfHealPlan }
// OUTPUTS: { bool }
// START_should_compact_self_heal_evidence
fn should_compact_self_heal_evidence(plan: &SelfHealPlan) -> bool {
    plan.iteration > 0
        && plan
            .iteration
            .is_multiple_of(SELF_HEAL_EVIDENCE_COMPACTION_INTERVAL)
}
// END_should_compact_self_heal_evidence

// START_CONTRACT_handoff_next_actions
// PURPOSE: Build compact fixer next actions from the latest unresolved self-heal diagnoses.
// INPUTS: { plan: &SelfHealPlan }
// OUTPUTS: { Vec<String> }
// START_handoff_next_actions
fn handoff_next_actions(plan: &SelfHealPlan) -> Vec<String> {
    let mut actions: Vec<String> = plan
        .diagnoses
        .iter()
        .rev()
        .filter(|diagnosis| !diagnosis.fix_applied)
        .take(3)
        .map(|diagnosis| {
            format!(
                "{}: {}",
                diagnosis.failure_group,
                diagnosis
                    .failed_checks
                    .first()
                    .map(String::as_str)
                    .unwrap_or("inspect failing check")
            )
        })
        .collect();
    if actions.is_empty() {
        actions.push("inspect self-heal diagnosis and apply smallest safe fix".into());
    }
    actions
}
// END_handoff_next_actions

// START_CONTRACT_failing_results
// PURPOSE: Extract failed verification groups from a verification result list
// INPUTS: { results: &[VerificationResult] }
// OUTPUTS: { Vec<VerificationResult> }
// START_failing_results
fn failing_results(results: &[VerificationResult]) -> Vec<VerificationResult> {
    results
        .iter()
        .filter(|result| !result.passed)
        .cloned()
        .collect()
}
// END_failing_results

// START_CONTRACT_diagnose_failures
// PURPOSE: Convert failing verification groups into self-heal diagnoses
// INPUTS: { root: &Path }, { failures: &[VerificationResult] }
// OUTPUTS: { Vec<SelfHealDiagnosis> }
// START_diagnose_failures
fn diagnose_failures(root: &Path, failures: &[VerificationResult]) -> Vec<SelfHealDiagnosis> {
    failures
        .iter()
        .map(|failure| diagnose_failure(root, failure))
        .collect()
}
// END_diagnose_failures

// START_CONTRACT_diagnose_failure
// PURPOSE: Diagnose one failing verification group through the indexed debugger
// INPUTS: { root: &Path }, { failure: &VerificationResult }
// OUTPUTS: { SelfHealDiagnosis }
// START_diagnose_failure
fn diagnose_failure(root: &Path, failure: &VerificationResult) -> SelfHealDiagnosis {
    let failed_checks: Vec<String> = failure
        .checks
        .iter()
        .filter(|check| !check.passed)
        .map(|check| format!("{} - {}", check.name, check.details))
        .collect();
    let description = format!(
        "Verification level '{}' failed: {}",
        failure.level,
        failed_checks.join("; ")
    );
    let repair_actions =
        ContractGenerator::repair_actions_from_failure_text(root, &description, true);
    match diagnose_sync(root.to_path_buf(), description.clone()) {
        Ok(result) => SelfHealDiagnosis {
            failure_group: failure.level.clone(),
            failed_checks,
            related_modules: result.related_modules,
            suggested_blocks: result.suggested_blocks,
            diagnosis: result.diagnosis,
            fix_applied: false,
            fix_evidence_ref: None,
            repair_actions,
        },
        Err(error) => SelfHealDiagnosis {
            failure_group: failure.level.clone(),
            failed_checks,
            related_modules: Vec::new(),
            suggested_blocks: Vec::new(),
            diagnosis: format!("diagnosis failed: {}", error),
            fix_applied: false,
            fix_evidence_ref: None,
            repair_actions,
        },
    }
}
// END_diagnose_failure

// START_CONTRACT_verify_project_sync
// PURPOSE: Run async GRACE verification from sync run actions without nesting a runtime
// INPUTS: { root: &Path }, { profile: GraceProfile }
// OUTPUTS: { anyhow::Result<Vec<VerificationResult>> }
// START_verify_project_sync
fn verify_project_sync(
    root: &Path,
    profile: GraceProfile,
) -> anyhow::Result<Vec<VerificationResult>> {
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(GraceEngine::verify_project_with_profile(&root, profile))
    })
    .join()
    .map_err(|_| anyhow::anyhow!("self-heal verification worker panicked"))?
}
// END_verify_project_sync

// START_CONTRACT_diagnose_sync
// PURPOSE: Run async debugger diagnosis from sync run actions without nesting a runtime
// INPUTS: { root: PathBuf }, { description: String }
// OUTPUTS: { anyhow::Result<FixResult> }
// START_diagnose_sync
fn diagnose_sync(
    root: PathBuf,
    description: String,
) -> anyhow::Result<crate::grace::fix::FixResult> {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(crate::grace::fix::Debugger::diagnose(&description, &root))
    })
    .join()
    .map_err(|_| anyhow::anyhow!("self-heal diagnosis worker panicked"))?
}
// END_diagnose_sync

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{RunGatePolicy, RunRecord};

    fn blocked_record(retry_budget: u32) -> RunRecord {
        let mut record = RunRecord::new(
            "goal".into(),
            "Phase-70".into(),
            "M-RUNNER-SELF-HEAL".into(),
            "diagnose failing verification".into(),
        );
        record.status = RunStatus::Blocked;
        record.blocked_reason = Some("verification gate blocked".into());
        record.policy = Some(RunGatePolicy {
            phase: "Phase-70".into(),
            module_id: "M-RUNNER-SELF-HEAL".into(),
            required_gates: Vec::new(),
            stop_on_block: true,
            retry_budget,
            escalation_target: "human-review".into(),
        });
        record
    }

    #[test]
    // START_CONTRACT_self_heal_plan_round_trips_in_run_metadata
    // PURPOSE: Verify self-heal plan JSON persists inside a run record metadata map
    // START_self_heal_plan_round_trips_in_run_metadata
    fn self_heal_plan_round_trips_in_run_metadata() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let mut record = blocked_record(2);
        let plan = SelfHealPlan::new(&record, GraceProfile::Lite, 2);

        manager.save_self_heal_plan(&mut record, &plan).unwrap();
        let loaded = manager.load_self_heal_plan(&record).unwrap();

        assert_eq!(loaded.run_id, record.run_id);
        assert_eq!(loaded.max_iterations, 2);
        assert_eq!(record.metadata["self_heal_remaining_budget"], "2");
    }
    // END_self_heal_plan_round_trips_in_run_metadata

    #[test]
    // START_CONTRACT_save_self_heal_plan_compacts_evidence_on_interval
    // PURPOSE: Verify self-heal plan saves compact evidence refs at periodic iteration checkpoints.
    // START_save_self_heal_plan_compacts_evidence_on_interval
    fn save_self_heal_plan_compacts_evidence_on_interval() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let mut record = blocked_record(5);
        record.evidence_refs = vec![
            "action://verify".into(),
            "action://verify".into(),
            "docs/runs/run-1/evidence.log".into(),
        ];
        let mut plan = SelfHealPlan::new(&record, GraceProfile::Lite, 5);
        plan.iteration = 5;

        manager.save_self_heal_plan(&mut record, &plan).unwrap();
        let restored = manager.load(&record.run_id).unwrap();

        assert_eq!(
            restored.evidence_refs,
            vec!["▶verify".to_string(), "📁run-1/evidence.log".to_string()]
        );
    }
    // END_save_self_heal_plan_compacts_evidence_on_interval

    #[test]
    // START_CONTRACT_save_self_heal_plan_preserves_evidence_before_interval
    // PURPOSE: Verify self-heal plan saves do not compact evidence before periodic checkpoints.
    // START_save_self_heal_plan_preserves_evidence_before_interval
    fn save_self_heal_plan_preserves_evidence_before_interval() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let mut record = blocked_record(5);
        record.evidence_refs = vec!["action://verify".into(), "action://verify".into()];
        let mut plan = SelfHealPlan::new(&record, GraceProfile::Lite, 5);
        plan.iteration = 4;

        manager.save_self_heal_plan(&mut record, &plan).unwrap();
        let restored = manager.load(&record.run_id).unwrap();

        assert_eq!(restored.evidence_refs, record.evidence_refs);
    }
    // END_save_self_heal_plan_preserves_evidence_before_interval

    #[test]
    // START_CONTRACT_execute_self_heal_exhausts_retry_budget
    // PURPOSE: Verify self-heal escalates a run after consuming its retry budget
    // START_execute_self_heal_exhausts_retry_budget
    fn execute_self_heal_exhausts_retry_budget() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let record = blocked_record(1);
        let run_id = record.run_id.clone();
        manager.save(&record).unwrap();

        let first = manager.execute_self_heal(&run_id, "lite").unwrap();
        assert!(!first.complete);
        assert!(!first.escalated);
        assert_eq!(first.iteration, 1);
        assert_eq!(first.max_iterations, 1);
        let handoff = manager.latest_handoff(&run_id).unwrap().unwrap();
        assert_eq!(handoff.from_role, HandoffRole::Verifier);
        assert_eq!(handoff.to_role, HandoffRole::Fixer);
        assert!(!handoff.next_actions.is_empty());

        let second = manager.execute_self_heal(&run_id, "lite").unwrap();
        assert!(!second.complete);
        assert!(second.escalated);
        assert_eq!(manager.load(&run_id).unwrap().status, RunStatus::Escalated);
    }
    // END_execute_self_heal_exhausts_retry_budget
}
