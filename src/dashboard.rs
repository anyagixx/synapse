// MODULE_CONTRACT
// MODULE_ID: M-DASHBOARD
// PURPOSE: Axum web dashboard — serves project health plus GRACE belief, mental-test, traceability, and cascade views/APIs
// SCOPE: HTTP server with health/status/graph/tokens APIs, GRACE state pages, traceability queries, cascade previews, and cascade history
// DEPENDS: M-GRACE-STATUS, M-GRACE-BELIEF-STATE, M-GRACE-MENTAL-TEST, M-GRACE-TRACEABILITY, M-GRACE-CASCADE, M-GRACE-CASCADE-CHANGE, M-GRAPHRAG, M-TRACKING, M-CONFIG
// LINKS:
//   -> V-M-DASHBOARD (verified_by) - dashboard route and JSON payload tests

// START_MODULE_MAP
// start_dashboard — Start Axum HTTP server with dashboard API routes
// belief_states_payload — Build JSON for belief-state coverage and detail data
// mental_tests_payload — Build JSON for MentalTest status and definitions
// traceability_payload — Build JSON for full or artifact-scoped traceability chains
// cascade_impact_payload — Build JSON for cascade impact preview
// cascade_history_payload — Build JSON for cascade changelog history
// render_*_page — Render local HTML pages backed by the JSON payload helpers
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.0.0 — Added GRACE belief-state, MentalTest, traceability, and cascade dashboard routes]
// END_CHANGE_SUMMARY

use crate::grace::belief_state::collect_project_belief_states;
use crate::grace::cascade::cascade_impact;
use crate::grace::cascade_change::list_changelog_entries;
use crate::grace::mental_test::scan_project_mental_tests;
use crate::grace::status::StatusCollector;
use crate::grace::traceability::{scan_project_traceability, TraceabilityChain};
use axum::{
    extract::{Path as AxumPath, Query},
    response::Html,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

// START_public_api

// START_CONTRACT_start_dashboard
// PURPOSE: Start the Axum web dashboard HTTP server
// INPUTS: { bind: &str — address:port to bind }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: starts HTTP server, blocks until shutdown
// START_start_dashboard
pub async fn start_dashboard(bind: &str) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index_html))
        .route("/health", get(health))
        .route("/belief-states", get(belief_states_html))
        .route("/belief-state/{module_id}", get(belief_state_detail_html))
        .route("/mental-tests", get(mental_tests_html))
        .route("/traceability/{artifact_id}", get(traceability_html))
        .route("/cascade/preview", get(cascade_preview_html))
        .route("/cascade/history", get(cascade_history_html))
        .route("/api/status", get(api_status))
        .route("/api/graph", get(api_graph))
        .route("/api/tokens", get(api_tokens))
        .route("/api/belief-states", get(api_belief_states))
        .route(
            "/api/belief-state/{module_id}",
            get(api_belief_state_detail),
        )
        .route("/api/mental-tests", get(api_mental_tests))
        .route("/api/traceability", get(api_traceability))
        .route("/api/cascade-impact", get(api_cascade_impact))
        .route("/api/cascade-history", get(api_cascade_history));

    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(
        "Synapse dashboard: http://{} (HTML: http://{}/)",
        bind,
        bind
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
// END_start_dashboard

// END_public_api

#[derive(Debug, Clone, Deserialize, Default)]
struct TraceabilityQuery {
    artifact: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct CascadeImpactQuery {
    artifact: Option<String>,
    change: Option<String>,
}

// START_CONTRACT_shutdown_signal
// PURPOSE: Wait for Ctrl+C and log dashboard shutdown
// OUTPUTS: { () }
// START_shutdown_signal
async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.ok();
    tracing::info!("Shutting down dashboard...");
}
// END_shutdown_signal

// START_CONTRACT_index_html
// PURPOSE: Render the dashboard landing page with health links and next-action summary
// OUTPUTS: { Html<String> }
// START_index_html
async fn index_html() -> Html<String> {
    let root = current_root();
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
        .map(|report| {
            format!(
                "<section class=\"card\"><h2>Token Economy</h2><pre>tracked: {}\nsaved: {}\navg savings: {:.1}%</pre></section>",
                report.token_economy.total_commands,
                report.token_economy.total_saved,
                report.token_economy.avg_savings_pct
            )
        })
        .unwrap_or_else(|| {
            "<section class=\"card\"><h2>Token Economy</h2><pre>unavailable</pre></section>"
                .into()
        });
    let next_actions = status
        .as_ref()
        .map(|report| {
            let items = report
                .next_actions
                .iter()
                .take(5)
                .map(|action| format!("<li>{}</li>", html_escape(action)))
                .collect::<Vec<_>>()
                .join("");
            format!("<section class=\"card\"><h2>Next Actions</h2><ul>{items}</ul></section>")
        })
        .unwrap_or_else(|| {
            "<section class=\"card\"><h2>Next Actions</h2><pre>unavailable</pre></section>".into()
        });
    Html(page_shell(
        "Synapse Dashboard",
        &format!(
            "<p>Version <strong>{}</strong>. Active phase: <strong>{}</strong></p>{}<div class=\"grid\">{}{}</div>",
            html_escape(crate::VERSION),
            html_escape(&active_phase),
            primary_nav(),
            token_block,
            next_actions
        ),
    ))
}
// END_index_html

// START_CONTRACT_health
// PURPOSE: Return dashboard process health metadata
// OUTPUTS: { Json<Value> }
// START_health
async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": crate::VERSION,
        "name": crate::NAME
    }))
}
// END_health

