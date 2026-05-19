// MODULE_CONTRACT
// MODULE_ID: M-GRAPHRAG-TYPES
// PURPOSE: Graph data types — CodeNode, CodeRelationship, CodeGraph, RelationType, GraphOverview
// SCOPE: Graph type definitions, node search, path finding, graph overview
// DEPENDS: N/A
// LINKS: N/A

// START_MODULE_MAP
// RelationType — Types of code relationships (implements, extends, imports, calls, etc.)
// CodeNode — Code graph node (module/file with symbols, imports, exports)
// CodeRelationship — Edge between two code nodes
// CodeGraph — Full code graph with search and navigation
// GraphOverview — Summary statistics for a graph
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.6.0 — Added function contracts for graph type methods]
// END_CHANGE_SUMMARY

use serde::{Deserialize, Serialize};

// START_public_api

// START_RelationType
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationType {
    Implements,
    Extends,
    Configures,
    ArchitecturalDependency,
    FactoryCreates,
    ObserverPattern,
    StrategyPattern,
    AdapterPattern,
    Imports,
    Calls,
    Uses,
    SiblingModule,
    ParentModule,
    ChildModule,
}
// END_RelationType

impl RelationType {
    // START_CONTRACT_RelationType::weight
    // PURPOSE: Return traversal weight for a relationship kind
    // OUTPUTS: { f64 }
    pub fn weight(&self) -> f64 {
        match self {
            Self::Implements | Self::Extends | Self::Configures => 1.0,
            Self::ArchitecturalDependency => 0.9,
            Self::FactoryCreates
            | Self::ObserverPattern
            | Self::StrategyPattern
            | Self::AdapterPattern => 0.8,
            Self::Imports | Self::Calls | Self::Uses => 0.7,
            Self::SiblingModule | Self::ParentModule | Self::ChildModule => 0.3,
        }
    }

    // START_CONTRACT_RelationType::label
    // PURPOSE: Return stable string label for a relationship kind
    // OUTPUTS: { &'static str }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Implements => "implements",
            Self::Extends => "extends",
            Self::Configures => "configures",
            Self::ArchitecturalDependency => "architectural_dependency",
            Self::FactoryCreates => "factory_creates",
            Self::ObserverPattern => "observer_pattern",
            Self::StrategyPattern => "strategy_pattern",
            Self::AdapterPattern => "adapter_pattern",
            Self::Imports => "imports",
            Self::Calls => "calls",
            Self::Uses => "uses",
            Self::SiblingModule => "sibling_module",
            Self::ParentModule => "parent_module",
            Self::ChildModule => "child_module",
        }
    }

    // START_CONTRACT_RelationType::parse
    // PURPOSE: Parse a relationship type from its stable string label
    // INPUTS: { s: &str — relationship label }
    // OUTPUTS: { Option<RelationType> }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "implements" => Some(Self::Implements),
            "extends" => Some(Self::Extends),
            "configures" => Some(Self::Configures),
            "architectural_dependency" => Some(Self::ArchitecturalDependency),
            "factory_creates" => Some(Self::FactoryCreates),
            "observer_pattern" => Some(Self::ObserverPattern),
            "strategy_pattern" => Some(Self::StrategyPattern),
            "adapter_pattern" => Some(Self::AdapterPattern),
            "imports" => Some(Self::Imports),
            "calls" => Some(Self::Calls),
            "uses" => Some(Self::Uses),
            "sibling_module" => Some(Self::SiblingModule),
            "parent_module" => Some(Self::ParentModule),
            "child_module" => Some(Self::ChildModule),
            _ => None,
        }
    }
}

// START_CodeNode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeNode {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub path: String,
    pub language: String,
    pub description: Option<String>,
    pub symbols: Vec<String>,
    pub size_lines: usize,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}
// END_CodeNode

// START_CodeRelationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRelationship {
    pub source_id: String,
    pub target_id: String,
    pub relation_type: RelationType,
    pub weight: f64,
    pub description: Option<String>,
}
// END_CodeRelationship

// START_CodeGraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGraph {
    pub nodes: Vec<CodeNode>,
    pub relationships: Vec<CodeRelationship>,
}
// END_CodeGraph

