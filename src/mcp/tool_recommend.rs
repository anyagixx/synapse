// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-RUN-TOOLS
// PURPOSE: Predict MCP tool subsets for the current agent task before clients request full schemas.
// SCOPE: tools/recommend handler, work-phase detection from run state and context, bounded recommendation lists, and custom tools/list profile construction.
// DEPENDS: M-RUNNER, M-RUNNER-AGENT-CONTEXT, M-MCP-SERVER-RESPONSE
// LINKS:
//   -> M-RUNNER (depends) - loads persisted run records
//   -> M-RUNNER-AGENT-CONTEXT (depends) - consumes compact run-state signals
//   -> M-MCP-SERVER-RESPONSE (depends) - emits JSON-RPC envelopes
//   -> NFR-003 (traces_to) - tool preselection reduces schema context tokens

// START_MODULE_MAP
// handle_recommend - MCP tools/recommend handler
// recommend_tools - Build recommendations from args and project root
// phase_from_run_state - Detect work phase from persisted run signals
// phase_from_context - Detect work phase from task context text
// recommendations_for_phase - Return bounded recommendation candidates
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added predictive MCP tool recommendation engine]
// END_CHANGE_SUMMARY

use super::server_response::{error, result};
use crate::run::agent_context::RunStateSignals;
use crate::run::RunManager;
use std::path::PathBuf;

// START_public_api

// START_WorkPhase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkPhase {
    Init,
    Plan,
    Implement,
    Verify,
    Debug,
    General,
}
// END_WorkPhase

impl WorkPhase {
    // START_CONTRACT_WorkPhase::label
    // PURPOSE: Return stable response label for a detected work phase.
    // OUTPUTS: { &'static str }
    // START_work_phase_label
    fn label(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Plan => "plan",
            Self::Implement => "implement",
            Self::Verify => "verify",
            Self::Debug => "debug",
            Self::General => "general",
        }
    }
    // END_work_phase_label
}

// START_CONTRACT_handle_recommend
// PURPOSE: Handle tools/recommend and return a compact recommended tool subset.
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_recommend
pub(crate) async fn handle_recommend(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    match recommend_tools(args) {
        Ok(payload) => result(id, payload),
        Err(message) => error(id, -32603, format!("tools/recommend: {message}")),
    }
}
// END_handle_recommend

// START_CONTRACT_recommend_tools
// PURPOSE: Build tool recommendations by prioritizing run state, then context, then general fallback.
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Result<serde_json::Value, String> }
// START_recommend_tools
fn recommend_tools(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let context = args
        .get("context")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let max_tools = args
        .get("max_tools")
        .and_then(serde_json::Value::as_u64)
        .map(|value| value.clamp(1, 8) as usize)
        .unwrap_or(8);
    let run_state = run_state_from_args(args)?;
    let run_state_phase = run_state.as_ref().map(phase_from_run_state);
    let context_phase = phase_from_context(context);
    let (phase, source) =
        if let Some(phase) = run_state_phase.filter(|phase| *phase != WorkPhase::General) {
            (phase, "run_state")
        } else if context_phase != WorkPhase::General {
            (context_phase, "context")
        } else {
            (WorkPhase::General, "general")
        };
    let recommended_tools = recommendations_for_phase(phase, max_tools);
    let names = recommended_tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();

    Ok(serde_json::json!({
        "phase": phase.label(),
        "source": source,
        "run_state": run_state,
        "recommended_tools": recommended_tools,
        "suggested_profile": format!("custom:{}", names.join(",")),
        "max_tools": max_tools
    }))
}
// END_recommend_tools

// START_CONTRACT_run_state_from_args
// PURPOSE: Load compact run-state signals when run_id is present.
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Result<Option<RunStateSignals>, String> }
// START_run_state_from_args
fn run_state_from_args(args: &serde_json::Value) -> Result<Option<RunStateSignals>, String> {
    let Some(run_id) = args
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|run_id| !run_id.is_empty())
    else {
        return Ok(None);
    };
    let manager = RunManager::new(project_root_arg(args)?);
    manager
        .run_state_signals(run_id)
        .map(Some)
        .map_err(|error| error.to_string())
}
// END_run_state_from_args

// START_CONTRACT_project_root_arg
// PURPOSE: Resolve optional project_root for run_id-aware recommendation tests and local clients.
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Result<PathBuf, String> }
// START_project_root_arg
fn project_root_arg(args: &serde_json::Value) -> Result<PathBuf, String> {
    match args.get("project_root") {
        None | Some(serde_json::Value::Null) => {
            std::env::current_dir().map_err(|error| format!("current_dir: {error}"))
        }
        Some(serde_json::Value::String(value)) if !value.trim().is_empty() => {
            Ok(PathBuf::from(value))
        }
        Some(serde_json::Value::String(_)) => Err("project_root must not be empty".into()),
        Some(_) => Err("project_root must be a string".into()),
    }
}
// END_project_root_arg