// START_CONTRACT_belief_states_html
// PURPOSE: Render the belief-state coverage page
// OUTPUTS: { Html<String> }
// START_belief_states_html
async fn belief_states_html() -> Html<String> {
    Html(render_belief_states_page(&current_root()))
}
// END_belief_states_html

// START_CONTRACT_belief_state_detail_html
// PURPOSE: Render one module's belief-state detail page
// INPUTS: { module_id: String — module id path parameter }
// OUTPUTS: { Html<String> }
// START_belief_state_detail_html
async fn belief_state_detail_html(AxumPath(module_id): AxumPath<String>) -> Html<String> {
    Html(render_belief_state_detail_page(&current_root(), &module_id))
}
// END_belief_state_detail_html

// START_CONTRACT_mental_tests_html
// PURPOSE: Render the MentalTest dashboard page
// OUTPUTS: { Html<String> }
// START_mental_tests_html
async fn mental_tests_html() -> Html<String> {
    Html(render_mental_tests_page(&current_root()))
}
// END_mental_tests_html

// START_CONTRACT_traceability_html
// PURPOSE: Render artifact-scoped traceability chains
// INPUTS: { artifact_id: String — artifact id path parameter }
// OUTPUTS: { Html<String> }
// START_traceability_html
async fn traceability_html(AxumPath(artifact_id): AxumPath<String>) -> Html<String> {
    Html(render_traceability_page(&current_root(), &artifact_id))
}
// END_traceability_html

// START_CONTRACT_cascade_preview_html
// PURPOSE: Render cascade preview form and optional impact result
// INPUTS: { query: CascadeImpactQuery — artifact and change query parameters }
// OUTPUTS: { Html<String> }
// START_cascade_preview_html
async fn cascade_preview_html(Query(query): Query<CascadeImpactQuery>) -> Html<String> {
    Html(render_cascade_preview_page(&current_root(), &query))
}
// END_cascade_preview_html

// START_CONTRACT_cascade_history_html
// PURPOSE: Render recorded cascade changelog history
// OUTPUTS: { Html<String> }
// START_cascade_history_html
async fn cascade_history_html() -> Html<String> {
    Html(render_cascade_history_page(&current_root()))
}
// END_cascade_history_html

// START_CONTRACT_api_status
// PURPOSE: Return the full project status report as JSON
// OUTPUTS: { Json<Value> }
// START_api_status
async fn api_status() -> Json<Value> {
    match StatusCollector::collect(&current_root()).await {
        Ok(report) => Json(serde_json::to_value(report).unwrap_or_default()),
        Err(error) => Json(error_payload(error)),
    }
}
// END_api_status

// START_CONTRACT_api_graph
// PURPOSE: Return GraphRAG overview and nodes as JSON
// OUTPUTS: { Json<Value> }
// START_api_graph
async fn api_graph() -> Json<Value> {
    let root = current_root();
    let mut graphrag = crate::graphrag::GraphRag::new();
    match graphrag.build(&root) {
        Ok(()) => {
            let overview = graphrag.overview();
            let nodes: Vec<Value> = graphrag
                .graph()
                .map(|graph| graph.search_nodes(""))
                .unwrap_or_default()
                .iter()
                .map(|node| {
                    json!({
                        "id": node.id,
                        "name": node.name,
                        "kind": node.kind,
                        "path": node.path,
                        "language": node.language,
                        "size": node.size_lines
                    })
                })
                .collect();
            Json(json!({ "overview": overview, "nodes": nodes }))
        }
        Err(error) => Json(error_payload(error)),
    }
}
// END_api_graph

// START_CONTRACT_api_tokens
// PURPOSE: Return token-savings tracker statistics as JSON
// OUTPUTS: { Json<Value> }
// START_api_tokens
async fn api_tokens() -> Json<Value> {
    let config = crate::config::Config::load_or_default();
    let tracker = crate::tracking::Tracker::new(&config);
    match tracker.get_stats().await {
        Ok(stats) => Json(json!({
            "total_commands": stats.total_commands,
            "total_input_tokens": stats.total_input_tokens,
            "total_output_tokens": stats.total_output_tokens,
            "total_saved_tokens": stats.total_saved_tokens,
            "avg_savings_pct": stats.avg_savings_pct,
            "top_commands": stats.top_commands,
        })),
        Err(error) => Json(error_payload(error)),
    }
}
// END_api_tokens

