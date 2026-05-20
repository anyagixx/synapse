// MODULE_CONTRACT
// MODULE_ID: M-GRAPHRAG-BUILDER
// PURPOSE: Graph builder — constructs CodeGraph from indexed storage, imports, hierarchy, and typed GRACE LINKS
// SCOPE: GraphBuilder struct, build from storage blocks/files/contracts, typed LINKS extraction, extract_imports for multi-language
// DEPENDS: M-GRAPHRAG-TYPES, M-GRACE-CONTRACT, M-INDEXER-STORAGE, M-INDEXER-WALKER
// LINKS: N/A

// START_MODULE_MAP
// GraphBuilder — Builds a CodeGraph from indexed code blocks, file list, and typed LINKS
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.11.0 — Added typed LINKS extraction into GraphRAG relationships]
// END_CHANGE_SUMMARY

use crate::grace::contract::{ContractValidator, GraceProfile, TypedLink};
use crate::graphrag::types::*;
use crate::indexer::storage::Storage;
use crate::indexer::walker::Walker;
use std::path::Path;

// START_public_api

// START_GraphBuilder
pub struct GraphBuilder;
// END_GraphBuilder

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphBuilder {
    // START_CONTRACT_GraphBuilder::new
    // PURPOSE: Create a new GraphBuilder
    // OUTPUTS: { Self }
    // START_gb_new
    pub fn new() -> Self {
        Self
    }
    // END_gb_new

    // START_CONTRACT_GraphBuilder::build
    // PURPOSE: Build a code graph from indexed storage — nodes from files, relationships from imports and hierarchy
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<CodeGraph> }
    // START_gb_build
    pub fn build(root: &Path) -> anyhow::Result<CodeGraph> {
        let mut graph = CodeGraph::new();

        // Load indexed blocks from storage
        let storage = Storage::new(root);
        let blocks = storage.all_blocks();
        let walker = Walker::new(root);
        let files = walker.walk();

        let mut contract_links: Vec<(String, Vec<TypedLink>)> = Vec::new();

        // 1. Create nodes from files and MODULE_CONTRACT ids
        for file in &files {
            let full_path = root.join(&file.path);
            let content = std::fs::read_to_string(&full_path).unwrap_or_default();
            let size = content.lines().count();

            // Extract symbols from blocks for this file
            let symbols: Vec<String> = blocks
                .iter()
                .filter(|b| b.path == file.path)
                .map(|b| b.name.clone())
                .collect();

            // Extract imports via regex
            let imports = extract_imports(&content, &file.language);

            let node = CodeNode {
                id: file.path.clone(),
                name: std::path::Path::new(&file.path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| file.path.clone()),
                kind: "file".into(),
                path: file.path.clone(),
                language: file.language.clone(),
                description: None,
                symbols,
                size_lines: size,
                imports,
                exports: Vec::new(),
            };
            graph.add_node(node);

            let contract =
                ContractValidator::scan_file_with_profile(&full_path, &content, GraceProfile::Lite);
            if let Some(module_id) = contract.module_id.clone().filter(|_| contract.has_contract) {
                let symbols: Vec<String> = contract
                    .function_contracts
                    .iter()
                    .map(|function| function.name.clone())
                    .collect();
                graph.add_node(CodeNode {
                    id: module_id.clone(),
                    name: module_id.clone(),
                    kind: "module".into(),
                    path: file.path.clone(),
                    language: file.language.clone(),
                    description: contract.purpose.clone(),
                    symbols: symbols.clone(),
                    size_lines: size,
                    imports: contract.depends.clone(),
                    exports: symbols.clone(),
                });
                contract_links.push((module_id.clone(), contract.typed_links()));
                for function in &contract.function_contracts {
                    let function_id = format!("{}::{}", module_id, function.name);
                    graph.add_node(CodeNode {
                        id: function_id.clone(),
                        name: function.name.clone(),
                        kind: "function_contract".into(),
                        path: file.path.clone(),
                        language: file.language.clone(),
                        description: function.purpose.clone(),
                        symbols: Vec::new(),
                        size_lines: 0,
                        imports: Vec::new(),
                        exports: Vec::new(),
                    });
                    contract_links.push((function_id, function.typed_links()));
                }
            }
        }

        // 2. Create relationships from imports
        let node_imports: Vec<(String, Vec<String>)> = graph
            .nodes
            .iter()
            .map(|n| (n.id.clone(), n.imports.clone()))
            .collect();
        let node_ids: Vec<String> = graph.nodes.iter().map(|n| n.id.clone()).collect();

        for (node_id, imports) in &node_imports {
            for import in imports {
                if let Some(target_id) = node_ids.iter().find(|id| {
                    id.ends_with(&format!("/{}", import))
                        || id.ends_with(&format!("/{}.rs", import))
                        || id.ends_with(&format!("/{}.py", import))
                        || id.ends_with(&format!("/{}.ts", import))
                        || id.ends_with(&format!("/{}.js", import))
                        || id.ends_with(&format!("/{}.go", import))
                        || id.contains(&format!("/{}/", import))
                }) {
                    graph.add_relationship(CodeRelationship {
                        source_id: node_id.clone(),
                        target_id: target_id.clone(),
                        relation_type: RelationType::Imports,
                        weight: RelationType::Imports.weight(),
                        description: Some(format!("imports {}", import)),
                    });
                }
            }
        }

        // 3. Create hierarchy relationships (parent/child module)
        let mut module_map: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for node_id in &node_ids {
            if let Some(parent) = std::path::Path::new(node_id).parent() {
                let parent_str = parent.to_string_lossy().to_string();
                if !parent_str.is_empty() && parent_str != "." {
                    module_map
                        .entry(parent_str)
                        .or_default()
                        .push(node_id.clone());
                }
            }
        }

        for (parent, children) in &module_map {
            let parent_id = parent.replace('\\', "/");
            if node_ids.contains(&parent_id) {
                for child in children {
                    graph.add_relationship(CodeRelationship {
                        source_id: parent_id.clone(),
                        target_id: child.clone(),
                        relation_type: RelationType::ParentModule,
                        weight: RelationType::ParentModule.weight(),
                        description: None,
                    });
                }
            }
            // Sibling relationships
            for i in 0..children.len() {
                for j in (i + 1)..children.len() {
                    graph.add_relationship(CodeRelationship {
                        source_id: children[i].clone(),
                        target_id: children[j].clone(),
                        relation_type: RelationType::SiblingModule,
                        weight: RelationType::SiblingModule.weight(),
                        description: None,
                    });
                }
            }
        }

        // 4. Create typed semantic relationships from GRACE LINKS.
        for (source_id, links) in contract_links {
            for link in links {
                ensure_artifact_node(&mut graph, &link.target);
                let relation_type = RelationType::from_link_type(&link.link_type);
                graph.add_typed_relationship(TypedCodeRelationship {
                    source_id: source_id.clone(),
                    target_id: link.target,
                    link_type: link.link_type,
                    direction: link.direction,
                    description: link.description,
                    weight: relation_type.weight(),
                });
            }
        }

        Ok(graph)
    }
    // END_gb_build
}

