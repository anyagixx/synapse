// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER
// PURPOSE: MCP JSON-RPC server facade — serves Synapse tools over clean stdio with guarded runtime initialization
// SCOPE: McpServer, SynapseHandler, runtime Config retention, config-bounded pipelined stdio loop, best-effort MCP metrics recording, session budget gates for expensive tools, context pressure metadata, pressure-aware tools/list disclosure, short-lived ETag cache hints, JSON-RPC request/notification routing including tools/recommend, analyze_logs, extract_belief_state, generate_requirements, generate_technology, generate_development_plan, mental_test_run, traceability_report, cascade_impact, cascade_execute, run_test_guide, submit_test_report, advance_phase, compact_evidence, check_budget, context_pressure, pre_commit_check, suggest_contract, and config-aware LSP tools, guarded index preload and indexed GraphRAG cache state
// DEPENDS: M-CONFIG, M-GRAPHRAG, M-INDEXER, M-MCP-PIPELINE, M-MCP-SERVER-CASCADE-TOOLS, M-MCP-SERVER-CODE-TOOLS, M-MCP-SERVER-GRACE-TOOLS, M-MCP-SERVER-RUN-TOOLS, M-MCP-SERVER-RESPONSE, M-MCP-SERVER-TOOLS, M-TRACKING, M-TRACKING-MCP-METRICS, M-UTILS
// LINKS: N/A
// START_MODULE_MAP
// McpServer — MCP stdio server entry point
// discover_project_count — Counts probable child projects for multi-root mode
// SynapseHandler — MCP message router, runtime config, and initialization state
// McpEtagCache — Bounded in-memory cache validator store for cacheable MCP tools
// McpEtagCacheEntry — One cache validator plus expiry deadline
// GraphCacheKey — Root/index signature used to invalidate cached GraphRAG state
// preload_index_storage — Loads index storage without panicking on poisoned locks
// cache_validator — Extracts _if_none_match from tool arguments
// maybe_not_modified_response — Returns compact cached-validator response before expensive handlers
// record_cache_metadata — Adds _meta.cache to tool results and records cacheable ETags
// budget_status_for_tool — Checks session budget for expensive tools
// budget_blocked_response / attach_budget_warning_metadata — Render budget gate results
// tool_recommend — Delegates tools/recommend to the run-state-aware recommendation engine
// tools_list_profile — Parses tools/list profile params
// tools_list_profile_label — Returns stable tools/list profile metadata
// tools_list_style — Parses tools/list schema style params
// schema_savings_pct — Computes tools/list schema economy percent
// classify_mcp_response — Converts JSON-RPC tool responses into tracking status metadata
// END_MODULE_MAP
// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.31.0 - Applied pressure-aware tools/list style]
// END_CHANGE_SUMMARY
use super::server_budget_tools::handle_check_budget as budget;
use super::server_budget_tools::handle_context_pressure as pressure;
use super::server_pressure::attach_context_pressure_metadata as pressure_meta;
use super::server_tools_pressure::effective_schema_style as eff_style;
use super::{
    pipeline::{self, McpPipelineConfig, PipelineHandler},
    server_cascade_tools, server_code_tools, server_contract_tools, server_grace_tools,
    server_response, server_run_tools, server_tools, tool_recommend,
};
use crate::config::Config;
use crate::graphrag::GraphRag;
use crate::indexer::storage::Storage;
use crate::indexer::Indexer;
use crate::skills::{SkillEngine, SkillRequest};
use crate::tracking::{mcp_metrics::McpCallStatus, BudgetLevel, BudgetStatus, Tracker};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
use std::time::{Duration, Instant, UNIX_EPOCH};
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
    etag_cache: RwLock<McpEtagCache>,
    skill_engine: SkillEngine,
    tracker: Tracker,
    initialized: AtomicBool,
}
// END_SynapseHandler

const MCP_ETAG_CACHE_LIMIT: usize = 100;

