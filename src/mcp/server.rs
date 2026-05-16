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
                        "protocolVersion": "2025-11-05",
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
                                    "query": {
                                        "type": "string",
                                        "description": "Search query (for search operation)"
                                    },
                                    "node_id": {
                                        "type": "string",
                                        "description": "Node ID (for get-node, get-relationships operations)"
                                    },
                                    "from": {
                                        "type": "string",
                                        "description": "Source node ID (for find-path)"
                                    },
                                    "to": {
                                        "type": "string",
                                        "description": "Target node ID (for find-path)"
                                    }
                                },
                                "required": ["operation"]
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
