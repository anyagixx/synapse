// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-CODE-TOOLS
// PURPOSE: MCP handlers for code search, GraphRAG typed queries, impact analysis, signature views, and guarded LSP lookups
// SCOPE: semantic_search with optional language/path filters, graphrag_query with indexed GraphRAG cache key validation, impact analysis, Mermaid output, and type filters, GraphRAG lock health, view_signatures, config-aware lsp_hover/lsp_references handlers with optional content override
// DEPENDS: M-CONFIG, M-GRACE-CONTRACT, M-INDEXER, M-GRAPHRAG, M-GRAPHRAG-IMPACT, M-GRAPHRAG-MERMAID, M-MCP-LSP, M-MCP-SERVER-RESPONSE, M-UTILS
// LINKS: docs/modules/M-MCP-SERVER.xml

// START_MODULE_MAP
// handle_search — Runs indexed semantic search with optional filters and formats MCP text content
// search_filters_from_args — Builds validated SearchFilters from MCP arguments
// handle_graphrag — Runs graph overview/search/node/path/type-filtered relationship/impact/Mermaid operations
// parse_mermaid_options_arg — Builds Mermaid render options from GraphRAG MCP arguments
// parse_impact_depth_arg — Builds bounded GraphRAG impact depth from MCP arguments
// ensure_graphrag — Builds or reuses GraphRAG state based on root/index cache key
// read_graphrag — Reads GraphRAG state without panicking on poisoned locks
// handle_view_signatures — Returns indexed signatures for a file
// handle_lsp_hover — Returns LSP hover contents using configured language servers
// handle_lsp_references — Returns LSP reference locations summary using configured language servers
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v4.1.0 — Added GraphRAG impact analysis MCP operation]
// END_CHANGE_SUMMARY

use super::server::GraphCacheKey;
use super::server_response::{error, result};
use crate::config::Config;
use crate::grace::contract::LinkType;
use crate::graphrag::{render_mermaid, GraphRag, MermaidGraphSubset, MermaidRenderOptions};
use crate::indexer::storage::SearchFilters;
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
    let filters = match search_filters_from_args(args) {
        Ok(filters) => filters,
        Err(message) => return error(id, -32602, message),
    };

    match indexer.search_with_filters(query, max, &filters).await {
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

// START_CONTRACT_search_filters_from_args
// PURPOSE: Validate optional semantic_search language and path filter arguments
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Result<SearchFilters, String> }
// START_search_filters_from_args
fn search_filters_from_args(args: &serde_json::Value) -> Result<SearchFilters, String> {
    Ok(SearchFilters::new(
        optional_string_arg(args, "language")?,
        optional_string_arg(args, "path")?,
        optional_string_arg(args, "path_contains")?,
    ))
}
// END_search_filters_from_args

// START_CONTRACT_optional_string_arg
// PURPOSE: Read an optional MCP string argument while rejecting incompatible JSON types
// INPUTS: { args: &serde_json::Value }, { key: &str }
// OUTPUTS: { Result<Option<String>, String> }
// START_optional_string_arg
fn optional_string_arg(args: &serde_json::Value, key: &str) -> Result<Option<String>, String> {
    match args.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("semantic_search `{}` filter must be a string", key)),
    }
}
// END_optional_string_arg

// START_CONTRACT_handle_graphrag
// PURPOSE: Execute GraphRAG overview, search, node lookup, relationship lookup, path, dependents, tracedown, impact, or Mermaid query
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
        "impact" => {
            let node_id = args["node_id"]
                .as_str()
                .or_else(|| args["target"].as_str())
                .unwrap_or("");
            if node_id.is_empty() {
                return error(id, -32602, "Missing 'node_id' or 'target' parameter");
            }
            let depth = match parse_impact_depth_arg(args) {
                Ok(depth) => depth,
                Err(message) => return error(id, -32602, message),
            };
            let include_tests = match parse_include_tests_arg(args) {
                Ok(include_tests) => include_tests,
                Err(message) => return error(id, -32602, message),
            };
            match graphrag.impact_analysis(node_id, depth, include_tests) {
                Some(analysis) => {
                    let text = serde_json::to_string_pretty(&analysis)
                        .unwrap_or_else(|_| format!("Impact analysis for {}", node_id));
                    result(
                        id,
                        serde_json::json!({
                            "content": [{"type": "text", "text": text}],
                            "format": "impact",
                            "impact": analysis,
                            "isError": false
                        }),
                    )
                }
                None => result(
                    id,
                    serde_json::json!({
                        "content": [{"type": "text", "text": format!("Node '{}' not found", node_id)}],
                        "format": "impact",
                        "isError": false
                    }),
                ),
            }
        }
        "mermaid" => {
            let options = match parse_mermaid_options_arg(args) {
                Ok(options) => options,
                Err(message) => return error(id, -32602, message),
            };
            let Some(graph) = graphrag.graph() else {
                return error(id, -32603, "GraphRAG not built. Run `syn index` first.");
            };
            let diagram = render_mermaid(graph, &options);
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": diagram}],
                    "format": "mermaid",
                    "subset": format!("{:?}", options.subset).to_ascii_lowercase(),
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

// START_CONTRACT_parse_impact_depth_arg
// PURPOSE: Parse bounded impact traversal depth from GraphRAG MCP arguments
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Result<usize, String> }
// LINKS:
//   -> M-GRAPHRAG-IMPACT (depends) - bounds impact traversal
//   -> NFR-002 (traces_to) - rejects incompatible MCP argument types
// START_parse_impact_depth_arg
fn parse_impact_depth_arg(args: &serde_json::Value) -> Result<usize, String> {
    match args.get("depth") {
        None | Some(serde_json::Value::Null) => Ok(2),
        Some(serde_json::Value::Number(value)) => value
            .as_u64()
            .map(|depth| (depth as usize).clamp(1, 8))
            .ok_or_else(|| "impact `depth` must be a positive integer".to_string()),
        Some(_) => Err("impact `depth` must be a positive integer".to_string()),
    }
}
// END_parse_impact_depth_arg

