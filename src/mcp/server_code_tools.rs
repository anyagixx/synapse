// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-CODE-TOOLS
// PURPOSE: MCP handlers for code search, GraphRAG typed queries, signature views, and guarded LSP lookups
// SCOPE: semantic_search, graphrag_query with indexed GraphRAG cache key validation and type filters, GraphRAG lock health, view_signatures, config-aware lsp_hover, lsp_references handlers
// DEPENDS: M-CONFIG, M-GRACE-CONTRACT, M-INDEXER, M-GRAPHRAG, M-MCP-LSP, M-MCP-SERVER-RESPONSE, M-UTILS
// LINKS: docs/modules/M-MCP-SERVER.xml

// START_MODULE_MAP
// handle_search — Runs indexed semantic search and formats MCP text content
// handle_graphrag — Runs graph overview/search/node/path/type-filtered relationship operations
// ensure_graphrag — Builds or reuses GraphRAG state based on root/index cache key
// read_graphrag — Reads GraphRAG state without panicking on poisoned locks
// handle_view_signatures — Returns indexed signatures for a file
// handle_lsp_hover — Returns LSP hover contents using configured language servers
// handle_lsp_references — Returns LSP reference locations summary using configured language servers
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.4.0 — Added root/index-keyed GraphRAG cache validation]
// END_CHANGE_SUMMARY

