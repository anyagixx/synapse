// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-RUN-TOOLS
// PURPOSE: MCP handlers for bounded run phase advancement and pre-commit verification tools.
// SCOPE: advance_phase and pre_commit_check JSON-RPC handlers with project_root resolution and structured MCP responses.
// DEPENDS: M-RUNNER, M-RUNNER-PHASE-ENGINE, M-RUNNER-PRECOMMIT, M-MCP-SERVER-RESPONSE
// LINKS:
//   -> M-RUNNER (depends) - invokes RunManager runtime APIs
//   -> M-RUNNER-PHASE-ENGINE (depends) - exposes phase gate advancement
//   -> M-RUNNER-PRECOMMIT (depends) - exposes pre-commit verification
//   -> UC-002 (implements) - MCP clients can gate bounded autonomous completion

// START_MODULE_MAP
// handle_advance_phase - Runs dry-run or applied phase advancement
// handle_pre_commit_check - Runs pre-commit verification for one persisted run
// project_root_arg - Resolves optional MCP project_root argument
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added run gate MCP handlers]
// END_CHANGE_SUMMARY

use super::server_response::{error, result};
use crate::run::RunManager;
use std::path::PathBuf;

// START_public_api

// START_CONTRACT_handle_advance_phase
// PURPOSE: Execute phase gate check and optionally advance active MyGRACE phase.
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// LINKS:
//   -> M-RUNNER-PHASE-ENGINE (depends) - performs gate check and XML updates
// START_handle_advance_phase
pub(crate) async fn handle_advance_phase(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let dry_run = args["dry_run"].as_bool().unwrap_or(true);
    let root = match project_root_arg(args) {
        Ok(root) => root,
        Err(message) => return error(id, -32602, message),
    };
    let manager = RunManager::new(root);
    match manager.advance_phase(dry_run) {
        Ok(report) => result(
            id,
            serde_json::json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&report).unwrap_or_else(|_| "phase report".into())}],
                "report": report,
                "isError": false
            }),
        ),
        Err(err) => error(id, -32603, format!("advance_phase: {}", err)),
    }
}
// END_handle_advance_phase

// START_CONTRACT_handle_pre_commit_check
// PURPOSE: Execute pre-commit verification for a persisted bounded run.
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// LINKS:
//   -> M-RUNNER-PRECOMMIT (depends) - evaluates final completion gates
// START_handle_pre_commit_check
pub(crate) async fn handle_pre_commit_check(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let run_id = args["run_id"].as_str().unwrap_or("");
    if run_id.is_empty() {
        return error(id, -32602, "Missing 'run_id' parameter");
    }
    let root = match project_root_arg(args) {
        Ok(root) => root,
        Err(message) => return error(id, -32602, message),
    };
    let manager = RunManager::new(root);
    match manager.pre_commit_verify(run_id) {
        Ok(report) => result(
            id,
            serde_json::json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&report).unwrap_or_else(|_| "pre-commit report".into())}],
                "report": report,
                "isError": false
            }),
        ),
        Err(err) => error(id, -32603, format!("pre_commit_check: {}", err)),
    }
}
// END_handle_pre_commit_check

// END_public_api

// START_CONTRACT_project_root_arg
// PURPOSE: Resolve optional project_root argument or current working directory.
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Result<PathBuf, String> }
// START_project_root_arg
fn project_root_arg(args: &serde_json::Value) -> Result<PathBuf, String> {
    match args.get("project_root") {
        None | Some(serde_json::Value::Null) => {
            std::env::current_dir().map_err(|err| format!("current_dir: {}", err))
        }
        Some(serde_json::Value::String(value)) if !value.trim().is_empty() => {
            Ok(PathBuf::from(value))
        }
        Some(serde_json::Value::String(_)) => Err("project_root must not be empty".into()),
        Some(_) => Err("project_root must be a string".into()),
    }
}
// END_project_root_arg

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_handle_pre_commit_check_requires_run_id
    // PURPOSE: Verify pre_commit_check rejects missing run_id before touching project state.
    // START_test_handle_pre_commit_check_requires_run_id
    #[tokio::test]
    async fn test_handle_pre_commit_check_requires_run_id() {
        let response =
            handle_pre_commit_check(Some(serde_json::json!(1)), &serde_json::json!({})).await;

        assert_eq!(response["error"]["code"], -32602);
    }
    // END_test_handle_pre_commit_check_requires_run_id

    // START_CONTRACT_test_handle_advance_phase_rejects_invalid_project_root
    // PURPOSE: Verify advance_phase validates project_root type.
    // START_test_handle_advance_phase_rejects_invalid_project_root
    #[tokio::test]
    async fn test_handle_advance_phase_rejects_invalid_project_root() {
        let response = handle_advance_phase(
            Some(serde_json::json!(1)),
            &serde_json::json!({"project_root": ["bad"]}),
        )
        .await;

        assert_eq!(response["error"]["code"], -32602);
    }
    // END_test_handle_advance_phase_rejects_invalid_project_root
}
