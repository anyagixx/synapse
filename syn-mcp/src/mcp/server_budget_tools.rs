// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-GRACE-TOOLS
// PURPOSE: MCP handlers for reporting current session token budget and context pressure status.
// SCOPE: check_budget and context_pressure tools/call handlers, estimated token affordability checks, context-window pressure estimates, and structured budget response envelopes.
// DEPENDS: M-CONFIG, M-TRACKING, M-MCP-SERVER-RESPONSE
// LINKS:
//   -> M-CONFIG (depends) - reads configured session token budget and context window thresholds
//   -> M-TRACKING (depends) - queries current session token usage
//   -> M-MCP-SERVER-RESPONSE (depends) - emits JSON-RPC envelopes
//   -> NFR-003 (traces_to) - budget visibility protects session token economy

// START_MODULE_MAP
// handle_check_budget - Return current session budget status and optional affordability result
// handle_context_pressure - Return current session context pressure status
// optional_u64_arg - Parse optional unsigned integer tool arguments
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 - Added context_pressure MCP handler]
// END_CHANGE_SUMMARY

use super::server_response::{error, result};
use syn_core::config::Config;
use syn_core::tracking::Tracker;

// START_public_api

// START_CONTRACT_handle_check_budget
// PURPOSE: Return current session token budget status and optional estimated-token affordability.
// INPUTS: { config: &Config }, { tracker: &Tracker }, { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// LINKS:
//   -> M-CONFIG (depends) - budget thresholds come from Config
//   -> M-TRACKING (depends) - usage comes from current Tracker session scope
// START_handle_check_budget
pub(crate) async fn handle_check_budget(
    config: &Config,
    tracker: &Tracker,
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let estimated_tokens = match optional_u64_arg(args, "estimated_tokens") {
        Ok(value) => value,
        Err(message) => return error(id, -32602, message),
    };
    let budget = &config.budget;
    let status = match tracker
        .check_budget(
            budget.session_token_limit,
            budget.warn_at_pct,
            budget.block_at_pct,
        )
        .await
    {
        Ok(status) => status,
        Err(err) => return error(id, -32603, format!("check_budget: {}", err)),
    };
    let can_afford_estimated = match estimated_tokens {
        Some(tokens) => match tracker
            .can_afford(tokens, budget.session_token_limit, budget.block_at_pct)
            .await
        {
            Ok(value) => Some(value),
            Err(err) => return error(id, -32603, format!("check_budget: {}", err)),
        },
        None => None,
    };

    result(
        id,
        serde_json::json!({
            "content": [{"type": "text", "text": status.message}],
            "budget": status,
            "estimated_tokens": estimated_tokens,
            "can_afford_estimated": can_afford_estimated,
            "isError": false
        }),
    )
}
// END_handle_check_budget

// START_CONTRACT_handle_context_pressure
// PURPOSE: Return current session context-window pressure from configured context limit.
// INPUTS: { config: &Config }, { tracker: &Tracker }, { id: Option<serde_json::Value> }
// OUTPUTS: { serde_json::Value }
// LINKS:
//   -> M-CONFIG (depends) - context limit comes from Config budget settings
//   -> M-TRACKING (depends) - pressure comes from current Tracker session scope
// START_handle_context_pressure
pub(crate) async fn handle_context_pressure(
    config: &Config,
    tracker: &Tracker,
    id: Option<serde_json::Value>,
) -> serde_json::Value {
    let pressure = match tracker
        .context_pressure(config.budget.context_window_limit)
        .await
    {
        Ok(pressure) => pressure,
        Err(err) => return error(id, -32603, format!("context_pressure: {}", err)),
    };

    result(
        id,
        serde_json::json!({
            "content": [{"type": "text", "text": pressure.recommendation}],
            "session_id": pressure.session_id,
            "estimated_context_tokens": pressure.estimated_context_tokens,
            "context_window_limit": pressure.context_window_limit,
            "pressure_pct": pressure.pressure_pct,
            "level": pressure.level,
            "recommendation": pressure.recommendation,
            "headroom": pressure.headroom,
            "isError": false
        }),
    )
}
// END_handle_context_pressure