// START_CONTRACT_api_belief_states
// PURPOSE: Return belief-state coverage and discovered states as JSON
// OUTPUTS: { Json<Value> }
// START_api_belief_states
async fn api_belief_states() -> Json<Value> {
    Json(belief_states_payload(&current_root()))
}
// END_api_belief_states

// START_CONTRACT_api_belief_state_detail
// PURPOSE: Return one module's belief-state detail as JSON
// INPUTS: { module_id: String — module id path parameter }
// OUTPUTS: { Json<Value> }
// START_api_belief_state_detail
async fn api_belief_state_detail(AxumPath(module_id): AxumPath<String>) -> Json<Value> {
    Json(belief_state_detail_payload(&current_root(), &module_id))
}
// END_api_belief_state_detail

// START_CONTRACT_api_mental_tests
// PURPOSE: Return MentalTest report as JSON
// OUTPUTS: { Json<Value> }
// START_api_mental_tests
async fn api_mental_tests() -> Json<Value> {
    Json(mental_tests_payload(&current_root()))
}
// END_api_mental_tests

// START_CONTRACT_api_traceability
// PURPOSE: Return full or artifact-scoped traceability report as JSON
// INPUTS: { query: TraceabilityQuery — optional artifact query parameter }
// OUTPUTS: { Json<Value> }
// START_api_traceability
async fn api_traceability(Query(query): Query<TraceabilityQuery>) -> Json<Value> {
    Json(traceability_payload(&current_root(), &query))
}
// END_api_traceability

// START_CONTRACT_api_cascade_impact
// PURPOSE: Return cascade impact preview as JSON
// INPUTS: { query: CascadeImpactQuery — artifact and optional change query parameters }
// OUTPUTS: { Json<Value> }
// SIDE_EFFECTS: writes cascade preview cache through M-GRACE-CASCADE
// START_api_cascade_impact
async fn api_cascade_impact(Query(query): Query<CascadeImpactQuery>) -> Json<Value> {
    Json(cascade_impact_payload(&current_root(), &query))
}
// END_api_cascade_impact

// START_CONTRACT_api_cascade_history
// PURPOSE: Return cascade changelog history as JSON
// OUTPUTS: { Json<Value> }
// START_api_cascade_history
async fn api_cascade_history() -> Json<Value> {
    Json(cascade_history_payload(&current_root()))
}
// END_api_cascade_history

// START_CONTRACT_belief_states_payload
// PURPOSE: Build JSON payload for belief-state coverage and discovered state rows
// INPUTS: { root: &Path — project root }
// OUTPUTS: { Value }
// START_belief_states_payload
fn belief_states_payload(root: &Path) -> Value {
    let summary = match crate::grace::belief_state::scan_project_belief_states(root) {
        Ok(report) => report,
        Err(error) => return error_payload(error),
    };
    let states = match collect_project_belief_states(root) {
        Ok(states) => states,
        Err(error) => return error_payload(error),
    };
    json!({ "summary": summary, "states": states })
}
// END_belief_states_payload

// START_CONTRACT_belief_state_detail_payload
// PURPOSE: Build JSON payload for one module's belief-state detail
// INPUTS: { root: &Path }, { module_id: &str }
// OUTPUTS: { Value }
// START_belief_state_detail_payload
fn belief_state_detail_payload(root: &Path, module_id: &str) -> Value {
    let wanted = module_id.trim();
    let summary = match crate::grace::belief_state::scan_project_belief_states(root) {
        Ok(report) => report,
        Err(error) => return error_payload(error),
    };
    let missing = summary
        .missing_modules
        .iter()
        .any(|module| module == wanted);
    let states = match collect_project_belief_states(root) {
        Ok(states) => states
            .into_iter()
            .filter(|state| state.module_id == wanted)
            .collect::<Vec<_>>(),
        Err(error) => return error_payload(error),
    };
    json!({
        "module_id": wanted,
        "found": !states.is_empty(),
        "missing": missing,
        "states": states,
        "summary": summary
    })
}
// END_belief_state_detail_payload

// START_CONTRACT_mental_tests_payload
// PURPOSE: Build JSON payload for MentalTest project status
// INPUTS: { root: &Path — project root }
// OUTPUTS: { Value }
// START_mental_tests_payload
fn mental_tests_payload(root: &Path) -> Value {
    match scan_project_mental_tests(root) {
        Ok(report) => serde_json::to_value(report).unwrap_or_default(),
        Err(error) => error_payload(error),
    }
}
// END_mental_tests_payload

