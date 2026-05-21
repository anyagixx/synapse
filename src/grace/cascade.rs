// MODULE_CONTRACT
// MODULE_ID: M-GRACE-CASCADE
// PURPOSE: Cascade update engine for GRACE artifact impact analysis, preview, execution, and drift checks
// SCOPE: ImpactAnalysis, affected artifact traversal, preview caching, cascade execution proposals, and cascade-no-drift reporting
// DEPENDS: M-GRACE-CASCADE-CHANGE, M-GRACE-CONTRACT, M-GRACE-DEVELOPMENT-PLAN, M-GRACE-INVENTORY, M-GRACE-LAYOUT, M-GRACE-TRACEABILITY
// LINKS:
//   -> V-M-GRACE-CASCADE (verified_by) - impact, preview, execution, and drift tests

// START_MODULE_MAP
// ImpactLevel - Direct, indirect, contractual, or code impact labels
// AffectedModule - One downstream artifact affected by a cascade trigger
// ImpactAnalysis - Full cascade impact report and preview text
// CascadeReport - Cascade execution report with changelog path
// cascade_impact - Analyze and cache downstream impact for one changed artifact
// cascade_execute - Execute a cached cascade preview and write changelog/proposals
// cascade_no_drift - Verify no explicit pending cascades remain unexecuted
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added cascade impact analysis and execution engine]
// END_CHANGE_SUMMARY

use crate::grace::cascade_change::{self, CascadeChange};
use crate::grace::inventory::MyGraceInventory;
use crate::grace::layout::DocsLayout;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

const CASCADE_SCAN_LIMIT: usize = 256;
const HIGH_EFFORT_AFFECTED_THRESHOLD: usize = 12;
const HIGH_EFFORT_CODE_THRESHOLD: usize = 5;
const MEDIUM_EFFORT_AFFECTED_THRESHOLD: usize = 3;
const MEDIUM_EFFORT_CODE_THRESHOLD: usize = 1;

// START_public_api

// START_ImpactLevel
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImpactLevel {
    Direct,
    Indirect,
    Contractual,
    Code,
}
// END_ImpactLevel

impl ImpactLevel {
    // START_CONTRACT_ImpactLevel::as_str
    // PURPOSE: Return the stable lowercase impact label for reports
    // OUTPUTS: { &'static str }
    // START_impact_level_as_str
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Indirect => "indirect",
            Self::Contractual => "contractual",
            Self::Code => "code",
        }
    }
    // END_impact_level_as_str
}

// START_AffectedModule
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AffectedModule {
    pub module_id: String,
    pub impact_level: ImpactLevel,
    pub change_required: String,
    pub distance: usize,
    pub path: Vec<String>,
}
// END_AffectedModule

// START_ImpactAnalysis
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImpactAnalysis {
    pub cascade_id: String,
    pub changed_artifact: String,
    pub change_type: String,
    pub change_description: String,
    pub affected_modules: Vec<AffectedModule>,
    pub total_affected: usize,
    pub estimated_effort: String,
    pub preview: String,
}
// END_ImpactAnalysis

// START_CascadeExecuteOptions
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CascadeExecuteOptions {
    pub cascade_id: String,
    pub auto_apply_contracts: bool,
    pub auto_apply_code: bool,
    pub apply_to_phases: Vec<String>,
}
// END_CascadeExecuteOptions

// START_CascadeReport
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CascadeReport {
    pub cascade_id: String,
    pub changed_artifact: String,
    pub applied_changes: usize,
    pub proposed_changes: usize,
    pub blocked_changes: usize,
    pub pending_review: bool,
    pub changelog_path: Option<String>,
    pub proposal_path: Option<String>,
    pub drift_issues: Vec<String>,
}
// END_CascadeReport

// START_CascadeDriftReport
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct CascadeDriftReport {
    pub passed: bool,
    pub pending_cascades: usize,
    pub changelog_count: usize,
    pub issues: Vec<String>,
}
// END_CascadeDriftReport

// START_CONTRACT_cascade_impact
// PURPOSE: Analyze downstream impact for a changed artifact and cache a preview for optional execution
// INPUTS: { root: &Path }, { changed_artifact: &str }, { change_description: &str }
// OUTPUTS: { anyhow::Result<ImpactAnalysis> }
// SIDE_EFFECTS: writes docs/cascade/previews and docs/cascade/last-impact.xml
// START_cascade_impact
pub fn cascade_impact(
    root: &Path,
    changed_artifact: &str,
    change_description: &str,
) -> anyhow::Result<ImpactAnalysis> {
    let graph = CascadeGraph::from_project(root)?;
    let cascade_id = cascade_id(changed_artifact, change_description);
    let mut affected_modules = graph.reachable(changed_artifact);
    affected_modules.sort_by(|left, right| {
        left.distance
            .cmp(&right.distance)
            .then_with(|| left.module_id.cmp(&right.module_id))
    });
    let total_affected = affected_modules.len();
    let estimated_effort = estimated_effort(&affected_modules);
    let mut analysis = ImpactAnalysis {
        cascade_id,
        changed_artifact: changed_artifact.to_string(),
        change_type: infer_change_type(changed_artifact),
        change_description: change_description.to_string(),
        affected_modules,
        total_affected,
        estimated_effort,
        preview: String::new(),
    };
    analysis.preview = format_cascade_preview(&analysis);
    write_preview(root, &analysis)?;
    Ok(analysis)
}
// END_cascade_impact