fn ensure_artifact_node(graph: &mut CodeGraph, target: &str) {
    if graph.get_node(target).is_some() || target == "N/A" {
        return;
    }
    graph.add_node(CodeNode {
        id: target.to_string(),
        name: target.to_string(),
        kind: artifact_kind(target).into(),
        path: target.to_string(),
        language: "artifact".into(),
        description: None,
        symbols: Vec::new(),
        size_lines: 0,
        imports: Vec::new(),
        exports: Vec::new(),
    });
}

fn artifact_kind(target: &str) -> &'static str {
    if target.starts_with("M-") {
        "module_ref"
    } else if target.starts_with("V-") {
        "verification"
    } else if target.starts_with("UC-") {
        "use_case"
    } else if target.starts_with("REQ-") {
        "requirement"
    } else if target.starts_with("Entity:") {
        "entity"
    } else if target.ends_with(".xml") || target.ends_with(".md") || target.contains('/') {
        "artifact"
    } else {
        "external"
    }
}

fn extract_imports(content: &str, language: &str) -> Vec<String> {
    let mut imports = Vec::new();
    match language {
        "rust" => {
            for line in content.lines() {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix("use ") {
                    let path = rest.split("::").next().unwrap_or("").trim().to_string();
                    if !path.is_empty()
                        && !path.starts_with("crate")
                        && !path.starts_with("self")
                        && !path.starts_with("super")
                    {
                        imports.push(path);
                    }
                }
                if let Some(rest) = t.strip_prefix("extern crate ") {
                    imports.push(rest.trim_end_matches(';').to_string());
                }
                if let Some(rest) = t.strip_prefix("mod ") {
                    if rest.contains(';') || !rest.contains('{') {
                        imports.push(rest.trim_end_matches(';').to_string());
                    }
                }
            }
        }
        "python" => {
            for line in content.lines() {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix("import ") {
                    let name = rest
                        .split(' ')
                        .next()
                        .unwrap_or("")
                        .split('.')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    if !name.is_empty() {
                        imports.push(name);
                    }
                }
                if let Some(rest) = t.strip_prefix("from ") {
                    let name = rest
                        .split(' ')
                        .nth(1)
                        .unwrap_or("")
                        .split('.')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    if !name.is_empty() {
                        imports.push(name);
                    }
                }
            }
        }
        "javascript" | "typescript" => {
            for line in content.lines() {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix("import ") {
                    if let Some(from) = rest.split("from ").nth(1) {
                        let path = from.trim().trim_matches('"').trim_matches('\'').to_string();
                        if !path.starts_with('.') && !path.starts_with('/') {
                            imports.push(path);
                        }
                    }
                }
                if let Some(rest) = t.strip_prefix("require(") {
                    let path = rest
                        .trim_end_matches(')')
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    if !path.starts_with('.') {
                        imports.push(path);
                    }
                }
            }
        }
        "go" => {
            for line in content.lines() {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix("import ") {
                    for path in rest.split_whitespace() {
                        let p = path.trim_matches('"').trim();
                        if !p.is_empty() && p != "(" && p != ")" {
                            if let Some(last) = p.rsplit('/').next() {
                                imports.push(last.to_string());
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    imports
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grace::contract::LinkType;

    #[test]
    fn test_build_extracts_typed_links_from_contracts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).expect("create src");
        std::fs::write(
            src.join("order.rs"),
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-ORDER\n",
                "// PURPOSE: Order module\n",
                "// SCOPE: Test typed LINKS graph extraction\n",
                "// DEPENDS: N/A\n",
                "// LINKS:\n",
                "//   → UC-001 (implements) — order use case\n",
                "//   → M-STORAGE (depends) — storage module\n",
                "\n",
                "// START_MODULE_MAP\n",
                "// place_order — places order\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 — Initial]\n",
                "// END_CHANGE_SUMMARY\n",
                "\n",
                "// START_CONTRACT_place_order\n",
                "// PURPOSE: Place order\n",
                "// LINKS:\n",
                "//   → REQ-001 (traces_to) — requirement\n",
                "// START_place_order\n",
                "pub fn place_order() {}\n",
                "// END_place_order\n",
            ),
        )
        .expect("write source");

        let graph = GraphBuilder::build(dir.path()).expect("build graph");
        assert!(graph.get_node("M-ORDER").is_some());
        assert!(graph.get_node("UC-001").is_some());
        assert!(graph.get_node("M-ORDER::place_order").is_some());
        assert_eq!(
            graph
                .get_typed_relationships("M-ORDER", Some(LinkType::Implements))
                .len(),
            1
        );
        assert_eq!(
            graph
                .get_typed_relationships("M-ORDER::place_order", Some(LinkType::TracesTo))
                .len(),
            1
        );
    }
}
// END_public_api