// START_CONTRACT_traceability_payload
// PURPOSE: Build JSON payload for full or artifact-scoped traceability chains and gaps
// INPUTS: { root: &Path }, { query: &TraceabilityQuery }
// OUTPUTS: { Value }
// START_traceability_payload
fn traceability_payload(root: &Path, query: &TraceabilityQuery) -> Value {
    let report = match scan_project_traceability(root) {
        Ok(report) => report,
        Err(error) => return error_payload(error),
    };
    let artifact = query.artifact.as_deref().map(str::trim).unwrap_or_default();
    if artifact.is_empty() {
        return serde_json::to_value(report).unwrap_or_default();
    }

    let chains = report
        .chains
        .iter()
        .filter(|chain| chain_mentions_artifact(chain, artifact))
        .cloned()
        .collect::<Vec<_>>();
    let gaps = report
        .gaps
        .iter()
        .filter(|gap| gap.source_id == artifact || gap.description.contains(artifact))
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "artifact": artifact,
        "chains": chains,
        "gaps": gaps,
        "summary": {
            "requirements_total": report.requirements_total,
            "use_cases_total": report.use_cases_total,
            "traceability_score": report.traceability_score,
            "enforcement_mode": report.enforcement_mode,
            "total_chains": report.chains.len()
        }
    })
}
// END_traceability_payload

// START_CONTRACT_cascade_impact_payload
// PURPOSE: Build JSON payload for cascade impact preview
// INPUTS: { root: &Path }, { query: &CascadeImpactQuery }
// OUTPUTS: { Value }
// SIDE_EFFECTS: writes cascade preview cache through M-GRACE-CASCADE
// START_cascade_impact_payload
fn cascade_impact_payload(root: &Path, query: &CascadeImpactQuery) -> Value {
    let artifact = query.artifact.as_deref().map(str::trim).unwrap_or_default();
    if artifact.is_empty() {
        return json!({ "error": "artifact query parameter is required" });
    }
    let change = query
        .change
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Dashboard preview");
    match cascade_impact(root, artifact, change) {
        Ok(analysis) => serde_json::to_value(analysis).unwrap_or_default(),
        Err(error) => error_payload(error),
    }
}
// END_cascade_impact_payload

// START_CONTRACT_cascade_history_payload
// PURPOSE: Build JSON payload for cascade changelog history
// INPUTS: { root: &Path — project root }
// OUTPUTS: { Value }
// START_cascade_history_payload
fn cascade_history_payload(root: &Path) -> Value {
    match list_changelog_entries(root) {
        Ok(history) => serde_json::to_value(history).unwrap_or_default(),
        Err(error) => error_payload(error),
    }
}
// END_cascade_history_payload