// START_CONTRACT_cascade_execute
// PURPOSE: Execute a cached cascade preview by writing contract/code proposals and a changelog
// INPUTS: { root: &Path }, { options: CascadeExecuteOptions }
// OUTPUTS: { anyhow::Result<CascadeReport> }
// SIDE_EFFECTS: writes docs/cascade/proposals and docs/cascade/changelogs
// START_cascade_execute
pub fn cascade_execute(
    root: &Path,
    options: CascadeExecuteOptions,
) -> anyhow::Result<CascadeReport> {
    let analysis = read_preview(root, &options.cascade_id)?;
    let mut changes = Vec::new();
    let mut applied_changes = 0usize;
    let mut proposed_changes = 0usize;
    let mut blocked_changes = 0usize;

    for affected in &analysis.affected_modules {
        let change_type = artifact_change_type(&affected.module_id);
        let action = cascade_action(
            &change_type,
            options.auto_apply_contracts,
            options.auto_apply_code,
        );
        match action.as_str() {
            "updated" | "regenerated" => applied_changes += 1,
            "proposed" => proposed_changes += 1,
            _ => blocked_changes += 1,
        }
        changes.push(CascadeChange {
            artifact: affected.module_id.clone(),
            change_type,
            action,
            before: current_artifact_summary(root, &affected.module_id),
            after: proposed_artifact_summary(&analysis, affected),
            verification_status: "pending".into(),
        });
    }

    let proposal_path = write_proposals(root, &analysis, &options, &changes)?;
    let log = cascade_change::new_change_log(
        &analysis.cascade_id,
        &analysis.changed_artifact,
        &analysis.change_description,
        "synapse",
        changes,
    );
    let changelog_path = cascade_change::write_changelog(root, &log)?;
    clear_pending_marker(root, &analysis.cascade_id)?;
    let drift = cascade_no_drift(root)?;

    Ok(CascadeReport {
        cascade_id: analysis.cascade_id,
        changed_artifact: analysis.changed_artifact,
        applied_changes,
        proposed_changes,
        blocked_changes,
        pending_review: proposed_changes > 0 || blocked_changes > 0,
        changelog_path: Some(changelog_path.display().to_string()),
        proposal_path: Some(proposal_path.display().to_string()),
        drift_issues: drift.issues,
    })
}
// END_cascade_execute

// START_CONTRACT_format_cascade_preview
// PURPOSE: Render a compact human-readable cascade preview
// INPUTS: { analysis: &ImpactAnalysis }
// OUTPUTS: { String }
// START_format_cascade_preview
pub fn format_cascade_preview(analysis: &ImpactAnalysis) -> String {
    let mut output = String::new();
    output.push_str("CASCADE PREVIEW\n");
    output.push_str("---------------\n");
    output.push_str(&format!(
        "Cascade: {}\nChange: {} ({}) - {}\n\n",
        analysis.cascade_id,
        analysis.changed_artifact,
        analysis.change_type,
        analysis.change_description
    ));
    output.push_str("Impact chain:\n");
    output.push_str(&format!("  {} (modified)\n", analysis.changed_artifact));
    if analysis.affected_modules.is_empty() {
        output.push_str("  no downstream artifacts detected\n");
    }
    for affected in &analysis.affected_modules {
        output.push_str(&format!(
            "  -> {} [{} / {} / distance {}]\n",
            affected.module_id,
            affected.impact_level.as_str(),
            affected.change_required,
            affected.distance
        ));
        output.push_str(&format!("     path: {}\n", affected.path.join(" -> ")));
    }
    let direct = analysis
        .affected_modules
        .iter()
        .filter(|item| item.impact_level == ImpactLevel::Direct)
        .count();
    let code = analysis
        .affected_modules
        .iter()
        .filter(|item| item.impact_level == ImpactLevel::Code)
        .count();
    output.push_str(&format!(
        "\nAffected: {} total, {} direct, {} code artifacts\nEstimated effort: {}\n",
        analysis.total_affected, direct, code, analysis.estimated_effort
    ));
    output
}
// END_format_cascade_preview

// START_CONTRACT_cascade_no_drift
// PURPOSE: Verify no explicit pending cascade markers remain without changelog execution
// INPUTS: { root: &Path }
// OUTPUTS: { anyhow::Result<CascadeDriftReport> }
// START_cascade_no_drift
pub fn cascade_no_drift(root: &Path) -> anyhow::Result<CascadeDriftReport> {
    let layout = DocsLayout::new(root);
    let pending = list_pending_cascades(&layout.cascade_pending_dir());
    let history = cascade_change::list_changelog_entries(root)?;
    let issues: Vec<String> = pending
        .iter()
        .map(|id| format!("Pending cascade {} has no recorded execution changelog", id))
        .collect();
    Ok(CascadeDriftReport {
        passed: issues.is_empty(),
        pending_cascades: pending.len(),
        changelog_count: history.changelog_count,
        issues,
    })
}
// END_cascade_no_drift

