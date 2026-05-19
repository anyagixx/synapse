// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-CODE-TOOLS
// PURPOSE: MCP handlers for code search, GraphRAG queries, signature views, and guarded LSP lookups
// SCOPE: semantic_search, graphrag_query, GraphRAG lock health, view_signatures, lsp_hover, lsp_references handlers
// DEPENDS: M-INDEXER, M-GRAPHRAG, M-MCP-LSP, M-MCP-SERVER-RESPONSE
// LINKS: docs/modules/M-MCP-SERVER.xml

// START_MODULE_MAP
// handle_search — Runs indexed semantic search and formats MCP text content
// handle_graphrag — Runs graph overview/search/node/path operations
// read_graphrag — Reads GraphRAG state without panicking on poisoned locks
// handle_view_signatures — Returns indexed signatures for a file
// handle_lsp_hover — Returns LSP hover contents
// handle_lsp_references — Returns LSP reference locations summary
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.9.0 — Guarded GraphRAG lock errors in MCP code tools]
// END_CHANGE_SUMMARY

use super::server_response::{error, result};
use crate::graphrag::GraphRag;
use crate::indexer::Indexer;
use std::sync::{RwLock, RwLockReadGuard};

// START_public_api

// START_CONTRACT_handle_search
// PURPOSE: Execute semantic_search and return formatted code block matches
// INPUTS: { indexer: &Indexer }, { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_search
pub(crate) async fn handle_search(
    indexer: &Indexer,
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let query = args["query"].as_str().unwrap_or("");
    let max = args["max_results"].as_u64().unwrap_or(10) as usize;

    match indexer.search(query, max).await {
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
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Search error: {}", e)),
    }
}
// END_handle_search

// START_CONTRACT_handle_graphrag
// PURPOSE: Execute GraphRAG overview, search, node lookup, relationship lookup, or path query
// INPUTS: { graphrag: &RwLock<Option<GraphRag>> }, { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_graphrag
pub(crate) fn handle_graphrag(
    graphrag: &RwLock<Option<GraphRag>>,
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let operation = args["operation"].as_str().unwrap_or("search");
    let guard = match read_graphrag(graphrag) {
        Ok(guard) => guard,
        Err(message) => return error(id, -32603, message),
    };
    let graphrag = match guard.as_ref() {
        Some(g) => g,
        None => return error(id, -32603, "GraphRAG not built. Run `syn index` first."),
    };

    match operation {
        "overview" => match graphrag.overview() {
            Some(ov) => result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": format!(
                        "Graph Overview:\n  Nodes: {}\n  Relationships: {}\n  Types:\n    {}",
                        ov.total_nodes, ov.total_relationships, ov.node_types.join("\n    ")
                    )}],
                    "isError": false
                }),
            ),
            None => result(
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
                return error(id, -32602, "Missing 'query' parameter");
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
            result(
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
                return error(id, -32602, "Missing 'node_id' parameter");
            }
            match graphrag.get_node(node_id) {
                Some(node) => result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": format!(
                            "Node: {} ({})\n  Path: {}\n  Language: {}\n  Symbols: {}\n  Imports: {}\n  Size: {} lines",
                            node.name, node.kind, node.path, node.language,
                            node.symbols.join(", "),
                            node.imports.join(", "),
                            node.size_lines
                        )}],
                        "isError": false
                    }),
                ),
                None => result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": format!("Node '{}' not found", node_id)}],
                        "isError": false
                    }),
                ),
            }
        }
        "get-relationships" => {
            let node_id = args["node_id"].as_str().unwrap_or("");
            if node_id.is_empty() {
                return error(id, -32602, "Missing 'node_id' parameter");
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
            result(
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
                return error(id, -32602, "Missing 'from' or 'to' parameter");
            }
            let path = graphrag.find_path(from, to);
            let text = if path.is_empty() {
                format!("No path between '{}' and '{}'", from, to)
            } else {
                path.join(" -> ")
            };
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        _ => error(
            id,
            -32601,
            format!("Unknown graphrag operation: {}", operation),
        ),
    }
}
// END_handle_graphrag

// START_CONTRACT_read_graphrag
// PURPOSE: Read GraphRAG state and convert poisoned-lock panics into JSON-RPC-safe errors
// INPUTS: { graphrag: &RwLock<Option<GraphRag>> }
// OUTPUTS: { Result<RwLockReadGuard<Option<GraphRag>>, String> }
// START_read_graphrag
fn read_graphrag(
    graphrag: &RwLock<Option<GraphRag>>,
) -> Result<RwLockReadGuard<'_, Option<GraphRag>>, String> {
    graphrag
        .read()
        .map_err(|_| "GraphRAG lock unavailable; restart MCP server".to_string())
}
// END_read_graphrag

// START_CONTRACT_handle_view_signatures
// PURPOSE: Execute view_signatures and format signature lines for MCP response content
// INPUTS: { indexer: &Indexer }, { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_view_signatures
pub(crate) async fn handle_view_signatures(
    indexer: &Indexer,
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let path = args["path"].as_str().unwrap_or("");

    match indexer.view_signatures(path).await {
        Ok(sigs) => {
            let text = if sigs.is_empty() {
                format!("No signatures found in {}", path)
            } else {
                sigs.join("\n")
            };
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Error: {}", e)),
    }
}
// END_handle_view_signatures

// START_CONTRACT_handle_lsp_hover
// PURPOSE: Execute lsp_hover and return hover contents as MCP text
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_lsp_hover
pub(crate) async fn handle_lsp_hover(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let file = args["file"].as_str().unwrap_or("");
    let line = args["line"].as_u64().unwrap_or(0) as u32;
    let col = args["column"].as_u64().unwrap_or(0) as u32;
    match crate::mcp::lsp::LspClient::new().hover(file, line, col) {
        Ok(h) => result(
            id,
            serde_json::json!({
                "content": [{"type": "text", "text": h.contents}],
                "isError": false
            }),
        ),
        Err(e) => error(id, -32603, format!("LSP hover: {}", e)),
    }
}
// END_handle_lsp_hover

// START_CONTRACT_handle_lsp_references
// PURPOSE: Execute lsp_references and summarize returned reference locations
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_lsp_references
pub(crate) async fn handle_lsp_references(
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
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("LSP references: {}", e)),
    }
}
// END_handle_lsp_references

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_handle_graphrag_reports_poisoned_lock
    // PURPOSE: Verify poisoned GraphRAG locks are returned as MCP JSON-RPC errors instead of panics
    // START_test_handle_graphrag_reports_poisoned_lock
    #[test]
    fn test_handle_graphrag_reports_poisoned_lock() {
        let graphrag = RwLock::new(Some(GraphRag::new()));
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let poisoned = std::panic::catch_unwind(|| {
            let _guard = graphrag.write().unwrap();
            panic!("poison GraphRAG lock");
        })
        .is_err();
        std::panic::set_hook(previous_hook);
        assert!(poisoned);

        let resp = handle_graphrag(
            &graphrag,
            Some(serde_json::json!(1)),
            &serde_json::json!({"operation": "overview"}),
        );
        assert_eq!(resp["error"]["code"], -32603);
        assert!(
            resp["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("GraphRAG lock unavailable")),
            "unexpected response: {}",
            resp
        );
    }
    // END_test_handle_graphrag_reports_poisoned_lock
}

// END_public_api