// START_McpEtagCache
#[derive(Debug, Default)]
struct McpEtagCache {
    entries: BTreeMap<String, McpEtagCacheEntry>,
}
// END_McpEtagCache

// START_McpEtagCacheEntry
#[derive(Clone, Debug)]
struct McpEtagCacheEntry {
    metadata: server_response::CacheMetadata,
    expires_at: Instant,
}
// END_McpEtagCacheEntry

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
            etag_cache: RwLock::new(McpEtagCache::default()),
            skill_engine,
            tracker,
            initialized: AtomicBool::new(false),
        }
    }
    // END_sh_new

    // START_CONTRACT_SynapseHandler::new_for_test
    // PURPOSE: Create a handler with explicit config and tracker for scoped protocol tests.
    // INPUTS: { config: Config }, { tracker: Tracker }
    // OUTPUTS: { Self }
    // START_sh_new_for_test
    #[cfg(test)]
    fn new_for_test(config: Config, tracker: Tracker) -> Self {
        let indexer = Indexer::new(&config);
        let skill_engine = SkillEngine::new(&config);
        Self {
            config,
            indexer,
            graphrag: RwLock::new(None),
            graph_cache_key: RwLock::new(None),
            etag_cache: RwLock::new(McpEtagCache::default()),
            skill_engine,
            tracker,
            initialized: AtomicBool::new(false),
        }
    }
    // END_sh_new_for_test

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
                let params = &msg["params"];
                let profile = tools_list_profile(params);
                let style = eff_style(&self.config, &self.tracker, tools_list_style(params)).await;
                let total_available = server_tools::tool_definitions().len();
                let full_tools = server_tools::tool_definitions_for_profile(&profile);
                let full_schema_bytes = server_tools::tool_definitions_json_bytes(&full_tools);
                let tools = server_tools::apply_tool_schema_style(full_tools, style);
                let schema_bytes = server_tools::tool_definitions_json_bytes(&tools);
                let total_visible = tools.len();
                server_response::result(
                    id,
                    serde_json::json!({
                        "tools": tools,
                        "profile": tools_list_profile_label(&profile),
                        "style": style.label(),
                        "total_available": total_available,
                        "total_visible": total_visible,
                        "schema_economy": {
                            "full_bytes": full_schema_bytes,
                            "visible_bytes": schema_bytes,
                            "saved_bytes": full_schema_bytes.saturating_sub(schema_bytes),
                            "savings_pct": schema_savings_pct(full_schema_bytes, schema_bytes)
                        }
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
                let budget_status = match self.budget_status_for_tool(name).await {
                    Ok(status) => status,
                    Err(error) => {
                        tracing::warn!(
                            "[SynapseHandler][handle_message][MCP_BUDGET_CHECK] {}",
                            error
                        );
                        None
                    }
                };

                let mut response = if let Some(status) = budget_status
                    .as_ref()
                    .filter(|status| status.status == BudgetLevel::Blocked)
                {
                    budget_blocked_response(id.clone(), status)
                } else if let Some(response) =
                    self.maybe_not_modified_response(id.clone(), name, args)
                {
                    response
                } else {
                    let mut response = match name {
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
                        "mental_test_run" => {
                            server_grace_tools::handle_mental_test_run(id, args).await
                        }
                        "traceability_report" => {
                            server_grace_tools::handle_traceability_report(id, args).await
                        }
                        "cascade_impact" => {
                            server_cascade_tools::handle_cascade_impact(id, args).await
                        }
                        "cascade_execute" => {
                            server_cascade_tools::handle_cascade_execute(id, args).await
                        }
                        "run_test_guide" => {
                            server_grace_tools::handle_run_test_guide(id, args).await
                        }
                        "submit_test_report" => {
                            server_grace_tools::handle_submit_test_report(id, args).await
                        }
                        "self_heal" => server_grace_tools::handle_self_heal(id, args).await,
                        "advance_phase" => server_run_tools::handle_advance_phase(id, args).await,
                        "pre_commit_check" => {
                            server_run_tools::handle_pre_commit_check(id, args).await
                        }
                        "compact_evidence" => {
                            server_run_tools::handle_compact_evidence(id, args).await
                        }
                        "tools/recommend" => tool_recommend::handle_recommend(id, args).await,
                        "token_savings" => server_grace_tools::handle_gain(id, args).await,
                        "check_budget" => budget(&self.config, &self.tracker, id, args).await,
                        "context_pressure" => pressure(&self.config, &self.tracker, id).await,
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
                    if let Some(status) = budget_status
                        .as_ref()
                        .filter(|status| status.status == BudgetLevel::Warning)
                    {
                        attach_budget_warning_metadata(&mut response, status);
                    }
                    self.record_cache_metadata(name, args, &mut response);
                    response
                };
                pressure_meta(&self.config, &self.tracker, args, &mut response).await;
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

    // START_CONTRACT_SynapseHandler::maybe_not_modified_response
    // PURPOSE: Return a compact not-modified result when _if_none_match matches a fresh cached ETag
    // INPUTS: { id: Option<serde_json::Value> }, { tool_name: &str }, { args: &serde_json::Value }
    // OUTPUTS: { Option<serde_json::Value> }
    // START_sh_maybe_not_modified_response
    fn maybe_not_modified_response(
        &self,
        id: Option<serde_json::Value>,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        if !server_response::is_tool_cacheable(tool_name) {
            return None;
        }
        let validator = cache_validator(args)?;
        let key = server_response::cache_key_for_tool_call(tool_name, args);
        let metadata = match self.etag_cache.write() {
            Ok(mut cache) => cache.matching_metadata(&key, validator),
            Err(error) => {
                tracing::warn!("[SynapseHandler][handle_message][MCP_CACHE_READ] {}", error);
                None
            }
        }?;
        Some(server_response::not_modified_result(id, &metadata))
    }
    // END_sh_maybe_not_modified_response

    // START_CONTRACT_SynapseHandler::record_cache_metadata
    // PURPOSE: Attach cache metadata to a JSON-RPC tool result and store cacheable validators
    // INPUTS: { tool_name: &str }, { args: &serde_json::Value }, { response: &mut serde_json::Value }
    // OUTPUTS: { () }
    // SIDE_EFFECTS: mutates response _meta.cache and in-memory ETag cache
    // START_sh_record_cache_metadata
    fn record_cache_metadata(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        response: &mut serde_json::Value,
    ) {
        let Some(metadata) = server_response::with_cache_metadata(response, tool_name, args) else {
            return;
        };
        if metadata.ttl_secs == 0 {
            return;
        }
        let key = server_response::cache_key_for_tool_call(tool_name, args);
        match self.etag_cache.write() {
            Ok(mut cache) => cache.insert(key, metadata),
            Err(error) => tracing::warn!(
                "[SynapseHandler][handle_message][MCP_CACHE_WRITE] {}",
                error
            ),
        }
    }
    // END_sh_record_cache_metadata

    // START_CONTRACT_SynapseHandler::budget_status_for_tool
    // PURPOSE: Check configured session budget for expensive MCP tools.
    // INPUTS: { tool_name: &str }
    // OUTPUTS: { anyhow::Result<Option<BudgetStatus>> }
    // LINKS:
    //   -> M-TRACKING (depends) - reads current session budget state
    // START_sh_budget_status_for_tool
    async fn budget_status_for_tool(
        &self,
        tool_name: &str,
    ) -> anyhow::Result<Option<BudgetStatus>> {
        let budget = &self.config.budget;
        if budget.session_token_limit == 0 || !is_budget_gated_tool(tool_name) {
            return Ok(None);
        }
        self.tracker
            .check_budget(
                budget.session_token_limit,
                budget.warn_at_pct,
                budget.block_at_pct,
            )
            .await
            .map(Some)
    }
    // END_sh_budget_status_for_tool

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

impl McpEtagCache {
    // START_CONTRACT_McpEtagCache::matching_metadata
    // PURPOSE: Return metadata for a matching fresh validator and evict expired entries
    // INPUTS: { key: &str }, { validator: &str }
    // OUTPUTS: { Option<server_response::CacheMetadata> }
    // START_mcp_etag_cache_matching_metadata
    fn matching_metadata(
        &mut self,
        key: &str,
        validator: &str,
    ) -> Option<server_response::CacheMetadata> {
        let now = Instant::now();
        if self
            .entries
            .get(key)
            .is_some_and(|entry| entry.expires_at <= now)
        {
            self.entries.remove(key);
            return None;
        }
        self.entries
            .get(key)
            .and_then(|entry| (entry.metadata.etag == validator).then_some(entry.metadata.clone()))
    }
    // END_mcp_etag_cache_matching_metadata

    // START_CONTRACT_McpEtagCache::insert
    // PURPOSE: Store a fresh cache validator and clear all entries when the bounded cache would exceed 100
    // INPUTS: { key: String }, { metadata: server_response::CacheMetadata }
    // OUTPUTS: { () }
    // SIDE_EFFECTS: mutates in-memory cache entries
    // START_mcp_etag_cache_insert
    fn insert(&mut self, key: String, metadata: server_response::CacheMetadata) {
        if metadata.ttl_secs == 0 {
            return;
        }
        if self.entries.len() >= MCP_ETAG_CACHE_LIMIT {
            self.entries.clear();
        }
        let expires_at = Instant::now() + Duration::from_secs(metadata.ttl_secs);
        self.entries.insert(
            key,
            McpEtagCacheEntry {
                metadata,
                expires_at,
            },
        );
    }
    // END_mcp_etag_cache_insert

    // START_CONTRACT_McpEtagCache::len
    // PURPOSE: Return the number of stored cache validators for tests and diagnostics
    // OUTPUTS: { usize }
    // START_mcp_etag_cache_len
    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
    // END_mcp_etag_cache_len
}

// START_CONTRACT_cache_validator
// PURPOSE: Extract a non-empty _if_none_match validator from MCP tool arguments
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Option<&str> }
// START_cache_validator
fn cache_validator(args: &serde_json::Value) -> Option<&str> {
    args.get("_if_none_match")
        .and_then(serde_json::Value::as_str)
        .filter(|validator| !validator.trim().is_empty())
}
// END_cache_validator

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

// START_CONTRACT_tools_list_profile
// PURPOSE: Parse tools/list profile params into a stable disclosure profile
// INPUTS: { params: &serde_json::Value }
// OUTPUTS: { server_tools::ToolProfile }
// LINKS:
//   -> M-MCP-SERVER-TOOLS (depends) - profile matching registry
// START_tools_list_profile
fn tools_list_profile(params: &serde_json::Value) -> server_tools::ToolProfile {
    let value = params
        .get("profile")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("all");
    server_tools::ToolProfile::parse(value)
}
// END_tools_list_profile

// START_CONTRACT_tools_list_profile_label
// PURPOSE: Return a compact normalized profile label for tools/list response metadata
// INPUTS: { profile: &server_tools::ToolProfile }
// OUTPUTS: { &'static str }
// LINKS:
//   -> NFR-003 (traces_to) - clients can observe schema disclosure decisions
// START_tools_list_profile_label
fn tools_list_profile_label(profile: &server_tools::ToolProfile) -> &'static str {
    match profile {
        server_tools::ToolProfile::All => "all",
        server_tools::ToolProfile::Verification => "verification",
        server_tools::ToolProfile::Planning => "planning",
        server_tools::ToolProfile::Implementation => "implementation",
        server_tools::ToolProfile::Debugging => "debugging",
        server_tools::ToolProfile::Minimal => "minimal",
        server_tools::ToolProfile::Custom(_) => "custom",
    }
}
// END_tools_list_profile_label

// START_CONTRACT_tools_list_style
// PURPOSE: Parse tools/list schema style params while defaulting to backward-compatible full schemas
// INPUTS: { params: &serde_json::Value }
// OUTPUTS: { server_tools::ToolSchemaStyle }
// LINKS:
//   -> NFR-003 (traces_to) - style metadata exposes token economy mode
// START_tools_list_style
fn tools_list_style(params: &serde_json::Value) -> server_tools::ToolSchemaStyle {
    let value = params
        .get("style")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("full");
    server_tools::ToolSchemaStyle::parse(value)
}
// END_tools_list_style

// START_CONTRACT_is_budget_gated_tool
// PURPOSE: Return true for expensive tools that should be blocked when the session budget is exhausted.
// INPUTS: { tool_name: &str }
// OUTPUTS: { bool }
// START_is_budget_gated_tool
fn is_budget_gated_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "semantic_search"
            | "graphrag_query"
            | "analyze_logs"
            | "extract_belief_state"
            | "generate_requirements"
            | "generate_technology"
            | "generate_development_plan"
            | "mental_test_run"
            | "traceability_report"
            | "cascade_impact"
            | "cascade_execute"
            | "run_test_guide"
            | "submit_test_report"
            | "self_heal"
            | "diagnose_failure"
            | "repair_contract"
            | "suggest_contract"
            | "lsp_hover"
            | "lsp_references"
    )
}
// END_is_budget_gated_tool

