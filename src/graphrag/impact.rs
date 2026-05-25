// MODULE_CONTRACT
// MODULE_ID: M-GRAPHRAG-IMPACT
// PURPOSE: GraphRAG impact analysis for bounded dependency blast-radius estimation.
// SCOPE: Impact target modeling, direct/transitive dependent traversal, affected verification/use-case extraction, advisory risk scoring, and suggested actions.
// DEPENDS: M-GRAPHRAG-TYPES
// LINKS:
//   -> M-GRAPHRAG-TYPES (depends) - reads CodeGraph nodes and relationships
//   -> UC-001 (implements) - agents inspect change impact before edits
//   -> NFR-002 (traces_to) - bounded deterministic traversal prevents runaway graph scans

// START_MODULE_MAP
// ImpactTarget - Stable impacted graph node or artifact summary
// ImpactRisk - Advisory risk score for the impact set
// ImpactAnalysis - Complete impact report returned to callers
// analyze_impact - Runs bounded impact traversal and risk scoring
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added GraphRAG impact analysis model and traversal]
// END_CHANGE_SUMMARY

use super::types::{CodeGraph, CodeNode, CodeRelationship};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const CRITICAL_RISK_SCORE: usize = 16;
const HIGH_RISK_SCORE: usize = 10;
const MEDIUM_RISK_SCORE: usize = 5;

// START_public_api
// START_ImpactTarget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactTarget {
    pub node_id: String,
    pub name: String,
    pub kind: String,
    pub path: String,
    pub distance: usize,
    pub relation_type: String,
    pub relation_weight: f64,
    pub description: Option<String>,
}
// END_ImpactTarget

// START_ImpactRisk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactRisk {
    pub level: String,
    pub score: usize,
    pub reasons: Vec<String>,
}
// END_ImpactRisk

// START_ImpactAnalysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    pub node_id: String,
    pub depth: usize,
    pub include_tests: bool,
    pub direct_dependents: Vec<ImpactTarget>,
    pub transitive_dependents: Vec<ImpactTarget>,
    pub dependencies: Vec<ImpactTarget>,
    pub affected_verification: Vec<ImpactTarget>,
    pub affected_use_cases: Vec<ImpactTarget>,
    pub risk: ImpactRisk,
    pub suggested_actions: Vec<String>,
}
// END_ImpactAnalysis

// START_CONTRACT_analyze_impact
// PURPOSE: Analyze a node's bounded dependency impact surface.
// INPUTS: { graph: &CodeGraph - graph to traverse }, { node_id: &str - changed node id }, { depth: usize - max reverse traversal depth }, { include_tests: bool - whether verification/test targets are included }
// OUTPUTS: { Option<ImpactAnalysis> - impact report when node_id is present in the graph }
// LINKS:
//   -> M-GRAPHRAG-TYPES (depends) - traverses CodeGraph relationships
//   -> UC-001 (implements) - answers agent impact queries
// START_analyze_impact
pub fn analyze_impact(
    graph: &CodeGraph,
    node_id: &str,
    depth: usize,
    include_tests: bool,
) -> Option<ImpactAnalysis> {
    graph.get_node(node_id)?;
    let depth = depth.clamp(1, 8);

    let direct_dependents = direct_dependents(graph, node_id, include_tests);
    let transitive_dependents = transitive_dependents(graph, node_id, depth, include_tests);
    let dependencies = dependencies(graph, node_id, include_tests);

    let mut impacted_targets = Vec::new();
    impacted_targets.extend(direct_dependents.iter().cloned());
    impacted_targets.extend(transitive_dependents.iter().cloned());
    impacted_targets.extend(dependencies.iter().cloned());
    let impacted_targets = dedupe_targets(impacted_targets);

    let affected_verification = affected_targets_by_prefix(&impacted_targets, "V-");
    let affected_use_cases = affected_targets_by_prefix(&impacted_targets, "UC-");
    let risk = score_risk(
        &direct_dependents,
        &transitive_dependents,
        &affected_verification,
        &affected_use_cases,
    );
    let suggested_actions = suggested_actions(&risk, &affected_verification, &affected_use_cases);

    Some(ImpactAnalysis {
        node_id: node_id.to_string(),
        depth,
        include_tests,
        direct_dependents,
        transitive_dependents,
        dependencies,
        affected_verification,
        affected_use_cases,
        risk,
        suggested_actions,
    })
}
// END_analyze_impact

// END_public_api

