// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER
// PURPOSE: Attach context pressure metadata and new-session recommendations to MCP tool responses.
// SCOPE: context pressure metadata mutation for JSON-RPC result envelopes and agent_resume auto-recommendation hints.
// DEPENDS: M-CONFIG, M-TRACKING, M-MCP-SERVER
// LINKS:
//   -> M-CONFIG (depends) - reads configured context_window_limit
//   -> M-TRACKING (depends) - queries current session context pressure
//   -> NFR-003 (traces_to) - pressure metadata protects token economy during long MCP sessions

// START_MODULE_MAP
// attach_context_pressure_metadata - Add context pressure and auto recommendation metadata when thresholds require it
// attach_pressure_fields - Mutate a JSON-RPC result object with pressure metadata
// run_id_hint - Resolve an optional run id from tool args or result payload
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added MCP context pressure response metadata]
// END_CHANGE_SUMMARY

use crate::config::Config;
use crate::tracking::{PressureLevel, Tracker};

const AUTO_RESUME_PRESSURE_PCT: f64 = 90.0;

// START_public_api

// START_CONTRACT_attach_context_pressure_metadata
// PURPOSE: Add context pressure metadata to a JSON-RPC result envelope when the current session is near exhaustion.
// INPUTS: { config: &Config }, { tracker: &Tracker }, { args: &serde_json::Value }, { response: &mut serde_json::Value }
// SIDE_EFFECTS: mutates response result metadata when pressure thresholds are crossed
// LINKS:
//   -> M-TRACKING (depends) - uses Tracker::context_pressure for threshold state
// START_attach_context_pressure_metadata
pub(crate) async fn attach_context_pressure_metadata(
    config: &Config,
    tracker: &Tracker,
    args: &serde_json::Value,
    response: &mut serde_json::Value,
) {
    let pressure = match tracker
        .context_pressure(config.budget.context_window_limit)
        .await
    {
        Ok(pressure) => pressure,
        Err(error) => {
            tracing::warn!(
                "[SynapseHandler][handle_message][MCP_CONTEXT_PRESSURE] {}",
                error
            );
            return;
        }
    };
    attach_pressure_fields(args, response, &pressure);
}
// END_attach_context_pressure_metadata

// START_CONTRACT_attach_pressure_fields
// PURPOSE: Mutate one JSON-RPC result object with critical pressure and auto-resume metadata.
// INPUTS: { args: &serde_json::Value }, { response: &mut serde_json::Value }, { pressure: &crate::tracking::ContextPressure }
// SIDE_EFFECTS: inserts _context_pressure and/or _auto_recommendation fields
// START_attach_pressure_fields
fn attach_pressure_fields(
    args: &serde_json::Value,
    response: &mut serde_json::Value,
    pressure: &crate::tracking::ContextPressure,
) {
    let Some(result) = response
        .get_mut("result")
        .and_then(|value| value.as_object_mut())
    else {
        return;
    };
    if pressure.level == PressureLevel::Critical {
        result.insert(
            "_context_pressure".into(),
            serde_json::json!({
                "level": pressure.level,
                "pressure_pct": pressure.pressure_pct,
                "headroom": pressure.headroom,
                "recommendation": pressure.recommendation
            }),
        );
    }
    if pressure.pressure_pct > AUTO_RESUME_PRESSURE_PCT {
        result.insert(
            "_auto_recommendation".into(),
            serde_json::json!({
                "action": "agent_resume",
                "reason": format!(
                    "Context window at {:.1}%. Save progress and continue in a new session.",
                    pressure.pressure_pct
                ),
                "run_id": run_id_hint(args, result)
            }),
        );
    }
}
// END_attach_pressure_fields

// START_CONTRACT_run_id_hint
// PURPOSE: Resolve an optional run_id from tool arguments or an existing result payload.
// INPUTS: { args: &serde_json::Value }, { result: &serde_json::Map<String, serde_json::Value> }
// OUTPUTS: { serde_json::Value }
// START_run_id_hint
fn run_id_hint(
    args: &serde_json::Value,
    result: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    args.get("run_id")
        .or_else(|| result.get("run_id"))
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}
// END_run_id_hint

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_pressure
    // PURPOSE: Build a pressure report for response metadata tests.
    // INPUTS: { pressure_pct: f64 }, { level: PressureLevel }
    // OUTPUTS: { crate::tracking::ContextPressure }
    // START_test_pressure
    fn test_pressure(pressure_pct: f64, level: PressureLevel) -> crate::tracking::ContextPressure {
        crate::tracking::ContextPressure {
            session_id: "pressure-test".into(),
            estimated_context_tokens: 950,
            context_window_limit: 1_000,
            pressure_pct,
            level,
            recommendation: "resume soon".into(),
            headroom: 50,
        }
    }
    // END_test_pressure

    // START_CONTRACT_critical_pressure_adds_context_and_resume_metadata
    // PURPOSE: Verify critical pressure adds both context pressure and agent_resume recommendation metadata.
    // START_critical_pressure_adds_context_and_resume_metadata
    #[test]
    fn critical_pressure_adds_context_and_resume_metadata() {
        let mut response = serde_json::json!({"result": {"run_id": "run-1"}});
        let pressure = test_pressure(95.1, PressureLevel::Critical);

        attach_pressure_fields(&serde_json::json!({}), &mut response, &pressure);

        assert_eq!(response["result"]["_context_pressure"]["level"], "critical");
        assert_eq!(
            response["result"]["_auto_recommendation"]["action"],
            "agent_resume"
        );
        assert_eq!(
            response["result"]["_auto_recommendation"]["run_id"],
            "run-1"
        );
    }
    // END_critical_pressure_adds_context_and_resume_metadata

    // START_CONTRACT_high_pressure_above_resume_threshold_skips_context_metadata
    // PURPOSE: Verify high pressure above 90 percent adds auto-resume metadata without critical metadata.
    // START_high_pressure_above_resume_threshold_skips_context_metadata
    #[test]
    fn high_pressure_above_resume_threshold_skips_context_metadata() {
        let mut response = serde_json::json!({"result": {}});
        let pressure = test_pressure(90.1, PressureLevel::High);

        attach_pressure_fields(
            &serde_json::json!({"run_id": "arg-run"}),
            &mut response,
            &pressure,
        );

        assert!(response["result"].get("_context_pressure").is_none());
        assert_eq!(
            response["result"]["_auto_recommendation"]["run_id"],
            "arg-run"
        );
    }
    // END_high_pressure_above_resume_threshold_skips_context_metadata
}