// END_public_api

// START_CascadeGraph
struct CascadeGraph {
    edges: BTreeMap<String, Vec<CascadeEdge>>,
    artifact_types: BTreeMap<String, String>,
}
// END_CascadeGraph

// START_CascadeEdge
#[derive(Debug, Clone)]
struct CascadeEdge {
    target: String,
    relationship: String,
}
// END_CascadeEdge

impl CascadeGraph {
    // START_CONTRACT_CascadeGraph::from_project
    // PURPOSE: Build a downstream artifact graph from traceability chains, module dependencies, and DataFlows
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<CascadeGraph> }
    // START_cascade_graph_from_project
    fn from_project(root: &Path) -> anyhow::Result<Self> {
        let mut graph = Self {
            edges: BTreeMap::new(),
            artifact_types: BTreeMap::new(),
        };
        graph.collect_traceability_edges(root)?;
        graph.collect_inventory_edges(root)?;
        graph.collect_plan_edges(root)?;
        Ok(graph)
    }
    // END_cascade_graph_from_project

    // START_CONTRACT_CascadeGraph::collect_traceability_edges
    // PURPOSE: Add requirement/use-case to implementation and module-to-function links
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<()> }
    // START_collect_traceability_edges
    fn collect_traceability_edges(&mut self, root: &Path) -> anyhow::Result<()> {
        let report = crate::grace::traceability::scan_project_traceability(root)?;
        for chain in report.chains {
            self.artifact_types.insert(
                chain.artifact_id.clone(),
                chain.artifact_type.as_str().into(),
            );
            for link in chain.traced_by {
                self.artifact_types
                    .entry(link.target_id.clone())
                    .or_insert(link.target_type.clone());
                self.add_edge(&chain.artifact_id, &link.target_id, &link.relationship);
            }
            for link in chain.traces_to {
                if link.relationship == "contains" {
                    self.artifact_types
                        .entry(link.target_id.clone())
                        .or_insert(link.target_type.clone());
                    self.add_edge(&chain.artifact_id, &link.target_id, &link.relationship);
                }
            }
        }
        Ok(())
    }
    // END_collect_traceability_edges

    // START_CONTRACT_CascadeGraph::collect_inventory_edges
    // PURPOSE: Add reverse module dependency edges so dependency changes reach dependents
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<()> }
    // START_collect_inventory_edges
    fn collect_inventory_edges(&mut self, root: &Path) -> anyhow::Result<()> {
        let inventory = MyGraceInventory::collect(root)?;
        for module in inventory.code_modules {
            self.artifact_types
                .insert(module.id.clone(), "module".into());
            for dep in module.depends {
                self.artifact_types
                    .entry(dep.clone())
                    .or_insert("module".into());
                self.add_edge(&dep, &module.id, "depended_on_by");
            }
        }
        Ok(())
    }
    // END_collect_inventory_edges

    // START_CONTRACT_CascadeGraph::collect_plan_edges
    // PURPOSE: Add DataFlow source-to-target and contract-module-to-target impact edges
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<()> }
    // START_collect_plan_edges
    fn collect_plan_edges(&mut self, root: &Path) -> anyhow::Result<()> {
        let plan = crate::grace::development_plan::validate_development_plan(root)?;
        for flow in plan.data_flows {
            if is_artifact_id(&flow.from) && is_artifact_id(&flow.to) {
                self.add_edge(&flow.from, &flow.to, "dataflow");
            }
            if is_artifact_id(&flow.contract_module) && is_artifact_id(&flow.to) {
                self.add_edge(&flow.contract_module, &flow.to, "contract_dataflow");
            }
        }
        Ok(())
    }
    // END_collect_plan_edges

    // START_CONTRACT_CascadeGraph::add_edge
    // PURPOSE: Add one deduplicated directed downstream edge
    // INPUTS: { source: &str }, { target: &str }, { relationship: &str }
    // OUTPUTS: { () }
    // START_cascade_graph_add_edge
    fn add_edge(&mut self, source: &str, target: &str, relationship: &str) {
        if source.trim().is_empty() || target.trim().is_empty() || source == target {
            return;
        }
        let edges = self.edges.entry(source.to_string()).or_default();
        if !edges
            .iter()
            .any(|edge| edge.target == target && edge.relationship == relationship)
        {
            edges.push(CascadeEdge {
                target: target.to_string(),
                relationship: relationship.to_string(),
            });
        }
    }
    // END_cascade_graph_add_edge

    // START_CONTRACT_CascadeGraph::reachable
    // PURPOSE: Traverse downstream artifacts reachable from one changed artifact
    // INPUTS: { changed_artifact: &str }
    // OUTPUTS: { Vec<AffectedModule> }
    // START_cascade_graph_reachable
    fn reachable(&self, changed_artifact: &str) -> Vec<AffectedModule> {
        let mut affected = Vec::new();
        let mut queue = VecDeque::new();
        let mut visited = BTreeSet::new();
        visited.insert(changed_artifact.to_string());
        queue.push_back((
            changed_artifact.to_string(),
            vec![changed_artifact.to_string()],
            0usize,
        ));

        while let Some((source, path, distance)) = queue.pop_front() {
            if distance > CASCADE_SCAN_LIMIT {
                break;
            }
            let Some(edges) = self.edges.get(&source) else {
                continue;
            };
            for edge in edges {
                if !visited.insert(edge.target.clone()) {
                    continue;
                }
                let mut next_path = path.clone();
                next_path.push(edge.target.clone());
                let next_distance = distance + 1;
                affected.push(self.affected_from_node(
                    &edge.target,
                    next_distance,
                    next_path.clone(),
                ));
                queue.push_back((edge.target.clone(), next_path, next_distance));
            }
        }
        affected
    }
    // END_cascade_graph_reachable

    // START_CONTRACT_CascadeGraph::affected_from_node
    // PURPOSE: Convert a reachable artifact node into an affected module report row
    // INPUTS: { node: &str }, { distance: usize }, { path: Vec<String> }
    // OUTPUTS: { AffectedModule }
    // START_affected_from_node
    fn affected_from_node(&self, node: &str, distance: usize, path: Vec<String>) -> AffectedModule {
        let artifact_type = self
            .artifact_types
            .get(node)
            .map(String::as_str)
            .unwrap_or_else(|| infer_artifact_type(node));
        let impact_level = impact_level(node, artifact_type, distance);
        let change_required = change_required(node, artifact_type, &impact_level);
        AffectedModule {
            module_id: node.to_string(),
            impact_level,
            change_required,
            distance,
            path,
        }
    }
    // END_affected_from_node
}