// START_CONTRACT_direct_dependents
// PURPOSE: Return direct incoming relationship sources for a target node.
// INPUTS: { graph: &CodeGraph }, { node_id: &str }, { include_tests: bool }
// OUTPUTS: { Vec<ImpactTarget> }
// START_direct_dependents
fn direct_dependents(graph: &CodeGraph, node_id: &str, include_tests: bool) -> Vec<ImpactTarget> {
    let mut targets: Vec<ImpactTarget> = graph
        .relationships()
        .iter()
        .filter(|rel| rel.target_id == node_id)
        .filter_map(|rel| target_from_relationship(graph, &rel.source_id, rel, 1, include_tests))
        .collect();
    sort_targets(&mut targets);
    targets
}
// END_direct_dependents

// START_CONTRACT_dependencies
// PURPOSE: Return direct outgoing dependency targets for a source node.
// INPUTS: { graph: &CodeGraph }, { node_id: &str }, { include_tests: bool }
// OUTPUTS: { Vec<ImpactTarget> }
// START_dependencies
fn dependencies(graph: &CodeGraph, node_id: &str, include_tests: bool) -> Vec<ImpactTarget> {
    let mut targets: Vec<ImpactTarget> = graph
        .relationships()
        .iter()
        .filter(|rel| rel.source_id == node_id)
        .filter_map(|rel| target_from_relationship(graph, &rel.target_id, rel, 1, include_tests))
        .collect();
    sort_targets(&mut targets);
    targets
}
// END_dependencies

// START_CONTRACT_transitive_dependents
// PURPOSE: Traverse incoming relationships up to depth and return indirect dependents.
// INPUTS: { graph: &CodeGraph }, { node_id: &str }, { depth: usize }, { include_tests: bool }
// OUTPUTS: { Vec<ImpactTarget> }
// START_transitive_dependents
fn transitive_dependents(
    graph: &CodeGraph,
    node_id: &str,
    depth: usize,
    include_tests: bool,
) -> Vec<ImpactTarget> {
    let mut visited = BTreeSet::from([node_id.to_string()]);
    let mut queue = VecDeque::from([(node_id.to_string(), 0usize)]);
    let mut targets = Vec::new();

    while let Some((current, distance)) = queue.pop_front() {
        if distance >= depth {
            continue;
        }

        for rel in graph
            .relationships()
            .iter()
            .filter(|rel| rel.target_id == current)
        {
            if !visited.insert(rel.source_id.clone()) {
                continue;
            }

            let next_distance = distance + 1;
            if let Some(target) =
                target_from_relationship(graph, &rel.source_id, rel, next_distance, include_tests)
            {
                if next_distance > 1 {
                    targets.push(target);
                }
                queue.push_back((rel.source_id.clone(), next_distance));
            }
        }
    }

    sort_targets(&mut targets);
    targets
}
// END_transitive_dependents

// START_CONTRACT_target_from_relationship
// PURPOSE: Convert a relationship endpoint into a stable impact target.
// INPUTS: { graph: &CodeGraph }, { endpoint_id: &str }, { rel: &CodeRelationship }, { distance: usize }, { include_tests: bool }
// OUTPUTS: { Option<ImpactTarget> }
// START_target_from_relationship
fn target_from_relationship(
    graph: &CodeGraph,
    endpoint_id: &str,
    rel: &CodeRelationship,
    distance: usize,
    include_tests: bool,
) -> Option<ImpactTarget> {
    let node = graph.get_node(endpoint_id);
    if !include_tests && is_test_target(endpoint_id, node) {
        return None;
    }

    Some(ImpactTarget {
        node_id: endpoint_id.to_string(),
        name: node
            .map(|node| node.name.clone())
            .unwrap_or_else(|| endpoint_id.to_string()),
        kind: node
            .map(|node| node.kind.clone())
            .unwrap_or_else(|| artifact_kind(endpoint_id).to_string()),
        path: node.map(|node| node.path.clone()).unwrap_or_default(),
        distance,
        relation_type: rel.relation_type.label().to_string(),
        relation_weight: rel.weight,
        description: rel.description.clone(),
    })
}
// END_target_from_relationship

// START_CONTRACT_artifact_kind
// PURPOSE: Classify relationship endpoints that do not have graph nodes.
// INPUTS: { endpoint_id: &str }
// OUTPUTS: { &'static str }
// START_artifact_kind
fn artifact_kind(endpoint_id: &str) -> &'static str {
    if endpoint_id.starts_with("V-") {
        "verification"
    } else if endpoint_id.starts_with("UC-") {
        "use_case"
    } else if endpoint_id.starts_with("NFR-") {
        "requirement"
    } else {
        "artifact"
    }
}
// END_artifact_kind

