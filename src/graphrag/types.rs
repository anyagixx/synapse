use serde::{Deserialize, Serialize};

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

impl RelationType {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRelationship {
    pub source_id: String,
    pub target_id: String,
    pub relation_type: RelationType,
    pub weight: f64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGraph {
    pub nodes: Vec<CodeNode>,
    pub relationships: Vec<CodeRelationship>,
}

impl Default for CodeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            relationships: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: CodeNode) {
        if !self.nodes.iter().any(|n| n.id == node.id) {
            self.nodes.push(node);
        }
    }

    pub fn add_relationship(&mut self, rel: CodeRelationship) {
        self.relationships.push(rel);
    }

    pub fn get_node(&self, id: &str) -> Option<&CodeNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_relationships(&self, node_id: &str) -> Vec<&CodeRelationship> {
        self.relationships
            .iter()
            .filter(|r| r.source_id == node_id || r.target_id == node_id)
            .collect()
    }

    pub fn find_path(&self, from: &str, to: &str) -> Vec<String> {
        let mut visited = std::collections::HashSet::new();
        let mut path = Vec::new();
        self.dfs(from, to, &mut visited, &mut path);
        path
    }

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

#[derive(Debug, Clone, Serialize)]
pub struct GraphOverview {
    pub total_nodes: usize,
    pub total_relationships: usize,
    pub node_types: Vec<String>,
}