// START_CONTRACT_parse_include_tests_arg
// PURPOSE: Parse include_tests flag for GraphRAG impact analysis
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Result<bool, String> }
// LINKS:
//   -> M-GRAPHRAG-IMPACT (depends) - controls verification/test target inclusion
// START_parse_include_tests_arg
fn parse_include_tests_arg(args: &serde_json::Value) -> Result<bool, String> {
    match args.get("include_tests") {
        None | Some(serde_json::Value::Null) => Ok(true),
        Some(serde_json::Value::Bool(value)) => Ok(*value),
        Some(_) => Err("impact `include_tests` must be a boolean".to_string()),
    }
}
// END_parse_include_tests_arg

// START_CONTRACT_parse_mermaid_options_arg
// PURPOSE: Build Mermaid render options from GraphRAG MCP arguments
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Result<MermaidRenderOptions, String> }
// LINKS:
//   -> M-GRAPHRAG-MERMAID (depends) - configures Mermaid graph subset output
//   -> UC-001 (implements) - agents request Mermaid graph views through MCP
// START_parse_mermaid_options_arg
fn parse_mermaid_options_arg(args: &serde_json::Value) -> Result<MermaidRenderOptions, String> {
    Ok(MermaidRenderOptions {
        subset: parse_mermaid_subset_arg(args)?,
        focus_ids: parse_mermaid_focus_ids_arg(args)?,
        max_nodes: args["max_nodes"].as_u64().unwrap_or(80) as usize,
    }
    .normalized())
}
// END_parse_mermaid_options_arg

// START_CONTRACT_parse_mermaid_subset_arg
// PURPOSE: Parse Mermaid subset argument for GraphRAG MCP output
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Result<MermaidGraphSubset, String> }
// LINKS:
//   -> M-GRAPHRAG-MERMAID (depends) - selects Mermaid graph subset mode
//   -> UC-001 (implements) - agents can choose module, symbol, or relation views
// START_parse_mermaid_subset_arg
fn parse_mermaid_subset_arg(args: &serde_json::Value) -> Result<MermaidGraphSubset, String> {
    match args["subset"]
        .as_str()
        .unwrap_or("relations")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "module" | "modules" => Ok(MermaidGraphSubset::Modules),
        "symbol" | "symbols" => Ok(MermaidGraphSubset::Symbols),
        "relation" | "relations" | "graph" => Ok(MermaidGraphSubset::Relations),
        other => Err(format!(
            "Invalid mermaid subset '{}'. Expected modules, symbols, or relations",
            other
        )),
    }
}
// END_parse_mermaid_subset_arg

