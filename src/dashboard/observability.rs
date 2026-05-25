// MODULE_CONTRACT
// MODULE_ID: M-DASHBOARD-OBSERVABILITY
// PURPOSE: Dashboard observability helpers and endpoints for runtime health, readiness, and MCP metrics summaries
// SCOPE: Health payloads, readiness payloads, config-gated MCP stats payloads, route handlers, and dashboard-facing observability serialization
// DEPENDS: M-CONFIG, M-DASHBOARD, M-TRACKING-MCP-METRICS, M-INDEXER
// LINKS:
//   -> M-CONFIG (depends) - observability toggles and payload limits
//   -> M-DASHBOARD (depends) - dashboard route integration
//   -> M-TRACKING-MCP-METRICS (depends) - MCP runtime metrics source
//   -> M-INDEXER (depends) - readiness signal source

// START_MODULE_MAP
// health — Return cheap dashboard liveness metadata
// health_ready — Return dashboard readiness and MCP runtime state
// api_mcp_stats — Return MCP runtime metrics for dashboard polling
// health_payload — Build the stable /health payload
// readiness_payload_with_tracker — Build readiness payload from an injected tracker
// readiness_payload_with_config — Build readiness payload with explicit observability settings
// mcp_stats_payload_with_config — Build MCP stats payload with explicit observability settings
// mcp_stats_payload_with_tracker — Build MCP stats payload from an injected tracker
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 - Applied observability config toggles and limits]
// END_CHANGE_SUMMARY

use crate::config::ObservabilityConfig;
use crate::indexer::storage::Storage;
use crate::tracking::{mcp_metrics::McpMetricsSnapshot, Tracker};
use axum::Json;
use serde_json::{json, Value};
use std::path::Path;

// START_public_api

// START_CONTRACT_health
// PURPOSE: Return cheap dashboard process liveness metadata
// OUTPUTS: { Json<Value> }
// LINKS:
//   -> UC-001 (implements) - inspect local GRACE and runtime state through dashboard surfaces
// START_health
pub(crate) async fn health() -> Json<Value> {
    Json(health_payload())
}
// END_health

// START_CONTRACT_health_ready
// PURPOSE: Return readiness state for dashboard runtime dependencies and MCP activity
// OUTPUTS: { Json<Value> }
// LINKS:
//   -> UC-001 (implements) - dashboard readiness is a user-facing inspection surface
//   -> M-TRACKING-MCP-METRICS (depends) - active MCP request and latency source
//   -> M-INDEXER (depends) - index presence readiness signal
// START_health_ready
pub(crate) async fn health_ready() -> Json<Value> {
    let config = crate::config::Config::load_or_default();
    let tracker = Tracker::new(&config);
    Json(
        readiness_payload_with_config(&super::current_root(), &tracker, &config.observability)
            .await,
    )
}
// END_health_ready

// START_CONTRACT_api_mcp_stats
// PURPOSE: Return aggregate MCP runtime metrics for dashboard polling
// OUTPUTS: { Json<Value> }
// LINKS:
//   -> UC-001 (implements) - dashboard API exposes MCP runtime metrics to users
//   -> M-TRACKING-MCP-METRICS (depends) - MCP metrics aggregate query source
// START_api_mcp_stats
pub(crate) async fn api_mcp_stats() -> Json<Value> {
    let config = crate::config::Config::load_or_default();
    let tracker = Tracker::new(&config);
    Json(mcp_stats_payload_with_config(&tracker, &config.observability).await)
}
// END_api_mcp_stats

// START_CONTRACT_health_payload
// PURPOSE: Build the stable liveness JSON payload without touching storage
// OUTPUTS: { Value }
// LINKS:
//   -> UC-001 (implements) - dashboard liveness remains cheap and predictable
// START_health_payload
pub(crate) fn health_payload() -> Value {
    json!({
        "status": "ok",
        "version": crate::VERSION,
        "name": crate::NAME
    })
}
// END_health_payload

// START_CONTRACT_readiness_payload_with_tracker
// PURPOSE: Build readiness JSON from explicit project root and tracker inputs using default observability settings
// INPUTS: { root: &Path }, { tracker: &Tracker }
// OUTPUTS: { Value }
// LINKS:
//   -> UC-001 (implements) - dashboard readiness combines runtime health signals
//   -> M-TRACKING-MCP-METRICS (depends) - readiness reports active MCP requests
//   -> M-INDEXER (depends) - readiness reports index storage presence
// START_readiness_payload_with_tracker
#[cfg(test)]
pub(crate) async fn readiness_payload_with_tracker(root: &Path, tracker: &Tracker) -> Value {
    readiness_payload_with_config(root, tracker, &ObservabilityConfig::default()).await
}
// END_readiness_payload_with_tracker

