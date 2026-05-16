pub mod contract;
pub mod explain;
pub mod fix;
pub mod review;
pub mod semantic;
pub mod status;
pub mod verify;

use contract::ContractValidator;
use std::path::Path;
use verify::Verifier;

pub use contract::ModuleContract;

pub struct GraceEngine;

impl Default for GraceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GraceEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn verify_project(root: &Path) -> anyhow::Result<Vec<verify::VerificationResult>> {
        Verifier::verify_all(root).await
    }

    pub async fn review_project(root: &Path, mode: &str) -> anyhow::Result<review::ReviewReport> {
        review::Reviewer::review(root, mode)
    }

    pub fn contract_report(root: &Path) -> anyhow::Result<contract::ContractReport> {
        ContractValidator::validate_project(root)
    }

    pub fn semantic_report(root: &Path) -> anyhow::Result<semantic::SemanticReport> {
        semantic::SemanticExtractor::scan_project(root)
    }
}
