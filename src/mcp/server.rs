// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER
// PURPOSE: MCP JSON-RPC server facade — serves Synapse tools over clean stdio with guarded runtime initialization
// SCOPE: McpServer, SynapseHandler, runtime Config retention, config-bounded pipelined stdio loop, best-effort MCP metrics recording, JSON-RPC request/notification routing including analyze_logs, extract_belief_state, generate_requirements, generate_technology, generate_development_plan, mental_test_run, traceability_report, cascade_impact, cascade_execute, run_test_guide, submit_test_report, suggest_contract, and config-aware LSP tools, guarded index preload and indexed GraphRAG cache state
// DEPENDS: M-CONFIG, M-GRAPHRAG, M-INDEXER, M-MCP-PIPELINE, M-MCP-SERVER-CASCADE-TOOLS, M-MCP-SERVER-CODE-TOOLS, M-MCP-SERVER-GRACE-TOOLS, M-MCP-SERVER-RESPONSE, M-MCP-SERVER-TOOLS, M-TRACKING, M-TRACKING-MCP-METRICS, M-UTILS
// LINKS: N/A

// START_MODULE_MAP
// McpServer — MCP stdio server entry point
// discover_project_count — Counts probable child projects for multi-root mode
// SynapseHandler — MCP message router, runtime config, and initialization state
// GraphCacheKey — Root/index signature used to invalidate cached GraphRAG state
// preload_index_storage — Loads index storage without panicking on poisoned locks
// classify_mcp_response — Converts JSON-RPC tool responses into tracking status metadata
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.19.0 - Simplified MCP elapsed millis conversion for release clippy gate]
// END_CHANGE_SUMMARY

use super::{
    pipeline::{self, McpPipelineConfig, PipelineHandler},
    server_cascade_tools, server_code_tools, server_contract_tools, server_grace_tools,
    server_response, server_tools,
};
use crate::config::Config;
use crate::graphrag::GraphRag;
use crate::indexer::storage::Storage;
use crate::indexer::Indexer;
use crate::skills::{SkillEngine, SkillRequest};
use crate::tracking::{mcp_metrics::McpCallStatus, Tracker};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
use std::time::{Instant, UNIX_EPOCH};

// START_public_api

// START_McpServer
pub struct McpServer;
// END_McpServer

impl McpServer {
    // START_CONTRACT_McpServer::new
    // PURPOSE: Create a new McpServer
    // OUTPUTS: { Self }
    // START_mcp_server_new
    pub fn new(_config: Config) -> Self {
        Self
    }
    // END_mcp_server_new

    // START_CONTRACT_McpServer::start_stdio
    // PURPOSE: Start MCP server on stdio — reads JSON-RPC messages from stdin, writes responses to stdout
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: blocks on stdio I/O
    // START_mcp_server_start_stdio
    pub async fn start_stdio(self) -> anyhow::Result<()> {
        tracing::info!("MCP server running on stdio");
        let root = std::env::current_dir()
            .map_err(|e| anyhow::anyhow!("MCP server cannot resolve current directory: {}", e))?;

        // Auto-discover: if current dir is not a project root but parent has .opencode/
        // or multiple subdirs with source code, use multi-root mode.
        let multi_root = if !root.join(".opencode").exists()
            && !root.join("opencode.jsonc").exists()
            && !root.join("opencode.json").exists()
        {
            let parent_has_projects = discover_project_count(&root);
            parent_has_projects > 1
        } else {
            false
        };
        if multi_root {
            tracing::info!("Multi-root mode: serving {} projects", "multiple");
        }

        let config = Config::load_or_default();
        let pipeline_config = McpPipelineConfig {
            response_queue_capacity: config.observability.pipeline_response_queue_capacity(),
            max_concurrent_requests: config.observability.pipeline_max_concurrent_requests(),
            max_message_bytes: config.observability.pipeline_max_line_bytes(),
        };
        let handler = Arc::new(SynapseHandler::new());
        let pipeline_handler: PipelineHandler = Arc::new(move |line| {
            let handler = handler.clone();
            Box::pin(async move {
                tracing::debug!("MCP << {}", crate::utils::truncate_chars(&line, 200));
                let response = handler.handle_message(&line).await;
                if let Some(response) = &response {
                    if let Ok(msg) = serde_json::to_string(response) {
                        tracing::debug!("MCP >> {}", crate::utils::truncate_chars(&msg, 200));
                    }
                }
                response
            })
        });
        pipeline::run_stdio_pipeline(
            tokio::io::stdin(),
            tokio::io::stdout(),
            pipeline_handler,
            pipeline_config,
        )
        .await
    }
    // END_mcp_server_start_stdio
}

