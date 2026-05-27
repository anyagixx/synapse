// MODULE_CONTRACT
// MODULE_ID: M-DASHBOARD
// PURPOSE: Dashboard run cockpit — builds run queue, blocked board, review actions, and replay views for bounded agent workflows
// SCOPE: run payload classification, blocked/queue summaries, selected run replay timeline, review action persistence, and HTML cockpit rendering
// DEPENDS: M-DASHBOARD, M-RUNNER
// LINKS:
//   -> V-M-DASHBOARD (verified_by) - dashboard route and payload tests
//   -> V-M-RUNNER (verified_by) - persisted run state verification

// START_MODULE_MAP
// runs_payload — Build JSON for runs, queue, blocked runs, completed runs, and provenance timeline
// review_run_payload — Persist blocked-run review decisions and return updated payload
// render_runs_page — Render the run cockpit page
// render_review_result_page — Render review result and updated cockpit
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 — Added blocked-run review action surface and replay timeline]
// END_CHANGE_SUMMARY

use super::render::{html_escape, json_pre, page_shell, primary_nav};
use syn_run::{RunManager, RunRecord, RunStatus};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

// START_public_api

#[derive(Debug, Clone, Deserialize)]
pub(in crate::dashboard) struct RunReviewForm {
    pub run_id: String,
    pub decision: String,
    pub reviewer: Option<String>,
    pub reason: Option<String>,
}

// START_CONTRACT_runs_payload
// PURPOSE: Build JSON payload for bounded run queue, blocked records, and selected-run provenance
// INPUTS: { root: &Path }, { run_id: &str }
// OUTPUTS: { Value }
// START_runs_payload
pub(in crate::dashboard) fn runs_payload(root: &Path, run_id: &str) -> Value {
    let runs = RunManager::new(root).list().unwrap_or_default();
    let selected = selected_run(&runs, run_id);
    let selected_id = selected
        .map(|run| run.run_id.clone())
        .unwrap_or_else(|| run_id.to_string());
    let all = runs.iter().map(run_json).collect::<Vec<_>>();
    let queue = runs
        .iter()
        .filter(|run| {
            matches!(
                run.status,
                RunStatus::Planned | RunStatus::Ready | RunStatus::Running
            )
        })
        .map(run_json)
        .collect::<Vec<_>>();
    let blocked = runs
        .iter()
        .filter(|run| {
            matches!(
                run.status,
                RunStatus::Blocked | RunStatus::Failed | RunStatus::Escalated
            )
        })
        .map(run_json)
        .collect::<Vec<_>>();
    let completed = runs
        .iter()
        .filter(|run| matches!(run.status, RunStatus::Completed))
        .map(run_json)
        .collect::<Vec<_>>();
    let replay = selected.map(|run| run.replay());
    let events = replay
        .as_ref()
        .map(|replay| {
            replay
                .events
                .iter()
                .map(replay_event_json)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "run_id": selected_id,
        "summary": {
            "total": runs.len(),
            "queue": queue.len(),
            "blocked": blocked.len(),
            "completed": completed.len(),
        },
        "runs": all,
        "queue": queue,
        "blocked": blocked,
        "completed": completed,
        "events": events,
        "replay": replay,
    })
}
// END_runs_payload

// START_CONTRACT_review_run_payload
// PURPOSE: Persist a blocked-run review decision and return updated run cockpit payload
// INPUTS: { root: &Path }, { form: &RunReviewForm }
// OUTPUTS: { Value }
// LINKS:
//   → UC-002 (implements) - dashboard payload records human review over autonomous retry
// START_review_run_payload
pub(in crate::dashboard) fn review_run_payload(root: &Path, form: &RunReviewForm) -> Value {
    let manager = RunManager::new(root);
    let reviewer = form.reviewer.as_deref().unwrap_or("dashboard");
    let reason = form.reason.as_deref().unwrap_or("review decision recorded");
    let result = match form.decision.as_str() {
        "approve" | "approved" => manager.approve_run(&form.run_id, reviewer, reason, None),
        "reject" | "rejected" => manager.reject_run(&form.run_id, reviewer, reason, None),
        other => Err(anyhow::anyhow!("unknown review decision {}", other)),
    };
    let mut payload = runs_payload(root, &form.run_id);
    match result {
        Ok(run) => {
            payload["review"] = json!({
                "ok": true,
                "run_id": run.run_id,
                "decision": form.decision.clone(),
                "latest_review": run.latest_review_status().map(|status| format!("{:?}", status)),
            });
        }
        Err(error) => {
            payload["review"] = json!({
                "ok": false,
                "error": error.to_string(),
            });
        }
    }
    payload
}
// END_review_run_payload

// START_CONTRACT_render_runs_page
// PURPOSE: Render bounded run queue, blocked board, and selected-run provenance timeline
// INPUTS: { root: &Path }, { run_id: &str }
// OUTPUTS: { String }
// START_render_runs_page
pub(in crate::dashboard) fn render_runs_page(root: &Path, run_id: &str) -> String {
    let payload = runs_payload(root, run_id);
    let body = format!(
        "{}<section class=\"grid\">{}{}{}{}</section><section class=\"card\"><h2>Queue</h2>{}</section><section class=\"card\"><h2>Blocked</h2>{}</section><section class=\"card\"><h2>Review</h2>{}</section><section class=\"card\"><h2>Replay</h2>{}</section>",
        primary_nav(),
        metric("Total", &payload["summary"]["total"]),
        metric("Queue", &payload["summary"]["queue"]),
        metric("Blocked", &payload["summary"]["blocked"]),
        metric("Done", &payload["summary"]["completed"]),
        runs_table(&payload["queue"]),
        blocked_table(&payload["blocked"]),
        review_form(&payload["run_id"]),
        timeline_table(&payload["events"]),
    );
    page_shell("Run Cockpit", &body)
}
// END_render_runs_page

