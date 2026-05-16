use crate::graphrag::types::*;
use crate::indexer::storage::Storage;
use crate::indexer::walker::Walker;
use std::path::Path;

pub struct GraphBuilder;

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Build a code graph from the indexed storage
    pub fn build(root: &Path) -> anyhow::Result<CodeGraph> {
        let mut graph = CodeGraph::new();

        // Load indexed blocks from storage
        let storage = Storage::new(root);
        let blocks = storage.all_blocks();
        let walker = Walker::new(root);
        let files = walker.walk();

        // 1. Create nodes from files
        for file in &files {
            let full_path = root.join(&file.path);
            let content = std::fs::read_to_string(&full_path).ok();
            let size = content.as_ref().map(|c| c.lines().count()).unwrap_or(0);

            // Extract symbols from blocks for this file
            let symbols: Vec<String> = blocks
                .iter()
                .filter(|b| b.path == file.path)
                .map(|b| b.name.clone())
                .collect();

            // Extract imports via regex
            let imports = extract_imports(&content.unwrap_or_default(), &file.language);

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

        Ok(graph)
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
