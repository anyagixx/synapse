// MODULE_CONTRACT
// MODULE_ID: M-GRAPHRAG
// PURPOSE: GraphRAG facade — knowledge graph navigation with search, typed relationships, and path finding
// SCOPE: GraphRag struct, build from storage, search_nodes, get_node, get_relationships, typed relationship filters, find_path, overview
// DEPENDS: M-GRACE-CONTRACT, M-GRAPHRAG-TYPES, M-GRAPHRAG-BUILDER
// LINKS: N/A

// START_MODULE_MAP
// GraphRag — Knowledge graph facade wrapping CodeGraph
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.11.0 — Added typed LINKS query facade methods]
// END_CHANGE_SUMMARY

pub mod builder;
pub mod types;

use crate::grace::contract::LinkType;
use builder::GraphBuilder;
use std::path::Path;

pub use types::*;

// START_public_api

// START_GraphRag
pub struct GraphRag {
    graph: Option<CodeGraph>,
}
// END_GraphRag

impl Default for GraphRag {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphRag {
    // START_CONTRACT_GraphRag::new
    // PURPOSE: Create a new empty GraphRag
    // OUTPUTS: { Self }
    // START_graphrag_new
    pub fn new() -> Self {
        Self { graph: None }
    }
    // END_graphrag_new

    // START_CONTRACT_GraphRag::build
    // PURPOSE: Build the code graph from indexed storage
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<()> }
    // START_graphrag_build
    pub fn build(&mut self, root: &Path) -> anyhow::Result<()> {
        let graph = GraphBuilder::build(root)?;
        let nodes = graph.nodes().len();
        let rels = graph.relationships().len();
        tracing::info!(
            "GraphRAG: built graph with {} nodes and {} relationships",
            nodes,
            rels
        );
        self.graph = Some(graph);
        Ok(())
    }
    // END_graphrag_build

    // START_CONTRACT_GraphRag::graph
    // PURPOSE: Return a reference to the built code graph
    // OUTPUTS: { Option<&CodeGraph> }
    // START_graphrag_graph
    pub fn graph(&self) -> Option<&CodeGraph> {
        self.graph.as_ref()
    }
    // END_graphrag_graph

    // START_CONTRACT_GraphRag::search_nodes
    // PURPOSE: Search graph nodes by text query
    // INPUTS: { query: &str }
    // OUTPUTS: { Vec<&CodeNode> }
    // START_graphrag_search_nodes
    pub fn search_nodes(&self, query: &str) -> Vec<&CodeNode> {
        self.graph
            .as_ref()
            .map(|g| g.search_nodes(query))
            .unwrap_or_default()
    }
    // END_graphrag_search_nodes

    // START_CONTRACT_GraphRag::get_node
    // PURPOSE: Get a single node by ID
    // INPUTS: { id: &str — node ID }
    // OUTPUTS: { Option<&CodeNode> }
    // START_graphrag_get_node
    pub fn get_node(&self, id: &str) -> Option<&CodeNode> {
        self.graph.as_ref().and_then(|g| g.get_node(id))
    }
    // END_graphrag_get_node

    // START_CONTRACT_GraphRag::get_relationships
    // PURPOSE: Get all relationships for a node
    // INPUTS: { node_id: &str }
    // OUTPUTS: { Vec<&CodeRelationship> }
    // START_graphrag_get_relationships
    pub fn get_relationships(&self, node_id: &str) -> Vec<&CodeRelationship> {
        self.graph
            .as_ref()
            .map(|g| g.get_relationships(node_id))
            .unwrap_or_default()
    }
    // END_graphrag_get_relationships

    // START_CONTRACT_GraphRag::get_typed_relationships
    // PURPOSE: Get typed LINKS relationships for a node with an optional relationship type filter
    // INPUTS: { node_id: &str }, { link_type: Option<LinkType> }
    // OUTPUTS: { Vec<&TypedCodeRelationship> }
    // START_graphrag_get_typed_relationships
    pub fn get_typed_relationships(
        &self,
        node_id: &str,
        link_type: Option<LinkType>,
    ) -> Vec<&TypedCodeRelationship> {
        self.graph
            .as_ref()
            .map(|g| g.get_typed_relationships(node_id, link_type))
            .unwrap_or_default()
    }
    // END_graphrag_get_typed_relationships

    // START_CONTRACT_GraphRag::get_incoming_typed_relationships
    // PURPOSE: Get typed LINKS relationships that point to a target artifact
    // INPUTS: { target_id: &str }, { link_type: Option<LinkType> }
    // OUTPUTS: { Vec<&TypedCodeRelationship> }
    // START_graphrag_get_incoming_typed_relationships
    pub fn get_incoming_typed_relationships(
        &self,
        target_id: &str,
        link_type: Option<LinkType>,
    ) -> Vec<&TypedCodeRelationship> {
        self.graph
            .as_ref()
            .map(|g| g.get_incoming_typed_relationships(target_id, link_type))
            .unwrap_or_default()
    }
    // END_graphrag_get_incoming_typed_relationships

    // START_CONTRACT_GraphRag::find_path
    // PURPOSE: Find a path between two nodes via DFS
    // INPUTS: { from: &str }, { to: &str }
    // OUTPUTS: { Vec<String> — path of node IDs }
    // START_graphrag_find_path
    pub fn find_path(&self, from: &str, to: &str) -> Vec<String> {
        self.graph
            .as_ref()
            .map(|g| g.find_path(from, to))
            .unwrap_or_default()
    }
    // END_graphrag_find_path

    // START_CONTRACT_GraphRag::find_path_by_link_type
    // PURPOSE: Find a path using only typed LINKS of the requested type when provided
    // INPUTS: { from: &str }, { to: &str }, { link_type: Option<LinkType> }
    // OUTPUTS: { Vec<String> — path of node IDs }
    // START_graphrag_find_path_by_link_type
    pub fn find_path_by_link_type(
        &self,
        from: &str,
        to: &str,
        link_type: Option<LinkType>,
    ) -> Vec<String> {
        self.graph
            .as_ref()
            .map(|g| g.find_path_by_link_type(from, to, link_type))
            .unwrap_or_default()
    }
    // END_graphrag_find_path_by_link_type

    // START_CONTRACT_GraphRag::overview
    // PURPOSE: Return a graph overview with node and relationship counts
    // OUTPUTS: { Option<GraphOverview> }
    // START_graphrag_overview
    pub fn overview(&self) -> Option<GraphOverview> {
        self.graph.as_ref().map(|g| g.overview())
    }
    // END_graphrag_overview
}
// END_public_api