// START_CONTRACT_affected_targets_by_prefix
// PURPOSE: Filter an impact target set by stable artifact id prefix.
// INPUTS: { targets: &[ImpactTarget] }, { prefix: &str }
// OUTPUTS: { Vec<ImpactTarget> }
// START_affected_targets_by_prefix
fn affected_targets_by_prefix(targets: &[ImpactTarget], prefix: &str) -> Vec<ImpactTarget> {
    let mut matches: Vec<ImpactTarget> = targets
        .iter()
        .filter(|target| target.node_id.starts_with(prefix))
        .cloned()
        .collect();
    sort_targets(&mut matches);
    matches
}
// END_affected_targets_by_prefix

// START_CONTRACT_score_risk
// PURPOSE: Score impact risk from dependent count and traceability-critical targets.
// INPUTS: { direct: &[ImpactTarget] }, { transitive: &[ImpactTarget] }, { verification: &[ImpactTarget] }, { use_cases: &[ImpactTarget] }
// OUTPUTS: { ImpactRisk }
// START_score_risk
fn score_risk(
    direct: &[ImpactTarget],
    transitive: &[ImpactTarget],
    verification: &[ImpactTarget],
    use_cases: &[ImpactTarget],
) -> ImpactRisk {
    let score = direct.len() * 2 + transitive.len() + verification.len() * 3 + use_cases.len() * 3;
    let level = if score >= CRITICAL_RISK_SCORE {
        "critical"
    } else if score >= HIGH_RISK_SCORE {
        "high"
    } else if score >= MEDIUM_RISK_SCORE {
        "medium"
    } else {
        "low"
    };

    let mut reasons = Vec::new();
    if !direct.is_empty() {
        reasons.push(format!("{} direct dependent(s)", direct.len()));
    }
    if !transitive.is_empty() {
        reasons.push(format!("{} transitive dependent(s)", transitive.len()));
    }
    if !verification.is_empty() {
        reasons.push(format!(
            "{} affected verification target(s)",
            verification.len()
        ));
    }
    if !use_cases.is_empty() {
        reasons.push(format!("{} affected use case(s)", use_cases.len()));
    }
    if reasons.is_empty() {
        reasons.push("No downstream impact detected".to_string());
    }

    ImpactRisk {
        level: level.to_string(),
        score,
        reasons,
    }
}
// END_score_risk

// START_CONTRACT_suggested_actions
// PURPOSE: Produce advisory next actions for callers based on impact shape.
// INPUTS: { risk: &ImpactRisk }, { verification: &[ImpactTarget] }, { use_cases: &[ImpactTarget] }
// OUTPUTS: { Vec<String> }
// START_suggested_actions
fn suggested_actions(
    risk: &ImpactRisk,
    verification: &[ImpactTarget],
    use_cases: &[ImpactTarget],
) -> Vec<String> {
    let mut actions = vec![
        "Review direct_dependents before editing the target module".to_string(),
        "Run module-local verification for the changed module".to_string(),
    ];
    if !verification.is_empty() {
        actions
            .push("Run affected verification shards listed in affected_verification".to_string());
    }
    if !use_cases.is_empty() {
        actions.push("Regenerate or inspect traceability for affected_use_cases".to_string());
    }
    if matches!(risk.level.as_str(), "high" | "critical") {
        actions.push("Run phase-level verify/review before release gating".to_string());
    }
    actions
}
// END_suggested_actions

// START_CONTRACT_is_test_target
// PURPOSE: Identify test and verification targets for include_tests filtering.
// INPUTS: { endpoint_id: &str }, { node: Option<&CodeNode> }
// OUTPUTS: { bool }
// START_is_test_target
fn is_test_target(endpoint_id: &str, node: Option<&CodeNode>) -> bool {
    let id = endpoint_id.to_ascii_lowercase();
    if id.starts_with("v-") || id.contains("test") || id.contains("verification") {
        return true;
    }
    node.is_some_and(|node| {
        let kind = node.kind.to_ascii_lowercase();
        let path = node.path.to_ascii_lowercase();
        kind.contains("test")
            || kind.contains("verification")
            || path.contains("/tests/")
            || path.ends_with("_test.rs")
    })
}
// END_is_test_target

