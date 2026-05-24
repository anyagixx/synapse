// MODULE_CONTRACT
// MODULE_ID: M-DASHBOARD
// PURPOSE: Dashboard run cockpit — builds run queue, blocked board, and provenance views for bounded agent workflows
// SCOPE: run payload classification, blocked/queue summaries, selected run provenance timeline, and HTML cockpit rendering
// DEPENDS: M-DASHBOARD, M-RUNNER
// LINKS:
//   -> V-M-DASHBOARD (verified_by) - dashboard route and payload tests
//   -> V-M-RUNNER (verified_by) - persisted run state verification

// START_MODULE_MAP
// runs_payload — Build JSON for runs, queue, blocked runs, completed runs, and provenance timeline
// render_runs_page — Render the run cockpit page
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Added run queue and blocked cockpit view]
// END_CHANGE_SUMMARY

use super::render::{html_escape, json_pre, page_shell, primary_nav};
use crate::run::{RunManager, RunRecord, RunStatus};
use serde_json::{json, Value};
use std::path::Path;

// START_public_api

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
    let events = selected.map(run_timeline).unwrap_or_default();
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
    })
}
// END_runs_payload

// START_CONTRACT_render_runs_page
// PURPOSE: Render bounded run queue, blocked board, and selected-run provenance timeline
// INPUTS: { root: &Path }, { run_id: &str }
// OUTPUTS: { String }
// START_render_runs_page
pub(in crate::dashboard) fn render_runs_page(root: &Path, run_id: &str) -> String {
    let payload = runs_payload(root, run_id);
    let body = format!(
        "{}<section class=\"grid\">{}{}{}{}</section><section class=\"card\"><h2>Queue</h2>{}</section><section class=\"card\"><h2>Blocked</h2>{}</section><section class=\"card\"><h2>Provenance</h2>{}</section>",
        primary_nav(),
        metric("Total", &payload["summary"]["total"]),
        metric("Queue", &payload["summary"]["queue"]),
        metric("Blocked", &payload["summary"]["blocked"]),
        metric("Done", &payload["summary"]["completed"]),
        runs_table(&payload["queue"]),
        blocked_table(&payload["blocked"]),
        timeline_table(&payload["events"]),
    );
    page_shell("Run Cockpit", &body)
}
// END_render_runs_page

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
        "traceability_summary": run.metadata.get("traceability_summary").cloned(),
        "retry_count": run.metadata.get("retry_count").cloned(),
        "updated_at": run.updated_at.clone(),
    })
}

fn run_timeline(run: &RunRecord) -> Vec<Value> {
    let mut events = vec![json!({
        "kind": "state",
        "status": format!("{:?}", run.status),
        "detail": run.blocked_reason.clone().unwrap_or_else(|| run.objective.clone()),
        "evidence": run.evidence_refs.clone(),
    })];
    events.extend(run.required_gates.iter().map(|gate| {
        json!({
            "kind": "gate",
            "status": format!("{:?}", gate.status),
            "detail": gate.reason.clone().unwrap_or_else(|| gate.name.clone()),
            "evidence": gate.evidence_refs.clone(),
        })
    }));
    events.extend(run.steps.iter().map(|step| {
        json!({
            "kind": "step",
            "status": format!("{:?}", step.status),
            "detail": step.error.clone().unwrap_or_else(|| step.description.clone()),
            "evidence": step.evidence_refs.clone(),
        })
    }));
    events
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
            "retry_count",
        ],
    )
}

fn timeline_table(rows: &Value) -> String {
    rows_table(rows, &["kind", "status", "detail", "evidence"])
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