// START_CONTRACT_optional_u64_arg
// PURPOSE: Parse an optional unsigned integer tool argument.
// INPUTS: { args: &serde_json::Value }, { key: &str }
// OUTPUTS: { Result<Option<u64>, String> }
// START_optional_u64_arg
fn optional_u64_arg(args: &serde_json::Value, key: &str) -> Result<Option<u64>, String> {
    match args.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("{key} must be an unsigned integer")),
    }
}
// END_optional_u64_arg

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_tracker
    // PURPOSE: Create isolated config and tracker state for budget MCP handler tests.
    // OUTPUTS: { (tempfile::TempDir, tempfile::TempDir, Config, Tracker) }
    // START_test_tracker
    fn test_tracker(limit: u64) -> (tempfile::TempDir, tempfile::TempDir, Config, Tracker) {
        let data_home = tempfile::tempdir().expect("data home");
        let project_root = tempfile::tempdir().expect("project root");
        let mut config = Config::default();
        config.budget.session_token_limit = limit;
        config.budget.warn_at_pct = 80;
        config.budget.block_at_pct = 100;
        let tracker = Tracker::new_for_test(
            &config,
            data_home.path(),
            project_root.path(),
            Some("budget-tool"),
        );
        (data_home, project_root, config, tracker)
    }
    // END_test_tracker

    // START_CONTRACT_test_handle_check_budget_unlimited_can_afford
    // PURPOSE: Verify check_budget reports unlimited default budget and can afford any estimate.
    // START_test_handle_check_budget_unlimited_can_afford
    #[tokio::test]
    async fn test_handle_check_budget_unlimited_can_afford() {
        let (_data_home, _project_root, config, tracker) = test_tracker(0);

        let response = handle_check_budget(
            &config,
            &tracker,
            Some(serde_json::json!(1)),
            &serde_json::json!({"estimated_tokens": 500000}),
        )
        .await;

        assert_eq!(response["result"]["budget"]["limit"], 0);
        assert_eq!(response["result"]["budget"]["status"], "normal");
        assert_eq!(response["result"]["can_afford_estimated"], true);
    }
    // END_test_handle_check_budget_unlimited_can_afford

    // START_CONTRACT_test_handle_check_budget_warning_estimate
    // PURPOSE: Verify check_budget returns warning status and affordability for bounded budgets.
    // START_test_handle_check_budget_warning_estimate
    #[tokio::test]
    async fn test_handle_check_budget_warning_estimate() {
        let (_data_home, _project_root, config, tracker) = test_tracker(100);
        tracker.record("spent", 85, 10).await.expect("seed spend");

        let response = handle_check_budget(
            &config,
            &tracker,
            Some(serde_json::json!(1)),
            &serde_json::json!({"estimated_tokens": 20}),
        )
        .await;

        assert_eq!(response["result"]["budget"]["status"], "warning");
        assert_eq!(response["result"]["budget"]["used_input_tokens"], 85);
        assert_eq!(response["result"]["can_afford_estimated"], false);
    }
    // END_test_handle_check_budget_warning_estimate

    // START_CONTRACT_test_handle_check_budget_rejects_invalid_estimate
    // PURPOSE: Verify check_budget validates estimated_tokens type before querying budget state.
    // START_test_handle_check_budget_rejects_invalid_estimate
    #[tokio::test]
    async fn test_handle_check_budget_rejects_invalid_estimate() {
        let (_data_home, _project_root, config, tracker) = test_tracker(100);

        let response = handle_check_budget(
            &config,
            &tracker,
            Some(serde_json::json!(1)),
            &serde_json::json!({"estimated_tokens": "large"}),
        )
        .await;

        assert_eq!(response["error"]["code"], -32602);
    }
    // END_test_handle_check_budget_rejects_invalid_estimate

    // START_CONTRACT_test_handle_context_pressure_reports_configured_limit
    // PURPOSE: Verify context_pressure reports configured context limit and high pressure state.
    // START_test_handle_context_pressure_reports_configured_limit
    #[tokio::test]
    async fn test_handle_context_pressure_reports_configured_limit() {
        let (_data_home, _project_root, mut config, tracker) = test_tracker(0);
        config.budget.context_window_limit = 1_000;
        tracker
            .record("pressure", 700, 0)
            .await
            .expect("seed spend");

        let response = handle_context_pressure(&config, &tracker, Some(serde_json::json!(1))).await;

        assert_eq!(response["result"]["context_window_limit"], 1_000);
        assert_eq!(response["result"]["level"], "high");
        assert_eq!(response["result"]["isError"], false);
    }
    // END_test_handle_context_pressure_reports_configured_limit
}