// START_CONTRACT_discover_project_count
// PURPOSE: Count probable child projects for MCP multi-root mode without hiding directory scan errors
// INPUTS: { root: &Path }
// OUTPUTS: { usize }
// START_discover_project_count
fn discover_project_count(root: &Path) -> usize {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!(
                "[McpServer][start_stdio][DISCOVER] cannot scan {}: {}",
                root.display(),
                e
            );
            return 0;
        }
    };

    entries
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry),
            Err(e) => {
                tracing::debug!(
                    "[McpServer][start_stdio][DISCOVER] skipping unreadable entry: {}",
                    e
                );
                None
            }
        })
        .filter(|entry| {
            entry.path().join(".opencode").exists()
                || entry.path().join("src").exists()
                || entry.path().join("Cargo.toml").exists()
                || entry.path().join("package.json").exists()
        })
        .count()
}
// END_discover_project_count

// START_SynapseHandler
pub struct SynapseHandler {
    config: Config,
    indexer: Indexer,
    graphrag: RwLock<Option<GraphRag>>,
    graph_cache_key: RwLock<Option<GraphCacheKey>>,
    skill_engine: SkillEngine,
    tracker: Tracker,
    initialized: AtomicBool,
}
// END_SynapseHandler

// START_GraphCacheKey
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GraphCacheKey {
    root: PathBuf,
    index_modified_nanos: Option<u128>,
    index_len: Option<u64>,
}
// END_GraphCacheKey

impl Default for SynapseHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SynapseHandler {
    // START_CONTRACT_SynapseHandler::new
    // PURPOSE: Create a new SynapseHandler with pre-loaded index storage and lazy graph state
    // OUTPUTS: { Self }
    // START_sh_new
    pub fn new() -> Self {
        let config = Config::load_or_default();
        let indexer = Indexer::new(&config);
        let skill_engine = SkillEngine::new(&config);
        let tracker = Tracker::new(&config);
        // Try to find index from current directory.
        if let Ok(cwd) = std::env::current_dir() {
            if let Err(e) = preload_index_storage(&indexer, &cwd) {
                tracing::warn!("[SynapseHandler][new][INDEX] {}", e);
            }
        }
        Self {
            config,
            indexer,
            graphrag: RwLock::new(None),
            graph_cache_key: RwLock::new(None),
            skill_engine,
            tracker,
            initialized: AtomicBool::new(false),
        }
    }
    // END_sh_new