// START_CONTRACT_budget_blocked_response
// PURPOSE: Build a compact MCP result for an expensive tool blocked by session budget.
// INPUTS: { id: Option<serde_json::Value> }, { status: &BudgetStatus }
// OUTPUTS: { serde_json::Value }
// START_budget_blocked_response
fn budget_blocked_response(
    id: Option<serde_json::Value>,
    status: &BudgetStatus,
) -> serde_json::Value {
    server_response::result(
        id,
        serde_json::json!({
            "content": [{"type": "text", "text": status.message}],
            "budget_exhausted": true,
            "isError": true,
            "_meta": {"budget": status}
        }),
    )
}
// END_budget_blocked_response

// START_CONTRACT_attach_budget_warning_metadata
// PURPOSE: Attach budget warning metadata to a successful tool result.
// INPUTS: { response: &mut serde_json::Value }, { status: &BudgetStatus }
// OUTPUTS: { () }
// START_attach_budget_warning_metadata
fn attach_budget_warning_metadata(response: &mut serde_json::Value, status: &BudgetStatus) {
    if let Some(result) = response
        .get_mut("result")
        .and_then(serde_json::Value::as_object_mut)
    {
        let meta = result
            .entry("_meta")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(meta) = meta.as_object_mut() {
            meta.insert("budget".into(), serde_json::json!(status));
        }
    }
}
// END_attach_budget_warning_metadata

