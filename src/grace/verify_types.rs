// MODULE_CONTRACT
// MODULE_ID: M-GRACE-VERIFY-TYPES
// PURPOSE: Verification result types shared by MyGRACE verification modules
// SCOPE: VerificationResult and CheckResult data structures
// DEPENDS: N/A
// LINKS: docs/modules/M-GRACE-VERIFY.xml

// START_MODULE_MAP
// VerificationResult — Per-level verification result with check list
// CheckResult — Single verification check result
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.8.0 — Extracted verification result types from M-GRACE-VERIFY]
// END_CHANGE_SUMMARY

// START_public_api

// START_VerificationResult
#[derive(Debug, Clone, serde::Serialize)]
pub struct VerificationResult {
    pub level: String,
    pub passed: bool,
    pub checks: Vec<CheckResult>,
}
// END_VerificationResult

// START_CheckResult
#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub details: String,
}
// END_CheckResult

// START_CONTRACT_public_api
// PURPOSE: Export verification result data structures
// OUTPUTS: { VerificationResult, CheckResult }
// END_public_api