impl Default for CodeGraph {
    // START_CONTRACT_CodeGraph::default
    // PURPOSE: Create an empty code graph via Default
    // OUTPUTS: { CodeGraph }
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGraph {
    // START_CONTRACT_CodeGraph::new
    // PURPOSE: Create an empty code graph
    // OUTPUTS: { CodeGraph }
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            relationships: Vec::new(),
        }
    }

    // START_CONTRACT_CodeGraph::add_node
    // PURPOSE: Add a node if another node with the same id does not already exist
    // INPUTS: { node: CodeNode — node to insert }
    // SIDE_EFFECTS: mutates graph nodes
    pub fn add_node(&mut self, node: CodeNode) {
        if !self.nodes.iter().any(|n| n.id == node.id) {
            self.nodes.push(node);
        }
    }

    // START_CONTRACT_CodeGraph::add_relationship
    // PURPOSE: Add a relationship edge to the graph
    // INPUTS: { rel: CodeRelationship — edge to insert }
    // SIDE_EFFECTS: mutates graph relationships
    pub fn add_relationship(&mut self, rel: CodeRelationship) {
        self.relationships.push(rel);
    }

    // START_CONTRACT_CodeGraph::get_node
    // PURPOSE: Return a graph node by id
    // INPUTS: { id: &str — node id }
    // OUTPUTS: { Option<&CodeNode> }
    pub fn get_node(&self, id: &str) -> Option<&CodeNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    // START_CONTRACT_CodeGraph::get_relationships
    // PURPOSE: Return relationships touching a node
    // INPUTS: { node_id: &str — node id }
    // OUTPUTS: { Vec<&CodeRelationship> }
    pub fn get_relationships(&self, node_id: &str) -> Vec<&CodeRelationship> {
        self.relationships
            .iter()
            .filter(|r| r.source_id == node_id || r.target_id == node_id)
            .collect()
    }

    // START_CONTRACT_CodeGraph::find_path
    // PURPOSE: Find a relationship path between two node ids
    // INPUTS: { from: &str — source id }, { to: &str — target id }
    // OUTPUTS: { Vec<String> — path ids or empty if no path }
    pub fn find_path(&self, from: &str, to: &str) -> Vec<String> {
        let mut visited = std::collections::HashSet::new();
        let mut path = Vec::new();
        self.dfs(from, to, &mut visited, &mut path);
        path
    }

    // START_CONTRACT_CodeGraph::dfs
    // PURPOSE: Depth-first traversal helper for path discovery
    // INPUTS: { current: &str }, { target: &str }, { visited: &mut HashSet<String> }, { path: &mut Vec<String> }
    // OUTPUTS: { bool — true if target found }
    // SIDE_EFFECTS: mutates visited and path
    fn dfs(
        &self,
        current: &str,
        target: &str,
        visited: &mut std::collections::HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        if current == target {
            path.push(current.to_string());
            return true;
        }
        if !visited.insert(current.to_string()) {
            return false;
        }
        path.push(current.to_string());
        for rel in &self.relationships {
            let next = if rel.source_id == current {
                Some(&rel.target_id)
            } else if rel.target_id == current {
                Some(&rel.source_id)
            } else {
                None
            };
            if let Some(n) = next {
                if self.dfs(n, target, visited, path) {
                    return true;
                }
            }
        }
        path.pop();
        false
    }

    // START_CONTRACT_CodeGraph::search_nodes
    // PURPOSE: Search graph nodes by name, path, or symbol text
    // INPUTS: { query: &str — search text }
    // OUTPUTS: { Vec<&CodeNode> }
    /// Find nodes by text search (name, path, symbols)
    pub fn search_nodes(&self, query: &str) -> Vec<&CodeNode> {
        let q = query.to_lowercase();
        let mut results: Vec<(f64, &CodeNode)> = self
            .nodes
            .iter()
            .map(|n| {
                let mut score = 0.0;
                if n.name.to_lowercase().contains(&q) {
                    score += 10.0;
                }
                if n.path.to_lowercase().contains(&q) {
                    score += 5.0;
                }
                for sym in &n.symbols {
                    if sym.to_lowercase().contains(&q) {
                        score += 3.0;
                    }
                }
                (score, n)
            })
            .filter(|(s, _)| *s > 0.0)
            .collect();
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().map(|(_, n)| n).collect()
    }

    // START_CONTRACT_CodeGraph::overview
    // PURPOSE: Produce graph summary counts by node kind
    // OUTPUTS: { GraphOverview }
    pub fn overview(&self) -> GraphOverview {
        let node_types: Vec<&str> = self.nodes.iter().map(|n| n.kind.as_str()).collect();
        let mut type_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for t in node_types {
            *type_counts.entry(t).or_insert(0) += 1;
        }
        GraphOverview {
            total_nodes: self.nodes.len(),
            total_relationships: self.relationships.len(),
            node_types: type_counts
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect(),
        }
    }
}

// START_GraphOverview
#[derive(Debug, Clone, Serialize)]
pub struct GraphOverview {
    pub total_nodes: usize,
    pub total_relationships: usize,
    pub node_types: Vec<String>,
}
// END_GraphOverview
// END_public_api
