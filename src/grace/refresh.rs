// MODULE_CONTRACT
// MODULE_ID: M-GRACE-REFRESH
// PURPOSE: Artifact synchronization — detects and fixes drift between code contracts, cascade state, and MyGRACE shards
// SCOPE: Refresher struct, RefreshReport, canonical inventory-backed drift detection, cascade pending drift, and sync including generated DevelopmentPlan refresh
// DEPENDS: M-GRACE-CASCADE, M-GRACE-INVENTORY, M-GRACE-DEVELOPMENT-PLAN
// LINKS: docs/graph-index.xml, docs/verification-index.xml, docs/modules/, docs/verification/, docs/development-plan.xml, docs/cascade/

// START_MODULE_MAP
// RefreshReport — Drift detection report with suggested actions
// Refresher — Reports or fixes canonical MyGRACE artifact drift
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.23.0 - Made refresh --fix idempotent for clean canonical artifacts]
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
    pub cascade_drift: crate::grace::cascade::CascadeDriftReport,
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
    // PURPOSE: Repair canonical MyGRACE artifacts and generate DevelopmentPlan only when drift or invalid plan state exists
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<RefreshReport> }
    // SIDE_EFFECTS: writes docs/ indexes, shard files, or development-plan.xml only when repair is needed
    // START_refresher_fix
    pub fn fix(root: &Path) -> anyhow::Result<RefreshReport> {
        let initial_drift = MyGraceInventory::drift(root)?;
        let plan_valid = crate::grace::development_plan::validate_development_plan(root)
            .map(|report| report.valid)
            .unwrap_or(false);
        if initial_drift.is_clean() && plan_valid {
            return report_from_drift(root, initial_drift, false);
        }

        if !initial_drift.is_clean() {
            MyGraceInventory::sync(root)?;
        }
        if !plan_valid {
            crate::grace::development_plan::generate_development_plan_file(
                root,
                true,
                "auto",
                "topological",
            )?;
        }
        report_from_drift(root, MyGraceInventory::drift(root)?, true)
    }
    // END_refresher_fix
}
// END_public_api

fn report_from_drift(
    root: &Path,
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
    let cascade_drift = crate::grace::cascade::cascade_no_drift(root)?;
    let mut suggested_actions = drift.suggested_actions.clone();
    if !cascade_drift.passed {
        suggested_actions.push("Run cascade_execute for pending cascade markers".into());
    }

    Ok(RefreshReport {
        total_modules,
        in_graph,
        not_in_graph,
        in_graph_not_in_code,
        in_verification,
        not_in_verification,
        in_verification_not_in_code,
        contract_issues: drift.contract_issues.clone(),
        suggested_actions,
        fixed,
        canonical_drift: drift,
        cascade_drift,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_refresher_fix_idempotent_with_duplicate_module_contracts
    // PURPOSE: Verify refresh --fix coalesces duplicate MODULE_ID contracts and is a no-op on the second clean run
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temporary project files
    // START_test_refresher_fix_idempotent_with_duplicate_module_contracts
    #[test]
    fn test_refresher_fix_idempotent_with_duplicate_module_contracts() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/a.rs"),
            "// MODULE_CONTRACT\n// MODULE_ID: M-DUP\n// PURPOSE: Duplicate module part A\n// SCOPE: Part A source file\n// DEPENDS: M-A\n// LINKS: docs/modules/M-DUP.xml\n\n// START_MODULE_MAP\n// alpha - alpha work\n// END_MODULE_MAP\n\n// START_CHANGE_SUMMARY\n// LAST_CHANGE: [v1.0.0 - test]\n// END_CHANGE_SUMMARY\n\n// START_CONTRACT_alpha\n// PURPOSE: Alpha work\n// OUTPUTS: { () }\n// START_alpha\npub fn alpha() {}\n// END_alpha\n",
        )
        .unwrap();
        std::fs::write(
            root.join("src/b.rs"),
            "// MODULE_CONTRACT\n// MODULE_ID: M-DUP\n// PURPOSE: Duplicate module part B with longer canonical text\n// SCOPE: Part B source file\n// DEPENDS: M-B\n// LINKS: docs/modules/M-DUP.xml\n\n// START_MODULE_MAP\n// beta - beta work\n// END_MODULE_MAP\n\n// START_CHANGE_SUMMARY\n// LAST_CHANGE: [v1.0.0 - test]\n// END_CHANGE_SUMMARY\n\n// START_CONTRACT_beta\n// PURPOSE: Beta work\n// OUTPUTS: { () }\n// START_beta\npub fn beta() {}\n// END_beta\n",
        )
        .unwrap();

        let first = Refresher::fix(root).unwrap();
        assert!(first.canonical_drift.duplicate_graph_ids.is_empty());
        assert!(first.canonical_drift.duplicate_verification_ids.is_empty());

        let graph_path = root.join("docs/graph-index.xml");
        let verification_path = root.join("docs/verification-index.xml");
        let shard_path = root.join("docs/modules/M-DUP.xml");
        let plan_path = root.join("docs/development-plan.xml");
        let graph = std::fs::read_to_string(&graph_path).unwrap();
        let verification = std::fs::read_to_string(&verification_path).unwrap();
        let shard = std::fs::read_to_string(&shard_path).unwrap();
        let plan = std::fs::read_to_string(&plan_path).unwrap();

        assert_eq!(graph.matches(r#"<MODULE id="M-DUP""#).count(), 1);
        assert_eq!(
            verification
                .matches(r#"<VERIFICATION id="V-M-DUP" module="M-DUP""#)
                .count(),
            1
        );
        assert!(shard.contains("<FILE>src/a.rs</FILE>"));
        assert!(shard.contains("<FILE>src/b.rs</FILE>"));

        let second = Refresher::fix(root).unwrap();
        assert!(!second.fixed);
        assert_eq!(std::fs::read_to_string(graph_path).unwrap(), graph);
        assert_eq!(
            std::fs::read_to_string(verification_path).unwrap(),
            verification
        );
        assert_eq!(std::fs::read_to_string(shard_path).unwrap(), shard);
        assert_eq!(std::fs::read_to_string(plan_path).unwrap(), plan);
    }
    // END_test_refresher_fix_idempotent_with_duplicate_module_contracts
}
