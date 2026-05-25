// MODULE_CONTRACT
// MODULE_ID: M-RUNNER-PRECOMMIT
// PURPOSE: Pre-commit verification gate for bounded autonomous runs before final completion.
// SCOPE: PreCommitReport data model, required gate evaluation, optional project verification, run metadata updates, and blocked-run failure metadata.
// DEPENDS: M-RUNNER, M-GRACE-VERIFY
// LINKS:
//   -> M-RUNNER (depends) - reads and updates durable run records
//   -> M-GRACE-VERIFY (depends) - optionally runs project verification for real project roots
//   -> UC-002 (implements) - bounded runs cannot complete without explicit verification gates
//   -> NFR-002 (traces_to) - failed gates block completion with structured metadata

// START_MODULE_MAP
// PreCommitGateFailure - Structured failed gate metadata
// PreCommitReport - Verification summary returned to action queue and MCP clients
// RunManager::pre_commit_verify - Evaluate pre-commit gates and update run state
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added bounded run pre-commit verification]
// END_CHANGE_SUMMARY

use super::{
    write_provenance_event, RunGateStatus, RunManager, RunOutcome, RunOutcomeKind, RunRecord,
    RunStatus, RunStepStatus,
};
use crate::grace::verify::Verifier;
use serde::{Deserialize, Serialize};
use std::path::Path;

// START_public_api
// START_PreCommitGateFailure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreCommitGateFailure {
    pub gate_id: String,
    pub gate_name: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}
// END_PreCommitGateFailure

// START_PreCommitReport
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreCommitReport {
    pub run_id: String,
    pub passed: bool,
    pub checked_at: String,
    pub gate_failures: Vec<PreCommitGateFailure>,
    pub verification_failures: Vec<String>,
}
// END_PreCommitReport

impl RunManager {
    // START_CONTRACT_RunManager::pre_commit_verify
    // PURPOSE: Evaluate pre-commit gates and block the run when final completion is unsafe.
    // INPUTS: { run_id: &str }
    // OUTPUTS: { anyhow::Result<PreCommitReport> }
    // LINKS:
    //   -> UC-002 (implements) - pre-commit gates must pass before run completion
    //   -> NFR-002 (traces_to) - failed gates persist explicit blocked metadata
    // START_run_manager_pre_commit_verify
    pub fn pre_commit_verify(&self, run_id: &str) -> anyhow::Result<PreCommitReport> {
        let mut record = self.load(run_id)?;
        let checked_at = chrono::Utc::now().to_rfc3339();
        let mut gate_failures = required_gate_failures(&record);
        let mut verification_failures = Vec::new();

        if should_run_project_verify(&self.root, &record) {
            verification_failures = project_verification_failures(&self.root)?;
        }
        if record.required_gates.is_empty() && verification_failures.is_empty() {
            gate_failures.push(PreCommitGateFailure {
                gate_id: "gate-pre-commit-present".into(),
                gate_name: "pre_commit_gate_presence".into(),
                reason: "no required pre-commit gates or project verification evidence".into(),
                evidence_refs: Vec::new(),
            });
        }

        let passed = gate_failures.is_empty() && verification_failures.is_empty();
        let report = PreCommitReport {
            run_id: run_id.to_string(),
            passed,
            checked_at,
            gate_failures,
            verification_failures,
        };
        apply_pre_commit_report(self, &mut record, &report)?;
        Ok(report)
    }
    // END_run_manager_pre_commit_verify
}
// END_public_api

// START_CONTRACT_required_gate_failures
// PURPOSE: Collect failed or blocked required gates from a run record.
// INPUTS: { record: &RunRecord }
// OUTPUTS: { Vec<PreCommitGateFailure> }
// START_required_gate_failures
fn required_gate_failures(record: &RunRecord) -> Vec<PreCommitGateFailure> {
    record
        .required_gates
        .iter()
        .filter(|gate| {
            gate.required && matches!(gate.status, RunGateStatus::Blocked | RunGateStatus::Failed)
        })
        .map(|gate| PreCommitGateFailure {
            gate_id: gate.id.clone(),
            gate_name: gate.name.clone(),
            reason: gate
                .reason
                .clone()
                .unwrap_or_else(|| format!("gate {} did not pass", gate.name)),
            evidence_refs: gate.evidence_refs.clone(),
        })
        .collect()
}
// END_required_gate_failures

// START_CONTRACT_should_run_project_verify
// PURPOSE: Decide whether pre-commit should run full project verification for the current root.
// INPUTS: { root: &Path }, { record: &RunRecord }
// OUTPUTS: { bool }
// START_should_run_project_verify
fn should_run_project_verify(root: &Path, record: &RunRecord) -> bool {
    record
        .metadata
        .get("pre_commit_project_verify")
        .is_some_and(|value| value == "true")
        || root.join("docs/plan-index.xml").exists()
}
// END_should_run_project_verify

// START_CONTRACT_project_verification_failures
// PURPOSE: Run project verification synchronously and summarize failed groups/checks.
// INPUTS: { root: &Path }
// OUTPUTS: { anyhow::Result<Vec<String>> }
// START_project_verification_failures
fn project_verification_failures(root: &Path) -> anyhow::Result<Vec<String>> {
    let root = root.to_path_buf();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let results = runtime.block_on(Verifier::verify_all(&root))?;
    let mut failures = Vec::new();
    for result in results {
        if result.passed {
            continue;
        }
        failures.push(format!("verification group '{}' failed", result.level));
        for check in result.checks.iter().filter(|check| !check.passed) {
            failures.push(format!("{}: {}", check.name, check.details));
        }
    }
    Ok(failures)
}
// END_project_verification_failures

