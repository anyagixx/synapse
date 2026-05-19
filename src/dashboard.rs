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
        .route("/", get(index_html))
        .route("/api/status", get(api_status))
        .route("/api/graph", get(api_graph))
        .route("/api/tokens", get(api_tokens));

    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(
        "Synapse dashboard: http://{} (HTML: http://{}/)",
        bind,
        bind
    );
    // Graceful shutdown on Ctrl+C
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.ok();
    tracing::info!("Shutting down dashboard...");
}

async fn index_html() -> axum::response::Html<String> {
    let root = std::env::current_dir().unwrap_or_default();
    let layout = crate::grace::layout::DocsLayout::new(&root);
    let status = StatusCollector::collect(&root).await.ok();
    let active_phase = std::fs::read_to_string(layout.plan_index_path())
        .ok()
        .and_then(|content| {
            regex::Regex::new(r#"<ACTIVE_PHASE>([^<]+)</ACTIVE_PHASE>"#)
                .ok()
                .and_then(|re| re.captures(&content).map(|c| c[1].to_string()))
        })
        .unwrap_or_else(|| "unknown".into());
    let token_block = status
        .as_ref()
        .map(|r| format!(
            "<div class=\"card\"><h2>Token Economy</h2><pre>tracked: {}\nsaved: {}\navg savings: {:.1}%</pre></div>",
            r.token_economy.total_commands, r.token_economy.total_saved, r.token_economy.avg_savings_pct
        ))
        .unwrap_or_else(|| "<div class=\"card\"><h2>Token Economy</h2><pre>unavailable</pre></div>".into());
    let next_actions = status
        .as_ref()
        .map(|r| {
            let items = r
                .next_actions
                .iter()
                .take(5)
                .map(|a| format!("<li>{}</li>", a))
                .collect::<Vec<_>>()
                .join("");
            format!(
                "<div class=\"card\"><h2>Next Actions</h2><ul>{}</ul></div>",
                items
            )
        })
        .unwrap_or_else(|| {
            "<div class=\"card\"><h2>Next Actions</h2><pre>unavailable</pre></div>".into()
        });
    axum::response::Html(format!(
        r#"<!DOCTYPE html><html><head><title>Synapse Dashboard</title><meta charset="utf-8"><style>body{{font-family:system-ui;max-width:1000px;margin:2em auto;padding:1em;background:#111;color:#eee}}h1{{color:#5c9cf5}}.grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:1rem}}.card{{background:#1a1a1a;border-radius:8px;padding:1em;margin:1em 0}}pre{{background:#0a0a0a;padding:1em;border-radius:4px;overflow-x:auto}}.pass{{color:#4caf50}}.fail{{color:#f44336}}.node{{display:inline-block;background:#2a2a3a;border-radius:4px;padding:0.3em 0.6em;margin:0.2em}}</style></head><body><h1>Synapse Dashboard v{}</h1><p>Active phase: <strong>{}</strong></p><div class="grid"><div class="card"><h2>Health</h2><a href="/api/status" class="pass">/api/status</a> | <a href="/api/graph">/api/graph</a> | <a href="/api/tokens">/api/tokens</a></div>{}{}</div></body></html>"#,
        crate::VERSION,
        active_phase,
        token_block,
        next_actions,
    ))
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
    let config = crate::config::Config::load_or_default();
    let tracker = crate::tracking::Tracker::new(&config);
    match tracker.get_stats().await {
        Ok(stats) => Json(serde_json::json!({
            "total_commands": stats.total_commands,
            "total_input_tokens": stats.total_input_tokens,
            "total_output_tokens": stats.total_output_tokens,
            "total_saved_tokens": stats.total_saved_tokens,
            "avg_savings_pct": stats.avg_savings_pct,
            "top_commands": stats.top_commands,
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}
// END_public_api
