// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER
// PURPOSE: MCP JSON-RPC server — serves Synapse tools over stdio for AI agent consumption
// SCOPE: McpServer, SynapseHandler, JSON-RPC message handling, 12 MCP tools (semantic_search, view_signatures, graphrag_query, etc.)
// DEPENDS: M-CONFIG, M-GRAPHRAG, M-INDEXER, M-GRACE, M-TRACKING, M-COMPRESS
// LINKS: N/A

// START_MODULE_MAP
// McpServer — MCP server entry point (stdio and HTTP stubs)
// SynapseHandler — MCP message handler with 8+ tool implementations
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY

use crate::config::Config;
use crate::graphrag::GraphRag;
use crate::indexer::Indexer;
use crate::skills::{registry::SKILL_DEFS, SkillEngine, SkillRequest};
use std::fmt::Display;
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
        // or multiple subdirs with source code, use multi-root mode
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

        // Try to find index from current directory
        if let Ok(cwd) = std::env::current_dir() {
            let mut guard = indexer.storage.write().unwrap();
            if guard.is_none() {
                *guard = Some(crate::indexer::storage::Storage::new(&cwd));
            }
            // Build graph from storage
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

    pub async fn handle_message(&mut self, line: &str) -> serde_json::Value {
        let msg: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => return self.error(None, -32700, format!("Parse error: {}", e)),
        };

        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");

        match method {
            "initialize" => {
                self.initialized = true;
                self.result(
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
                    return self.error(id, -32000, "Not initialized");
                }
                self.result(
                    id,
                    serde_json::json!({
                        "tools": self.tool_definitions()
                    }),
                )
            }
            "tools/call" => {
                if !self.initialized {
                    return self.error(id, -32000, "Not initialized");
                }
                let params = &msg["params"];
                let name = params["name"].as_str().unwrap_or("");
                let args = &params["arguments"];

                match name {
                    "semantic_search" => self.handle_search(id, args).await,
                    "view_signatures" => self.handle_view_signatures(id, args).await,
                    "graphrag_query" => self.handle_graphrag(id, args),
                    "verify_project" => self.handle_verify(id, args).await,
                    "review_code" => self.handle_review(id, args).await,
                    "project_status" => self.handle_status(id, args).await,
                    "token_savings" => self.handle_gain(id, args).await,
                    "compress_text" => self.handle_compress(id, args).await,
                    "refresh_project" => self.handle_refresh(id, args).await,
                    "suggest_contract" => self.handle_suggest_contract(id, args).await,
                    "lsp_hover" => self.handle_lsp_hover(id, args).await,
                    "lsp_references" => self.handle_lsp_references(id, args).await,
                    name if name.starts_with("grace_") => {
                        self.handle_grace_skill(id, name, args).await
                    }
                    _ => self.error(id, -32601, format!("Unknown tool: {}", name)),
                }
            }
            "notifications/initialized" => {
                serde_json::json!({})
            }
            _ => {
                if !self.initialized && method != "initialize" {
                    return self.error(id, -32000, "Not initialized");
                }
                self.error(id, -32601, format!("Method not found: {}", method))
            }
        }
    }

    async fn handle_search(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        let query = args["query"].as_str().unwrap_or("");
        let max = args["max_results"].as_u64().unwrap_or(10) as usize;

        match self.indexer.search(query, max).await {
            Ok(results) => {
                let text = if results.is_empty() {
                    "No results found. Try running `syn index` first.".to_string()
                } else {
                    results
                        .iter()
                        .enumerate()
                        .map(|(i, r)| {
                            let preview = if r.content.len() > 150 {
                                format!("{}...", &r.content[..150])
                            } else {
                                r.content.clone()
                            };
                            format!(
                                "{}. {} ({}:{}-{})\n   {}",
                                i + 1,
                                r.path,
                                r.language,
                                r.start_line,
                                r.end_line,
                                preview
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n")
                };
                self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    }),
                )
            }
            Err(e) => self.error(id, -32603, format!("Search error: {}", e)),
        }
    }

    fn handle_graphrag(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        let operation = args["operation"].as_str().unwrap_or("search");
        let guard = self.graphrag.read().unwrap();
        let graphrag = match guard.as_ref() {
            Some(g) => g,
            None => return self.error(id, -32603, "GraphRAG not built. Run `syn index` first."),
        };

        match operation {
            "overview" => match graphrag.overview() {
                Some(ov) => self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": format!(
                            "Graph Overview:\n  Nodes: {}\n  Relationships: {}\n  Types:\n    {}",
                            ov.total_nodes, ov.total_relationships, ov.node_types.join("\n    ")
                        )}],
                        "isError": false
                    }),
                ),
                None => self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": "Empty graph"}],
                        "isError": false
                    }),
                ),
            },
            "search" => {
                let query = args["query"].as_str().unwrap_or("");
                if query.is_empty() {
                    return self.error(id, -32602, "Missing 'query' parameter");
                }
                let nodes = graphrag.search_nodes(query);
                let text = if nodes.is_empty() {
                    "No nodes found".to_string()
                } else {
                    nodes
                        .iter()
                        .enumerate()
                        .map(|(i, n)| {
                            format!(
                                "{}. {} — {} ({}:{})",
                                i + 1,
                                n.name,
                                n.kind,
                                n.path,
                                n.size_lines
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    }),
                )
            }
            "get-node" => {
                let node_id = args["node_id"].as_str().unwrap_or("");
                if node_id.is_empty() {
                    return self.error(id, -32602, "Missing 'node_id' parameter");
                }
                match graphrag.get_node(node_id) {
                    Some(node) => self.result(id, serde_json::json!({
                        "content": [{"type": "text", "text": format!(
                            "Node: {} ({})\n  Path: {}\n  Language: {}\n  Symbols: {}\n  Imports: {}\n  Size: {} lines",
                            node.name, node.kind, node.path, node.language,
                            node.symbols.join(", "),
                            node.imports.join(", "),
                            node.size_lines
                        )}],
                        "isError": false
                    })),
                    None => self.result(id, serde_json::json!({
                        "content": [{"type": "text", "text": format!("Node '{}' not found", node_id)}],
                        "isError": false
                    })),
                }
            }
            "get-relationships" => {
                let node_id = args["node_id"].as_str().unwrap_or("");
                if node_id.is_empty() {
                    return self.error(id, -32602, "Missing 'node_id' parameter");
                }
                let rels = graphrag.get_relationships(node_id);
                let text = if rels.is_empty() {
                    format!("No relationships for {}", node_id)
                } else {
                    rels.iter()
                        .map(|r| {
                            format!(
                                "{} --[{}]--> {} (w={})",
                                r.source_id,
                                r.relation_type.label(),
                                r.target_id,
                                r.weight
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    }),
                )
            }
            "find-path" => {
                let from = args["from"].as_str().unwrap_or("");
                let to = args["to"].as_str().unwrap_or("");
                if from.is_empty() || to.is_empty() {
                    return self.error(id, -32602, "Missing 'from' or 'to' parameter");
                }
                let path = graphrag.find_path(from, to);
                let text = if path.is_empty() {
                    format!("No path between '{}' and '{}'", from, to)
                } else {
                    path.join(" -> ")
                };
                self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    }),
                )
            }
            _ => self.error(
                id,
                -32601,
                format!("Unknown graphrag operation: {}", operation),
            ),
        }
    }

    async fn handle_view_signatures(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        let path = args["path"].as_str().unwrap_or("");

        match self.indexer.view_signatures(path).await {
            Ok(sigs) => {
                let text = if sigs.is_empty() {
                    format!("No signatures found in {}", path)
                } else {
                    sigs.join("\n")
                };
                self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    }),
                )
            }
            Err(e) => self.error(id, -32603, format!("Error: {}", e)),
        }
    }

    async fn handle_verify(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        let root = match std::env::current_dir() {
            Ok(r) => r,
            Err(e) => return self.error(id, -32603, format!("cwd error: {}", e)),
        };
        let level = args["level"].as_str().unwrap_or("all");
        match crate::grace::GraceEngine::verify_project(&root).await {
            Ok(results) => {
                let filtered: Vec<_> = if level == "all" {
                    results
                } else {
                    results.into_iter().filter(|r| r.level == level).collect()
                };
                let mut text = String::new();
                let mut failures = Vec::new();
                for r in &filtered {
                    let status = if r.passed { "PASS" } else { "FAIL" };
                    text.push_str(&format!("\n[{}] {}\n", status, r.level));
                    for c in &r.checks {
                        let mark = if c.passed { "✓" } else { "✗" };
                        text.push_str(&format!("  {} {} — {}\n", mark, c.name, c.details));
                        if !c.passed {
                            failures.push(FailurePacket {
                                check: c.name.clone(),
                                details: c.details.clone(),
                                suggested: suggest_fix(&c.name),
                            });
                        }
                    }
                }
                // Append failure packets if any
                if !failures.is_empty() {
                    text.push_str("\n--- FAILURE PACKETS ---\n");
                    for (i, fp) in failures.iter().enumerate() {
                        text.push_str(&format!(
                            "\n{}. {} FAILED\n   Observed: {}\n   Suggested: {}\n",
                            i + 1,
                            fp.check,
                            fp.details,
                            fp.suggested
                        ));
                    }
                }
                self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    }),
                )
            }
            Err(e) => self.error(id, -32603, format!("Verify error: {}", e)),
        }
    }

    async fn handle_review(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        let root = match std::env::current_dir() {
            Ok(r) => r,
            Err(e) => return self.error(id, -32603, format!("cwd error: {}", e)),
        };
        let mode = args["mode"].as_str().unwrap_or("scoped");
        match crate::grace::review::Reviewer::review(&root, mode) {
            Ok(report) => {
                let mut text = format!("=== GRACE Review ({}) ===\n", report.mode);
                for s in &report.sections {
                    let status = if s.passed { "✓" } else { "✗" };
                    text.push_str(&format!("{} {} — {}\n", status, s.name, s.details));
                    for issue in &s.issues {
                        text.push_str(&format!("  ⚠ {}\n", issue));
                    }
                }
                if report.passed {
                    text.push_str("\nAll checks passed.");
                }
                self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    }),
                )
            }
            Err(e) => self.error(id, -32603, format!("Review error: {}", e)),
        }
    }

    async fn handle_status(
        &self,
        id: Option<serde_json::Value>,
        _args: &serde_json::Value,
    ) -> serde_json::Value {
        let root = match std::env::current_dir() {
            Ok(r) => r,
            Err(e) => return self.error(id, -32603, format!("cwd error: {}", e)),
        };
        match crate::grace::status::StatusCollector::collect(&root).await {
            Ok(report) => {
                let text = serde_json::to_string_pretty(&report).unwrap_or_default();
                self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    }),
                )
            }
            Err(e) => self.error(id, -32603, format!("Status error: {}", e)),
        }
    }

    async fn handle_gain(
        &self,
        id: Option<serde_json::Value>,
        _args: &serde_json::Value,
    ) -> serde_json::Value {
        let config = crate::config::Config::load().unwrap_or_default();
        let tracker = crate::tracking::Tracker::new(&config);
        match tracker.get_stats().await {
            Ok(stats) => {
                let mut text = String::new();
                text.push_str(&format!("Commands tracked:  {}\n", stats.total_commands));
                text.push_str(&format!(
                    "Input tokens:      {}\n",
                    stats.total_input_tokens
                ));
                text.push_str(&format!(
                    "Output tokens:     {}\n",
                    stats.total_output_tokens
                ));
                text.push_str(&format!(
                    "Tokens saved:      {}\n",
                    stats.total_saved_tokens
                ));
                text.push_str(&format!(
                    "Avg savings:       {:.1}%\n",
                    stats.avg_savings_pct
                ));
                if stats.total_commands > 0 {
                    let est = stats.total_saved_tokens as f64 * 0.000003;
                    text.push_str(&format!("Est. cost saved:   ${:.4}\n", est));
                }
                self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    }),
                )
            }
            Err(e) => self.error(id, -32603, format!("Gain error: {}", e)),
        }
    }

    async fn handle_compress(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        let text = match args["text"].as_str() {
            Some(t) if !t.is_empty() => t.to_string(),
            _ => return self.error(id, -32602, "Missing 'text' parameter"),
        };
        let level = args["level"].as_str().unwrap_or("full");
        let config = crate::config::Config::load().unwrap_or_default();
        let mut cfg = config;
        cfg.compress.output_level = level.to_string();
        let compressor = crate::compress::Compressor::new(&cfg);
        let result = compressor.compress_output(&text);
        let saved = text.len().saturating_sub(result.len());
        let pct = if text.is_empty() {
            0
        } else {
            saved * 100 / text.len()
        };
        let info = format!(
            "({} → {} chars, {}% saved)\n\n{}",
            text.len(),
            result.len(),
            pct,
            result
        );
        self.result(
            id,
            serde_json::json!({
                "content": [{"type": "text", "text": info}],
                "isError": false
            }),
        )
    }

    async fn handle_refresh(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        let root = match std::env::current_dir() {
            Ok(r) => r,
            Err(e) => return self.error(id, -32603, format!("cwd error: {}", e)),
        };
        let fix = args["fix"].as_bool().unwrap_or(false);
        let result = if fix {
            crate::grace::refresh::Refresher::fix(&root)
        } else {
            crate::grace::refresh::Refresher::refresh(&root)
        };
        match result {
            Ok(report) => {
                let text = serde_json::to_string_pretty(&report).unwrap_or_default();
                self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    }),
                )
            }
            Err(e) => self.error(id, -32603, format!("Refresh error: {}", e)),
        }
    }

    async fn handle_suggest_contract(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        let name = args["module_name"].as_str().unwrap_or("module");
        let purpose = args["purpose"].as_str().unwrap_or("TBD");
        let lang = args["language"].as_str().unwrap_or("rust");
        let comment = match lang {
            "py" => "#",
            _ => "//",
        };
        let module_id = format!("M-{}", name.to_uppercase().replace(' ', "_"));
        let text = format!(
            "{c} MODULE_CONTRACT\n{c} MODULE_ID: {id}\n{c} PURPOSE: {p}\n{c} SCOPE: {n}\n{c} DEPENDS:\n{c} LINKS:\n\n{c} START_MODULE_MAP\n{c} END_MODULE_MAP\n\n{c} START_CHANGE_SUMMARY\n{c} LAST_CHANGE: [v1.0.0 — Initial implementation]\n{c} END_CHANGE_SUMMARY\n\n{c} START_public_api\n{c} END_public_api",
            c = comment, id = module_id, p = purpose, n = name
        );
        self.result(
            id,
            serde_json::json!({
                "content": [{"type": "text", "text": text}],
                "isError": false
            }),
        )
    }

    async fn handle_lsp_hover(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        let file = args["file"].as_str().unwrap_or("");
        let line = args["line"].as_u64().unwrap_or(0) as u32;
        let col = args["column"].as_u64().unwrap_or(0) as u32;
        match crate::mcp::lsp::LspClient::new().hover(file, line, col) {
            Ok(h) => self.result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": h.contents}],
                    "isError": false
                }),
            ),
            Err(e) => self.error(id, -32603, format!("LSP hover: {}", e)),
        }
    }

    async fn handle_lsp_references(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        let file = args["file"].as_str().unwrap_or("");
        let line = args["line"].as_u64().unwrap_or(0) as u32;
        let col = args["column"].as_u64().unwrap_or(0) as u32;
        match crate::mcp::lsp::LspClient::new().references(file, line, col) {
            Ok(refs) => {
                let text = if refs.is_empty() {
                    "No references found".into()
                } else {
                    refs.iter()
                        .map(|r| format!("{} ({} ranges)", r.uri, r.ranges.len()))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                self.result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    }),
                )
            }
            Err(e) => self.error(id, -32603, format!("LSP references: {}", e)),
        }
    }

    async fn handle_grace_skill(
        &self,
        id: Option<serde_json::Value>,
        name: &str,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        match self
            .skill_engine
            .execute(SkillRequest {
                name: name.to_string(),
                arguments: args.clone(),
            })
            .await
        {
            Ok(result) => self.result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": format!("{}\n\n{}", result.title, result.body)}],
                    "isError": false
                }),
            ),
            Err(e) => self.error(id, -32603, format!("Skill error: {}", e)),
        }
    }

    fn tool_definitions(&self) -> Vec<serde_json::Value> {
        let mut tools = vec![
            serde_json::json!({
                "name": "semantic_search",
                "description": "Search codebase by natural language. Returns code blocks with paths and line numbers.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query" },
                        "max_results": { "type": "number", "default": 10 }
                    },
                    "required": ["query"]
                }
            }),
            serde_json::json!({
                "name": "view_signatures",
                "description": "View function and class signatures in a file.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "File path" }
                    },
                    "required": ["path"]
                }
            }),
            serde_json::json!({
                "name": "graphrag_query",
                "description": "Query the code knowledge graph. Supports: search, get-node, get-relationships, find-path, overview",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "operation": { "type": "string", "description": "Operation: search | get-node | get-relationships | find-path | overview" },
                        "query": { "type": "string", "description": "Search query" },
                        "node_id": { "type": "string", "description": "Node ID" },
                        "from": { "type": "string", "description": "Source node ID" },
                        "to": { "type": "string", "description": "Target node ID" }
                    },
                    "required": ["operation"]
                }
            }),
            serde_json::json!({
                "name": "verify_project",
                "description": "Run GRACE verification checks (module-local, wave, phase). Returns pass/fail status with details.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "level": { "type": "string", "description": "Verification level: module-local | wave | phase | all" }
                    }
                }
            }),
            serde_json::json!({
                "name": "review_code",
                "description": "Run GRACE integrity review — checks semantic markup, contracts, naming, secrets. Returns issues list.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "mode": { "type": "string", "description": "Review mode: scoped | full" }
                    }
                }
            }),
            serde_json::json!({
                "name": "project_status",
                "description": "Full project health report — contracts, semantic markup, verification, token economy, system info.",
                "inputSchema": { "type": "object", "properties": {} }
            }),
            serde_json::json!({
                "name": "token_savings",
                "description": "View token savings analytics — total commands, tokens saved, average savings %, estimated cost saved.",
                "inputSchema": { "type": "object", "properties": {} }
            }),
            serde_json::json!({
                "name": "compress_text",
                "description": "Compress text for AI context efficiency. Levels: lite (remove filler), full (also pleasantries), ultra (telegraphic).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "description": "Text to compress" },
                        "level": { "type": "string", "description": "Compression level: lite | full | ultra" }
                    },
                    "required": ["text"]
                }
            }),
            serde_json::json!({
                "name": "refresh_project",
                "description": "Report or fix canonical MyGRACE drift between code contracts and sharded artifacts.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "fix": { "type": "boolean", "description": "When true, rewrite canonical indexes and shards from source MODULE_ID contracts" }
                    }
                }
            }),
            serde_json::json!({
                "name": "suggest_contract",
                "description": "Generate a MODULE_CONTRACT template for a new module. Provide module name and purpose.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "module_name": { "type": "string", "description": "Module name (e.g. 'auth')" },
                        "purpose": { "type": "string", "description": "What the module does" },
                        "language": { "type": "string", "description": "Language: rust | python | ts | go" }
                    },
                    "required": ["module_name", "purpose"]
                }
            }),
            serde_json::json!({
                "name": "lsp_hover",
                "description": "Get type/signature information at a position via LSP.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file": { "type": "string", "description": "File path" },
                        "line": { "type": "number", "description": "Line (1-based)" },
                        "column": { "type": "number", "description": "Column (0-based)" }
                    },
                    "required": ["file", "line", "column"]
                }
            }),
            serde_json::json!({
                "name": "lsp_references",
                "description": "Find all references to a symbol at a position via LSP.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file": { "type": "string", "description": "File path" },
                        "line": { "type": "number", "description": "Line (1-based)" },
                        "column": { "type": "number", "description": "Column (0-based)" }
                    },
                    "required": ["file", "line", "column"]
                }
            }),
        ];

        for skill in SKILL_DEFS {
            tools.push(serde_json::json!({
                "name": skill.name,
                "description": skill.description,
                "inputSchema": skill.input_schema(),
            }));
        }

        tools
    }

    fn result(
        &self,
        id: Option<serde_json::Value>,
        result: serde_json::Value,
    ) -> serde_json::Value {
        let mut resp = serde_json::json!({
            "jsonrpc": "2.0",
            "result": result
        });
        if let Some(ref id_val) = id {
            resp["id"] = id_val.clone();
        }
        resp
    }

    fn error(
        &self,
        id: Option<serde_json::Value>,
        code: i32,
        message: impl Display,
    ) -> serde_json::Value {
        let mut resp = serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": code,
                "message": message.to_string()
            }
        });
        if let Some(ref id_val) = id {
            resp["id"] = id_val.clone();
        }
        resp
    }
}

struct FailurePacket {
    check: String,
    details: String,
    suggested: String,
}

fn suggest_fix(check: &str) -> String {
    match check {
        "contract-exists" => "Add MODULE_CONTRACT headers to source files".into(),
        "contract-valid" => "Add PURPOSE field to MODULE_CONTRACT".into(),
        "module-map" => "Add START_MODULE_MAP / END_MODULE_MAP".into(),
        "change-summary" => "Add START_CHANGE_SUMMARY / END_CHANGE_SUMMARY".into(),
        "function-contracts" => {
            "Add START_CONTRACT_name blocks with PURPOSE, INPUTS, OUTPUTS".into()
        }
        "semantic-blocks" => "Close all START_/END_ pairs".into(),
        "unique-block-names" => "Rename duplicate blocks to be unique per file".into(),
        "500-token-rule" => "Split large files into smaller units".into(),
        "no-todos" => "Resolve TODO/FIXME or convert to tracked issues".into(),
        "file-size-limit" => "Split files exceeding 500 lines into modules".into(),
        "trace-assertions" => "Add [Module][function][BLOCK_NAME] log markers".into(),
        _ => "Review the check details and fix the reported issue".into(),
    }
}
// END_public_api
