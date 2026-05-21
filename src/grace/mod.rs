// MODULE_CONTRACT
// MODULE_ID: M-GRACE
// PURPOSE: GraceEngine facade — unified entry point for GRACE methodology tools (verify, review, inventory, semantic, refresh, belief state, anchors, requirements, technology, development plan, mental tests, traceability, non-human patterns, agent-based testing, cascade updates)
// SCOPE: Module declarations, GraceEngine struct, profile-aware delegation to sub-modules, belief state/requirements/technology/development-plan/mental-test/traceability/non-human pattern/testing/cascade reporting, and anchor normalization export
// DEPENDS: M-GRACE-ANCHOR, M-GRACE-BOOTSTRAP, M-GRACE-BELIEF-STATE, M-GRACE-CASCADE, M-GRACE-CASCADE-CHANGE, M-GRACE-CONTRACT, M-GRACE-DEVELOPMENT-PLAN, M-GRACE-MENTAL-TEST, M-GRACE-TRACEABILITY, M-GRACE-NON-HUMAN-PATTERNS, M-GRACE-TESTING, M-GRACE-INVENTORY, M-GRACE-INVENTORY-ARTIFACTS, M-GRACE-INVENTORY-PLAN, M-GRACE-INVENTORY-TYPES, M-GRACE-INVENTORY-VERIFICATION, M-GRACE-LOG, M-GRACE-REQUIREMENTS, M-GRACE-TECHNOLOGY, M-GRACE-VERIFY, M-GRACE-VERIFY-PHASE, M-GRACE-VERIFY-TYPES, M-GRACE-REVIEW, M-GRACE-SEMANTIC, M-GRACE-REFRESH
// LINKS: N/A

// START_MODULE_MAP
// ModuleContract and GraceProfile — Re-exports from contract module
// GraceEngine — Facade for all GRACE methodology operations and belief state reports
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.22.0 - Registered cascade update modules]
// END_CHANGE_SUMMARY

pub mod anchor;
pub mod belief_state;
pub mod bootstrap;
pub mod cascade;
pub mod cascade_change;
pub mod contract;
pub mod development_plan;
pub mod explain;
pub mod fix;
pub mod inventory;
pub mod inventory_artifacts;
pub mod inventory_plan;
pub mod inventory_types;
pub mod inventory_verification;
pub mod layout;
pub mod log;
pub mod mental_test;
pub mod non_human_patterns;
pub mod refresh;
pub mod requirements;
pub mod review;
pub mod semantic;
pub mod status;
pub mod technology;
pub mod testing;
pub mod traceability;
pub mod verify;
pub mod verify_phase;
pub mod verify_types;

use contract::ContractValidator;
use std::path::Path;
use verify::Verifier;

