pub mod types;
pub mod builder;

use std::path::Path;
use builder::GraphBuilder;

pub use types::*;

pub struct GraphRag {
    graph: Option<CodeGraph>,
}

impl GraphRag {
    pub fn new() -> Self {
        Self { graph: None }
    }

    pub fn build(&mut self, root: &Path) -> anyhow::Result<()> {
        let graph = GraphBuilder::build(root)?;
        let nodes = graph.nodes.len();
        let rels = graph.relationships.len();
        tracing::info!("GraphRAG: built graph with {} nodes and {} relationships", nodes, rels);
        self.graph = Some(graph);
        Ok(())
    }

    pub fn graph(&self) -> Option<&CodeGraph> {
        self.graph.as_ref()
    }

    pub fn search_nodes(&self, query: &str) -> Vec<&CodeNode> {
        self.graph.as_ref()
            .map(|g| g.search_nodes(query))
            .unwrap_or_default()
    }

    pub fn get_node(&self, id: &str) -> Option<&CodeNode> {
        self.graph.as_ref().and_then(|g| g.get_node(id))
    }

    pub fn get_relationships(&self, node_id: &str) -> Vec<&CodeRelationship> {
        self.graph.as_ref()
            .map(|g| g.get_relationships(node_id))
            .unwrap_or_default()
    }

    pub fn find_path(&self, from: &str, to: &str) -> Vec<String> {
        self.graph.as_ref()
            .map(|g| g.find_path(from, to))
            .unwrap_or_default()
    }

    pub fn overview(&self) -> Option<GraphOverview> {
        self.graph.as_ref().map(|g| g.overview())
    }
}