// START_CONTRACT_parse_mermaid_focus_ids_arg
// PURPOSE: Parse optional Mermaid focus ids from string or array GraphRAG MCP arguments
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Result<Vec<String>, String> }
// LINKS:
//   -> M-GRAPHRAG-MERMAID (depends) - focuses Mermaid graph neighborhoods
//   -> UC-001 (implements) - agents can request bounded focused diagrams
// START_parse_mermaid_focus_ids_arg
fn parse_mermaid_focus_ids_arg(args: &serde_json::Value) -> Result<Vec<String>, String> {
    if let Some(raw) = args["focus_ids"].as_array() {
        return raw
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(|text| text.trim().to_string())
                    .filter(|text| !text.is_empty())
                    .ok_or_else(|| "focus_ids entries must be non-empty strings".to_string())
            })
            .collect();
    }
    let focus = args["focus"]
        .as_str()
        .or_else(|| args["focus_id"].as_str())
        .or_else(|| args["node_id"].as_str())
        .unwrap_or("")
        .trim();
    if focus.is_empty() {
        Ok(Vec::new())
    } else {
        Ok(vec![focus.to_string()])
    }
}
// END_parse_mermaid_focus_ids_arg

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
// LINKS:
//   -> M-MCP-LSP (depends) - uses content override aware hover
//   -> NFR-002 (traces_to) - avoids stale LSP reads after edits
// START_handle_lsp_hover
pub(crate) async fn handle_lsp_hover(
    config: &Config,
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let file = args["file"].as_str().unwrap_or("");
    let line = args["line"].as_u64().unwrap_or(0) as u32;
    let col = args["column"].as_u64().unwrap_or(0) as u32;
    let content = args["content"].as_str();
    match crate::mcp::lsp::LspClient::new(config).hover_with_content(file, line, col, content) {
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
// LINKS:
//   -> M-MCP-LSP (depends) - uses content override aware references
//   -> NFR-002 (traces_to) - avoids stale LSP reads after edits
// START_handle_lsp_references
pub(crate) async fn handle_lsp_references(
    config: &Config,
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let file = args["file"].as_str().unwrap_or("");
    let line = args["line"].as_u64().unwrap_or(0) as u32;
    let col = args["column"].as_u64().unwrap_or(0) as u32;
    let content = args["content"].as_str();
    match crate::mcp::lsp::LspClient::new(config).references_with_content(file, line, col, content)
    {
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

    // START_CONTRACT_test_semantic_search_accepts_filters
    // PURPOSE: Verify semantic_search accepts language/path filters and returns only in-scope indexed blocks
    // START_test_semantic_search_accepts_filters
    #[tokio::test]
    async fn test_semantic_search_accepts_filters() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        let tests = dir.path().join("tests");
        std::fs::create_dir_all(&src).expect("create src");
        std::fs::create_dir_all(&tests).expect("create tests");
        std::fs::write(src.join("workflow.rs"), "pub fn shared_login() {}\n").expect("write src");
        std::fs::write(
            tests.join("workflow_test.rs"),
            "pub fn shared_login_test() {}\n",
        )
        .expect("write tests");

        let indexer = Indexer::new(&Config::default());
        indexer.index_directory(dir.path()).await.expect("index");
        let response = handle_search(
            &indexer,
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "query": "shared_login",
                "max_results": 10,
                "path": "src"
            }),
        )
        .await;
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("response text");

        assert!(text.contains("src/workflow.rs"), "unexpected text: {text}");
        assert!(
            !text.contains("tests/workflow_test.rs"),
            "unexpected text: {text}"
        );
    }
    // END_test_semantic_search_accepts_filters

    // START_CONTRACT_test_semantic_search_rejects_invalid_filter_type
    // PURPOSE: Verify semantic_search reports invalid params for non-string filter values
    // START_test_semantic_search_rejects_invalid_filter_type
    #[tokio::test]
    async fn test_semantic_search_rejects_invalid_filter_type() {
        let indexer = Indexer::new(&Config::default());
        let response = handle_search(
            &indexer,
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "query": "shared_login",
                "language": ["rust"]
            }),
        )
        .await;

        assert_eq!(response["error"]["code"], -32602);
    }
    // END_test_semantic_search_rejects_invalid_filter_type

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

    // START_CONTRACT_write_graphrag_fixture
    // PURPOSE: Write a minimal GRACE-contracted source fixture for GraphRAG MCP tests.
    // START_write_graphrag_fixture
    fn write_graphrag_fixture(
        src: &std::path::Path,
        file: &str,
        module_id: &str,
        purpose: &str,
        depends: &str,
        links: &[&str],
    ) {
        let link_lines = links
            .iter()
            .map(|link| format!("//   {}\n", link))
            .collect::<String>();
        let function_name = module_id
            .trim_start_matches("V-")
            .trim_start_matches("M-")
            .to_ascii_lowercase()
            .replace('-', "_");
        let content = format!(
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: {module_id}\n",
                "// PURPOSE: {purpose}\n",
                "// SCOPE: GraphRAG MCP fixture\n",
                "// DEPENDS: {depends}\n",
                "// LINKS:\n",
                "{link_lines}",
                "\n",
                "// START_MODULE_MAP\n",
                "// build_{function_name} - Fixture function\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 - Test fixture]\n",
                "// END_CHANGE_SUMMARY\n"
            ),
            module_id = module_id,
            purpose = purpose,
            depends = depends,
            link_lines = link_lines,
            function_name = function_name
        );
        std::fs::write(src.join(file), content).expect("write GraphRAG fixture");
    }
    // END_write_graphrag_fixture

    // START_CONTRACT_test_graphrag_query_impact
    // PURPOSE: Verify graphrag_query operation=impact returns bounded dependent and traceability targets.
    // START_test_graphrag_query_impact
    #[tokio::test]
    async fn test_graphrag_query_impact() {
        let _cwd = crate::utils::test_cwd_lock().lock().await;
        let previous = std::env::current_dir().expect("cwd");
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).expect("src dir");
        write_graphrag_fixture(
            &src,
            "core.rs",
            "M-CORE",
            "Core module",
            "N/A",
            &["-> UC-002 (implements) - fixture use case"],
        );
        write_graphrag_fixture(
            &src,
            "api.rs",
            "M-API",
            "API module",
            "M-CORE",
            &["-> M-CORE (depends) - fixture dependency"],
        );
        write_graphrag_fixture(
            &src,
            "ui.rs",
            "M-UI",
            "UI module",
            "M-API",
            &["-> M-API (depends) - fixture dependency"],
        );
        write_graphrag_fixture(
            &src,
            "verify_core.rs",
            "V-M-CORE",
            "Core verification",
            "M-CORE",
            &["-> M-CORE (verified_by) - fixture verification"],
        );
        std::env::set_current_dir(dir.path()).expect("set cwd");

        let response = handle_graphrag(
            &RwLock::new(None),
            &RwLock::new(None),
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "operation": "impact",
                "node_id": "M-CORE",
                "depth": 3,
                "include_tests": true
            }),
        );
        std::env::set_current_dir(previous).expect("restore cwd");

        assert_eq!(response["result"]["format"], "impact");
        let impact = &response["result"]["impact"];
        assert!(impact["direct_dependents"]
            .as_array()
            .expect("direct dependents")
            .iter()
            .any(|target| target["node_id"] == "M-API"));
        assert!(impact["transitive_dependents"]
            .as_array()
            .expect("transitive dependents")
            .iter()
            .any(|target| target["node_id"] == "M-UI"));
        assert!(impact["affected_verification"]
            .as_array()
            .expect("verification")
            .iter()
            .any(|target| target["node_id"] == "V-M-CORE"));
        assert!(impact["affected_use_cases"]
            .as_array()
            .expect("use cases")
            .iter()
            .any(|target| target["node_id"] == "UC-002"));
        assert!(!response["result"]["isError"].as_bool().unwrap_or(true));
    }
    // END_test_graphrag_query_impact

    // START_CONTRACT_test_graphrag_query_mermaid_output
    // PURPOSE: Verify graphrag_query can return Mermaid output without changing MCP content shape
    // START_test_graphrag_query_mermaid_output
    #[tokio::test]
    async fn test_graphrag_query_mermaid_output() {
        let _cwd = crate::utils::test_cwd_lock().lock().await;
        let previous = std::env::current_dir().expect("cwd");
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).expect("src dir");
        std::fs::write(
            src.join("a.rs"),
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-A\n",
                "// PURPOSE: A module\n",
                "// SCOPE: Mermaid test\n",
                "// DEPENDS: M-B\n",
                "// LINKS:\n",
                "//   -> M-B (depends) - test dependency\n",
                "\n",
                "// START_MODULE_MAP\n",
                "// build_a - Builds A\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 - Test fixture]\n",
                "// END_CHANGE_SUMMARY\n",
                "\n",
                "// START_CONTRACT_build_a\n",
                "// PURPOSE: Build A\n",
                "// LINKS:\n",
                "//   -> UC-001 (implements) - fixture\n",
                "// START_build_a\n",
                "pub fn build_a() {}\n",
                "// END_build_a\n",
            ),
        )
        .expect("write a");
        std::fs::write(
            src.join("b.rs"),
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-B\n",
                "// PURPOSE: B module\n",
                "// SCOPE: Mermaid test\n",
                "// DEPENDS: N/A\n",
                "// LINKS:\n",
                "\n",
                "// START_MODULE_MAP\n",
                "// build_b - Builds B\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 - Test fixture]\n",
                "// END_CHANGE_SUMMARY\n",
            ),
        )
        .expect("write b");
        std::env::set_current_dir(dir.path()).expect("set cwd");

        let response = handle_graphrag(
            &RwLock::new(None),
            &RwLock::new(None),
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "operation": "mermaid",
                "subset": "modules",
                "focus_ids": ["M-A"],
                "max_nodes": 10
            }),
        );
        std::env::set_current_dir(previous).expect("restore cwd");
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("mermaid text");

        assert_eq!(response["result"]["format"], "mermaid");
        assert!(text.starts_with("graph TD"), "unexpected text: {text}");
        assert!(text.contains("M-A"), "unexpected text: {text}");
        assert!(!response["result"]["isError"].as_bool().unwrap_or(true));
    }
    // END_test_graphrag_query_mermaid_output
}

// END_public_api
