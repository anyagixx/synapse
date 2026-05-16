// MODULE_CONTRACT
// MODULE_ID: M-DASHBOARD
// PURPOSE: Axum web dashboard — serves health, status, graph, and tokens API endpoints
// SCOPE: HTTP server with /health, /api/status, /api/graph, /api/tokens routes
// DEPENDS: M-GRACE-STATUS, M-GRAPHRAG, M-TRACKING, M-CONFIG
// LINKS: N/A

// START_MODULE_MAP
// start_dashboard — Start Axum HTTP server with dashboard API routes
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY

use crate::grace::status::StatusCollector;
use axum::{routing::get, Json, Router};

// START_public_api

// START_CONTRACT_start_dashboard
// PURPOSE: Start the Axum web dashboard HTTP server
// INPUTS: { bind: &str — address:port to bind }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: starts HTTP server, blocks until shutdown
// START_start_dashboard
pub async fn start_dashboard(bind: &str) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/status", get(api_status))
        .route("/api/graph", get(api_graph))
        .route("/api/tokens", get(api_tokens));

    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!("Synapse dashboard: http://{}", bind);
    axum::serve(listener, app).await?;
    Ok(())
}
// END_start_dashboard

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": crate::VERSION,
        "name": crate::NAME
    }))
}

async fn api_status() -> Json<serde_json::Value> {
    let root = std::env::current_dir().unwrap_or_default();
    match StatusCollector::collect(&root).await {
        Ok(report) => Json(serde_json::to_value(report).unwrap_or_default()),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn api_graph() -> Json<serde_json::Value> {
    let root = std::env::current_dir().unwrap_or_default();
    let mut graphrag = crate::graphrag::GraphRag::new();
    match graphrag.build(&root) {
        Ok(()) => {
            let ov = graphrag.overview();
            let nodes: Vec<serde_json::Value> = graphrag
                .graph()
                .map(|g| g.search_nodes(""))
                .unwrap_or_default()
                .iter()
                .map(|n| {
                    serde_json::json!({
                        "id": n.id, "name": n.name, "kind": n.kind,
                        "path": n.path, "language": n.language, "size": n.size_lines
                    })
                })
                .collect();
            Json(serde_json::json!({
                "overview": ov,
                "nodes": nodes
            }))
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn api_tokens() -> Json<serde_json::Value> {
    let config = crate::config::Config::load().unwrap_or_default();
    let tracker = crate::tracking::Tracker::new(&config);
    match tracker.get_stats().await {
        Ok(stats) => Json(serde_json::to_value(stats).unwrap_or_default()),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}
// END_public_api