// START_CONTRACT_render_belief_states_page
// PURPOSE: Render belief-state coverage and module state links
// INPUTS: { root: &Path — project root }
// OUTPUTS: { String }
// START_render_belief_states_page
fn render_belief_states_page(root: &Path) -> String {
    let payload = belief_states_payload(root);
    if payload.get("error").is_some() {
        return page_shell("Belief States", &json_pre(&payload));
    }
    let summary = &payload["summary"];
    let states = payload["states"].as_array().cloned().unwrap_or_default();
    let rows = states
        .iter()
        .map(|state| {
            let module_id = state["module_id"].as_str().unwrap_or("unknown");
            let valid = state["valid"].as_bool().unwrap_or(false);
            let path = state["file_path"].as_str().unwrap_or("");
            format!(
                "<tr><td><a href=\"/belief-state/{}\">{}</a></td><td>{}</td><td>{}</td></tr>",
                url_component(module_id),
                html_escape(module_id),
                status_badge(valid),
                html_escape(path)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let missing = summary["missing_modules"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .take(50)
        .map(|module| {
            format!(
                "<li><a href=\"/belief-state/{}\">{}</a></li>",
                url_component(module.as_str().unwrap_or_default()),
                html_escape(module.as_str().unwrap_or_default())
            )
        })
        .collect::<Vec<_>>()
        .join("");
    page_shell(
        "Belief States",
        &format!(
            "{}<section class=\"card\"><h2>Coverage</h2><pre>modules: {}\nstates: {}\nvalid: {}\ninvalid: {}\ncoverage: {:.1}%</pre></section><section class=\"card\"><h2>States</h2><table><thead><tr><th>Module</th><th>Status</th><th>Path</th></tr></thead><tbody>{}</tbody></table></section><section class=\"card\"><h2>Missing</h2><ul>{}</ul></section>",
            primary_nav(),
            summary["total_modules"].as_u64().unwrap_or(0),
            summary["states_found"].as_u64().unwrap_or(0),
            summary["valid_states"].as_u64().unwrap_or(0),
            summary["invalid_states"].as_u64().unwrap_or(0),
            summary["coverage_pct"].as_f64().unwrap_or(0.0),
            if rows.is_empty() {
                "<tr><td colspan=\"3\">No belief states discovered</td></tr>".to_string()
            } else {
                rows
            },
            if missing.is_empty() {
                "<li>None</li>".to_string()
            } else {
                missing
            }
        ),
    )
}
// END_render_belief_states_page

// START_CONTRACT_render_belief_state_detail_page
// PURPOSE: Render detailed belief state information for one module
// INPUTS: { root: &Path }, { module_id: &str }
// OUTPUTS: { String }
// START_render_belief_state_detail_page
fn render_belief_state_detail_page(root: &Path, module_id: &str) -> String {
    let payload = belief_state_detail_payload(root, module_id);
    let body = if payload.get("error").is_some() {
        json_pre(&payload)
    } else {
        let states = payload["states"].as_array().cloned().unwrap_or_default();
        let details = states.iter().map(json_pre).collect::<Vec<_>>().join("");
        format!(
            "{}<section class=\"card\"><h2>{}</h2><pre>found: {}\nmissing: {}</pre>{}</section>",
            primary_nav(),
            html_escape(module_id),
            payload["found"].as_bool().unwrap_or(false),
            payload["missing"].as_bool().unwrap_or(false),
            if details.is_empty() {
                "<pre>No belief state found for this module.</pre>".to_string()
            } else {
                details
            }
        )
    };
    page_shell("Belief State", &body)
}
// END_render_belief_state_detail_page

// START_CONTRACT_render_mental_tests_page
// PURPOSE: Render MentalTest summary and definitions
// INPUTS: { root: &Path — project root }
// OUTPUTS: { String }
// START_render_mental_tests_page
fn render_mental_tests_page(root: &Path) -> String {
    let payload = mental_tests_payload(root);
    if payload.get("error").is_some() {
        return page_shell("Mental Tests", &json_pre(&payload));
    }
    let rows = payload["tests"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|test| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(test["id"].as_str().unwrap_or_default()),
                html_escape(test["target"].as_str().unwrap_or_default()),
                html_escape(test["status"].as_str().unwrap_or_default()),
                html_escape(test["description"].as_str().unwrap_or_default())
            )
        })
        .collect::<Vec<_>>()
        .join("");
    page_shell(
        "Mental Tests",
        &format!(
            "{}<section class=\"card\"><h2>Summary</h2><pre>total: {}\npassed: {}\nfailed: {}\nnot_run: {}\nneeds_clarification: {}</pre></section><section class=\"card\"><h2>Tests</h2><table><thead><tr><th>ID</th><th>Target</th><th>Status</th><th>Description</th></tr></thead><tbody>{}</tbody></table></section>",
            primary_nav(),
            payload["total"].as_u64().unwrap_or(0),
            payload["passed"].as_u64().unwrap_or(0),
            payload["failed"].as_u64().unwrap_or(0),
            payload["not_run"].as_u64().unwrap_or(0),
            payload["needs_clarification"].as_u64().unwrap_or(0),
            if rows.is_empty() {
                "<tr><td colspan=\"4\">No MentalTests found</td></tr>".to_string()
            } else {
                rows
            }
        ),
    )
}
// END_render_mental_tests_page

// START_CONTRACT_render_traceability_page
// PURPOSE: Render artifact-scoped traceability data as readable JSON
// INPUTS: { root: &Path }, { artifact_id: &str }
// OUTPUTS: { String }
// START_render_traceability_page
fn render_traceability_page(root: &Path, artifact_id: &str) -> String {
    let query = TraceabilityQuery {
        artifact: Some(artifact_id.to_string()),
    };
    let payload = traceability_payload(root, &query);
    page_shell(
        "Traceability",
        &format!(
            "{}<section class=\"card\"><h2>{}</h2>{}</section>",
            primary_nav(),
            html_escape(artifact_id),
            json_pre(&payload)
        ),
    )
}
// END_render_traceability_page

// START_CONTRACT_render_cascade_preview_page
// PURPOSE: Render cascade preview form and optional impact details
// INPUTS: { root: &Path }, { query: &CascadeImpactQuery }
// OUTPUTS: { String }
// SIDE_EFFECTS: writes cascade preview cache when artifact is supplied
// START_render_cascade_preview_page
fn render_cascade_preview_page(root: &Path, query: &CascadeImpactQuery) -> String {
    let artifact = query.artifact.as_deref().unwrap_or_default();
    let change = query.change.as_deref().unwrap_or_default();
    let result = if artifact.trim().is_empty() {
        "<pre>Provide an artifact id to preview downstream impact.</pre>".to_string()
    } else {
        json_pre(&cascade_impact_payload(root, query))
    };
    page_shell(
        "Cascade Preview",
        &format!(
            "{}<section class=\"card\"><h2>Preview</h2><form method=\"get\" action=\"/cascade/preview\"><label>Artifact <input name=\"artifact\" value=\"{}\"></label><label>Change <input name=\"change\" value=\"{}\"></label><button type=\"submit\">Preview</button></form>{}</section>",
            primary_nav(),
            html_escape(artifact),
            html_escape(change),
            result
        ),
    )
}
// END_render_cascade_preview_page

// START_CONTRACT_render_cascade_history_page
// PURPOSE: Render cascade changelog history
// INPUTS: { root: &Path — project root }
// OUTPUTS: { String }
// START_render_cascade_history_page
fn render_cascade_history_page(root: &Path) -> String {
    let payload = cascade_history_payload(root);
    page_shell(
        "Cascade History",
        &format!(
            "{}<section class=\"card\"><h2>Changelogs</h2>{}</section>",
            primary_nav(),
            json_pre(&payload)
        ),
    )
}
// END_render_cascade_history_page

// START_CONTRACT_chain_mentions_artifact
// PURPOSE: Return true when a traceability chain references an artifact in either direction
// INPUTS: { chain: &TraceabilityChain }, { artifact: &str }
// OUTPUTS: { bool }
// START_chain_mentions_artifact
fn chain_mentions_artifact(chain: &TraceabilityChain, artifact: &str) -> bool {
    chain.artifact_id == artifact
        || chain
            .traces_to
            .iter()
            .any(|link| link.target_id == artifact)
        || chain
            .traced_by
            .iter()
            .any(|link| link.target_id == artifact)
}
// END_chain_mentions_artifact

// START_CONTRACT_current_root
// PURPOSE: Return current project root for dashboard handlers
// OUTPUTS: { PathBuf }
// START_current_root
fn current_root() -> PathBuf {
    std::env::current_dir().unwrap_or_default()
}
// END_current_root

// START_CONTRACT_error_payload
// PURPOSE: Convert an error into a stable dashboard JSON envelope
// INPUTS: { error: impl ToString }
// OUTPUTS: { Value }
// START_error_payload
fn error_payload(error: impl ToString) -> Value {
    json!({ "error": error.to_string() })
}
// END_error_payload

// START_CONTRACT_page_shell
// PURPOSE: Wrap dashboard page body in shared HTML and styles
// INPUTS: { title: &str }, { body: &str }
// OUTPUTS: { String }
// START_page_shell
fn page_shell(title: &str, body: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><title>{}</title><meta charset="utf-8"><style>body{{font-family:system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;max-width:1120px;margin:2em auto;padding:1em;background:#111;color:#eee}}a{{color:#8cb9ff}}h1{{color:#5c9cf5}}h2{{margin-top:0}}.grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:1rem}}.card{{background:#1a1a1a;border:1px solid #2d2d2d;border-radius:8px;padding:1em;margin:1em 0}}pre{{background:#0a0a0a;padding:1em;border-radius:4px;overflow-x:auto;white-space:pre-wrap}}table{{width:100%;border-collapse:collapse}}th,td{{border-bottom:1px solid #333;padding:.55em;text-align:left;vertical-align:top}}nav{{display:flex;flex-wrap:wrap;gap:.75rem;margin:1rem 0 1.25rem}}.pass{{color:#4caf50}}.fail{{color:#f44336}}.badge{{display:inline-block;border-radius:999px;padding:.15em .55em;background:#2b2b2b}}label{{display:block;margin:.7em 0}}input{{background:#0a0a0a;color:#eee;border:1px solid #444;border-radius:4px;padding:.45em;width:min(100%,38rem)}}button{{background:#2f6fed;color:white;border:0;border-radius:4px;padding:.55em .8em;cursor:pointer}}</style></head><body><h1>{}</h1>{}</body></html>"#,
        html_escape(title),
        html_escape(title),
        body
    )
}
// END_page_shell

// START_CONTRACT_primary_nav
// PURPOSE: Render stable dashboard navigation links
// OUTPUTS: { String }
// START_primary_nav
fn primary_nav() -> String {
    let links = [
        ("/api/status", "Status API"),
        ("/api/graph", "Graph API"),
        ("/api/tokens", "Tokens API"),
        ("/belief-states", "Belief States"),
        ("/mental-tests", "Mental Tests"),
        ("/traceability/REQ-001", "Traceability"),
        ("/cascade/preview", "Cascade Preview"),
        ("/cascade/history", "Cascade History"),
    ]
    .iter()
    .map(|(href, label)| format!("<a href=\"{}\">{}</a>", href, html_escape(label)))
    .collect::<Vec<_>>()
    .join("");
    format!("<nav>{links}</nav>")
}
// END_primary_nav

// START_CONTRACT_json_pre
// PURPOSE: Render JSON as escaped pretty-print HTML
// INPUTS: { value: &Value }
// OUTPUTS: { String }
// START_json_pre
fn json_pre(value: &Value) -> String {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into());
    format!("<pre>{}</pre>", html_escape(&text))
}
// END_json_pre

// START_CONTRACT_status_badge
// PURPOSE: Render boolean status as pass/fail HTML badge
// INPUTS: { passed: bool }
// OUTPUTS: { String }
// START_status_badge
fn status_badge(passed: bool) -> String {
    if passed {
        "<span class=\"badge pass\">valid</span>".into()
    } else {
        "<span class=\"badge fail\">invalid</span>".into()
    }
}
// END_status_badge

// START_CONTRACT_html_escape
// PURPOSE: Escape text for safe dashboard HTML rendering
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_html_escape
fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
// END_html_escape

// START_CONTRACT_url_component
// PURPOSE: Encode a simple artifact id for path links
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_url_component
fn url_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':') {
                ch.to_string()
            } else {
                format!("%{:02X}", ch as u32)
            }
        })
        .collect()
}
// END_url_component

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grace::belief_state::{
        BeliefState, BeliefStrategy, BeliefUnderstanding, DataFlowBelief,
    };
    use crate::grace::cascade_change::{new_change_log, write_changelog, CascadeChange};

    // START_CONTRACT_test_belief_states_payload_returns_state_rows
    // PURPOSE: Verify belief-state API payload includes persisted state details
    // OUTPUTS: { () }
    // START_test_belief_states_payload_returns_state_rows
    #[test]
    fn test_belief_states_payload_returns_state_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_dashboard_fixture(dir.path());

        let payload = belief_states_payload(dir.path());
        assert_eq!(payload["states"][0]["module_id"], "M-DASHBOARD");
        assert!(render_belief_states_page(dir.path()).contains("/belief-state/M-DASHBOARD"));
    }
    // END_test_belief_states_payload_returns_state_rows

    // START_CONTRACT_test_belief_state_detail_payload_reports_missing_module
    // PURPOSE: Verify module detail payload distinguishes found and missing belief states
    // OUTPUTS: { () }
    // START_test_belief_state_detail_payload_reports_missing_module
    #[test]
    fn test_belief_state_detail_payload_reports_missing_module() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_dashboard_fixture(dir.path());

        let payload = belief_state_detail_payload(dir.path(), "M-UNKNOWN");
        assert_eq!(payload["found"], false);
        assert!(render_belief_state_detail_page(dir.path(), "M-DASHBOARD").contains("found: true"));
    }
    // END_test_belief_state_detail_payload_reports_missing_module

    // START_CONTRACT_test_mental_tests_payload_returns_summary
    // PURPOSE: Verify MentalTest dashboard payload exposes parsed test counts
    // OUTPUTS: { () }
    // START_test_mental_tests_payload_returns_summary
    #[test]
    fn test_mental_tests_payload_returns_summary() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_dashboard_fixture(dir.path());

        let payload = mental_tests_payload(dir.path());
        assert_eq!(payload["total"], 1);
        assert_eq!(payload["passed"], 1);
        assert!(render_mental_tests_page(dir.path()).contains("MT-DASHBOARD"));
    }
    // END_test_mental_tests_payload_returns_summary

    // START_CONTRACT_test_traceability_payload_filters_artifact
    // PURPOSE: Verify traceability API can scope chains to one artifact
    // OUTPUTS: { () }
    // START_test_traceability_payload_filters_artifact
    #[test]
    fn test_traceability_payload_filters_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_dashboard_fixture(dir.path());

        let payload = traceability_payload(
            dir.path(),
            &TraceabilityQuery {
                artifact: Some("REQ-001".into()),
            },
        );
        assert_eq!(payload["artifact"], "REQ-001");
        assert!(payload["chains"]
            .as_array()
            .is_some_and(|chains| !chains.is_empty()));
    }
    // END_test_traceability_payload_filters_artifact

    // START_CONTRACT_test_cascade_payloads_return_preview_and_history
    // PURPOSE: Verify cascade dashboard APIs expose impact preview and changelog history
    // OUTPUTS: { () }
    // START_test_cascade_payloads_return_preview_and_history
    #[test]
    fn test_cascade_payloads_return_preview_and_history() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_dashboard_fixture(dir.path());

        let impact = cascade_impact_payload(
            dir.path(),
            &CascadeImpactQuery {
                artifact: Some("REQ-001".into()),
                change: Some("Dashboard smoke".into()),
            },
        );
        assert!(impact["cascade_id"]
            .as_str()
            .is_some_and(|value| value.starts_with("CSC-")));
        assert!(impact["preview"]
            .as_str()
            .unwrap_or_default()
            .contains("CASCADE PREVIEW"));

        let history = cascade_history_payload(dir.path());
        assert_eq!(history["changelog_count"], 1);
        assert!(render_cascade_history_page(dir.path()).contains("CSC-DASHBOARD"));
    }
    // END_test_cascade_payloads_return_preview_and_history

    // START_CONTRACT_write_dashboard_fixture
    // PURPOSE: Create minimal docs artifacts for dashboard payload tests
    // INPUTS: { root: &Path }
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temporary docs artifacts
    // START_write_dashboard_fixture
    fn write_dashboard_fixture(root: &Path) {
        std::fs::create_dir_all(root.join("docs/belief-states")).expect("belief states dir");
        std::fs::create_dir_all(root.join("docs/cascade/changelogs")).expect("cascade dir");
        std::fs::write(root.join("docs/requirements.xml"), requirements_fixture())
            .expect("requirements");
        std::fs::write(
            root.join("docs/development-plan.xml"),
            development_plan_fixture(),
        )
        .expect("development plan");
        let state = BeliefState {
            module_id: "M-DASHBOARD".into(),
            version: "1.0".into(),
            understanding: BeliefUnderstanding {
                responsibility: "Render GRACE dashboard state".into(),
                key_data_flows: vec![
                    DataFlowBelief {
                        direction: "INPUT".into(),
                        description: "GRACE reports and docs artifacts".into(),
                    },
                    DataFlowBelief {
                        direction: "PROCESSING".into(),
                        description: "Aggregate payloads and render pages".into(),
                    },
                    DataFlowBelief {
                        direction: "OUTPUT".into(),
                        description: "HTML and JSON dashboard responses".into(),
                    },
                ],
                critical_invariants: vec!["Payloads remain valid JSON".into()],
            },
            implementation_strategy: BeliefStrategy {
                approach: "Use existing GRACE scanners as data sources".into(),
                complexity_areas: vec!["route coverage".into()],
                patterns_applied: vec!["source-of-truth reuse".into()],
            },
            verification_intent: vec!["Dashboard payload helpers return valid JSON".into()],
            risks_acknowledged: vec!["Cascade preview writes cache artifacts".into()],
            file_path: None,
            valid: true,
            errors: Vec::new(),
        };
        std::fs::write(
            root.join("docs/belief-states/M-DASHBOARD.xml"),
            state.to_xml(),
        )
        .expect("belief state");
        let changelog = new_change_log(
            "CSC-DASHBOARD",
            "REQ-001",
            "Dashboard fixture",
            "test",
            vec![CascadeChange {
                artifact: "M-DASHBOARD".into(),
                change_type: "contract".into(),
                action: "proposed".into(),
                before: "old".into(),
                after: "new".into(),
                verification_status: "pending".into(),
            }],
        );
        write_changelog(root, &changelog).expect("changelog");
    }
    // END_write_dashboard_fixture

    // START_CONTRACT_requirements_fixture
    // PURPOSE: Return minimal requirements XML for traceability and cascade tests
    // OUTPUTS: { &'static str }
    // START_requirements_fixture
    fn requirements_fixture() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<RequirementsAnalysis>
  <Goals><Goal id="G-001">Show GRACE state</Goal></Goals>
  <Entities><Entity name="Dashboard" /></Entities>
  <UseCases><UseCase id="UC-001"><Title>Inspect GRACE state</Title></UseCase></UseCases>
  <Requirements><Requirement id="REQ-001">Dashboard exposes GRACE state</Requirement></Requirements>