// START_CONTRACT_readiness_payload_with_config
// PURPOSE: Build readiness JSON from explicit project root, tracker, and observability settings
// INPUTS: { root: &Path }, { tracker: &Tracker }, { observability: &ObservabilityConfig }
// OUTPUTS: { Value }
// LINKS:
//   -> UC-001 (implements) - dashboard readiness combines runtime health signals
//   -> M-CONFIG (depends) - observability toggles and limits control readiness payload size
//   -> M-TRACKING-MCP-METRICS (depends) - readiness reports active MCP requests
//   -> M-INDEXER (depends) - readiness reports index storage presence
// START_readiness_payload_with_config
pub(crate) async fn readiness_payload_with_config(
    root: &Path,
    tracker: &Tracker,
    observability: &ObservabilityConfig,
) -> Value {
    let root_signal = root_signal(root);
    let index_signal = index_signal(root);
    let mcp_stats = if observability.mcp_metrics_enabled() {
        mcp_stats_payload_with_tracker(tracker, observability.readiness_mcp_stats_limit()).await
    } else {
        disabled_mcp_stats_payload()
    };
    let tracking_ready = mcp_stats["status"].as_str() == Some("ok");
    let ready = observability.enabled()
        && root_signal["ready"].as_bool().unwrap_or(false)
        && tracking_ready;

    json!({
        "status": readiness_status(observability, ready),
        "ready": ready,
        "version": crate::VERSION,
        "name": crate::NAME,
        "root": root.display().to_string(),
        "components": {
            "observability": {
                "ready": observability.enabled(),
                "status": if observability.enabled() { "ok" } else { "disabled" }
            },
            "root": root_signal,
            "index": index_signal,
            "tracking": {
                "ready": tracking_ready,
                "status": mcp_stats["status"].clone(),
                "error": mcp_stats.get("error").cloned().unwrap_or(Value::Null)
            }
        },
        "mcp": mcp_stats["metrics"].clone()
    })
}
// END_readiness_payload_with_config

// START_CONTRACT_mcp_stats_payload_with_tracker
// PURPOSE: Build dashboard-safe MCP metrics JSON from tracking storage
// INPUTS: { tracker: &Tracker }, { limit: usize }
// OUTPUTS: { Value }
// LINKS:
//   -> NFR-003 (traces_to) - MCP metrics quantify runtime reliability and economics
//   -> M-TRACKING-MCP-METRICS (depends) - MCP metrics aggregate query source
// START_mcp_stats_payload_with_tracker
pub(crate) async fn mcp_stats_payload_with_tracker(tracker: &Tracker, limit: usize) -> Value {
    match tracker.get_mcp_metrics(limit).await {
        Ok(metrics) => json!({
            "status": "ok",
            "metrics": metrics
        }),
        Err(error) => json!({
            "status": "degraded",
            "error": error.to_string(),
            "metrics": McpMetricsSnapshot::default()
        }),
    }
}
// END_mcp_stats_payload_with_tracker

// START_CONTRACT_mcp_stats_payload_with_config
// PURPOSE: Build dashboard-safe MCP metrics JSON using observability toggles and limits
// INPUTS: { tracker: &Tracker }, { observability: &ObservabilityConfig }
// OUTPUTS: { Value }
// LINKS:
//   -> UC-001 (implements) - dashboard API exposes config-governed MCP runtime metrics
//   -> M-CONFIG (depends) - observability toggles and limits control MCP stats
//   -> M-TRACKING-MCP-METRICS (depends) - MCP metrics aggregate query source
// START_mcp_stats_payload_with_config
pub(crate) async fn mcp_stats_payload_with_config(
    tracker: &Tracker,
    observability: &ObservabilityConfig,
) -> Value {
    if !observability.mcp_metrics_enabled() {
        return disabled_mcp_stats_payload();
    }
    mcp_stats_payload_with_tracker(tracker, observability.mcp_stats_limit()).await
}
// END_mcp_stats_payload_with_config
// END_public_api