// START_CONTRACT_write_preview
// PURPOSE: Persist an impact preview for later cascade_execute lookup
// INPUTS: { root: &Path }, { analysis: &ImpactAnalysis }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes docs/cascade/previews and last-impact XML
// START_write_preview
fn write_preview(root: &Path, analysis: &ImpactAnalysis) -> anyhow::Result<()> {
    let layout = DocsLayout::new(root);
    std::fs::create_dir_all(layout.cascade_previews_dir())?;
    std::fs::create_dir_all(layout.cascade_dir())?;
    std::fs::write(
        layout
            .cascade_previews_dir()
            .join(format!("{}.json", analysis.cascade_id)),
        serde_json::to_string_pretty(analysis)?,
    )?;
    std::fs::write(
        layout.cascade_last_impact_path(),
        render_last_impact_xml(analysis),
    )?;
    Ok(())
}
// END_write_preview

// START_CONTRACT_read_preview
// PURPOSE: Load a cached cascade preview by id
// INPUTS: { root: &Path }, { cascade_id: &str }
// OUTPUTS: { anyhow::Result<ImpactAnalysis> }
// START_read_preview
fn read_preview(root: &Path, cascade_id: &str) -> anyhow::Result<ImpactAnalysis> {
    let layout = DocsLayout::new(root);
    let path = layout
        .cascade_previews_dir()
        .join(format!("{}.json", cascade_id));
    let content = std::fs::read_to_string(&path)
        .map_err(|error| anyhow::anyhow!("cascade preview {} not found: {}", cascade_id, error))?;
    Ok(serde_json::from_str(&content)?)
}
// END_read_preview

// START_CONTRACT_write_proposals
// PURPOSE: Persist contract/code proposal XML for one executed cascade
// INPUTS: { root: &Path }, { analysis: &ImpactAnalysis }, { options: &CascadeExecuteOptions }, { changes: &[CascadeChange] }
// OUTPUTS: { anyhow::Result<PathBuf> }
// SIDE_EFFECTS: writes docs/cascade/proposals
// START_write_proposals
fn write_proposals(
    root: &Path,
    analysis: &ImpactAnalysis,
    options: &CascadeExecuteOptions,
    changes: &[CascadeChange],
) -> anyhow::Result<PathBuf> {
    let layout = DocsLayout::new(root);
    std::fs::create_dir_all(layout.cascade_proposals_dir())?;
    let path = layout
        .cascade_proposals_dir()
        .join(format!("{}.xml", analysis.cascade_id));
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<CascadeProposal id=\"{}\" trigger=\"{}\">\n",
        xml_attr(&analysis.cascade_id),
        xml_attr(&analysis.changed_artifact)
    );
    xml.push_str(&format!(
        "  <Options auto_apply_contracts=\"{}\" auto_apply_code=\"{}\" phases=\"{}\" />\n",
        options.auto_apply_contracts,
        options.auto_apply_code,
        xml_attr(&options.apply_to_phases.join(","))
    ));
    xml.push_str("  <Changes>\n");
    for change in changes {
        xml.push_str(&format!(
            "    <Change artifact=\"{}\" type=\"{}\" action=\"{}\" />\n",
            xml_attr(&change.artifact),
            xml_attr(&change.change_type),
            xml_attr(&change.action)
        ));
    }
    xml.push_str("  </Changes>\n</CascadeProposal>\n");
    std::fs::write(&path, xml)?;
    Ok(path)
}
// END_write_proposals