    // START_CONTRACT_SynapseHandler::handle_message
    // PURPOSE: Route one JSON-RPC message to MCP initialization, tool listing, or tool execution
    // INPUTS: { line: &str — JSON-RPC request line }
    // OUTPUTS: { Option<serde_json::Value> — JSON-RPC response object for requests, None for notifications }
    // START_sh_handle_message
    pub async fn handle_message(&self, line: &str) -> Option<serde_json::Value> {
        let msg: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                return Some(server_response::error(
                    None,
                    -32700,
                    format!("Parse error: {}", e),
                ))
            }
        };

        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let is_notification = id.is_none();

        let response = match method {
            "initialize" => {
                self.initialized.store(true, Ordering::SeqCst);
                server_response::result(
                    id,
                    serde_json::json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "syn",
                            "version": crate::VERSION
                        }
                    }),
                )
            }
            "tools/list" => {
                if !self.initialized.load(Ordering::SeqCst) {
                    return Some(server_response::error(id, -32000, "Not initialized"));
                }
                server_response::result(
                    id,
                    serde_json::json!({
                        "tools": server_tools::tool_definitions()
                    }),
                )
            }
            "tools/call" => {
                if !self.initialized.load(Ordering::SeqCst) {
                    return Some(server_response::error(id, -32000, "Not initialized"));
                }
                let params = &msg["params"];
                let name = params["name"].as_str().unwrap_or("");
                let args = &params["arguments"];
                let started = Instant::now();
                let active_id = match self.tracker.start_mcp_request(name).await {
                    Ok(active_id) => active_id,
                    Err(error) => {
                        tracing::warn!(
                            "[SynapseHandler][handle_message][MCP_METRICS_START] {}",
                            error
                        );
                        None
                    }
                };

                let response = match name {
                    "semantic_search" => {
                        server_code_tools::handle_search(&self.indexer, id, args).await
                    }
                    "view_signatures" => {
                        server_code_tools::handle_view_signatures(&self.indexer, id, args).await
                    }
                    "graphrag_query" => server_code_tools::handle_graphrag(
                        &self.graphrag,
                        &self.graph_cache_key,
                        id,
                        args,
                    ),
                    "verify_project" => server_grace_tools::handle_verify(id, args).await,
                    "review_code" => server_grace_tools::handle_review(id, args).await,
                    "project_status" => server_grace_tools::handle_status(id, args).await,
                    "analyze_logs" => server_grace_tools::handle_analyze_logs(id, args).await,
                    "extract_belief_state" => {
                        server_grace_tools::handle_extract_belief_state(id, args).await
                    }
                    "generate_requirements" => {
                        server_grace_tools::handle_generate_requirements(id, args).await
                    }
                    "generate_technology" => {
                        server_grace_tools::handle_generate_technology(id, args).await
                    }
                    "generate_development_plan" => {
                        server_grace_tools::handle_generate_development_plan(id, args).await
                    }
                    "mental_test_run" => server_grace_tools::handle_mental_test_run(id, args).await,
                    "traceability_report" => {
                        server_grace_tools::handle_traceability_report(id, args).await
                    }
                    "cascade_impact" => server_cascade_tools::handle_cascade_impact(id, args).await,
                    "cascade_execute" => {
                        server_cascade_tools::handle_cascade_execute(id, args).await
                    }
                    "run_test_guide" => server_grace_tools::handle_run_test_guide(id, args).await,
                    "submit_test_report" => {
                        server_grace_tools::handle_submit_test_report(id, args).await
                    }
                    "self_heal" => server_grace_tools::handle_self_heal(id, args).await,
                    "token_savings" => server_grace_tools::handle_gain(id, args).await,
                    "compress_text" => server_grace_tools::handle_compress(id, args).await,
                    "refresh_project" => server_grace_tools::handle_refresh(id, args).await,
                    "diagnose_failure" => {
                        server_contract_tools::handle_diagnose_failure(id, args).await
                    }
                    "repair_contract" => {
                        server_contract_tools::handle_repair_contract(id, args).await
                    }
                    "suggest_contract" => {
                        server_contract_tools::handle_suggest_contract(id, args).await
                    }
                    "lsp_hover" => {
                        server_code_tools::handle_lsp_hover(&self.config, id, args).await
                    }
                    "lsp_references" => {
                        server_code_tools::handle_lsp_references(&self.config, id, args).await
                    }
                    name if name.starts_with("grace_") => {
                        server_grace_tools::handle_grace_skill(
                            &self.skill_engine,
                            SkillRequest {
                                name: name.to_string(),
                                arguments: args.clone(),
                            },
                            id,
                        )
                        .await
                    }
                    _ => server_response::error(id, -32601, format!("Unknown tool: {}", name)),
                };
                let (status, error_message) = classify_mcp_response(&response);
                let duration_ms = elapsed_millis_u64(started);
                if let Err(error) = self
                    .tracker
                    .finish_mcp_request(active_id, name, duration_ms, status, &error_message)
                    .await
                {
                    tracing::warn!(
                        "[SynapseHandler][handle_message][MCP_METRICS_FINISH] {}",
                        error
                    );
                }
                response
            }
            "notifications/initialized" => return None,
            _ => {
                if !self.initialized.load(Ordering::SeqCst) && method != "initialize" {
                    return Some(server_response::error(id, -32000, "Not initialized"));
                }
                server_response::error(id, -32601, format!("Method not found: {}", method))
            }
        };

        if is_notification {
            None
        } else {
            Some(response)
        }
    }
    // END_sh_handle_message

    // START_CONTRACT_SynapseHandler::invalidate_graph_cache
    // PURPOSE: Clear cached GraphRAG state so the next graph query rebuilds it
    // SIDE_EFFECTS: mutates graph cache locks when available
    // START_sh_invalidate_graph_cache
    pub fn invalidate_graph_cache(&self) {
        if let Ok(mut graph) = self.graphrag.write() {
            *graph = None;
        }
        if let Ok(mut key) = self.graph_cache_key.write() {
            *key = None;
        }
    }
    // END_sh_invalidate_graph_cache
}

