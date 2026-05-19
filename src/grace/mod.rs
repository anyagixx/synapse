// MODULE_CONTRACT
// MODULE_ID: M-GRACE
// PURPOSE: GraceEngine facade — unified entry point for GRACE methodology tools (verify, review, inventory, semantic, refresh)
// SCOPE: Module declarations, GraceEngine struct, delegation to sub-modules
// DEPENDS: M-GRACE-BOOTSTRAP, M-GRACE-CONTRACT, M-GRACE-INVENTORY, M-GRACE-VERIFY, M-GRACE-REVIEW, M-GRACE-SEMANTIC, M-GRACE-REFRESH
// LINKS: N/A

// START_MODULE_MAP
// ModuleContract — Re-export from contract module
// GraceEngine — Facade for all GRACE methodology operations
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.5.0 — Added canonical inventory module]
// END_CHANGE_SUMMARY

pub mod bootstrap;
pub mod contract;
pub mod explain;
pub mod fix;
pub mod inventory;
pub mod layout;
pub mod refresh;
pub mod review;
pub mod semantic;
pub mod status;
pub mod verify;

use contract::ContractValidator;
use std::path::Path;
use verify::Verifier;

pub use contract::ModuleContract;

// START_public_api

// START_GraceEngine
pub struct GraceEngine;
// END_GraceEngine

impl GraceEngine {
    // START_CONTRACT_GraceEngine::new
    // PURPOSE: Create a new GraceEngine
    // OUTPUTS: { Self }
    // START_grace_engine_new
    pub fn new() -> Self {
        Self
    }
    // END_grace_engine_new

    // START_CONTRACT_GraceEngine::verify_project
    // PURPOSE: Run all 3 verification levels on a project
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<Vec<VerificationResult>> }
    // START_grace_engine_verify_project
    pub async fn verify_project(root: &Path) -> anyhow::Result<Vec<verify::VerificationResult>> {
        Verifier::verify_all(root).await
    }
    // END_grace_engine_verify_project

    // START_CONTRACT_GraceEngine::review_project
    // PURPOSE: Run GRACE integrity review
    // INPUTS: { root: &Path }, { mode: &str — scoped|wave-audit|full }
    // OUTPUTS: { anyhow::Result<ReviewReport> }
    // START_grace_engine_review_project
    pub async fn review_project(root: &Path, mode: &str) -> anyhow::Result<review::ReviewReport> {
        review::Reviewer::review(root, mode)
    }
    // END_grace_engine_review_project

    // START_CONTRACT_GraceEngine::contract_report
    // PURPOSE: Validate MODULE_CONTRACT blocks across all source files
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<ContractReport> }
    // START_grace_engine_contract_report
    pub fn contract_report(root: &Path) -> anyhow::Result<contract::ContractReport> {
        ContractValidator::validate_project(root)
    }
    // END_grace_engine_contract_report

    // START_CONTRACT_GraceEngine::semantic_report
    // PURPOSE: Scan project for semantic START/END blocks
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<SemanticReport> }
    // START_grace_engine_semantic_report
    pub fn semantic_report(root: &Path) -> anyhow::Result<semantic::SemanticReport> {
        semantic::SemanticExtractor::scan_project(root)
    }
    // END_grace_engine_semantic_report

    // START_CONTRACT_GraceEngine::refresh_project
    // PURPOSE: Sync knowledge graph and verification plan with code
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<RefreshReport> }
    // START_grace_engine_refresh_project
    pub fn refresh_project(root: &Path) -> anyhow::Result<refresh::RefreshReport> {
        refresh::Refresher::refresh(root)
    }
    // END_grace_engine_refresh_project
}

impl Default for GraceEngine {
    fn default() -> Self {
        Self::new()
    }
}
// END_public_api