// START_CONTRACT_dedupe_targets
// PURPOSE: Keep one deterministic target entry per node id.
// INPUTS: { targets: Vec<ImpactTarget> }
// OUTPUTS: { Vec<ImpactTarget> }
// START_dedupe_targets
fn dedupe_targets(targets: Vec<ImpactTarget>) -> Vec<ImpactTarget> {
    let mut by_id: BTreeMap<String, ImpactTarget> = BTreeMap::new();
    for target in targets {
        by_id
            .entry(target.node_id.clone())
            .and_modify(|existing| {
                if target.distance < existing.distance {
                    *existing = target.clone();
                }
            })
            .or_insert(target);
    }
    let mut targets: Vec<ImpactTarget> = by_id.into_values().collect();
    sort_targets(&mut targets);
    targets
}
// END_dedupe_targets

// START_CONTRACT_sort_targets
// PURPOSE: Sort targets by distance and id for stable MCP output.
// INPUTS: { targets: &mut [ImpactTarget] }
// SIDE_EFFECTS: mutates target ordering
// START_sort_targets
fn sort_targets(targets: &mut [ImpactTarget]) {
    targets.sort_by(|left, right| {
        left.distance
            .cmp(&right.distance)
            .then_with(|| left.node_id.cmp(&right.node_id))
            .then_with(|| left.relation_type.cmp(&right.relation_type))
    });
}
// END_sort_targets

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphrag::RelationType;

    fn node(id: &str, kind: &str, path: &str) -> CodeNode {
        CodeNode {
            id: id.to_string(),
            name: id.to_string(),
            kind: kind.to_string(),
            path: path.to_string(),
            language: "rust".to_string(),
            description: None,
            symbols: Vec::new(),
            size_lines: 10,
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }

    fn rel(source: &str, target: &str, relation_type: RelationType) -> CodeRelationship {
        CodeRelationship {
            source_id: source.to_string(),
            target_id: target.to_string(),
            relation_type,
            weight: 1.0,
            description: None,
        }
    }

    fn sample_graph() -> CodeGraph {
        let mut graph = CodeGraph::new();
        graph.add_node(node("M-CORE", "module", "src/core.rs"));
        graph.add_node(node("M-API", "module", "src/api.rs"));
        graph.add_node(node("M-UI", "module", "src/ui.rs"));
        graph.add_node(node(
            "V-M-CORE",
            "verification",
            "docs/verification/V-M-CORE.xml",
        ));
        graph.add_relationship(rel("M-API", "M-CORE", RelationType::Depends));
        graph.add_relationship(rel("M-UI", "M-API", RelationType::Depends));
        graph.add_relationship(rel("V-M-CORE", "M-CORE", RelationType::VerifiedBy));
        graph.add_relationship(rel("M-CORE", "UC-002", RelationType::Implements));
        graph
    }

    // START_CONTRACT_test_analyze_impact_reports_dependents_and_risk
    // PURPOSE: Verify impact analysis reports direct/transitive dependents, traceability targets, and risk.
    // START_test_analyze_impact_reports_dependents_and_risk
    #[test]
    fn test_analyze_impact_reports_dependents_and_risk() {
        let analysis = analyze_impact(&sample_graph(), "M-CORE", 3, true).expect("impact");

        assert_eq!(analysis.node_id, "M-CORE");
        assert!(analysis
            .direct_dependents
            .iter()
            .any(|target| target.node_id == "M-API"));
        assert!(analysis
            .transitive_dependents
            .iter()
            .any(|target| target.node_id == "M-UI"));
        assert_eq!(analysis.affected_verification[0].node_id, "V-M-CORE");
        assert_eq!(analysis.affected_use_cases[0].node_id, "UC-002");
        assert_eq!(analysis.risk.level, "high");
        assert!(analysis
            .suggested_actions
            .iter()
            .any(|action| action.contains("phase-level")));
    }
    // END_test_analyze_impact_reports_dependents_and_risk

    // START_CONTRACT_test_analyze_impact_filters_tests_when_requested
    // PURPOSE: Verify include_tests=false removes verification/test targets from impact output.
    // START_test_analyze_impact_filters_tests_when_requested
    #[test]
    fn test_analyze_impact_filters_tests_when_requested() {
        let analysis = analyze_impact(&sample_graph(), "M-CORE", 2, false).expect("impact");

        assert!(analysis
            .direct_dependents
            .iter()
            .all(|target| target.node_id != "V-M-CORE"));
        assert!(analysis.affected_verification.is_empty());
        assert!(analysis
            .direct_dependents
            .iter()
            .any(|target| target.node_id == "M-API"));
    }
    // END_test_analyze_impact_filters_tests_when_requested
}
