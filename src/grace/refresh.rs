// MODULE_CONTRACT
// MODULE_ID: M-GRACE-REFRESH
// PURPOSE: Artifact synchronization — detects drift between code and knowledge graph/verification plan
// SCOPE: Refresher struct, RefreshReport, knowledge graph parsing, verification plan parsing, drift detection
// DEPENDS: M-GRACE-CONTRACT
// LINKS: docs/knowledge-graph.xml, docs/verification-plan.xml

// START_MODULE_MAP
// RefreshReport — Drift detection report with suggested actions
// Refresher — Syncs code modules with KG and verification plan
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY

use crate::grace::contract::ContractValidator;
use std::path::Path;

// START_public_api

// START_RefreshReport
#[derive(Debug, Clone, serde::Serialize)]
pub struct RefreshReport {
    pub total_modules: usize,
    pub in_graph: usize,
    pub not_in_graph: Vec<String>,
    pub in_graph_not_in_code: Vec<String>,
    pub in_verification: usize,
    pub not_in_verification: Vec<String>,
    pub in_verification_not_in_code: Vec<String>,
    pub contract_issues: Vec<String>,
    pub suggested_actions: Vec<String>,
}
// END_RefreshReport

// START_Refresher
pub struct Refresher;
// END_Refresher

impl Default for Refresher {
    fn default() -> Self {
        Self::new()
    }
}

impl Refresher {
    // START_CONTRACT_Refresher::new
    // PURPOSE: Create a new Refresher
    // OUTPUTS: { Self }
    // START_refresher_new
    pub fn new() -> Self {
        Self
    }
    // END_refresher_new

    // START_CONTRACT_Refresher::refresh
    // PURPOSE: Sync knowledge graph and verification plan with code, detect drift
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<RefreshReport> }
    // START_refresher_refresh
    pub fn refresh(root: &Path) -> anyhow::Result<RefreshReport> {
        let contract_report = ContractValidator::validate_project(root)?;
        let mut report = RefreshReport {
            total_modules: 0,
            in_graph: 0,
            not_in_graph: Vec::new(),
            in_graph_not_in_code: Vec::new(),
            in_verification: 0,
            not_in_verification: Vec::new(),
            in_verification_not_in_code: Vec::new(),
            contract_issues: Vec::new(),
            suggested_actions: Vec::new(),
        };

        // Collect module IDs from source code
        let code_modules: Vec<&crate::grace::contract::ModuleContract> = contract_report
            .contracts
            .iter()
            .filter(|c| c.has_contract)
            .collect();
        report.total_modules = code_modules.len();

        // Read knowledge-graph.xml
        let kg_path = root.join("docs").join("knowledge-graph.xml");
        let kg_content = std::fs::read_to_string(&kg_path).unwrap_or_default();

        // Extract M-xxx node IDs from knowledge graph
        let graph_nodes: Vec<String> = {
            let re = regex::Regex::new(r#"<([A-Z]+-[A-Za-z0-9_]+)\b"#).unwrap();
            re.captures_iter(&kg_content)
                .filter_map(|c| {
                    let id = c[1].to_string();
                    if id.starts_with("M-") {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect()
        };

        // Check code modules against graph
        for mc in &code_modules {
            let mid = mc.module_id.as_deref().unwrap_or("?");
            if graph_nodes.contains(&mid.to_string()) {
                report.in_graph += 1;
            } else {
                report
                    .not_in_graph
                    .push(format!("{} ({})", mc.file_path, mid));
                report
                    .suggested_actions
                    .push(format!("Add {} to knowledge-graph.xml", mid));
            }
            // Contract issues
            for err in &mc.errors {
                report
                    .contract_issues
                    .push(format!("{}: {}", mc.file_path, err));
            }
        }

        // Check graph nodes against code (stale entries)
        for gn in &graph_nodes {
            let found = code_modules
                .iter()
                .any(|mc| mc.module_id.as_deref() == Some(gn.as_str()));
            if !found {
                report.in_graph_not_in_code.push(gn.clone());
                report
                    .suggested_actions
                    .push(format!("Remove stale {} from knowledge-graph.xml", gn));
            }
        }

        // Read verification-plan.xml
        let vp_path = root.join("docs").join("verification-plan.xml");
        let vp_content = std::fs::read_to_string(&vp_path).unwrap_or_default();

        let vp_modules: Vec<String> = {
            let re = regex::Regex::new(r#"MODULE="([^"]+)""#).unwrap();
            re.captures_iter(&vp_content)
                .map(|c| c[1].to_string())
                .collect()
        };

        // Check code modules against verification plan
        for mc in &code_modules {
            let mid = mc.module_id.as_deref().unwrap_or("?");
            if vp_modules.contains(&mid.to_string()) {
                report.in_verification += 1;
            } else if !mid.starts_with('?') {
                report
                    .not_in_verification
                    .push(format!("{} ({})", mc.file_path, mid));
                report
                    .suggested_actions
                    .push(format!("Add V-M-{} to verification-plan.xml", mid));
            }
        }

        // Check verification modules against code
        for vm in &vp_modules {
            let found = code_modules
                .iter()
                .any(|mc| mc.module_id.as_deref() == Some(vm.as_str()));
            if !found {
                report.in_verification_not_in_code.push(vm.clone());
                report.suggested_actions.push(format!(
                    "Remove stale V-M-{} from verification-plan.xml",
                    vm
                ));
            }
        }

        // Phase 0 check
        let phase0_files = [
            "requirements.xml",
            "technology.xml",
            "development-plan.xml",
            "verification-plan.xml",
            "knowledge-graph.xml",
        ];
        for f in &phase0_files {
            if !root.join("docs").join(f).exists() {
                report
                    .suggested_actions
                    .push(format!("Create docs/{} (Phase 0 incomplete)", f));
            }
        }

        report.suggested_actions.sort();
        report.suggested_actions.dedup();
        Ok(report)
    }
    // END_refresher_refresh
}
// END_public_api