// START_CONTRACT_disabled_mcp_stats_payload
// PURPOSE: Build a stable MCP stats payload when observability metrics are disabled
// OUTPUTS: { Value }
// LINKS:
//   -> UC-001 (implements) - dashboard preserves JSON shape when metrics are disabled
// START_disabled_mcp_stats_payload
fn disabled_mcp_stats_payload() -> Value {
    json!({
        "status": "disabled",
        "metrics": McpMetricsSnapshot::default()
    })
}
// END_disabled_mcp_stats_payload

// START_CONTRACT_readiness_status
// PURPOSE: Convert observability enablement and readiness into a stable status string
// INPUTS: { observability: &ObservabilityConfig }, { ready: bool }
// OUTPUTS: { &'static str }
// LINKS:
//   -> UC-001 (implements) - dashboard readiness status remains predictable
// START_readiness_status
fn readiness_status(observability: &ObservabilityConfig, ready: bool) -> &'static str {
    if !observability.enabled() {
        "disabled"
    } else if ready {
        "ready"
    } else {
        "degraded"
    }
}
// END_readiness_status

// START_CONTRACT_root_signal
// PURPOSE: Build root filesystem readiness metadata
// INPUTS: { root: &Path }
// OUTPUTS: { Value }
// LINKS:
//   -> UC-001 (implements) - readiness exposes dashboard project root accessibility
// START_root_signal
fn root_signal(root: &Path) -> Value {
    let ready = root.is_dir();
    json!({
        "ready": ready,
        "status": if ready { "ok" } else { "missing" },
        "path": root.display().to_string()
    })
}
// END_root_signal

// START_CONTRACT_index_signal
// PURPOSE: Build index storage readiness metadata without opening the index
// INPUTS: { root: &Path }
// OUTPUTS: { Value }
// LINKS:
//   -> UC-001 (implements) - readiness exposes indexed-project state to users
//   -> M-INDEXER (depends) - readiness reports index database presence
// START_index_signal
fn index_signal(root: &Path) -> Value {
    let path = Storage::db_path_for_root(root);
    let present = path.exists();
    json!({
        "ready": present,
        "status": if present { "ok" } else { "missing" },
        "path": path.display().to_string()
    })
}
// END_index_signal

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tracking::mcp_metrics::McpCallStatus;

    fn tracker_for_test(data_home: &Path, project_home: &Path) -> Tracker {
        Tracker::new_for_test(
            &Config::default(),
            data_home,
            project_home,
            Some("dashboard-observability-test"),
        )
    }

    #[test]
    fn health_payload_keeps_liveness_shape() {
        let payload = health_payload();

        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["version"], crate::VERSION);
        assert_eq!(payload["name"], crate::NAME);
    }

    #[tokio::test]
    async fn mcp_stats_payload_returns_recorded_metrics() {
        let data_home = tempfile::tempdir().expect("data home");
        let project_home = tempfile::tempdir().expect("project home");
        let tracker = tracker_for_test(data_home.path(), project_home.path());

        tracker
            .record_mcp_call("semantic_search", 24, McpCallStatus::Ok, "")
            .await
            .expect("record metric");
        let payload = mcp_stats_payload_with_tracker(&tracker, 10).await;

        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["metrics"]["total_calls"], 1);
        assert_eq!(
            payload["metrics"]["tools"][0]["tool_name"],
            "semantic_search"
        );
    }

    #[tokio::test]
    async fn mcp_stats_payload_degrades_on_tracking_error() {
        let data_home = tempfile::NamedTempFile::new().expect("data file");
        let project_home = tempfile::tempdir().expect("project home");
        let tracker = tracker_for_test(data_home.path(), project_home.path());

        let payload = mcp_stats_payload_with_tracker(&tracker, 10).await;

        assert_eq!(payload["status"], "degraded");
        assert_eq!(payload["metrics"]["total_calls"], 0);
        assert!(!payload["error"].as_str().unwrap_or_default().is_empty());
    }

    #[tokio::test]
    async fn mcp_stats_payload_honors_disabled_observability() {
        let data_home = tempfile::tempdir().expect("data home");
        let project_home = tempfile::tempdir().expect("project home");
        let tracker = tracker_for_test(data_home.path(), project_home.path());
        let mut observability = ObservabilityConfig::default();
        observability.mcp_metrics_enabled = false;

        let payload = mcp_stats_payload_with_config(&tracker, &observability).await;

        assert_eq!(payload["status"], "disabled");
        assert_eq!(payload["metrics"]["total_calls"], 0);
    }
}