// START_CONTRACT_phase_from_run_state
// PURPOSE: Detect work phase from compact persisted run-state signals.
// INPUTS: { signals: &RunStateSignals }
// OUTPUTS: { WorkPhase }
// START_phase_from_run_state
fn phase_from_run_state(signals: &RunStateSignals) -> WorkPhase {
    let text = format!(
        "{} {} {} {}",
        signals.status,
        signals.current_gate.as_deref().unwrap_or(""),
        signals.module,
        signals.next_action.as_deref().unwrap_or("")
    );
    detect_phase(&text)
}
// END_phase_from_run_state

// START_CONTRACT_phase_from_context
// PURPOSE: Detect work phase from optional task context text.
// INPUTS: { context: &str }
// OUTPUTS: { WorkPhase }
// START_phase_from_context
fn phase_from_context(context: &str) -> WorkPhase {
    if context.trim().is_empty() {
        WorkPhase::General
    } else {
        detect_phase(context)
    }
}
// END_phase_from_context

// START_CONTRACT_detect_phase
// PURPOSE: Classify free text into the six recommendation work phases.
// INPUTS: { text: &str }
// OUTPUTS: { WorkPhase }
// START_detect_phase
fn detect_phase(text: &str) -> WorkPhase {
    let lower = text.to_ascii_lowercase();
    if contains_any(
        &lower,
        &["debug", "failure", "failed", "error", "repair", "diagnose"],
    ) {
        WorkPhase::Debug
    } else if contains_any(
        &lower,
        &[
            "verify",
            "review",
            "contract",
            "traceability",
            "gate",
            "test",
        ],
    ) {
        WorkPhase::Verify
    } else if contains_any(
        &lower,
        &[
            "requirement",
            "technology",
            "architecture",
            "plan",
            "design",
        ],
    ) {
        WorkPhase::Plan
    } else if contains_any(&lower, &["init", "bootstrap", "setup", "install"]) {
        WorkPhase::Init
    } else if contains_any(
        &lower,
        &["implement", "code", "develop", "edit", "refactor", "module"],
    ) {
        WorkPhase::Implement
    } else {
        WorkPhase::General
    }
}
// END_detect_phase

// START_CONTRACT_contains_any
// PURPOSE: Return true when text contains at least one keyword.
// INPUTS: { text: &str }, { needles: &[&str] }
// OUTPUTS: { bool }
// START_contains_any
fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}
// END_contains_any

// START_CONTRACT_recommendations_for_phase
// PURPOSE: Return no more than max_tools recommendations for one work phase.
// INPUTS: { phase: WorkPhase }, { max_tools: usize }
// OUTPUTS: { Vec<serde_json::Value> }
// START_recommendations_for_phase
fn recommendations_for_phase(phase: WorkPhase, max_tools: usize) -> Vec<serde_json::Value> {
    candidates_for_phase(phase)
        .iter()
        .take(max_tools.min(8))
        .map(|candidate| {
            serde_json::json!({
                "name": candidate.0,
                "relevance": candidate.1,
                "reason": candidate.2,
                "phase": phase.label()
            })
        })
        .collect()
}
// END_recommendations_for_phase