// START_CONTRACT_clear_pending_marker
// PURPOSE: Remove explicit pending cascade marker files after execution
// INPUTS: { root: &Path }, { cascade_id: &str }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: removes docs/cascade/pending marker files for the cascade id
// START_clear_pending_marker
fn clear_pending_marker(root: &Path, cascade_id: &str) -> anyhow::Result<()> {
    let layout = DocsLayout::new(root);
    for ext in ["json", "xml"] {
        let path = layout
            .cascade_pending_dir()
            .join(format!("{}.{}", cascade_id, ext));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}
// END_clear_pending_marker

// START_CONTRACT_list_pending_cascades
// PURPOSE: Return explicit pending cascade marker ids
// INPUTS: { pending_dir: &Path }
// OUTPUTS: { Vec<String> }
// START_list_pending_cascades
fn list_pending_cascades(pending_dir: &Path) -> Vec<String> {
    let mut ids: Vec<String> = std::fs::read_dir(pending_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok()))
        .filter_map(|entry| {
            let path = entry.path();
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("json") | Some("xml") => path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string()),
                _ => None,
            }
        })
        .collect();
    ids.sort();
    ids.dedup();
    ids
}
// END_list_pending_cascades

// START_CONTRACT_render_last_impact_xml
// PURPOSE: Render the latest cascade impact preview for dashboard/status consumers
// INPUTS: { analysis: &ImpactAnalysis }
// OUTPUTS: { String }
// START_render_last_impact_xml
fn render_last_impact_xml(analysis: &ImpactAnalysis) -> String {
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<CascadeImpact id=\"{}\" artifact=\"{}\" type=\"{}\" effort=\"{}\">\n",
        xml_attr(&analysis.cascade_id),
        xml_attr(&analysis.changed_artifact),
        xml_attr(&analysis.change_type),
        xml_attr(&analysis.estimated_effort)
    );
    xml.push_str(&format!(
        "  <Summary total_affected=\"{}\" />\n  <Affected>\n",
        analysis.total_affected
    ));
    for affected in &analysis.affected_modules {
        xml.push_str(&format!(
            "    <Artifact id=\"{}\" impact=\"{}\" required=\"{}\" distance=\"{}\" />\n",
            xml_attr(&affected.module_id),
            affected.impact_level.as_str(),
            xml_attr(&affected.change_required),
            affected.distance
        ));
    }
    xml.push_str("  </Affected>\n</CascadeImpact>\n");
    xml
}
// END_render_last_impact_xml

// START_CONTRACT_cascade_id
// PURPOSE: Build a stable short cascade id from trigger artifact and change description
// INPUTS: { changed_artifact: &str }, { change_description: &str }
// OUTPUTS: { String }
// START_cascade_id
fn cascade_id(changed_artifact: &str, change_description: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(changed_artifact.as_bytes());
    hash.update(b"\0");
    hash.update(change_description.as_bytes());
    let digest = hash.finalize();
    let short: String = digest
        .iter()
        .take(4)
        .map(|byte| format!("{:02X}", byte))
        .collect();
    format!("CSC-{}", short)
}
// END_cascade_id

// START_CONTRACT_infer_change_type
// PURPOSE: Infer cascade change type from the changed artifact id
// INPUTS: { changed_artifact: &str }
// OUTPUTS: { String }
// START_infer_change_type
fn infer_change_type(changed_artifact: &str) -> String {
    if changed_artifact.starts_with("REQ-")
        || changed_artifact.starts_with("NFR-")
        || changed_artifact.starts_with("UC-")
        || changed_artifact.starts_with("G-")
    {
        "requirement".into()
    } else if changed_artifact.contains("::") {
        "implementation".into()
    } else if changed_artifact.starts_with("M-") {
        "contract".into()
    } else {
        "interface".into()
    }
}
// END_infer_change_type

// START_CONTRACT_infer_artifact_type
// PURPOSE: Infer artifact kind from an id when traceability metadata is absent
// INPUTS: { artifact_id: &str }
// OUTPUTS: { &'static str }
// START_infer_artifact_type
fn infer_artifact_type(artifact_id: &str) -> &'static str {
    if artifact_id.contains("::") {
        "function"
    } else if artifact_id.starts_with("M-") {
        "module"
    } else if artifact_id.starts_with("V-") {
        "verification"
    } else if artifact_id.starts_with("UC-") {
        "use_case"
    } else if artifact_id.starts_with("REQ-") || artifact_id.starts_with("NFR-") {
        "requirement"
    } else {
        "artifact"
    }
}
// END_infer_artifact_type

// START_CONTRACT_impact_level
// PURPOSE: Classify one reachable artifact impact level
// INPUTS: { node: &str }, { artifact_type: &str }, { distance: usize }
// OUTPUTS: { ImpactLevel }
// START_impact_level
fn impact_level(node: &str, artifact_type: &str, distance: usize) -> ImpactLevel {
    if node.contains("::") || matches!(artifact_type, "function" | "code_block" | "log_entry") {
        ImpactLevel::Code
    } else if matches!(artifact_type, "contract" | "verification") {
        ImpactLevel::Contractual
    } else if distance == 1 {
        ImpactLevel::Direct
    } else {
        ImpactLevel::Indirect
    }
}
// END_impact_level