impl GraphCacheKey {
    // START_CONTRACT_GraphCacheKey::for_root
    // PURPOSE: Build a conservative cache key from project root plus persisted index metadata
    // INPUTS: { root: &Path }
    // OUTPUTS: { GraphCacheKey }
    // START_graph_cache_key_for_root
    pub(crate) fn for_root(root: &Path) -> Self {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let index_path = Storage::db_path_for_root(&root);
        let metadata = std::fs::metadata(index_path).ok();
        let index_modified_nanos = metadata
            .as_ref()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos());
        let index_len = metadata.as_ref().map(|metadata| metadata.len());
        Self {
            root,
            index_modified_nanos,
            index_len,
        }
    }
    // END_graph_cache_key_for_root
}

// START_CONTRACT_preload_index_storage
// PURPOSE: Load index storage for MCP tools without panicking when the storage lock is poisoned
// INPUTS: { indexer: &Indexer }, { root: &Path }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: initializes indexer.storage when empty
// START_preload_index_storage
fn preload_index_storage(indexer: &Indexer, root: &Path) -> anyhow::Result<()> {
    let mut guard = indexer
        .storage
        .write()
        .map_err(|_| anyhow::anyhow!("indexer storage lock poisoned"))?;
    if guard.is_none() {
        *guard = Some(crate::indexer::storage::Storage::new(root));
    }
    Ok(())
}
// END_preload_index_storage

// START_CONTRACT_classify_mcp_response
// PURPOSE: Convert one JSON-RPC response into MCP metrics status and error text
// INPUTS: { response: &serde_json::Value }
// OUTPUTS: { (McpCallStatus, String) }
// LINKS:
//   -> NFR-003 (traces_to) - MCP metrics record per-tool errors without affecting responses
// START_classify_mcp_response
fn classify_mcp_response(response: &serde_json::Value) -> (McpCallStatus, String) {
    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        (McpCallStatus::Error, message)
    } else {
        (McpCallStatus::Ok, String::new())
    }
}
// END_classify_mcp_response

// START_CONTRACT_elapsed_millis_u64
// PURPOSE: Convert elapsed wall time into a saturating u64 millisecond value
// INPUTS: { started: Instant }
// OUTPUTS: { u64 }
// LINKS:
//   -> NFR-003 (traces_to) - MCP latency metrics use explicit bounded integer conversion
// START_elapsed_millis_u64
fn elapsed_millis_u64(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
// END_elapsed_millis_u64

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalidate_graph_cache_clears_graph_and_key() {
        let handler = SynapseHandler::new();
        *handler.graphrag.write().expect("graph lock") = Some(GraphRag::new());
        *handler.graph_cache_key.write().expect("key lock") = Some(GraphCacheKey::for_root(
            &std::env::current_dir().expect("cwd"),
        ));

        handler.invalidate_graph_cache();

        assert!(handler.graphrag.read().expect("graph lock").is_none());
        assert!(handler.graph_cache_key.read().expect("key lock").is_none());
    }

    #[test]
    fn test_classify_mcp_response_detects_error_status() {
        let response = server_response::error(Some(serde_json::json!(1)), -32601, "missing tool");

        let (status, message) = classify_mcp_response(&response);

        assert_eq!(status, McpCallStatus::Error);
        assert_eq!(message, "missing tool");
    }
}

// END_public_api
