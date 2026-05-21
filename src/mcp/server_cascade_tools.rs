// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-CASCADE-TOOLS
// PURPOSE: MCP handlers for GRACE cascade impact preview and execution tools
// SCOPE: cascade_impact and cascade_execute JSON-RPC tool handlers with argument parsing and text envelope formatting
// DEPENDS: M-GRACE-CASCADE, M-MCP-SERVER-RESPONSE
// LINKS: docs/modules/M-MCP-SERVER.xml

// START_MODULE_MAP
// handle_cascade_impact - Runs cascade impact analysis and optionally immediate execution
// handle_cascade_execute - Executes a cached cascade preview
// parse_phase_list - Parses optional apply_to_phases MCP argument
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added cascade MCP handlers]
// END_CHANGE_SUMMARY

use super::server_response::{error, result};
use crate::grace::cascade::{CascadeExecuteOptions, CascadeReport};

// START_public_api

// START_CONTRACT_handle_cascade_impact
// PURPOSE: Execute cascade_impact and optionally cascade_execute from one MCP call
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_cascade_impact
pub(crate) async fn handle_cascade_impact(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(error_value) => return error(id, -32603, format!("cwd error: {}", error_value)),
    };
    let changed_artifact = match args["changed_artifact"].as_str() {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => return error(id, -32602, "Missing 'changed_artifact' parameter"),
    };
    let change_description = args["change_description"]
        .as_str()
        .unwrap_or("Artifact changed");
    let preview_only = args["preview_only"].as_bool().unwrap_or(true);
    match crate::grace::GraceEngine::cascade_impact(&root, changed_artifact, change_description) {
        Ok(analysis) => {
            let mut text = analysis.preview.clone();
            let mut execution: Option<CascadeReport> = None;
            if !preview_only {
                match crate::grace::GraceEngine::cascade_execute(
                    &root,
                    CascadeExecuteOptions {
                        cascade_id: analysis.cascade_id.clone(),
                        auto_apply_contracts: true,
                        auto_apply_code: false,
                        apply_to_phases: Vec::new(),
                    },
                ) {
                    Ok(report) => {
                        text.push_str("\nExecution report:\n");
                        text.push_str(&serde_json::to_string_pretty(&report).unwrap_or_default());
                        execution = Some(report);
                    }
                    Err(error_value) => {
                        return error(
                            id,
                            -32603,
                            format!("Cascade execute error: {}", error_value),
                        )
                    }
                }
            }
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false,
                    "cascade_id": analysis.cascade_id,
                    "analysis": analysis,
                    "execution": execution
                }),
            )
        }
        Err(error_value) => error(id, -32603, format!("Cascade impact error: {}", error_value)),
    }
}
// END_handle_cascade_impact

// START_CONTRACT_handle_cascade_execute
// PURPOSE: Execute a cached cascade preview and return the execution report
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_cascade_execute
pub(crate) async fn handle_cascade_execute(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(error_value) => return error(id, -32603, format!("cwd error: {}", error_value)),
    };
    let cascade_id = match args["cascade_id"].as_str() {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return error(id, -32602, "Missing 'cascade_id' parameter"),
    };
    let options = CascadeExecuteOptions {
        cascade_id,
        auto_apply_contracts: args["auto_apply_contracts"].as_bool().unwrap_or(true),
        auto_apply_code: args["auto_apply_code"].as_bool().unwrap_or(false),
        apply_to_phases: parse_phase_list(args),
    };
    match crate::grace::GraceEngine::cascade_execute(&root, options) {
        Ok(report) => {
            let text = serde_json::to_string_pretty(&report).unwrap_or_default();
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false,
                    "report": report
                }),
            )
        }
        Err(error_value) => error(
            id,
            -32603,
            format!("Cascade execute error: {}", error_value),
        ),
    }
}
// END_handle_cascade_execute

// END_public_api

// START_CONTRACT_parse_phase_list
// PURPOSE: Parse optional apply_to_phases array from MCP arguments
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Vec<String> }
// START_parse_phase_list
fn parse_phase_list(args: &serde_json::Value) -> Vec<String> {
    args["apply_to_phases"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
// END_parse_phase_list

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    // START_CONTRACT_cwd_lock
    // PURPOSE: Return a process-wide lock for tests that change current directory
    // OUTPUTS: { &'static tokio::sync::Mutex<()> }
    // START_cwd_lock
    fn cwd_lock() -> &'static tokio::sync::Mutex<()> {
        static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
    }
    // END_cwd_lock

    // START_CONTRACT_test_handle_cascade_impact_returns_preview
    // PURPOSE: Verify cascade_impact MCP handler returns a preview envelope and cascade id
    // OUTPUTS: { () }
    // START_test_handle_cascade_impact_returns_preview
    #[tokio::test]
    async fn test_handle_cascade_impact_returns_preview() {
        let _guard = cwd_lock().lock().await;
        let dir = tempfile::tempdir().expect("tempdir");
        let old = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("set cwd");

        let response = handle_cascade_impact(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "changed_artifact": "UC-001",
                "change_description": "Add shipping validation"
            }),
        )
        .await;

        std::env::set_current_dir(old).expect("restore cwd");
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or("");
        assert!(text.contains("CASCADE PREVIEW"));
        assert!(response["result"]["cascade_id"]
            .as_str()
            .is_some_and(|value| value.starts_with("CSC-")));
    }
    // END_test_handle_cascade_impact_returns_preview

    // START_CONTRACT_test_handle_cascade_execute_writes_report
    // PURPOSE: Verify cascade_execute MCP handler executes a cached preview and writes a changelog
    // OUTPUTS: { () }
    // START_test_handle_cascade_execute_writes_report
    #[tokio::test]
    async fn test_handle_cascade_execute_writes_report() {
        let _guard = cwd_lock().lock().await;
        let dir = tempfile::tempdir().expect("tempdir");
        let old = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("set cwd");

        let preview = handle_cascade_impact(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "changed_artifact": "UC-001",
                "change_description": "Add shipping validation"
            }),
        )
        .await;
        let cascade_id = preview["result"]["cascade_id"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let response = handle_cascade_execute(
            Some(serde_json::json!(2)),
            &serde_json::json!({
                "cascade_id": cascade_id,
                "auto_apply_contracts": true,
                "auto_apply_code": false,
                "apply_to_phases": ["Phase-2"]
            }),
        )
        .await;

        std::env::set_current_dir(old).expect("restore cwd");
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or("");
        assert!(text.contains("changelog_path"));
        assert!(dir.path().join("docs/cascade/changelogs").exists());
    }
    // END_test_handle_cascade_execute_writes_report
}