// START_CONTRACT_schema_savings_pct
// PURPOSE: Compute one-decimal percentage savings for tools/list schema economy metadata
// INPUTS: { full_bytes: usize }, { visible_bytes: usize }
// OUTPUTS: { f64 }
// LINKS:
//   -> NFR-003 (traces_to) - tools/list reports observable token-economy impact
// START_schema_savings_pct
fn schema_savings_pct(full_bytes: usize, visible_bytes: usize) -> f64 {
    if full_bytes == 0 {
        return 0.0;
    }
    let saved = full_bytes.saturating_sub(visible_bytes) as f64;
    ((saved / full_bytes as f64) * 1000.0).round() / 10.0
}
// END_schema_savings_pct

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

    // START_CONTRACT_test_cache_validator_extracts_if_none_match
    // PURPOSE: Verify _if_none_match is accepted only when it is a non-empty string
    // START_test_cache_validator_extracts_if_none_match
    #[test]
    fn test_cache_validator_extracts_if_none_match() {
        assert_eq!(
            cache_validator(&serde_json::json!({"_if_none_match": "W/\"syn-a\""})),
            Some("W/\"syn-a\"")
        );
        assert_eq!(
            cache_validator(&serde_json::json!({"_if_none_match": "   "})),
            None
        );
        assert_eq!(cache_validator(&serde_json::json!({})), None);
    }
    // END_test_cache_validator_extracts_if_none_match

    // START_CONTRACT_test_mcp_etag_cache_matches_and_expires
    // PURPOSE: Verify cache validators match fresh ETags and expired entries are evicted
    // START_test_mcp_etag_cache_matches_and_expires
    #[test]
    fn test_mcp_etag_cache_matches_and_expires() {
        let metadata = server_response::CacheMetadata::new("W/\"syn-1\"".to_string(), 60);
        let mut cache = McpEtagCache::default();

        cache.insert("tool:{}".to_string(), metadata.clone());

        assert_eq!(
            cache.matching_metadata("tool:{}", "W/\"syn-1\""),
            Some(metadata)
        );
        assert_eq!(cache.matching_metadata("tool:{}", "W/\"syn-2\""), None);

        cache.entries.insert(
            "expired".to_string(),
            McpEtagCacheEntry {
                metadata: server_response::CacheMetadata::new("W/\"syn-old\"".to_string(), 60),
                expires_at: Instant::now() - Duration::from_secs(1),
            },
        );

        assert_eq!(cache.matching_metadata("expired", "W/\"syn-old\""), None);
        assert!(!cache.entries.contains_key("expired"));
    }
    // END_test_mcp_etag_cache_matches_and_expires

    // START_CONTRACT_test_mcp_etag_cache_clears_when_over_limit
    // PURPOSE: Verify the in-memory ETag cache clears all old entries before storing entry 101
    // START_test_mcp_etag_cache_clears_when_over_limit
    #[test]
    fn test_mcp_etag_cache_clears_when_over_limit() {
        let mut cache = McpEtagCache::default();

        for index in 0..MCP_ETAG_CACHE_LIMIT {
            cache.insert(
                format!("key-{index}"),
                server_response::CacheMetadata::new(format!("W/\"syn-{index}\""), 60),
            );
        }
        cache.insert(
            "overflow".to_string(),
            server_response::CacheMetadata::new("W/\"syn-overflow\"".to_string(), 60),
        );

        assert_eq!(cache.len(), 1);
        assert!(cache.matching_metadata("key-0", "W/\"syn-0\"").is_none());
        assert!(cache
            .matching_metadata("overflow", "W/\"syn-overflow\"")
            .is_some());
    }
    // END_test_mcp_etag_cache_clears_when_over_limit

    #[test]
    fn test_tools_list_params_default_to_all_full() {
        let params = serde_json::json!({});

        assert_eq!(tools_list_profile(&params), server_tools::ToolProfile::All);
        assert_eq!(
            tools_list_style(&params),
            server_tools::ToolSchemaStyle::Full
        );
    }

    #[test]
    fn test_tools_list_params_parse_profile_and_style() {
        let params = serde_json::json!({
            "profile": "custom:semantic_search,verify_project",
            "style": "terse"
        });

        assert_eq!(
            tools_list_profile(&params),
            server_tools::ToolProfile::Custom(vec![
                "semantic_search".into(),
                "verify_project".into()
            ])
        );
        assert_eq!(
            tools_list_style(&params),
            server_tools::ToolSchemaStyle::Terse
        );
    }

    #[tokio::test]
    async fn test_tools_list_profile_metadata_filters_tools() {
        let handler = SynapseHandler::new();
        handler
            .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
            .await
            .expect("initialize response");

        let response = handler
            .handle_message(
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"profile":"minimal"}}"#,
            )
            .await
            .expect("tools/list response");
        let result = &response["result"];
        let tools = result["tools"].as_array().expect("tools array");
        let names: Vec<_> = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();

        assert_eq!(result["profile"], "minimal");
        assert_eq!(result["style"], "full");
        assert_eq!(result["total_visible"], 6);
        assert_eq!(
            result["schema_economy"]["savings_pct"],
            schema_savings_pct(
                result["schema_economy"]["full_bytes"]
                    .as_u64()
                    .expect("full bytes") as usize,
                result["schema_economy"]["visible_bytes"]
                    .as_u64()
                    .expect("visible bytes") as usize,
            )
        );
        assert!(result["total_available"].as_u64().unwrap_or(0) > 6);
        assert_eq!(
            names,
            vec![
                "semantic_search",
                "graphrag_query",
                "verify_project",
                "review_code",
                "project_status",
                "mental_test_run"
            ]
        );
    }

    #[tokio::test]
    async fn test_tools_list_terse_style_removes_descriptions() {
        let handler = SynapseHandler::new();
        handler
            .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
            .await
            .expect("initialize response");

        let response = handler
            .handle_message(
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"profile":"custom:semantic_search","style":"terse"}}"#,
            )
            .await
            .expect("tools/list response");
        let result = &response["result"];
        let tool = &result["tools"][0];

        assert_eq!(result["profile"], "custom");
        assert_eq!(result["style"], "terse");
        assert_eq!(result["total_visible"], 1);
        assert!(tool.get("description").is_none());
        assert!(tool["inputSchema"]["properties"]["query"]
            .get("description")
            .is_none());
        assert_eq!(tool["inputSchema"]["required"][0], "query");
        assert!(result["schema_economy"]["savings_pct"]
            .as_f64()
            .is_some_and(|pct| pct > 0.0));
    }

    // START_CONTRACT_test_tools_recommend_routes_through_tools_call
    // PURPOSE: Verify tools/recommend is routed as a tools/call tool and returns bounded engine output
    // START_test_tools_recommend_routes_through_tools_call
    #[tokio::test]
    async fn test_tools_recommend_routes_through_tools_call() {
        let handler = SynapseHandler::new();
        handler
            .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
            .await
            .expect("initialize response");

        let response = handler
            .handle_message(
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tools/recommend","arguments":{"max_tools":3}}}"#,
            )
            .await
            .expect("tools/recommend response");

        assert_eq!(response["result"]["phase"], "general");
        assert_eq!(
            response["result"]["recommended_tools"]
                .as_array()
                .expect("recommendations")
                .len(),
            3
        );
        assert!(response["result"]["suggested_profile"]
            .as_str()
            .is_some_and(|profile| profile.starts_with("custom:semantic_search")));
    }
    // END_test_tools_recommend_routes_through_tools_call

    // START_CONTRACT_test_budget_gate_blocks_expensive_tools_and_allows_light_tools
    // PURPOSE: Verify exhausted budgets block expensive tools while light status tools remain available.
    // START_test_budget_gate_blocks_expensive_tools_and_allows_light_tools
    #[tokio::test]
    async fn test_budget_gate_blocks_expensive_tools_and_allows_light_tools() {
        let data_home = tempfile::tempdir().expect("data home");
        let project_root = tempfile::tempdir().expect("project root");
        let mut config = Config::default();
        config.budget.session_token_limit = 100;
        config.budget.warn_at_pct = 80;
        config.budget.block_at_pct = 100;
        let tracker = Tracker::new_for_test(
            &config,
            data_home.path(),
            project_root.path(),
            Some("budget-blocked"),
        );
        tracker.record("spent", 100, 20).await.expect("seed spend");
        let handler = SynapseHandler::new_for_test(config, tracker);
        handler
            .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
            .await
            .expect("initialize response");

        let blocked = handler
            .handle_message(
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"semantic_search","arguments":{"query":"Phase-87","max_results":1}}}"#,
            )
            .await
            .expect("blocked response");
        let light = handler
            .handle_message(
                r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"project_status","arguments":{}}}"#,
            )
            .await
            .expect("light response");

        assert_eq!(blocked["result"]["budget_exhausted"], true);
        assert_eq!(blocked["result"]["_meta"]["budget"]["status"], "blocked");
        assert_ne!(light["result"]["budget_exhausted"], true);
        assert!(is_budget_gated_tool("semantic_search"));
        assert!(!is_budget_gated_tool("project_status"));
        assert!(!is_budget_gated_tool("verify_project"));
    }
    // END_test_budget_gate_blocks_expensive_tools_and_allows_light_tools

    // START_CONTRACT_test_budget_gate_appends_warning_metadata
    // PURPOSE: Verify warning-level budgets append budget metadata without blocking the expensive tool.
    // START_test_budget_gate_appends_warning_metadata
    #[tokio::test]
    async fn test_budget_gate_appends_warning_metadata() {
        let data_home = tempfile::tempdir().expect("data home");
        let project_root = tempfile::tempdir().expect("project root");
        let mut config = Config::default();
        config.budget.session_token_limit = 100;
        config.budget.warn_at_pct = 80;
        config.budget.block_at_pct = 100;
        let tracker = Tracker::new_for_test(
            &config,
            data_home.path(),
            project_root.path(),
            Some("budget-warning"),
        );
        tracker.record("spent", 85, 20).await.expect("seed spend");
        let handler = SynapseHandler::new_for_test(config, tracker);
        handler
            .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
            .await
            .expect("initialize response");

        let response = handler
            .handle_message(
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"semantic_search","arguments":{"query":"Phase-87","max_results":1}}}"#,
            )
            .await
            .expect("warning response");

        assert_ne!(response["result"]["budget_exhausted"], true);
        assert_eq!(response["result"]["_meta"]["budget"]["status"], "warning");
        assert_eq!(
            response["result"]["_meta"]["budget"]["used_input_tokens"],
            85
        );
    }
    // END_test_budget_gate_appends_warning_metadata

    // START_CONTRACT_test_cache_not_modified_response_for_project_status
    // PURPOSE: Verify project_status emits cache metadata and a matching _if_none_match returns _not_modified
    // START_test_cache_not_modified_response_for_project_status
    #[tokio::test]
    async fn test_cache_not_modified_response_for_project_status() {
        let handler = SynapseHandler::new();
        handler
            .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
            .await
            .expect("initialize response");

        let first_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "project_status",
                "arguments": {}
            }
        })
        .to_string();
        let first = handler
            .handle_message(&first_request)
            .await
            .expect("first project_status response");
        let etag = first["result"]["_meta"]["cache"]["etag"]
            .as_str()
            .expect("etag")
            .to_string();

        let second_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "project_status",
                "arguments": {
                    "_if_none_match": etag
                }
            }
        })
        .to_string();
        let second = handler
            .handle_message(&second_request)
            .await
            .expect("second project_status response");

        assert_eq!(first["result"]["_meta"]["cache"]["ttl_secs"], 15);
        assert_eq!(second["id"], 3);
        assert_eq!(second["result"]["_not_modified"], true);
        assert_eq!(second["result"]["_meta"]["cache"]["ttl_secs"], 15);
    }
    // END_test_cache_not_modified_response_for_project_status

    // START_CONTRACT_test_cache_metadata_attaches_to_graphrag_overview
    // PURPOSE: Verify graphrag_query overview emits cache metadata without rebuilding an already-cached graph
    // START_test_cache_metadata_attaches_to_graphrag_overview
    #[tokio::test]
    async fn test_cache_metadata_attaches_to_graphrag_overview() {
        let handler = SynapseHandler::new();
        let root = std::env::current_dir().expect("cwd");
        *handler.graphrag.write().expect("graph lock") = Some(GraphRag::new());
        *handler.graph_cache_key.write().expect("key lock") = Some(GraphCacheKey::for_root(&root));
        handler
            .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
            .await
            .expect("initialize response");

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "graphrag_query",
                "arguments": {"operation": "overview"}
            }
        })
        .to_string();
        let response = handler
            .handle_message(&request)
            .await
            .expect("graphrag_query response");

        assert_eq!(response["result"]["_meta"]["cache"]["ttl_secs"], 120);
        assert!(response["result"]["_meta"]["cache"]["etag"]
            .as_str()
            .is_some_and(|etag| etag.starts_with("W/\"syn-")));
    }
    // END_test_cache_metadata_attaches_to_graphrag_overview
}
// END_public_api