// START_CONTRACT_apply_pre_commit_report
// PURPOSE: Persist pre-commit pass/fail metadata and block the run on failed gates.
// INPUTS: { manager: &RunManager }, { record: &mut RunRecord }, { report: &PreCommitReport }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes run record and provenance event
// START_apply_pre_commit_report
fn apply_pre_commit_report(
    manager: &RunManager,
    record: &mut RunRecord,
    report: &PreCommitReport,
) -> anyhow::Result<()> {
    record
        .metadata
        .insert("pre_commit_verified".into(), report.passed.to_string());
    record
        .metadata
        .insert("pre_commit_checked_at".into(), report.checked_at.clone());
    record.metadata.insert(
        "pre_commit_gate_failures".into(),
        serde_json::to_string(&report.gate_failures)?,
    );
    record.metadata.insert(
        "pre_commit_verification_failures".into(),
        serde_json::to_string(&report.verification_failures)?,
    );

    if !report.passed {
        let reason = pre_commit_block_reason(report);
        record.status = RunStatus::Blocked;
        record.blocked_reason = Some(reason.clone());
        if let Some(step) = record.steps.get_mut(record.current_step) {
            step.status = RunStepStatus::Failed;
            step.error = Some(reason.clone());
            step.evidence_refs.push("pre-commit://verify".into());
        }
        record.evidence_refs.push("pre-commit://verify".into());
        record.outcome = Some(RunOutcome {
            kind: RunOutcomeKind::Blocked,
            summary: format!("run {} blocked by pre-commit verification", record.run_id),
            blocked_reason: Some(reason),
            escalation_reason: None,
        });
    }

    record.touch();
    manager.save(record)?;
    write_provenance_event(
        &record.run_id,
        &record.module_id,
        &record.phase,
        &record.status,
        "pre_commit_verify",
        &format!(
            "passed={} gate_failures={} verification_failures={}",
            report.passed,
            report.gate_failures.len(),
            report.verification_failures.len()
        ),
    );
    Ok(())
}
// END_apply_pre_commit_report

// START_CONTRACT_pre_commit_block_reason
// PURPOSE: Create a compact blocked reason from pre-commit gate and verification failures.
// INPUTS: { report: &PreCommitReport }
// OUTPUTS: { String }
// START_pre_commit_block_reason
fn pre_commit_block_reason(report: &PreCommitReport) -> String {
    let first_gate = report
        .gate_failures
        .first()
        .map(|gate| gate.reason.as_str());
    let first_verification = report.verification_failures.first().map(String::as_str);
    match (first_gate, first_verification) {
        (Some(gate), _) => format!("pre-commit verification failed: {}", gate),
        (None, Some(verification)) => {
            format!("pre-commit verification failed: {}", verification)
        }
        (None, None) => "pre-commit verification failed".into(),
    }
}
// END_pre_commit_block_reason

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{RunGate, RunStep};

    fn run_with_gate(status: RunGateStatus) -> RunRecord {
        let mut record = RunRecord::new(
            "goal".into(),
            "Phase-73".into(),
            "M-RUNNER-PRECOMMIT".into(),
            "verify before commit".into(),
        );
        record.status = RunStatus::Running;
        record.required_gates.push(RunGate {
            id: "gate-verification".into(),
            name: "verification".into(),
            required: true,
            status,
            reason: Some("verification gate blocked".into()),
            evidence_refs: vec!["syn verify".into()],
        });
        record.steps.push(RunStep {
            id: "final".into(),
            name: "verify-and-review".into(),
            description: "final step".into(),
            status: RunStepStatus::Running,
            evidence_refs: Vec::new(),
            error: None,
        });
        record
    }

    // START_CONTRACT_pre_commit_verify_passes_with_passed_required_gates
    // PURPOSE: Verify pre-commit succeeds when required gates have already passed.
    // START_pre_commit_verify_passes_with_passed_required_gates
    #[test]
    fn pre_commit_verify_passes_with_passed_required_gates() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let record = run_with_gate(RunGateStatus::Passed);
        manager.save(&record).unwrap();

        let report = manager.pre_commit_verify(&record.run_id).unwrap();
        let loaded = manager.load(&record.run_id).unwrap();

        assert!(report.passed);
        assert_eq!(
            loaded
                .metadata
                .get("pre_commit_verified")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(loaded.status, RunStatus::Running);
    }
    // END_pre_commit_verify_passes_with_passed_required_gates

    // START_CONTRACT_pre_commit_verify_blocks_failed_required_gate
    // PURPOSE: Verify pre-commit blocks final completion when a required gate failed.
    // START_pre_commit_verify_blocks_failed_required_gate
    #[test]
    fn pre_commit_verify_blocks_failed_required_gate() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let record = run_with_gate(RunGateStatus::Failed);
        manager.save(&record).unwrap();

        let report = manager.pre_commit_verify(&record.run_id).unwrap();
        let loaded = manager.load(&record.run_id).unwrap();

        assert!(!report.passed);
        assert_eq!(loaded.status, RunStatus::Blocked);
        assert!(loaded
            .blocked_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("pre-commit verification failed")));
    }
    // END_pre_commit_verify_blocks_failed_required_gate
}