pub use anchor::normalize_anchor_syntax;
pub use contract::{GraceProfile, ModuleContract};

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

    // START_CONTRACT_GraceEngine::verify_project_with_profile
    // PURPOSE: Run all verification levels using a selected GRACE strictness profile
    // INPUTS: { root: &Path — project root }, { profile: GraceProfile }
    // OUTPUTS: { anyhow::Result<Vec<VerificationResult>> }
    // START_grace_engine_verify_project_with_profile
    pub async fn verify_project_with_profile(
        root: &Path,
        profile: GraceProfile,
    ) -> anyhow::Result<Vec<verify::VerificationResult>> {
        Verifier::verify_all_with_profile(root, profile).await
    }
    // END_grace_engine_verify_project_with_profile

    // START_CONTRACT_GraceEngine::review_project
    // PURPOSE: Run GRACE integrity review
    // INPUTS: { root: &Path }, { mode: &str — scoped|wave-audit|full }
    // OUTPUTS: { anyhow::Result<ReviewReport> }
    // START_grace_engine_review_project
    pub async fn review_project(root: &Path, mode: &str) -> anyhow::Result<review::ReviewReport> {
        review::Reviewer::review(root, mode)
    }
    // END_grace_engine_review_project

    // START_CONTRACT_GraceEngine::review_project_with_profile
    // PURPOSE: Run GRACE integrity review with selected strictness profile
    // INPUTS: { root: &Path }, { mode: &str — scoped|wave-audit|full }, { profile: GraceProfile }
    // OUTPUTS: { anyhow::Result<ReviewReport> }
    // START_grace_engine_review_project_with_profile
    pub async fn review_project_with_profile(
        root: &Path,
        mode: &str,
        profile: GraceProfile,
    ) -> anyhow::Result<review::ReviewReport> {
        review::Reviewer::review_with_profile(root, mode, profile)
    }
    // END_grace_engine_review_project_with_profile

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

    // START_CONTRACT_GraceEngine::belief_state_report
    // PURPOSE: Scan project belief states and return coverage/validation report
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<BeliefStateReport> }
    // START_grace_engine_belief_state_report
    pub fn belief_state_report(root: &Path) -> anyhow::Result<belief_state::BeliefStateReport> {
        belief_state::scan_project_belief_states(root)
    }
    // END_grace_engine_belief_state_report

    // START_CONTRACT_GraceEngine::requirements_report
    // PURPOSE: Validate RequirementsAnalysis artifact and return completeness report
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<RequirementsReport> }
    // START_grace_engine_requirements_report
    pub fn requirements_report(root: &Path) -> anyhow::Result<requirements::RequirementsReport> {
        requirements::validate_requirements(root)
    }
    // END_grace_engine_requirements_report

    // START_CONTRACT_GraceEngine::technology_report
    // PURPOSE: Validate Technology artifact and return exact-version stack report
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<TechnologyReport> }
    // START_grace_engine_technology_report
    pub fn technology_report(root: &Path) -> anyhow::Result<technology::TechnologyReport> {
        technology::validate_technology(root)
    }
    // END_grace_engine_technology_report

    // START_CONTRACT_GraceEngine::development_plan_report
    // PURPOSE: Validate DevelopmentPlan artifact and return DataFlow/GenerationOrder report
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<DevelopmentPlanReport> }
    // START_grace_engine_development_plan_report
    pub fn development_plan_report(
        root: &Path,
    ) -> anyhow::Result<development_plan::DevelopmentPlanReport> {
        development_plan::validate_development_plan(root)
    }
    // END_grace_engine_development_plan_report

    // START_CONTRACT_GraceEngine::mental_test_report
    // PURPOSE: Scan DevelopmentPlan MentalTests and return pass/fail/coverage report
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<MentalTestReport> }
    // START_grace_engine_mental_test_report
    pub fn mental_test_report(root: &Path) -> anyhow::Result<mental_test::MentalTestReport> {
        mental_test::scan_project_mental_tests(root)
    }
    // END_grace_engine_mental_test_report

    // START_CONTRACT_GraceEngine::traceability_report
    // PURPOSE: Scan project traceability chains and return coverage, score, and gap report
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<TraceabilityReport> }
    // START_grace_engine_traceability_report
    pub fn traceability_report(root: &Path) -> anyhow::Result<traceability::TraceabilityReport> {
        traceability::scan_project_traceability(root)
    }
    // END_grace_engine_traceability_report

    // START_CONTRACT_GraceEngine::non_human_pattern_report
    // PURPOSE: Scan project source for profile-aware non-human programming pattern violations
    // INPUTS: { root: &Path }, { profile: GraceProfile }
    // OUTPUTS: { anyhow::Result<NonHumanPatternProjectReport> }
    // START_grace_engine_non_human_pattern_report
    pub fn non_human_pattern_report(
        root: &Path,
        profile: GraceProfile,
    ) -> anyhow::Result<non_human_patterns::NonHumanPatternProjectReport> {
        non_human_patterns::check_project_patterns(root, profile)
    }
    // END_grace_engine_non_human_pattern_report

    // START_CONTRACT_GraceEngine::run_test_guide
    // PURPOSE: Run a natural-language tester-agent guide and persist summary/failure artifacts
    // INPUTS: { root: &Path }, { guide_path: &str }, { application_url: &str }, { agent_console_url: Option<&str> }, { collect_logs: bool }, { output_report: bool }
    // OUTPUTS: { anyhow::Result<TestGuideRun> }
    // START_grace_engine_run_test_guide
    pub fn run_test_guide(
        root: &Path,
        guide_path: &str,
        application_url: &str,
        agent_console_url: Option<&str>,
        collect_logs: bool,
        output_report: bool,
    ) -> anyhow::Result<testing::TestGuideRun> {
        testing::run_test_guide(
            root,
            guide_path,
            application_url,
            agent_console_url,
            collect_logs,
            output_report,
        )
    }
    // END_grace_engine_run_test_guide

    // START_CONTRACT_GraceEngine::submit_test_report
    // PURPOSE: Deliver a tester-agent XML failure report summary to a developer-agent recipient
    // INPUTS: { root: &Path }, { report_path: &str }, { to: &str }
    // OUTPUTS: { anyhow::Result<TestReportSubmission> }
    // START_grace_engine_submit_test_report
    pub fn submit_test_report(
        root: &Path,
        report_path: &str,
        to: &str,
    ) -> anyhow::Result<testing::TestReportSubmission> {
        testing::submit_test_report(root, report_path, to)
    }
    // END_grace_engine_submit_test_report

    // START_CONTRACT_GraceEngine::run_mental_test
    // PURPOSE: Run one MentalTest by module and id, persisting a trace artifact
    // INPUTS: { root: &Path }, { module_id: &str }, { mental_test_id: &str }, { step_by_step: bool }
    // OUTPUTS: { anyhow::Result<MentalTestRunReport> }
    // START_grace_engine_run_mental_test
    pub fn run_mental_test(
        root: &Path,
        module_id: &str,
        mental_test_id: &str,
        step_by_step: bool,
    ) -> anyhow::Result<mental_test::MentalTestRunReport> {
        mental_test::run_mental_test(root, module_id, mental_test_id, step_by_step)
    }
    // END_grace_engine_run_mental_test

    // START_CONTRACT_GraceEngine::refresh_project
    // PURPOSE: Sync knowledge graph and verification plan with code
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<RefreshReport> }
    // START_grace_engine_refresh_project
    pub fn refresh_project(root: &Path) -> anyhow::Result<refresh::RefreshReport> {
        refresh::Refresher::refresh(root)
    }
    // END_grace_engine_refresh_project

    // START_CONTRACT_GraceEngine::cascade_impact
    // PURPOSE: Analyze downstream cascade impact for one changed artifact
    // INPUTS: { root: &Path }, { changed_artifact: &str }, { change_description: &str }
    // OUTPUTS: { anyhow::Result<ImpactAnalysis> }
    // START_grace_engine_cascade_impact
    pub fn cascade_impact(
        root: &Path,
        changed_artifact: &str,
        change_description: &str,
    ) -> anyhow::Result<cascade::ImpactAnalysis> {
        cascade::cascade_impact(root, changed_artifact, change_description)
    }
    // END_grace_engine_cascade_impact

    // START_CONTRACT_GraceEngine::cascade_execute
    // PURPOSE: Execute a cached cascade preview and persist proposals/changelog
    // INPUTS: { root: &Path }, { options: cascade::CascadeExecuteOptions }
    // OUTPUTS: { anyhow::Result<CascadeReport> }
    // START_grace_engine_cascade_execute
    pub fn cascade_execute(
        root: &Path,
        options: cascade::CascadeExecuteOptions,
    ) -> anyhow::Result<cascade::CascadeReport> {
        cascade::cascade_execute(root, options)
    }
    // END_grace_engine_cascade_execute
}

impl Default for GraceEngine {
    fn default() -> Self {
        Self::new()
    }
}
// END_public_api
