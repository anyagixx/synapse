use crate::config::Config;
use crate::graphrag::GraphRag;
use crate::indexer::Indexer;
use std::fmt::Display;
use std::sync::Mutex;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct McpServer;

impl McpServer {
    pub fn new(_config: Config) -> Self {
        Self
    }

    pub async fn start_stdio(self) -> anyhow::Result<()> {
        tracing::info!("MCP server running on stdio");
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

    pub async fn start_http(self, bind: &str) -> anyhow::Result<()> {
        tracing::info!(
            "MCP HTTP not yet implemented. Use: socat tcp-l:{} exec:syn mcp",
            bind
        );
        self.start_stdio().await
    }
}

pub struct SynapseHandler {
    indexer: Indexer,
    graphrag: Mutex<Option<GraphRag>>,
    initialized: bool,
}

impl Default for SynapseHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SynapseHandler {
    pub fn new() -> Self {
        let config = Config::load().unwrap_or_default();
        let indexer = Indexer::new(&config);
        let mut graphrag = GraphRag::new();
        // Try to find index from current directory
        if let Ok(cwd) = std::env::current_dir() {
            let mut guard = indexer.storage.lock().unwrap();
            if guard.is_none() {
                *guard = Some(crate::indexer::storage::Storage::new(&cwd));
            }
            // Build graph from storage
            graphrag.build(&cwd).ok();
        }
        Self {
            indexer,
            graphrag: Mutex::new(Some(graphrag)),
            initialized: false,
        }
    }

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
                self.result(id, serde_json::json!({
                    "tools": [
                        {
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
                        },
                        {
                            "name": "view_signatures",
                            "description": "View function and class signatures in a file.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "File path" }
                                },
                                "required": ["path"]
                            }
                        },
                        {
                            "name": "graphrag_query",
                            "description": "Query the code knowledge graph. Supports: search, get-node, get-relationships, find-path, overview",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "operation": {
                                        "type": "string",
                                        "description": "Operation: search | get-node | get-relationships | find-path | overview"
                                    },
                                    "query": { "type": "string", "description": "Search query" },
                                    "node_id": { "type": "string", "description": "Node ID" },
                                    "from": { "type": "string", "description": "Source node ID" },
                                    "to": { "type": "string", "description": "Target node ID" }
                                },
                                "required": ["operation"]
                            }
                        },
                        {
                            "name": "verify_project",
                            "description": "Run GRACE verification checks (module-local, wave, phase). Returns pass/fail status with details.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "level": {
                                        "type": "string",
                                        "description": "Verification level: module-local | wave | phase | all"
                                    }
                                }
                            }
                        },
                        {
                            "name": "review_code",
                            "description": "Run GRACE integrity review — checks semantic markup, contracts, naming, secrets. Returns issues list.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "mode": {
                                        "type": "string",
                                        "description": "Review mode: scoped | full"
                                    }
                                }
                            }
                        },
                        {
                            "name": "project_status",
                            "description": "Full project health report — contracts, semantic markup, verification, token economy, system info.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "token_savings",
                            "description": "View token savings analytics — total commands, tokens saved, average savings %, estimated cost saved.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "compress_text",
                            "description": "Compress text for AI context efficiency. Levels: lite (remove filler), full (also pleasantries), ultra (telegraphic).",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "text": { "type": "string", "description": "Text to compress" },
                                    "level": {
                                        "type": "string",
                                        "description": "Compression level: lite | full | ultra"
                                    }
                                },
                                "required": ["text"]
                            }
                        }
                    ]
                }))
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
        let guard = self.graphrag.lock().unwrap();
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
                let text = if filtered.is_empty() {
                    "No verification results".to_string()
                } else {
                    let mut out = String::new();
                    for r in &filtered {
                        let status = if r.passed { "PASS" } else { "FAIL" };
                        out.push_str(&format!("\n[{}] {}\n", status, r.level));
                        for c in &r.checks {
                            let mark = if c.passed { "✓" } else { "✗" };
                            out.push_str(&format!("  {} {} — {}\n", mark, c.name, c.details));
                        }
                    }
                    out
                };
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