use super::server::GraphCacheKey;
use super::server_response::{error, result};
use crate::config::Config;
use crate::grace::contract::LinkType;
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
                        let preview = crate::utils::truncate_chars(&r.content, 150);
                        format!(
                            "{}. {} ({}:{}-{})\n   {}\n   reason: {}",
                            i + 1,
                            r.path,
                            r.language,
                            r.start_line,
                            r.end_line,
                            preview,
                            r.explanation
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
// PURPOSE: Execute GraphRAG overview, search, node lookup, relationship lookup, path, dependents, or tracedown query
// INPUTS: { graphrag: &RwLock<Option<GraphRag>> }, { graph_cache_key: &RwLock<Option<GraphCacheKey>> }, { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_graphrag
pub(crate) fn handle_graphrag(
    graphrag: &RwLock<Option<GraphRag>>,
    graph_cache_key: &RwLock<Option<GraphCacheKey>>,
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let operation = args["operation"].as_str().unwrap_or("search");
    if let Err(message) = ensure_graphrag(graphrag, graph_cache_key) {
        return error(id, -32603, message);
    }
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
                        "Graph Overview:\n  Nodes: {}\n  Relationships: {}\n  Typed LINKS: {}\n  Types:\n    {}",
                        ov.total_nodes, ov.total_relationships, ov.typed_relationships, ov.node_types.join("\n    ")
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
                        let rels = graphrag.get_relationships(&n.id);
                        let graph_summary = if rels.is_empty() {
                            String::new()
                        } else {
                            let summary = rels
                                .iter()
                                .take(4)
                                .map(|rel| {
                                    format!("{}→{}", rel.relation_type.label(), rel.target_id)
                                })
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!("\n   graph: {}", summary)
                        };
                        format!(
                            "{}. {} — {} ({}:{}){}",
                            i + 1,
                            n.name,
                            n.kind,
                            n.path,
                            n.size_lines,
                            graph_summary
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
            let link_type = match parse_link_type_arg(args) {
                Ok(link_type) => link_type,
                Err(message) => return error(id, -32602, message),
            };
            if link_type.is_some() {
                let rels = graphrag.get_typed_relationships(node_id, link_type);
                let text = if rels.is_empty() {
                    format!("No typed relationships for {}", node_id)
                } else {
                    rels.iter()
                        .map(|r| format_typed_relationship(r))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                return result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    }),
                );
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
            let link_type = match parse_link_type_arg(args) {
                Ok(link_type) => link_type,
                Err(message) => return error(id, -32602, message),
            };
            let path = if link_type.is_some() {
                graphrag.find_path_by_link_type(from, to, link_type)
            } else {
                graphrag.find_path(from, to)
            };
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
        "dependents" => {
            let target = args["target"]
                .as_str()
                .or_else(|| args["node_id"].as_str())
                .unwrap_or("");
            if target.is_empty() {
                return error(id, -32602, "Missing 'target' or 'node_id' parameter");
            }
            let link_type = match parse_link_type_arg(args) {
                Ok(link_type) => link_type.or(Some(LinkType::Depends)),
                Err(message) => return error(id, -32602, message),
            };
            let rels = graphrag.get_incoming_typed_relationships(target, link_type);
            let text = if rels.is_empty() {
                format!("No dependents for {}", target)
            } else {
                rels.iter()
                    .map(|r| format_typed_relationship(r))
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
        "tracedown" => {
            let target = args["target"]
                .as_str()
                .or_else(|| args["node_id"].as_str())
                .unwrap_or("");
            if target.is_empty() {
                return error(id, -32602, "Missing 'target' or 'node_id' parameter");
            }
            let rels = graphrag.get_incoming_typed_relationships(target, None);
            let text = if rels.is_empty() {
                format!("No typed LINKS trace down from {}", target)
            } else {
                rels.iter()
                    .filter(|rel| {
                        matches!(
                            rel.link_type,
                            LinkType::Implements | LinkType::TracesTo | LinkType::VerifiedBy
                        )
                    })
                    .map(|r| format_typed_relationship(r))
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
        _ => error(
            id,
            -32601,
            format!("Unknown graphrag operation: {}", operation),
        ),
    }
}
// END_handle_graphrag

// START_CONTRACT_ensure_graphrag
// PURPOSE: Build or reuse GraphRAG state using a root/index cache key
// INPUTS: { graphrag: &RwLock<Option<GraphRag>> }, { graph_cache_key: &RwLock<Option<GraphCacheKey>> }
// OUTPUTS: { Result<(), String> }
// SIDE_EFFECTS: reads current project source and populates GraphRAG state
// START_ensure_graphrag
fn ensure_graphrag(
    graphrag: &RwLock<Option<GraphRag>>,
    graph_cache_key: &RwLock<Option<GraphCacheKey>>,
) -> Result<(), String> {
    let root = std::env::current_dir()
        .map_err(|e| format!("GraphRAG cannot resolve current directory: {}", e))?;
    let current_key = GraphCacheKey::for_root(&root);
    let cached = {
        let graph_guard = graphrag
            .read()
            .map_err(|_| "GraphRAG lock unavailable; restart MCP server".to_string())?;
        let key_guard = graph_cache_key
            .read()
            .map_err(|_| "GraphRAG cache key lock unavailable; restart MCP server".to_string())?;
        graph_guard.is_some() && key_guard.as_ref() == Some(&current_key)
    };
    if cached {
        return Ok(());
    }

    let mut built = GraphRag::new();
    built
        .build(&root)
        .map_err(|e| format!("build GraphRAG for {}: {}", root.display(), e))?;

    let mut guard = graphrag
        .write()
        .map_err(|_| "GraphRAG lock unavailable; restart MCP server".to_string())?;
    let mut key_guard = graph_cache_key
        .write()
        .map_err(|_| "GraphRAG cache key lock unavailable; restart MCP server".to_string())?;
    *guard = Some(built);
    *key_guard = Some(current_key);
    Ok(())
}
// END_ensure_graphrag

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

fn parse_link_type_arg(args: &serde_json::Value) -> Result<Option<LinkType>, String> {
    let Some(raw) = args["link_type"]
        .as_str()
        .or_else(|| args["type"].as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    LinkType::parse(raw)
        .map(Some)
        .ok_or_else(|| format!("Invalid link_type '{}'", raw))
}

fn format_typed_relationship(rel: &crate::graphrag::types::TypedCodeRelationship) -> String {
    let direction = rel.direction.label();
    let description = rel
        .description
        .as_ref()
        .map(|value| format!(" — {}", value))
        .unwrap_or_default();
    format!(
        "{} --[{}:{}]--> {} (w={}){}",
        rel.source_id,
        rel.link_type.label(),
        direction,
        rel.target_id,
        rel.weight,
        description
    )
}

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
    config: &Config,
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let file = args["file"].as_str().unwrap_or("");
    let line = args["line"].as_u64().unwrap_or(0) as u32;
    let col = args["column"].as_u64().unwrap_or(0) as u32;
    match crate::mcp::lsp::LspClient::new(config).hover(file, line, col) {
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
    config: &Config,
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let file = args["file"].as_str().unwrap_or("");
    let line = args["line"].as_u64().unwrap_or(0) as u32;
    let col = args["column"].as_u64().unwrap_or(0) as u32;
    match crate::mcp::lsp::LspClient::new(config).references(file, line, col) {
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
            &RwLock::new(None),
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