// START_CONTRACT_change_required
// PURPOSE: Map artifact type and impact level to the required downstream action
// INPUTS: { node: &str }, { artifact_type: &str }, { impact_level: &ImpactLevel }
// OUTPUTS: { String }
// START_change_required
fn change_required(node: &str, artifact_type: &str, impact_level: &ImpactLevel) -> String {
    if node.contains("::") || matches!(artifact_type, "function" | "code_block") {
        "code_regeneration".into()
    } else if matches!(artifact_type, "verification") {
        "verification_review".into()
    } else if matches!(impact_level, ImpactLevel::Indirect) {
        "verification_review".into()
    } else {
        "contract_review".into()
    }
}
// END_change_required

// START_CONTRACT_estimated_effort
// PURPOSE: Estimate cascade effort from affected artifact count and code impact count
// INPUTS: { affected: &[AffectedModule] }
// OUTPUTS: { String }
// START_estimated_effort
fn estimated_effort(affected: &[AffectedModule]) -> String {
    let code_count = affected
        .iter()
        .filter(|item| item.impact_level == ImpactLevel::Code)
        .count();
    if affected.len() > HIGH_EFFORT_AFFECTED_THRESHOLD || code_count > HIGH_EFFORT_CODE_THRESHOLD {
        "high".into()
    } else if affected.len() > MEDIUM_EFFORT_AFFECTED_THRESHOLD
        || code_count > MEDIUM_EFFORT_CODE_THRESHOLD
    {
        "medium".into()
    } else {
        "low".into()
    }
}
// END_estimated_effort

// START_CONTRACT_artifact_change_type
// PURPOSE: Return changelog change type for one affected artifact id
// INPUTS: { artifact_id: &str }
// OUTPUTS: { String }
// START_artifact_change_type
fn artifact_change_type(artifact_id: &str) -> String {
    if artifact_id.contains("::") {
        "code".into()
    } else if artifact_id.starts_with("V-") {
        "verification".into()
    } else {
        "contract".into()
    }
}
// END_artifact_change_type

// START_CONTRACT_cascade_action
// PURPOSE: Decide whether a change is applied, regenerated, proposed, or blocked
// INPUTS: { change_type: &str }, { auto_apply_contracts: bool }, { auto_apply_code: bool }
// OUTPUTS: { String }
// START_cascade_action
fn cascade_action(change_type: &str, auto_apply_contracts: bool, auto_apply_code: bool) -> String {
    match change_type {
        "contract" if auto_apply_contracts => "updated".into(),
        "code" if auto_apply_code => "regenerated".into(),
        "contract" | "code" | "verification" => "proposed".into(),
        _ => "blocked".into(),
    }
}
// END_cascade_action

// START_CONTRACT_current_artifact_summary
// PURPOSE: Return a compact before-state summary for changelog output
// INPUTS: { root: &Path }, { artifact_id: &str }
// OUTPUTS: { String }
// START_current_artifact_summary
fn current_artifact_summary(root: &Path, artifact_id: &str) -> String {
    let module_id = artifact_id.split("::").next().unwrap_or(artifact_id);
    if module_id.starts_with("M-") {
        let path = DocsLayout::new(root)
            .modules_dir()
            .join(format!("{}.xml", module_id));
        return std::fs::read_to_string(path)
            .ok()
            .and_then(|content| tag_value(&content, "PURPOSE"))
            .unwrap_or_else(|| "module shard present; detailed contract unchanged".into());
    }
    "artifact summary unavailable".into()
}
// END_current_artifact_summary

// START_CONTRACT_proposed_artifact_summary
// PURPOSE: Return a compact after-state summary for changelog output
// INPUTS: { analysis: &ImpactAnalysis }, { affected: &AffectedModule }
// OUTPUTS: { String }
// START_proposed_artifact_summary
fn proposed_artifact_summary(analysis: &ImpactAnalysis, affected: &AffectedModule) -> String {
    format!(
        "Review {} because {} changed: {}",
        affected.change_required, analysis.changed_artifact, analysis.change_description
    )
}
// END_proposed_artifact_summary

// START_CONTRACT_is_artifact_id
// PURPOSE: Return true for ids that are useful cascade graph nodes
// INPUTS: { value: &str }
// OUTPUTS: { bool }
// START_is_artifact_id
fn is_artifact_id(value: &str) -> bool {
    value.starts_with("M-")
        || value.starts_with("REQ-")
        || value.starts_with("NFR-")
        || value.starts_with("UC-")
        || value.contains("::")
}
// END_is_artifact_id

// START_CONTRACT_tag_value
// PURPOSE: Extract one simple XML tag value from content
// INPUTS: { content: &str }, { tag: &str }
// OUTPUTS: { Option<String> }
// START_tag_value
fn tag_value(content: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = content.find(&open)? + open.len();
    let end = content[start..].find(&close)? + start;
    Some(content[start..end].to_string())
}
// END_tag_value

