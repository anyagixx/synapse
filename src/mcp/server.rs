// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER
// PURPOSE: MCP JSON-RPC server facade — serves Synapse tools over stdio for AI agent consumption
// SCOPE: McpServer, SynapseHandler, stdio loop, JSON-RPC routing
// DEPENDS: M-CONFIG, M-GRAPHRAG, M-INDEXER, M-MCP-SERVER-CODE-TOOLS, M-MCP-SERVER-GRACE-TOOLS, M-MCP-SERVER-RESPONSE, M-MCP-SERVER-TOOLS
// LINKS: N/A

// START_MODULE_MAP
// McpServer — MCP server entry point (stdio and HTTP stubs)
// SynapseHandler — MCP message router and initialization state
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.2.0 — Extracted MCP tool handlers, response helpers, and tool definitions]
// END_CHANGE_SUMMARY

use super::{server_code_tools, server_grace_tools, server_response, server_tools};
use crate::config::Config;
use crate::graphrag::GraphRag;
use crate::indexer::Indexer;
use crate::skills::{SkillEngine, SkillRequest};
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
        let root = std::env::current_dir().unwrap_or_default();
        let _handler = SynapseHandler::new();

        // Auto-discover: if current dir is not a project root but parent has .opencode/
        // or multiple subdirs with source code, use multi-root mode.
        let multi_root = if !root.join(".opencode").exists()
            && !root.join("opencode.jsonc").exists()
            && !root.join("opencode.json").exists()
        {
            let parent_has_projects = std::fs::read_dir(&root)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            e.path().join(".opencode").exists()
                                || e.path().join("src").exists()
                                || e.path().join("Cargo.toml").exists()
                                || e.path().join("package.json").exists()
                        })
                        .count()
                })
                .unwrap_or(0);
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
            let mut guard = indexer.storage.write().unwrap();
            if guard.is_none() {
                *guard = Some(crate::indexer::storage::Storage::new(&cwd));
            }
            graphrag.build(&cwd).ok();
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
// END_public_api