</RequirementsAnalysis>
"#
    }
    // END_requirements_fixture

    // START_CONTRACT_development_plan_fixture
    // PURPOSE: Return minimal DevelopmentPlan XML with one passing MentalTest
    // OUTPUTS: { &'static str }
    // START_development_plan_fixture
    fn development_plan_fixture() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<DevelopmentPlan project="dashboard" version="1.0">
  <ArchitectureGraph>
    <Module id="M-DASHBOARD" critical="true"><Link ref="REQ-001" type="implements" /></Module>
  </ArchitectureGraph>
  <MentalTests>
    <MentalTest id="MT-DASHBOARD" target="M-DASHBOARD::render" status="pass">
      <Description>Dashboard state rendering</Description>
      <Scenario>GIVEN docs artifacts WHEN dashboard payloads are built THEN valid state is returned</Scenario>
      <Steps>
        <Step n="1" action="render" expected="ok" actual="PASS ok">Render payload.</Step>
      </Steps>
      <EdgeCases>
        <Case id="EC-001" description="No changelogs" expectation="history returns empty list" />
      </EdgeCases>
      <Result>pass</Result>
      <Rationale>The dashboard delegates data extraction to tested GRACE modules.</Rationale>
    </MentalTest>
  </MentalTests>
</DevelopmentPlan>
"#
    }
    // END_development_plan_fixture
}