// START_CONTRACT_xml_text
// PURPOSE: Escape XML text node content
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_xml_text
fn xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
// END_xml_text

// START_CONTRACT_xml_attr
// PURPOSE: Escape XML attribute content
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_xml_attr
fn xml_attr(value: &str) -> String {
    xml_text(value).replace('"', "&quot;")
}
// END_xml_attr

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_write_cascade_fixture
    // PURPOSE: Create a minimal traceable MyGRACE project for cascade tests
    // INPUTS: { root: &Path }
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temporary docs and source files
    // START_write_cascade_fixture
    fn write_cascade_fixture(root: &Path) {
        std::fs::create_dir_all(root.join("src")).expect("src dir");
        std::fs::create_dir_all(root.join("docs/modules")).expect("modules dir");
        std::fs::create_dir_all(root.join("docs/verification")).expect("verification dir");
        std::fs::write(
            root.join("docs/requirements.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<RequirementsAnalysis>
  <Goals><Goal id="G-001">Ship orders</Goal></Goals>
  <Entities><Entity name="Order" /></Entities>
  <UseCases><UseCase id="UC-001"><Title>Place Order</Title></UseCase></UseCases>
  <Requirements><Requirement id="REQ-001">Order placement works</Requirement></Requirements>
  <NonFunctionalRequirements><Requirement id="NFR-001">Deterministic</Requirement></NonFunctionalRequirements>
  <Constraints><Constraint id="CON-001">Local only</Constraint></Constraints>
  <Glossary><Term name="Order">Purchase request</Term></Glossary>
</RequirementsAnalysis>
"#,
        )
        .expect("requirements");
        std::fs::write(
            root.join("docs/development-plan.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<DevelopmentPlan project="test" version="1.0">
  <ArchitectureGraph>
    <Module id="M-ORDER-SERVICE"><Link ref="UC-001" type="implements" /></Module>
    <Module id="M-INVENTORY-SERVICE"></Module>
  </ArchitectureGraph>
  <DataFlows>
    <DataFlow id="DF-001" from="M-ORDER-SERVICE" to="M-INVENTORY-SERVICE" data="order" protocol="function" contract="M-ORDER-SERVICE::place_order"><ErrorHandling>return Result</ErrorHandling></DataFlow>
  </DataFlows>
  <GenerationOrder>
    <Module id="M-INVENTORY-SERVICE" phase="Phase-1" order="1"></Module>
    <Module id="M-ORDER-SERVICE" phase="Phase-1" order="2"><DependsOn>M-INVENTORY-SERVICE</DependsOn></Module>
  </GenerationOrder>
  <MentalTests><MentalTest id="MT-001" module="M-ORDER-SERVICE" status="passed"></MentalTest></MentalTests>
  <NonHumanPatterns><Pattern name="ExplicitFlow" severity="error"><Rule>return Result</Rule></Pattern></NonHumanPatterns>
  <ContractGuidelines><Guideline name="ErrorCoverage"><Rule>cover errors</Rule></Guideline></ContractGuidelines>
</DevelopmentPlan>
"#,
        )
        .expect("development plan");
        let start_module_map = ["START", "MODULE", "MAP"].join("_");
        let end_module_map = ["END", "MODULE", "MAP"].join("_");
        let start_change_summary = ["START", "CHANGE", "SUMMARY"].join("_");
        let end_change_summary = ["END", "CHANGE", "SUMMARY"].join("_");
        let order_source = r#"// MODULE_CONTRACT
// MODULE_ID: M-ORDER-SERVICE
// PURPOSE: Order placement service
// SCOPE: Order validation and placement
// DEPENDS: M-INVENTORY-SERVICE
// LINKS:
//   -> UC-001 (implements) - place order use case

// __START_MODULE_MAP__
// place_order - Places an order
// __END_MODULE_MAP__

// __START_CHANGE_SUMMARY__
// LAST_CHANGE: [v1.0.0 - Initial fixture]
// __END_CHANGE_SUMMARY__

// START_CONTRACT_place_order
// PURPOSE: Place one order
// INPUTS: { id: &str - order id }
// OUTPUTS: { bool - success }
// LINKS:
//   -> UC-001 (implements) - place order use case
// START_place_order
pub fn place_order(id: &str) -> bool {
    !id.is_empty()
}
// END_place_order
"#
        .replace("__START_MODULE_MAP__", &start_module_map)
        .replace("__END_MODULE_MAP__", &end_module_map)
        .replace("__START_CHANGE_SUMMARY__", &start_change_summary)
        .replace("__END_CHANGE_SUMMARY__", &end_change_summary);
        std::fs::write(root.join("src/order.rs"), order_source).expect("source");
        let inventory_source = r#"// MODULE_CONTRACT
// MODULE_ID: M-INVENTORY-SERVICE
// PURPOSE: Inventory service
// SCOPE: Inventory checks
// DEPENDS:
// LINKS:

// __START_MODULE_MAP__
// reserve_inventory - Reserves stock
// __END_MODULE_MAP__

// __START_CHANGE_SUMMARY__
// LAST_CHANGE: [v1.0.0 - Initial fixture]
// __END_CHANGE_SUMMARY__

// START_CONTRACT_reserve_inventory
// PURPOSE: Reserve inventory
// OUTPUTS: { bool - success }
// START_reserve_inventory
pub fn reserve_inventory() -> bool {
    true
}
// END_reserve_inventory
"#
        .replace("__START_MODULE_MAP__", &start_module_map)
        .replace("__END_MODULE_MAP__", &end_module_map)
        .replace("__START_CHANGE_SUMMARY__", &start_change_summary)
        .replace("__END_CHANGE_SUMMARY__", &end_change_summary);
        std::fs::write(root.join("src/inventory.rs"), inventory_source).expect("source");
        MyGraceInventory::sync(root).expect("sync fixture");
    }
    // END_write_cascade_fixture

    // START_CONTRACT_test_requirement_impact_analysis_finds_module_and_function
    // PURPOSE: Verify requirement impact analysis reaches implementing module and function artifacts
    // OUTPUTS: { () }
    // START_test_requirement_impact_analysis_finds_module_and_function
    #[test]
    fn test_requirement_impact_analysis_finds_module_and_function() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_cascade_fixture(dir.path());
        let analysis =
            cascade_impact(dir.path(), "UC-001", "Add address verification").expect("impact");
        let ids: BTreeSet<_> = analysis
            .affected_modules
            .iter()
            .map(|item| item.module_id.as_str())
            .collect();
        assert!(ids.contains("M-ORDER-SERVICE"));
        assert!(ids.contains("M-ORDER-SERVICE::place_order"));
        assert!(analysis.total_affected >= 2);
    }
    // END_test_requirement_impact_analysis_finds_module_and_function

    // START_CONTRACT_test_unrelated_artifact_has_no_spurious_impact
    // PURPOSE: Verify an unrelated changed artifact produces no false downstream impacts
    // OUTPUTS: { () }
    // START_test_unrelated_artifact_has_no_spurious_impact
    #[test]
    fn test_unrelated_artifact_has_no_spurious_impact() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_cascade_fixture(dir.path());
        let analysis = cascade_impact(dir.path(), "UC-999", "Unknown flow").expect("impact");
        assert_eq!(analysis.total_affected, 0);
    }
    // END_test_unrelated_artifact_has_no_spurious_impact

    // START_CONTRACT_test_preview_chain_formatting
    // PURPOSE: Verify preview text includes the cascade id, impact chain, and effort summary
    // OUTPUTS: { () }
    // START_test_preview_chain_formatting
    #[test]
    fn test_preview_chain_formatting() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_cascade_fixture(dir.path());
        let analysis =
            cascade_impact(dir.path(), "UC-001", "Add address verification").expect("impact");
        assert!(analysis.preview.contains("CASCADE PREVIEW"));
        assert!(analysis.preview.contains("Impact chain"));
        assert!(analysis.preview.contains("Estimated effort"));
        assert!(dir.path().join("docs/cascade/last-impact.xml").exists());
    }
    // END_test_preview_chain_formatting

    // START_CONTRACT_test_execute_cascade_creates_changelog
    // PURPOSE: Verify executing a cached cascade writes proposal and changelog artifacts
    // OUTPUTS: { () }
    // START_test_execute_cascade_creates_changelog
    #[test]
    fn test_execute_cascade_creates_changelog() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_cascade_fixture(dir.path());
        let analysis =
            cascade_impact(dir.path(), "UC-001", "Add address verification").expect("impact");
        let report = cascade_execute(
            dir.path(),
            CascadeExecuteOptions {
                cascade_id: analysis.cascade_id,
                auto_apply_contracts: true,
                auto_apply_code: false,
                apply_to_phases: vec!["Phase-1".into()],
            },
        )
        .expect("execute");
        assert!(report
            .changelog_path
            .as_deref()
            .is_some_and(|path| path.contains("CSC-")));
        assert!(report
            .proposal_path
            .as_deref()
            .is_some_and(|path| path.contains("CSC-")));
        assert!(report.applied_changes > 0);
    }
    // END_test_execute_cascade_creates_changelog

    // START_CONTRACT_test_cascade_no_drift_detects_pending_marker
    // PURPOSE: Verify cascade-no-drift reports explicit pending cascade markers
    // OUTPUTS: { () }
    // START_test_cascade_no_drift_detects_pending_marker
    #[test]
    fn test_cascade_no_drift_detects_pending_marker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = DocsLayout::new(dir.path());
        std::fs::create_dir_all(layout.cascade_pending_dir()).expect("pending dir");
        std::fs::write(layout.cascade_pending_dir().join("CSC-PENDING.json"), "{}")
            .expect("pending marker");
        let drift = cascade_no_drift(dir.path()).expect("drift");
        assert!(!drift.passed);
        assert_eq!(drift.pending_cascades, 1);
    }
    // END_test_cascade_no_drift_detects_pending_marker
}
