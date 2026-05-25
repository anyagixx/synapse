// MODULE_CONTRACT
// MODULE_ID: M-GRAPHRAG-MERMAID
// PURPOSE: Deterministic Mermaid rendering for GraphRAG module, symbol, and relationship subsets
// SCOPE: Mermaid render options, subset selection, deterministic node/edge ordering, safe label escaping, bounded graph output
// DEPENDS: M-GRAPHRAG-TYPES
// LINKS:
//   -> M-GRAPHRAG-TYPES (depends) - CodeGraph, CodeNode, and CodeRelationship data source
//   -> UC-001 (implements) - agent-facing graph inspection output
//   -> NFR-002 (traces_to) - graph output is deterministic and bounded

// START_MODULE_MAP
// MermaidGraphSubset — Graph subset render modes
// MermaidRenderOptions — Mermaid render selection and output bounds
// render_mermaid — Render a CodeGraph subset as Mermaid
// escape_mermaid_label — Escape labels before Mermaid output
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added deterministic Mermaid graph rendering]
// END_CHANGE_SUMMARY

use super::types::{CodeGraph, CodeNode, CodeRelationship};
use std::collections::{BTreeMap, BTreeSet};

const DEFAULT_MAX_NODES: usize = 80;
const MAX_SYMBOLS_PER_NODE: usize = 12;

// START_public_api

// START_MermaidGraphSubset
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MermaidGraphSubset {
    Modules,
    Symbols,
    Relations,
}
// END_MermaidGraphSubset

impl Default for MermaidGraphSubset {
    // START_CONTRACT_MermaidGraphSubset::default
    // PURPOSE: Return the default Mermaid graph subset
    // OUTPUTS: { MermaidGraphSubset }
    // LINKS:
    //   -> UC-001 (implements) - relation graph is the default agent inspection surface
    // START_mermaid_graph_subset_default
    fn default() -> Self {
        Self::Relations
    }
    // END_mermaid_graph_subset_default
}

// START_MermaidRenderOptions
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MermaidRenderOptions {
    pub subset: MermaidGraphSubset,
    pub focus_ids: Vec<String>,
    pub max_nodes: usize,
}
// END_MermaidRenderOptions

impl Default for MermaidRenderOptions {
    // START_CONTRACT_MermaidRenderOptions::default
    // PURPOSE: Return bounded default Mermaid render options
    // OUTPUTS: { MermaidRenderOptions }
    // LINKS:
    //   -> NFR-002 (traces_to) - default graph output is bounded
    // START_mermaid_render_options_default
    fn default() -> Self {
        Self {
            subset: MermaidGraphSubset::default(),
            focus_ids: Vec::new(),
            max_nodes: DEFAULT_MAX_NODES,
        }
    }
    // END_mermaid_render_options_default
}

impl MermaidRenderOptions {
    // START_CONTRACT_MermaidRenderOptions::normalized
    // PURPOSE: Return render options with safe non-zero bounds and sorted focus ids
    // OUTPUTS: { MermaidRenderOptions }
    // LINKS:
    //   -> NFR-002 (traces_to) - Mermaid output bounds are normalized before rendering
    // START_mermaid_render_options_normalized
    pub fn normalized(&self) -> Self {
        let mut focus_ids = self.focus_ids.clone();
        focus_ids.sort();
        focus_ids.dedup();
        Self {
            subset: self.subset,
            focus_ids,
            max_nodes: if self.max_nodes == 0 {
                DEFAULT_MAX_NODES
            } else {
                self.max_nodes
            },
        }
    }
    // END_mermaid_render_options_normalized
}

// START_CONTRACT_render_mermaid
// PURPOSE: Render a deterministic Mermaid diagram for a CodeGraph subset
// INPUTS: { graph: &CodeGraph }, { options: &MermaidRenderOptions }
// OUTPUTS: { String }
// LINKS:
//   -> UC-001 (implements) - agents can inspect graph neighborhoods as Mermaid
//   -> M-GRAPHRAG-TYPES (depends) - reads CodeGraph nodes and relationships
// START_render_mermaid
pub fn render_mermaid(graph: &CodeGraph, options: &MermaidRenderOptions) -> String {
    let options = options.normalized();
    let nodes = selected_nodes(graph, &options);
    let mut lines = vec!["graph TD".to_string()];

    if nodes.is_empty() {
        lines.push("  empty[\"No graph nodes\"]".to_string());
        return lines.join("\n");
    }

    let id_map = mermaid_node_ids(&nodes);
    append_node_lines(&mut lines, &nodes, &id_map);
    match options.subset {
        MermaidGraphSubset::Symbols => append_symbol_lines(&mut lines, &nodes, &id_map),
        MermaidGraphSubset::Modules | MermaidGraphSubset::Relations => {
            append_relationship_lines(&mut lines, graph.relationships(), &id_map)
        }
    }

    lines.join("\n")
}
// END_render_mermaid

