// MODULE_CONTRACT
// MODULE_ID: M-GRACE-REFRESH
// PURPOSE: Artifact synchronization — detects and fixes drift between code contracts and MyGRACE shards
// SCOPE: Refresher struct, RefreshReport, canonical inventory-backed drift detection and sync including generated DevelopmentPlan refresh
// DEPENDS: M-GRACE-INVENTORY, M-GRACE-DEVELOPMENT-PLAN
// LINKS: docs/graph-index.xml, docs/verification-index.xml, docs/modules/, docs/verification/, docs/development-plan.xml

// START_MODULE_MAP
// RefreshReport — Drift detection report with suggested actions
// Refresher — Reports or fixes canonical MyGRACE artifact drift
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.6.0 — Refresh DevelopmentPlan from source contracts during --fix]
// END_CHANGE_SUMMARY

use crate::grace::inventory::{ArtifactDrift, MyGraceInventory};
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
    pub fixed: bool,
    pub canonical_drift: ArtifactDrift,
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
    // PURPOSE: Detect drift between code contracts and canonical MyGRACE artifacts
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<RefreshReport> }
    // START_refresher_refresh
    pub fn refresh(root: &Path) -> anyhow::Result<RefreshReport> {
        report_from_drift(root, MyGraceInventory::drift(root)?, false)
    }
    // END_refresher_refresh

    // START_CONTRACT_Refresher::fix
    // PURPOSE: Rewrite canonical MyGRACE artifacts and DevelopmentPlan from real source MODULE_ID contracts
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<RefreshReport> }
    // SIDE_EFFECTS: writes docs/ indexes and shard files
    // START_refresher_fix
    pub fn fix(root: &Path) -> anyhow::Result<RefreshReport> {
        let drift = MyGraceInventory::sync(root)?;
        crate::grace::development_plan::generate_development_plan_file(
            root,
            true,
            "auto",
            "topological",
        )?;
        report_from_drift(root, drift, true)
    }
    // END_refresher_fix
}
// END_public_api

fn report_from_drift(
    _root: &Path,
    drift: ArtifactDrift,
    fixed: bool,
) -> anyhow::Result<RefreshReport> {
    let total_modules = drift.total_code_modules;
    let not_in_graph = drift.code_not_in_graph.clone();
    let not_in_verification = drift.code_not_in_verification.clone();
    let in_graph_not_in_code = drift.graph_not_in_code.clone();
    let in_verification_not_in_code = drift.verification_not_in_code.clone();
    let in_graph = total_modules.saturating_sub(not_in_graph.len());
    let in_verification = total_modules.saturating_sub(not_in_verification.len());

    Ok(RefreshReport {
        total_modules,
        in_graph,
        not_in_graph,
        in_graph_not_in_code,
        in_verification,
        not_in_verification,
        in_verification_not_in_code,
        contract_issues: drift.contract_issues.clone(),
        suggested_actions: drift.suggested_actions.clone(),
        fixed,
        canonical_drift: drift,
    })
}
