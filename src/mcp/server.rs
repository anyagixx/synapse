// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER
// PURPOSE: MCP JSON-RPC server facade — serves Synapse tools over stdio with guarded runtime initialization
// SCOPE: McpServer, SynapseHandler, stdio loop, JSON-RPC routing, guarded index and GraphRAG preload
// DEPENDS: M-CONFIG, M-GRAPHRAG, M-INDEXER, M-MCP-SERVER-CODE-TOOLS, M-MCP-SERVER-GRACE-TOOLS, M-MCP-SERVER-RESPONSE, M-MCP-SERVER-TOOLS
// LINKS: N/A

// START_MODULE_MAP
// McpServer — MCP server entry point (stdio and HTTP stubs)
// discover_project_count — Counts probable child projects for multi-root mode
// SynapseHandler — MCP message router and initialization state
// preload_index_storage — Loads index storage without panicking on poisoned locks
// build_graphrag — Builds GraphRAG while preserving actionable errors
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.9.0 — Guarded MCP runtime preload error paths]
// END_CHANGE_SUMMARY

use super::{server_code_tools, server_grace_tools, server_response, server_tools};
use crate::config::Config;
use crate::graphrag::GraphRag;
use crate::indexer::Indexer;
use crate::skills::{SkillEngine, SkillRequest};
use std::path::Path;
use std::sync::RwLock;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

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

        let mut handler = SynapseHandler::new();
        let (stdin, mut stdout) = (tokio::io::stdin(), tokio::io::stdout());
        let mut lines = BufReader::new(stdin).lines();

        while let Some(line) = lines.next_line().await? {
            let line = line.trim().to_string();
            if line.is_empty() || !line.starts_with('{') {
                continue;
            }

            tracing::debug!("MCP << {}", &line[..line.len().min(200)]);
            let response = handler.handle_message(&line).await;

            let msg = serde_json::to_string(&response)?;
            tracing::debug!("MCP >> {}", &msg[..msg.len().min(200)]);

            let mut out = msg.into_bytes();
            out.push(b'\n');
            stdout.write_all(&out).await?;
            stdout.flush().await?;
        }
        Ok(())
    }
    // END_mcp_server_start_stdio

    // START_CONTRACT_McpServer::start_http
    // PURPOSE: HTTP MCP server stub — delegates to stdio for now
    // INPUTS: { bind: &str }
    // OUTPUTS: { anyhow::Result<()> }
    // START_mcp_server_start_http
    pub async fn start_http(self, bind: &str) -> anyhow::Result<()> {
        tracing::info!(
            "MCP HTTP not yet implemented. Use: socat tcp-l:{} exec:syn mcp",
            bind
        );
        self.start_stdio().await
    }
    // END_mcp_server_start_http
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
    indexer: Indexer,
    graphrag: RwLock<Option<GraphRag>>,
    skill_engine: SkillEngine,
    initialized: bool,
}
// END_SynapseHandler

impl Default for SynapseHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SynapseHandler {
    // START_CONTRACT_SynapseHandler::new
    // PURPOSE: Create a new SynapseHandler with pre-loaded indexer and graph
    // OUTPUTS: { Self }
    // START_sh_new
    pub fn new() -> Self {
        let config = Config::load_or_default();
        let indexer = Indexer::new(&config);
        let skill_engine = SkillEngine::new(&config);
        let mut graphrag = GraphRag::new();

        // Try to find index from current directory.
        if let Ok(cwd) = std::env::current_dir() {
            if let Err(e) = preload_index_storage(&indexer, &cwd) {
                tracing::warn!("[SynapseHandler][new][INDEX] {}", e);
            }
            if let Err(e) = build_graphrag(&mut graphrag, &cwd) {
                tracing::warn!("[SynapseHandler][new][GRAPHRAG] {}", e);
            }
        }
        Self {
            indexer,
            graphrag: RwLock::new(Some(graphrag)),
            skill_engine,
            initialized: false,
        }
    }
    // END_sh_new

    // START_CONTRACT_SynapseHandler::handle_message
    // PURPOSE: Route one JSON-RPC message to MCP initialization, tool listing, or tool execution
    // INPUTS: { line: &str — JSON-RPC request line }
    // OUTPUTS: { serde_json::Value — JSON-RPC response object }
    // START_sh_handle_message
    pub async fn handle_message(&mut self, line: &str) -> serde_json::Value {
        let msg: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => return server_response::error(None, -32700, format!("Parse error: {}", e)),
        };

        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");

        match method {
            "initialize" => {
                self.initialized = true;
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
                if !self.initialized {
                    return server_response::error(id, -32000, "Not initialized");
                }
                server_response::result(
                    id,
                    serde_json::json!({
                        "tools": server_tools::tool_definitions()
                    }),
                )
            }
            "tools/call" => {
                if !self.initialized {
                    return server_response::error(id, -32000, "Not initialized");
                }
                let params = &msg["params"];
                let name = params["name"].as_str().unwrap_or("");
                let args = &params["arguments"];

                match name {
                    "semantic_search" => {
                        server_code_tools::handle_search(&self.indexer, id, args).await
                    }
                    "view_signatures" => {
                        server_code_tools::handle_view_signatures(&self.indexer, id, args).await
                    }
                    "graphrag_query" => {
                        server_code_tools::handle_graphrag(&self.graphrag, id, args)
                    }
                    "verify_project" => server_grace_tools::handle_verify(id, args).await,
                    "review_code" => server_grace_tools::handle_review(id, args).await,
                    "project_status" => server_grace_tools::handle_status(id, args).await,
                    "token_savings" => server_grace_tools::handle_gain(id, args).await,
                    "compress_text" => server_grace_tools::handle_compress(id, args).await,
                    "refresh_project" => server_grace_tools::handle_refresh(id, args).await,
                    "suggest_contract" => {
                        server_grace_tools::handle_suggest_contract(id, args).await
                    }
                    "lsp_hover" => server_code_tools::handle_lsp_hover(id, args).await,
                    "lsp_references" => server_code_tools::handle_lsp_references(id, args).await,
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
                }
            }
            "notifications/initialized" => serde_json::json!({}),
            _ => {
                if !self.initialized && method != "initialize" {
                    return server_response::error(id, -32000, "Not initialized");
                }
                server_response::error(id, -32601, format!("Method not found: {}", method))
            }
        }
    }
    // END_sh_handle_message
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

// START_CONTRACT_build_graphrag
// PURPOSE: Build GraphRAG preload state with project path context in any returned error
// INPUTS: { graphrag: &mut GraphRag }, { root: &Path }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: populates GraphRAG graph when build succeeds
// START_build_graphrag
fn build_graphrag(graphrag: &mut GraphRag, root: &Path) -> anyhow::Result<()> {
    graphrag
        .build(root)
        .map_err(|e| anyhow::anyhow!("build GraphRAG for {}: {}", root.display(), e))
}
// END_build_graphrag
// END_public_api