// START_CONTRACT_render_review_result_page
// PURPOSE: Render review result and the updated run cockpit
// INPUTS: { root: &Path }, { form: &RunReviewForm }
// OUTPUTS: { String }
// LINKS:
//   → UC-002 (implements) - dashboard presents review evidence for blocked run triage
// START_render_review_result_page
pub(in crate::dashboard) fn render_review_result_page(root: &Path, form: &RunReviewForm) -> String {
    let payload = review_run_payload(root, form);
    let body = format!(
        "{}<section class=\"card\"><h2>Review Result</h2>{}</section><p><a href=\"/runs?run_id={}\">Back to run cockpit</a></p>",
        primary_nav(),
        json_pre(&payload["review"]),
        html_escape(&form.run_id)
    );
    page_shell("Run Review", &body)
}
// END_render_review_result_page

// END_public_api

fn selected_run<'a>(runs: &'a [RunRecord], run_id: &str) -> Option<&'a RunRecord> {
    if run_id.trim().is_empty() {
        runs.iter().rev().find(|run| {
            matches!(
                run.status,
                RunStatus::Blocked
                    | RunStatus::Failed
                    | RunStatus::Escalated
                    | RunStatus::Running
                    | RunStatus::Ready
            )
        })
    } else {
        runs.iter().find(|run| run.run_id == run_id)
    }
}

fn run_json(run: &RunRecord) -> Value {
    json!({
        "run_id": run.run_id.clone(),
        "goal": run.goal.clone(),
        "phase": run.phase.clone(),
        "module_id": run.module_id.clone(),
        "objective": run.objective.clone(),
        "status": format!("{:?}", run.status),
        "current_step": run.current_step,
        "blocked_reason": run.blocked_reason.clone(),
        "escalation_reason": run.escalation_reason.clone(),
        "approval_state": run.latest_review_status().map(|status| format!("{:?}", status)),
        "review_count": run.review_decisions.len(),
        "traceability_summary": run.metadata.get("traceability_summary").cloned(),
        "retry_count": run.metadata.get("retry_count").cloned(),
        "updated_at": run.updated_at.clone(),
    })
}

fn replay_event_json(event: &syn_run::RunReplayEvent) -> Value {
    json!({
        "kind": event.kind.clone(),
        "status": event.status.clone(),
        "detail": event.detail.clone(),
        "evidence": event.evidence_refs.clone(),
    })
}

fn metric(label: &str, value: &Value) -> String {
    format!(
        "<div class=\"card\"><h2>{}</h2><strong>{}</strong></div>",
        html_escape(label),
        html_escape(&value.to_string())
    )
}

fn runs_table(rows: &Value) -> String {
    rows_table(
        rows,
        &["run_id", "status", "module_id", "current_step", "objective"],
    )
}

fn blocked_table(rows: &Value) -> String {
    rows_table(
        rows,
        &[
            "run_id",
            "status",
            "blocked_reason",
            "escalation_reason",
            "approval_state",
            "retry_count",
        ],
    )
}

fn timeline_table(rows: &Value) -> String {
    rows_table(rows, &["kind", "status", "detail", "evidence"])
}

fn review_form(run_id: &Value) -> String {
    let run_id = run_id.as_str().unwrap_or_default();
    if run_id.is_empty() {
        return "<p>Select a blocked run to review.</p>".into();
    }
    format!(
        "<form method=\"post\" action=\"/runs/review\"><input type=\"hidden\" name=\"run_id\" value=\"{}\"><label>Decision <select name=\"decision\"><option value=\"approve\">Approve retry</option><option value=\"reject\">Reject and escalate</option></select></label><label>Reviewer <input name=\"reviewer\" value=\"dashboard\"></label><label>Reason <input name=\"reason\" value=\"bounded review decision\"></label><button type=\"submit\">Record review</button></form>",
        html_escape(run_id)
    )
}

fn rows_table(rows: &Value, columns: &[&str]) -> String {
    let Some(items) = rows.as_array() else {
        return json_pre(rows);
    };
    if items.is_empty() {
        return "<p>No records</p>".into();
    }
    let head = columns
        .iter()
        .map(|column| format!("<th>{}</th>", html_escape(column)))
        .collect::<Vec<_>>()
        .join("");
    let body = items
        .iter()
        .map(|row| {
            let cells = columns
                .iter()
                .map(|column| format!("<td>{}</td>", html_escape(&cell_text(&row[*column]))))
                .collect::<Vec<_>>()
                .join("");
            format!("<tr>{cells}</tr>")
        })
        .collect::<Vec<_>>()
        .join("");
    format!("<table><thead><tr>{head}</tr></thead><tbody>{body}</tbody></table>")
}

fn cell_text(value: &Value) -> String {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| value.to_string())
}