// START_CONTRACT_candidates_for_phase
// PURPOSE: Return ordered recommendation candidates for each work phase.
// INPUTS: { phase: WorkPhase }
// OUTPUTS: { &'static [(&'static str, f64, &'static str)] }
// START_candidates_for_phase
fn candidates_for_phase(phase: WorkPhase) -> &'static [(&'static str, f64, &'static str)] {
    match phase {
        WorkPhase::Init => &[
            ("grace_init", 0.96, "Initialize MyGRACE structure"),
            ("generate_requirements", 0.92, "Create requirements"),
            ("generate_technology", 0.88, "Create technology plan"),
            ("generate_development_plan", 0.84, "Create development plan"),
            ("project_status", 0.76, "Check project health"),
        ],
        WorkPhase::Plan => &[
            ("generate_requirements", 0.95, "Refine requirements"),
            ("generate_technology", 0.90, "Check stack compatibility"),
            (
                "generate_development_plan",
                0.88,
                "Plan implementation order",
            ),
            ("mental_test_run", 0.82, "Run mental tests"),
            ("cascade_impact", 0.78, "Preview downstream impact"),
            ("graphrag_query", 0.72, "Inspect architecture graph"),
        ],
        WorkPhase::Implement => &[
            ("semantic_search", 0.94, "Find existing code patterns"),
            ("graphrag_query", 0.88, "Inspect dependencies"),
            ("view_signatures", 0.82, "Inspect public APIs"),
            ("lsp_hover", 0.78, "Inspect types"),
            ("lsp_references", 0.74, "Find usages"),
            ("suggest_contract", 0.70, "Draft contracts"),
        ],
        WorkPhase::Verify => &[
            ("verify_project", 0.98, "Run verification"),
            ("review_code", 0.94, "Run code review"),
            ("traceability_report", 0.90, "Check traceability"),
            ("pre_commit_check", 0.82, "Check run completion gates"),
            ("project_status", 0.78, "Summarize health"),
            ("mental_test_run", 0.72, "Verify mental tests"),
        ],
        WorkPhase::Debug => &[
            ("diagnose_failure", 0.98, "Diagnose failure evidence"),
            ("self_heal", 0.92, "Run bounded self-heal"),
            ("analyze_logs", 0.88, "Analyze LOG evidence"),
            ("semantic_search", 0.82, "Find fault location"),
            ("graphrag_query", 0.78, "Inspect affected graph"),
            ("repair_contract", 0.74, "Repair contracts"),
        ],
        WorkPhase::General => &[
            ("semantic_search", 0.86, "Search code"),
            ("graphrag_query", 0.82, "Inspect graph"),
            ("verify_project", 0.78, "Check correctness"),
            ("review_code", 0.74, "Review changes"),
            ("project_status", 0.70, "Read project health"),
            ("tools/recommend", 0.66, "Refine tool subset"),
        ],
    }
}
// END_candidates_for_phase

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{RunGate, RunGateStatus, RunRecord, RunStatus};

    // START_CONTRACT_test_verify_context_recommends_verification_tools
    // PURPOSE: Verify verification context recommends verify, review, and traceability tools.
    // START_test_verify_context_recommends_verification_tools
    #[test]
    fn test_verify_context_recommends_verification_tools() {
        let payload = recommend_tools(&serde_json::json!({"context": "verify contracts"})).unwrap();
        let names = tool_names(&payload);

        assert_eq!(payload["phase"], "verify");
        assert!(names.contains(&"verify_project"));
        assert!(names.contains(&"review_code"));
        assert!(names.contains(&"traceability_report"));
    }
    // END_test_verify_context_recommends_verification_tools

    // START_CONTRACT_test_debug_context_recommends_debug_tools
    // PURPOSE: Verify debug context recommends failure diagnosis tools.
    // START_test_debug_context_recommends_debug_tools
    #[test]
    fn test_debug_context_recommends_debug_tools() {
        let payload = recommend_tools(&serde_json::json!({"context": "debug M-AUTH"})).unwrap();
        let names = tool_names(&payload);

        assert_eq!(payload["phase"], "debug");
        assert!(names.contains(&"diagnose_failure"));
        assert!(names.contains(&"self_heal"));
        assert!(names.contains(&"analyze_logs"));
    }
    // END_test_debug_context_recommends_debug_tools

    // START_CONTRACT_test_empty_context_returns_general_essentials
    // PURPOSE: Verify missing context falls back to general essentials and respects max_tools.
    // START_test_empty_context_returns_general_essentials
    #[test]
    fn test_empty_context_returns_general_essentials() {
        let payload = recommend_tools(&serde_json::json!({"max_tools": 3})).unwrap();
        let names = tool_names(&payload);

        assert_eq!(payload["phase"], "general");
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "semantic_search");
    }
    // END_test_empty_context_returns_general_essentials

    // START_CONTRACT_test_run_state_takes_priority_over_context
    // PURPOSE: Verify run_id signals override conflicting context text.
    // START_test_run_state_takes_priority_over_context
    #[test]
    fn test_run_state_takes_priority_over_context() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let mut record = RunRecord::new(
            "goal".into(),
            "Phase-86".into(),
            "M-AUTH".into(),
            "verify auth".into(),
        );
        record.status = RunStatus::Blocked;
        record.required_gates.push(RunGate {
            id: "gate-verification".into(),
            name: "Verification".into(),
            required: true,
            status: RunGateStatus::Blocked,
            reason: None,
            evidence_refs: Vec::new(),
        });
        manager.save(&record).unwrap();

        let payload = recommend_tools(&serde_json::json!({
            "context": "implement feature",
            "run_id": record.run_id,
            "project_root": root.path()
        }))
        .unwrap();

        assert_eq!(payload["source"], "run_state");
        assert_eq!(payload["phase"], "verify");
    }
    // END_test_run_state_takes_priority_over_context

    fn tool_names(payload: &serde_json::Value) -> Vec<&str> {
        payload["recommended_tools"]
            .as_array()
            .expect("recommended_tools")
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect()
    }
}
