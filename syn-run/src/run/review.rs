// MODULE_CONTRACT
// MODULE_ID: M-RUNNER
// PURPOSE: Run review decisions — persist human approval or rejection for blocked autonomous runs
// SCOPE: Review decision model, latest review lookup, approve/reject helpers, and provenance recording
// DEPENDS: M-RUNNER, M-TRACKING
// LINKS:
//   → V-M-RUNNER (verified_by) - review persistence and blocked-run resume tests

// START_MODULE_MAP
// RunReviewStatus — Human review decision status
// RunReviewDecision — Persisted review record
// approve_run — Record an approval decision
// reject_run — Record a rejection decision and escalate the run
// review_run — Shared review persistence implementation
// latest_review_status — Return latest recorded review status
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Added blocked-run review approval persistence]
// END_CHANGE_SUMMARY

use super::{write_provenance_event, RunManager, RunOutcome, RunOutcomeKind, RunRecord, RunStatus};
use serde::{Deserialize, Serialize};

// START_public_api

// START_RunReviewStatus
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunReviewStatus {
    Pending,
    Approved,
    Rejected,
}
// END_RunReviewStatus

// START_RunReviewDecision
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunReviewDecision {
    pub review_id: String,
    pub reviewer: String,
    pub status: RunReviewStatus,
    pub reason: String,
    pub evidence_refs: Vec<String>,
    pub created_at: String,
}
// END_RunReviewDecision

impl RunManager {
    // START_CONTRACT_RunManager::approve_run
    // PURPOSE: Persist a human approval for a blocked run without directly completing any gate
    // INPUTS: { run_id: &str }, { reviewer: &str }, { reason: &str }, { evidence_ref: Option<&str> }
    // OUTPUTS: { anyhow::Result<RunRecord> }
    // LINKS:
    //   → UC-002 (implements) - human approval controls bounded retry after blocked execution
    //   → NFR-002 (traces_to) - blocked resume must require explicit persisted review
    // START_run_manager_approve_run
    pub fn approve_run(
        &self,
        run_id: &str,
        reviewer: &str,
        reason: &str,
        evidence_ref: Option<&str>,
    ) -> anyhow::Result<RunRecord> {
        self.review_run(
            run_id,
            reviewer,
            RunReviewStatus::Approved,
            reason,
            evidence_ref,
        )
    }
    // END_run_manager_approve_run

    // START_CONTRACT_RunManager::reject_run
    // PURPOSE: Persist a human rejection and escalate the blocked run for handoff
    // INPUTS: { run_id: &str }, { reviewer: &str }, { reason: &str }, { evidence_ref: Option<&str> }
    // OUTPUTS: { anyhow::Result<RunRecord> }
    // LINKS:
    //   → UC-002 (implements) - human rejection escalates bounded autonomous work
    //   → NFR-002 (traces_to) - repeated or rejected failures must stop safely
    // START_run_manager_reject_run
    pub fn reject_run(
        &self,
        run_id: &str,
        reviewer: &str,
        reason: &str,
        evidence_ref: Option<&str>,
    ) -> anyhow::Result<RunRecord> {
        self.review_run(
            run_id,
            reviewer,
            RunReviewStatus::Rejected,
            reason,
            evidence_ref,
        )
    }
    // END_run_manager_reject_run

    // START_CONTRACT_RunManager::review_run
    // PURPOSE: Persist a review decision and update blocked-run state according to the decision
    // INPUTS: { run_id: &str }, { reviewer: &str }, { status: RunReviewStatus }, { reason: &str }, { evidence_ref: Option<&str> }
    // OUTPUTS: { anyhow::Result<RunRecord> }
    // LINKS:
    //   → UC-002 (implements) - review decision records human control over autonomous continuation
    //   → NFR-003 (traces_to) - durable review state supports resumable long-running work
    // START_run_manager_review_run
    pub fn review_run(
        &self,
        run_id: &str,
        reviewer: &str,
        status: RunReviewStatus,
        reason: &str,
        evidence_ref: Option<&str>,
    ) -> anyhow::Result<RunRecord> {
        let mut record = self.load(run_id)?;
        let created_at = chrono::Utc::now().to_rfc3339();
        let evidence_refs = evidence_ref
            .map(|evidence| vec![evidence.to_string()])
            .unwrap_or_default();
        record.review_decisions.push(RunReviewDecision {
            review_id: format!("review-{}", uuid::Uuid::new_v4()),
            reviewer: reviewer.to_string(),
            status: status.clone(),
            reason: reason.to_string(),
            evidence_refs: evidence_refs.clone(),
            created_at,
        });
        record.metadata.insert(
            "last_review_decision".into(),
            format!("{:?}", status).to_lowercase(),
        );
        record
            .metadata
            .insert("last_reviewer".into(), reviewer.to_string());
        record.evidence_refs.extend(evidence_refs);
        if matches!(status, RunReviewStatus::Rejected) {
            record.status = RunStatus::Escalated;
            record.escalation_reason = Some(reason.to_string());
            record.outcome = Some(RunOutcome {
                kind: RunOutcomeKind::Escalated,
                summary: format!("run {} rejected by reviewer", record.run_id),
                blocked_reason: record.blocked_reason.clone(),
                escalation_reason: Some(reason.to_string()),
            });
        }
        record.touch();
        self.save(&record)?;
        write_provenance_event(
            &record.run_id,
            &record.module_id,
            &record.phase,
            &record.status,
            "review_run",
            reason,
        );
        Ok(record)
    }
    // END_run_manager_review_run
}

impl RunRecord {
    // START_CONTRACT_RunRecord::latest_review_status
    // PURPOSE: Return the latest persisted human review status for a run
    // OUTPUTS: { Option<RunReviewStatus> }
    // LINKS:
    //   → NFR-002 (traces_to) - resume checks must read explicit review state before continuing
    // START_run_record_latest_review_status
    pub fn latest_review_status(&self) -> Option<RunReviewStatus> {
        self.review_decisions
            .last()
            .map(|decision| decision.status.clone())
    }
    // END_run_record_latest_review_status
}

// END_public_api