// START_CONTRACT_escape_mermaid_label
// PURPOSE: Escape unsafe label characters for quoted Mermaid labels
// INPUTS: { value: &str }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - Mermaid output is safe to paste into docs
// START_escape_mermaid_label
pub fn escape_mermaid_label(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '"' => '\'',
            '[' | '{' | '(' => '(',
            ']' | '}' | ')' => ')',
            '|' => '/',
            '\n' | '\r' => ' ',
            '<' => '‹',
            '>' => '›',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .to_string()
}
// END_escape_mermaid_label

// END_public_api

// START_CONTRACT_selected_nodes
// PURPOSE: Select sorted bounded graph nodes for the requested Mermaid subset
// INPUTS: { graph: &CodeGraph }, { options: &MermaidRenderOptions }
// OUTPUTS: { Vec<&CodeNode> }
// LINKS:
//   -> NFR-002 (traces_to) - Mermaid node selection is deterministic and bounded
// START_selected_nodes
fn selected_nodes<'a>(graph: &'a CodeGraph, options: &MermaidRenderOptions) -> Vec<&'a CodeNode> {
    let focus = selected_focus_ids(graph, options);
    let mut nodes = graph
        .nodes()
        .iter()
        .filter(|node| node_matches_subset(node, options.subset))
        .filter(|node| focus.is_empty() || focus.contains(&node.id))
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    nodes.truncate(options.max_nodes);
    nodes
}
// END_selected_nodes

// START_CONTRACT_selected_focus_ids
// PURPOSE: Expand focus ids to directly connected relationship neighbors
// INPUTS: { graph: &CodeGraph }, { options: &MermaidRenderOptions }
// OUTPUTS: { BTreeSet<String> }
// LINKS:
//   -> UC-001 (implements) - focused graph output includes immediate neighborhood context
// START_selected_focus_ids
fn selected_focus_ids(graph: &CodeGraph, options: &MermaidRenderOptions) -> BTreeSet<String> {
    if options.focus_ids.is_empty() {
        return BTreeSet::new();
    }
    let mut focus = options.focus_ids.iter().cloned().collect::<BTreeSet<_>>();
    for relationship in graph.relationships() {
        if focus.contains(&relationship.source_id) || focus.contains(&relationship.target_id) {
            focus.insert(relationship.source_id.clone());
            focus.insert(relationship.target_id.clone());
        }
    }
    focus
}
// END_selected_focus_ids

// START_CONTRACT_node_matches_subset
// PURPOSE: Return whether a node belongs to the requested Mermaid subset
// INPUTS: { node: &CodeNode }, { subset: MermaidGraphSubset }
// OUTPUTS: { bool }
// LINKS:
//   -> UC-001 (implements) - subset modes provide agent-friendly graph views
// START_node_matches_subset
fn node_matches_subset(node: &CodeNode, subset: MermaidGraphSubset) -> bool {
    match subset {
        MermaidGraphSubset::Modules => node.kind == "module" || node.id.starts_with("M-"),
        MermaidGraphSubset::Symbols | MermaidGraphSubset::Relations => true,
    }
}
// END_node_matches_subset

// START_CONTRACT_mermaid_node_ids
// PURPOSE: Build stable Mermaid node ids for selected graph nodes
// INPUTS: { nodes: &[&CodeNode] }
// OUTPUTS: { BTreeMap<String, String> }
// LINKS:
//   -> NFR-002 (traces_to) - Mermaid node identifiers are deterministic
// START_mermaid_node_ids
fn mermaid_node_ids(nodes: &[&CodeNode]) -> BTreeMap<String, String> {
    nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), format!("N{}", index)))
        .collect()
}
// END_mermaid_node_ids

// START_CONTRACT_append_node_lines
// PURPOSE: Append Mermaid node declarations in selected order
// INPUTS: { lines: &mut Vec<String> }, { nodes: &[&CodeNode] }, { id_map: &BTreeMap<String, String> }
// SIDE_EFFECTS: mutates lines
// LINKS:
//   -> NFR-002 (traces_to) - Mermaid node labels are escaped before output
// START_append_node_lines
fn append_node_lines(
    lines: &mut Vec<String>,
    nodes: &[&CodeNode],
    id_map: &BTreeMap<String, String>,
) {
    for node in nodes {
        if let Some(id) = id_map.get(&node.id) {
            let label = escape_mermaid_label(&format!("{} / {}", node.name, node.kind));
            lines.push(format!("  {}[\"{}\"]", id, label));
        }
    }
}
// END_append_node_lines

// START_CONTRACT_append_relationship_lines
// PURPOSE: Append Mermaid relationship edges whose endpoints are selected
// INPUTS: { lines: &mut Vec<String> }, { relationships: &[CodeRelationship] }, { id_map: &BTreeMap<String, String> }
// SIDE_EFFECTS: mutates lines
// LINKS:
//   -> NFR-002 (traces_to) - Mermaid relationship lines are deterministically ordered
// START_append_relationship_lines
fn append_relationship_lines(
    lines: &mut Vec<String>,
    relationships: &[CodeRelationship],
    id_map: &BTreeMap<String, String>,
) {
    let mut edges = relationships
        .iter()
        .filter_map(|relationship| {
            let source = id_map.get(&relationship.source_id)?;
            let target = id_map.get(&relationship.target_id)?;
            Some(format!(
                "  {} -->|{}| {}",
                source,
                relationship.relation_type.label(),
                target
            ))
        })
        .collect::<Vec<_>>();
    edges.sort();
    edges.dedup();
    lines.extend(edges);
}
// END_append_relationship_lines

// START_CONTRACT_append_symbol_lines
// PURPOSE: Append Mermaid symbol nodes and containment edges for selected graph nodes
// INPUTS: { lines: &mut Vec<String> }, { nodes: &[&CodeNode] }, { id_map: &BTreeMap<String, String> }
// SIDE_EFFECTS: mutates lines
// LINKS:
//   -> UC-001 (implements) - symbol subset exposes module symbol contents
// START_append_symbol_lines
fn append_symbol_lines(
    lines: &mut Vec<String>,
    nodes: &[&CodeNode],
    id_map: &BTreeMap<String, String>,
) {
    let mut symbol_index = 0_usize;
    for node in nodes {
        let Some(node_id) = id_map.get(&node.id) else {
            continue;
        };
        let mut symbols = node.symbols.iter().collect::<Vec<_>>();
        symbols.sort();
        symbols.truncate(MAX_SYMBOLS_PER_NODE);
        for symbol in symbols {
            let symbol_id = format!("S{}", symbol_index);
            symbol_index += 1;
            lines.push(format!(
                "  {}[\"{}\"]",
                symbol_id,
                escape_mermaid_label(symbol)
            ));
            lines.push(format!("  {} -->|symbol| {}", node_id, symbol_id));
        }
    }
}
// END_append_symbol_lines

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grace::contract::{LinkDirection, LinkType};
    use crate::graphrag::types::{CodeRelationship, RelationType, TypedCodeRelationship};

    fn sample_graph() -> CodeGraph {
        let mut graph = CodeGraph::new();
        graph.add_node(CodeNode {
            id: "M-B".into(),
            name: "Beta [unsafe]|node".into(),
            kind: "module".into(),
            path: "src/b.rs".into(),
            language: "rust".into(),
            description: None,
            symbols: vec!["handle|beta".into(), "render[beta]".into()],
            size_lines: 10,
            imports: Vec::new(),
            exports: Vec::new(),
        });
        graph.add_node(CodeNode {
            id: "M-A".into(),
            name: "Alpha \"quoted\"\nnode".into(),
            kind: "module".into(),
            path: "src/a.rs".into(),
            language: "rust".into(),
            description: None,
            symbols: vec!["build_alpha".into()],
            size_lines: 10,
            imports: Vec::new(),
            exports: Vec::new(),
        });
        graph.add_node(CodeNode {
            id: "UC-001".into(),
            name: "Inspect graph".into(),
            kind: "use_case".into(),
            path: "docs/requirements.xml".into(),
            language: "xml".into(),
            description: None,
            symbols: Vec::new(),
            size_lines: 0,
            imports: Vec::new(),
            exports: Vec::new(),
        });
        graph.add_relationship(CodeRelationship {
            source_id: "M-A".into(),
            target_id: "M-B".into(),
            relation_type: RelationType::Depends,
            weight: 0.9,
            description: None,
        });
        graph.add_typed_relationship(TypedCodeRelationship {
            source_id: "M-A".into(),
            target_id: "UC-001".into(),
            link_type: LinkType::Implements,
            direction: LinkDirection::Outgoing,
            description: None,
            weight: 1.0,
        });
        graph
    }

    #[test]
    fn render_mermaid_is_deterministic_and_escaped() {
        let graph = sample_graph();
        let options = MermaidRenderOptions::default();

        let first = render_mermaid(&graph, &options);
        let second = render_mermaid(&graph, &options);

        assert_eq!(first, second);
        assert!(first.starts_with("graph TD"));
        assert!(first.contains("N0[\"Alpha 'quoted' node / module\"]"));
        assert!(first.contains("N1[\"Beta (unsafe)/node / module\"]"));
        assert!(!first.contains("[unsafe]|node"));
    }

    #[test]
    fn render_mermaid_symbol_subset_reports_symbols() {
        let graph = sample_graph();
        let options = MermaidRenderOptions {
            subset: MermaidGraphSubset::Symbols,
            focus_ids: vec!["M-B".into()],
            max_nodes: 10,
        };

        let output = render_mermaid(&graph, &options);

        assert!(output.contains("handle/beta"));
        assert!(output.contains("render(beta)"));
        assert!(output.contains("-->|symbol|"));
    }

    #[test]
    fn render_mermaid_module_subset_is_bounded() {
        let graph = sample_graph();
        let options = MermaidRenderOptions {
            subset: MermaidGraphSubset::Modules,
            focus_ids: Vec::new(),
            max_nodes: 1,
        };

        let output = render_mermaid(&graph, &options);

        assert!(output.contains("N0[\"Alpha"));
        assert!(!output.contains("Beta"));
        assert!(!output.contains("UC-001"));
    }
}
