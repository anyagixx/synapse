// MODULE_CONTRACT
// MODULE_ID: M-GRACE-STATUS
// PURPOSE: Project health collector — aggregates contracts, requirements, technology, development plan, mental tests, traceability, cascade state, non-human patterns, belief states, semantic, verification, drift, token economy, RTK adoption, phase state, and system info
// SCOPE: StatusCollector, StatusReport, TokenEconomy, RtkAdoption, SystemInfo, requirements/technology selected-or-pending state, development-plan/mental-test/traceability/cascade/non-human pattern and belief-state coverage, print_report, active-phase display helpers
// DEPENDS: M-GRACE-BELIEF-STATE, M-GRACE-CASCADE, M-GRACE-CONTRACT, M-GRACE-DEVELOPMENT-PLAN, M-GRACE-MENTAL-TEST, M-GRACE-TRACEABILITY, M-GRACE-NON-HUMAN-PATTERNS, M-GRACE-REQUIREMENTS, M-GRACE-TECHNOLOGY, M-GRACE-SEMANTIC, M-GRACE-VERIFY, M-GRACE-REFRESH, M-TRACKING, M-CONFIG
// LINKS: docs/

// START_MODULE_MAP
// StatusReport — Full project health report with requirements, technology, development plan, belief state, and RTK adoption coverage
// TokenEconomy — Token usage statistics
// RtkAdoption — Advisory token-saving adoption diagnostics
// SystemInfo — System metadata (version, paths)
// StatusCollector — Collects and prints project health
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.25.0 - Display pending technology decision status]
// END_CHANGE_SUMMARY

use syn_core::config::Config;
use crate::grace::belief_state::BeliefStateReport;

/// Count of RTK discovery capabilities (discover + learn).
/// Mirrors syn_cli::capabilities::RTK_DISCOVERY_CAPABILITIES.len().
const RTK_DISCOVERY_COUNT: usize = 2;
use crate::grace::cascade::CascadeDriftReport;
use crate::grace::contract::{ContractReport, ContractValidator};
use crate::grace::development_plan::DevelopmentPlanReport;
use crate::grace::layout::DocsLayout;
use crate::grace::mental_test::MentalTestReport;
use crate::grace::non_human_patterns::NonHumanPatternProjectReport;
use crate::grace::refresh::Refresher;
use crate::grace::requirements::RequirementsReport;
use crate::grace::semantic::{SemanticExtractor, SemanticReport};
use crate::grace::technology::TechnologyReport;
use crate::grace::traceability::TraceabilityReport;
use crate::grace::verify::Verifier;
use syn_core::tracking::Tracker;
use std::path::Path;

const PHASE_ZERO_COMPLETION_ACTION: &str =
    "Complete sharded Phase 0: create graph/plan/verification indexes and shard directories";

// START_public_api

// START_StatusReport
#[derive(Debug, serde::Serialize)]
pub struct StatusReport {
    pub health: String,
    pub mygrace_issues: usize,
    pub contracts: ContractReport,
    pub requirements: RequirementsReport,
    pub technology: TechnologyReport,
    pub development_plan: DevelopmentPlanReport,
    pub mental_tests: MentalTestReport,
    pub traceability: TraceabilityReport,
    pub cascade: CascadeDriftReport,
    pub non_human_patterns: NonHumanPatternProjectReport,
    pub belief_state: BeliefStateReport,
    pub semantic: SemanticReport,
    pub verification: Vec<super::verify::VerificationResult>,
    pub drift: Option<super::refresh::RefreshReport>,
    pub token_economy: TokenEconomy,
    pub rtk_adoption: RtkAdoption,
    pub system: SystemInfo,
    pub next_actions: Vec<String>,
}
// END_StatusReport

// START_TokenEconomy
#[derive(Debug, serde::Serialize)]
pub struct TokenEconomy {
    pub total_commands: u64,
    pub total_saved: u64,
    pub avg_savings_pct: f64,
    pub db_size: String,
}
// END_TokenEconomy

// START_RtkAdoption
#[derive(Debug, serde::Serialize)]
pub struct RtkAdoption {
    pub discovery_capabilities: usize,
    pub hooks_installed: bool,
    pub tracked_commands: u64,
    pub gaps: Vec<String>,
}
// END_RtkAdoption

// START_SystemInfo
#[derive(Debug, serde::Serialize)]
pub struct SystemInfo {
    pub version: String,
    pub config_path: String,
    pub data_path: String,
    pub project_path: String,
}
// END_SystemInfo

// START_StatusCollector
pub struct StatusCollector;
// END_StatusCollector

impl Default for StatusCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusCollector {
    // START_CONTRACT_StatusCollector::new
    // PURPOSE: Create a new StatusCollector
    // OUTPUTS: { Self }
    // START_sc_new
    pub fn new() -> Self {
        Self
    }
    // END_sc_new

    // START_CONTRACT_StatusCollector::collect
    // PURPOSE: Collect full project health report
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<StatusReport> }
    // START_sc_collect
    pub async fn collect(root: &Path) -> anyhow::Result<StatusReport> {
        let contracts = ContractValidator::validate_project(root)?;
        let requirements = crate::grace::requirements::validate_requirements(root)?;
        let technology = crate::grace::technology::validate_technology(root)?;
        let development_plan = crate::grace::development_plan::validate_development_plan(root)?;
        let mental_tests = crate::grace::mental_test::scan_project_mental_tests(root)?;
        let traceability = crate::grace::traceability::scan_project_traceability(root)?;
        let cascade = crate::grace::cascade::cascade_no_drift(root)?;
        let non_human_patterns = crate::grace::non_human_patterns::check_project_patterns(
            root,
            crate::grace::GraceProfile::Strict,
        )?;
        let belief_state = crate::grace::belief_state::scan_project_belief_states(root)?;
        let semantic = SemanticExtractor::scan_project(root)?;
        let verification = Verifier::verify_all(root).await?;
        let drift = Refresher::refresh(root).ok();

        let config = Config::load_or_default();
        let tracker = Tracker::new(&config);
        let stats = tracker.get_stats().await?;
        let rtk_adoption = collect_rtk_adoption(root, stats.total_commands);

        let db_path = Tracker::db_path().unwrap_or_default();
        let db_size = std::fs::metadata(&db_path)
            .map(|m| format!("{:.1} KB", m.len() as f64 / 1024.0))
            .unwrap_or_else(|_| "N/A".into());

        let mut next_actions = Vec::new();
        if contracts.with_contract == 0 {
            next_actions.push("Add MODULE_CONTRACT to source files".into());
        }
        if contracts.invalid > 0 {
            next_actions.push(format!("Fix {} invalid contracts", contracts.invalid));
        }
        if !requirements.valid {
            next_actions.push(format!(
                "Complete RequirementsAnalysis: {} issues",
                requirements.errors.len()
            ));
        }
        if !technology.valid {
            next_actions.push(format!(
                "Complete Technology stack: {} issues",
                technology.errors.len()
            ));
        }
        if !development_plan.valid {
            next_actions.push(format!(
                "Complete DevelopmentPlan: {} issues",
                development_plan.errors.len()
            ));
        }
        if !mental_tests.mental_tests_defined() {
            next_actions.push(format!(
                "Add mental tests for {} critical targets",
                mental_tests.missing_required_targets.len()
            ));
        } else if !mental_tests.mental_tests_passed() {
            next_actions.push(format!(
                "Pass mental tests before code generation: failed={} not_run={}",
                mental_tests.failed, mental_tests.not_run
            ));
        } else if !mental_tests.mental_test_no_drift() {
            next_actions.push(format!(
                "Resolve {} mental test drift issues",
                mental_tests.drift_issues.len()
            ));
        }
        if !traceability.requirements_implemented_gate() {
            next_actions.push(format!(
                "Trace {} unimplemented requirements before strict release",
                traceability.untraced_requirements.len()
            ));
        }
        if !traceability.code_traced_gate() {
            next_actions.push(format!(
                "Add traceability links for {} untraced function contracts",
                traceability.untraced_functions.len()
            ));
        } else if traceability.enforcement_mode != "strict"
            && (!traceability.untraced_requirements.is_empty()
                || !traceability.untraced_functions.is_empty())
        {
            next_actions.push(format!(
                "Traceability is advisory: score {:.1}% with {} gaps",
                traceability.traceability_score * 100.0,
                traceability.gaps.len()
            ));
        }
        if !traceability.no_dangling_gate() {
            next_actions.push("Fix dangling traceability links".into());
        }
        if !cascade.passed {
            next_actions.push(format!(
                "Execute {} pending cascades before release",
                cascade.pending_cascades
            ));
        }
        if non_human_patterns.error_violations > 0 {
            next_actions.push(format!(
                "Fix {} blocking non-human pattern violations",
                non_human_patterns.error_violations
            ));
        } else if non_human_patterns.warning_violations > 0 {
            next_actions.push(format!(
                "Review {} advisory non-human pattern warnings",
                non_human_patterns.warning_violations
            ));
        }
        if belief_state.invalid_states > 0 {
            next_actions.push(format!(
                "Fix {} invalid BELIEF_STATE blocks",
                belief_state.invalid_states
            ));
        } else if belief_state.total_modules > 0
            && belief_state.states_found < belief_state.total_modules
        {
            next_actions.push(format!(
                "Add belief states for {} modules ({:.1}% coverage)",
                belief_state.missing_modules.len(),
                belief_state.coverage_pct
            ));
        }
        if !semantic.duplicate_name_blocks.is_empty() {
            next_actions.push(format!(
                "Fix {} duplicate block names",
                semantic.duplicate_name_blocks.len()
            ));
        }
        for gap in &rtk_adoption.gaps {
            next_actions.push(format!("RTK adoption: {gap}"));
        }

        let all_verify_pass = verification.iter().all(|v| v.passed);
        if !all_verify_pass {
            next_actions.push("Run 'syn verify' to see failing checks".into());
        }

        let layout = DocsLayout::new(root);
        let phase0_done = layout.graph_index_path().exists()
            && layout.plan_index_path().exists()
            && layout.verification_index_path().exists()
            && layout.modules_dir().exists()
            && layout.phases_dir().exists()
            && layout.verification_dir().exists();
        if !phase0_done {
            next_actions.push(PHASE_ZERO_COMPLETION_ACTION.into());
        }

        let graph_index = std::fs::read_to_string(layout.graph_index_path()).unwrap_or_default();
        let plan_index = std::fs::read_to_string(layout.plan_index_path()).unwrap_or_default();
        let verification_index =
            std::fs::read_to_string(layout.verification_index_path()).unwrap_or_default();
        let module_shards = std::fs::read_dir(layout.modules_dir())
            .map(|r| r.count())
            .unwrap_or(0);
        let phase_shards = std::fs::read_dir(layout.phases_dir())
            .map(|r| r.count())
            .unwrap_or(0);
        let verification_shards = std::fs::read_dir(layout.verification_dir())
            .map(|r| r.count())
            .unwrap_or(0);
        if graph_index.contains("status=\"planned\"") || plan_index.contains("status=\"active\"") {
            next_actions.push(format!(
                "Sharded docs loaded: {} module shards, {} phase shards, {} verification shards",
                module_shards, phase_shards, verification_shards
            ));
        }
        if verification_index.contains("priority=\"critical\"") {
            next_actions
                .push("Critical verification shards present in verification-index.xml".into());
        }

        if graph_index.is_empty() || plan_index.is_empty() || verification_index.is_empty() {
            next_actions.push(
                "Populate primary sharded indexes with module, phase, and verification refs".into(),
            );
        }

        next_actions.push(format!(
            "{} | shard coverage: graph={} plan={} verification={} modules={} phases={} verifications={}",
            active_phase_label(&plan_index),
            u8::from(layout.graph_index_path().exists()),
            u8::from(layout.plan_index_path().exists()),
            u8::from(layout.verification_index_path().exists()),
            module_shards,
            phase_shards,
            verification_shards
        ));

        let mygrace_issues = drift
            .as_ref()
            .map(|d| d.canonical_drift.issue_count())
            .unwrap_or(0);
        if let Some(ref d) = drift {
            if !d.canonical_drift.is_clean() {
                next_actions
                    .push("Run 'syn refresh --fix' to sync canonical MyGRACE artifacts".into());
            }
        }
        let health = if !all_verify_pass || mygrace_issues > 0 {
            "failing"
        } else if contracts.invalid > 0 || !semantic.duplicate_name_blocks.is_empty() {
            "degraded"
        } else {
            "healthy"
        }
        .to_string();

        Ok(StatusReport {
            health,
            mygrace_issues,
            token_economy: TokenEconomy {
                total_commands: stats.total_commands,
                total_saved: stats.total_saved_tokens,
                avg_savings_pct: stats.avg_savings_pct,
                db_size,
            },
            rtk_adoption,
            system: SystemInfo {
                version: crate::VERSION.into(),
                config_path: Config::path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                data_path: db_path.display().to_string(),
                project_path: root.display().to_string(),
            },
            contracts,
            requirements,
            technology,
            development_plan,
            mental_tests,
            traceability,
            cascade,
            non_human_patterns,
            belief_state,
            semantic,
            verification,
            drift,
            next_actions,
        })
    }
    // END_sc_collect

    // START_CONTRACT_StatusCollector::print_report
    // PURPOSE: Print formatted health report to stdout
    // INPUTS: { report: &StatusReport }
    // OUTPUTS: prints to stdout
    // SIDE_EFFECTS: prints to stdout
    // START_sc_print_report
    pub fn print_report(report: &StatusReport) {
        let project = std::path::Path::new(&report.system.project_path);
        let layout = DocsLayout::new(project);
        let module_shards = std::fs::read_dir(layout.modules_dir())
            .map(|it| it.filter_map(|e| e.ok()).count())
            .unwrap_or(0);
        let phase_shards = std::fs::read_dir(layout.phases_dir())
            .map(|it| it.filter_map(|e| e.ok()).count())
            .unwrap_or(0);
        let verification_shards = std::fs::read_dir(layout.verification_dir())
            .map(|it| it.filter_map(|e| e.ok()).count())
            .unwrap_or(0);
        let active_phase = std::fs::read_to_string(layout.plan_index_path())
            .map(|content| active_phase_table_value(&content))
            .unwrap_or_else(|_| "unknown".into());

        println!("╔══════════════════════════════════════╗");
        println!("║       Synapse Project Health        ║");
        println!("╠══════════════════════════════════════╣");
        println!("║ Version: {:<33}║", report.system.version);
        println!("║ Health:  {:<33}║", report.health);
        println!("║ MyGRACE issues: {:<24}║", report.mygrace_issues);
        println!("║ Project: {:<32}║", report.system.project_path);
        println!("╠══════════════════════════════════════╣");
        println!("║ CONTRACTS                            ║");
        println!(
            "║  Files with contract:    {:<12}║",
            report.contracts.with_contract
        );
        println!(
            "║  Files without contract: {:<12}║",
            report.contracts.without_contract
        );
        println!("║  Valid contracts:       {:<12}║", report.contracts.valid);
        println!(
            "║  Invalid contracts:     {:<12}║",
            report.contracts.invalid
        );
        let with_map: usize = report
            .contracts
            .contracts
            .iter()
            .filter(|c| c.has_module_map)
            .count();
        let with_cs: usize = report
            .contracts
            .contracts
            .iter()
            .filter(|c| c.has_change_summary)
            .count();
        let total_fn: usize = report
            .contracts
            .contracts
            .iter()
            .map(|c| c.function_contracts.len())
            .sum();
        println!("║  With MODULE_MAP:       {:<12}║", with_map);
        println!("║  With CHANGE_SUMMARY:   {:<12}║", with_cs);
        println!("║  Function contracts:   {:<12}║", total_fn);
        println!("╠══════════════════════════════════════╣");
        println!("║ REQUIREMENTS                         ║");
        println!(
            "║  Entities:       {:<20}║",
            report.requirements.entities.len()
        );
        println!(
            "║  Use cases:      {:<20}║",
            report.requirements.use_cases.len()
        );
        println!("║  Valid:          {:<20}║", report.requirements.valid);
        println!("╠══════════════════════════════════════╣");
        println!("║ TECHNOLOGY                           ║");
        println!("║  Status:         {:<20}║", report.technology.status);
        println!(
            "║  Components:     {:<20}║",
            report.technology.components.len()
        );
        println!(
            "║  Compatibility:  {:<20}║",
            report.technology.compatibility_checks.len()
        );
        println!("║  Valid:          {:<20}║", report.technology.valid);
        println!("╠══════════════════════════════════════╣");
        println!("║ DEVELOPMENT PLAN                     ║");
        println!(
            "║  DataFlows:      {:<20}║",
            report.development_plan.data_flows.len()
        );
        println!(
            "║  Gen modules:    {:<20}║",
            report.development_plan.generation_modules.len()
        );
        println!(
            "║  Completed:      {:<20}║",
            report.development_plan.completed_generation_modules
        );
        println!("║  Valid:          {:<20}║", report.development_plan.valid);
        println!("╠══════════════════════════════════════╣");
        println!("║ MENTAL TESTS                         ║");
        println!("║  Total:          {:<20}║", report.mental_tests.total);
        println!("║  Passed:         {:<20}║", report.mental_tests.passed);
        println!("║  Failed:         {:<20}║", report.mental_tests.failed);
        println!(
            "║  Required:       {:<20}║",
            report.mental_tests.required_targets.len()
        );
        println!("╠══════════════════════════════════════╣");
        println!("║ TRACEABILITY                         ║");
        println!(
            "║  Score:          {:<20}║",
            format!("{:.1}%", report.traceability.traceability_score * 100.0)
        );
        println!(
            "║  Functions:      {:<20}║",
            format!(
                "{}/{}",
                report.traceability.functions_with_traceability,
                report.traceability.total_functions
            )
        );
        println!(
            "║  LOGs:           {:<20}║",
            format!(
                "{}/{}",
                report.traceability.logs_with_traceability, report.traceability.total_logs
            )
        );
        println!(
            "║  Enforcement:    {:<20}║",
            report.traceability.enforcement_mode
        );
        println!("╠══════════════════════════════════════╣");
        println!("║ CASCADE                              ║");
        println!("║  Pending:       {:<20}║", report.cascade.pending_cascades);
        println!("║  Changelogs:    {:<20}║", report.cascade.changelog_count);
        println!("║  No drift:      {:<20}║", report.cascade.passed);
        println!("╠══════════════════════════════════════╣");
        println!("║ NON-HUMAN PATTERNS                  ║");
        println!(
            "║  Score:          {:<20}║",
            format!("{:.1}%", report.non_human_patterns.score * 100.0)
        );
        println!(
            "║  Files scanned:  {:<20}║",
            report.non_human_patterns.files_scanned
        );
        println!(
            "║  Errors:         {:<20}║",
            report.non_human_patterns.error_violations
        );
        println!(
            "║  Warnings:       {:<20}║",
            report.non_human_patterns.warning_violations
        );
        println!("╠══════════════════════════════════════╣");
        println!("║ BELIEF STATE                         ║");
        println!(
            "║  Covered modules: {:<20}║",
            format!(
                "{}/{}",
                report.belief_state.states_found, report.belief_state.total_modules
            )
        );
        println!(
            "║  Coverage:        {:<20}║",
            format!("{:.1}%", report.belief_state.coverage_pct)
        );
        println!(
            "║  Invalid states:  {:<20}║",
            report.belief_state.invalid_states
        );
        println!("╠══════════════════════════════════════╣");
        println!("║ SEMANTIC MARKUP                      ║");
        println!("║  Total blocks:    {:<19}║", report.semantic.total_blocks);
        println!("║  Unclosed blocks: {:<19}║", report.semantic.open_blocks);
        println!(
            "║  Duplicate names: {:<19}║",
            report.semantic.duplicate_name_blocks.len()
        );
        println!("╠══════════════════════════════════════╣");
        println!("║ VERIFICATION                         ║");
        for v in &report.verification {
            let status = if v.passed { "PASS" } else { "FAIL" };
            println!("║  {:<19} {:<10}║", v.level, status);
        }
        println!("╠══════════════════════════════════════╣");
        println!("║ TOKEN ECONOMY                        ║");
        println!(
            "║  Commands tracked:  {:<15}║",
            report.token_economy.total_commands
        );
        println!(
            "║  Tokens saved:      {:<15}║",
            report.token_economy.total_saved
        );
        println!(
            "║  Avg savings:       {:<15.1}║",
            report.token_economy.avg_savings_pct
        );
        println!(
            "║  DB size:           {:<15}║",
            report.token_economy.db_size
        );
        println!("╠══════════════════════════════════════╣");
        println!("║ RTK ADOPTION                         ║");
        println!(
            "║  Discover caps:    {:<17}║",
            report.rtk_adoption.discovery_capabilities
        );
        println!(
            "║  Hooks installed:  {:<17}║",
            report.rtk_adoption.hooks_installed
        );
        println!(
            "║  Tracked commands: {:<17}║",
            report.rtk_adoption.tracked_commands
        );
        println!(
            "║  Adoption gaps:    {:<17}║",
            report.rtk_adoption.gaps.len()
        );
        println!("╠══════════════════════════════════════╣");
        println!("║ SHARDS                               ║");
        println!("║  Active phase: {:<27}║", active_phase);
        println!("║  Module shards: {:<27}║", module_shards);
        println!("║  Phase shards: {:<29}║", phase_shards);
        println!("║  Verification shards: {:<20}║", verification_shards);
        println!("╠══════════════════════════════════════╣");

        if let Some(ref drift) = report.drift {
            if !drift.canonical_drift.is_clean() {
                println!();
                println!("DRIFT DETECTED (run 'syn refresh --fix' to sync):");
                if !drift.canonical_drift.code_not_in_graph.is_empty() {
                    println!(
                        "  {} modules not in graph-index.xml",
                        drift.canonical_drift.code_not_in_graph.len()
                    );
                }
                if !drift.canonical_drift.code_not_in_verification.is_empty() {
                    println!(
                        "  {} modules not in verification-index.xml",
                        drift.canonical_drift.code_not_in_verification.len()
                    );
                }
                if !drift.canonical_drift.files_without_contract.is_empty() {
                    println!(
                        "  {} governed files without MODULE_CONTRACT",
                        drift.canonical_drift.files_without_contract.len()
                    );
                }
            }
        }

        if !report.next_actions.is_empty() {
            println!();
            println!("NEXT ACTIONS:");
            for a in &report.next_actions {
                println!("  → {}", a);
            }
        }
    }
    // END_sc_print_report
}

// START_CONTRACT_collect_rtk_adoption
// PURPOSE: Build advisory RTK adoption diagnostics without changing project health gates.
// INPUTS: { root: &Path }, { tracked_commands: u64 }
// OUTPUTS: { RtkAdoption }
// START_collect_rtk_adoption
fn collect_rtk_adoption(root: &Path, tracked_commands: u64) -> RtkAdoption {
    let hooks_installed = root.join(".opencode/hooks/synapse-proxy.sh").exists()
        || root.join(".opencode/plugin/synapse-proxy.js").exists()
        || root.join("hooks/opencode/synapse-rewrite.sh").exists();
    let mut gaps = Vec::new();
    if tracked_commands == 0 {
        gaps.push(
            "no token-saving commands tracked yet; run syn discover or syn proxy to bootstrap"
                .into(),
        );
    }
    if !hooks_installed {
        gaps.push("OpenCode shell hooks are not installed; run syn hooks install opencode".into());
    }
    RtkAdoption {
        discovery_capabilities: RTK_DISCOVERY_COUNT,
        hooks_installed,
        tracked_commands,
        gaps,
    }
}
// END_collect_rtk_adoption

// START_CONTRACT_active_phase_label
// PURPOSE: Format active phase status for next-action output without reviving completed phases.
// INPUTS: { plan_index: &str — plan-index.xml content }
// OUTPUTS: { String — active phase or no-active-phase label }
// START_active_phase_label
fn active_phase_label(plan_index: &str) -> String {
    match active_phase_display_value(plan_index).as_str() {
        "none" | "unknown" => "No active phase".into(),
        phase => format!("Active phase: {}", phase),
    }
}
// END_active_phase_label

// START_CONTRACT_active_phase_table_value
// PURPOSE: Format active phase status for the status table without showing a completed phase as active.
// INPUTS: { plan_index: &str — plan-index.xml content }
// OUTPUTS: { String — active phase id or no-active-phase label }
// START_active_phase_table_value
fn active_phase_table_value(plan_index: &str) -> String {
    match active_phase_display_value(plan_index).as_str() {
        "none" | "unknown" => "No active phase".into(),
        phase => phase.into(),
    }
}
// END_active_phase_table_value

// START_CONTRACT_active_phase_display_value
// PURPOSE: Return ACTIVE_PHASE only when its plan-index entry is not done.
// INPUTS: { plan_index: &str — plan-index.xml content }
// OUTPUTS: { String — active phase id, none, or unknown }
// START_active_phase_display_value
fn active_phase_display_value(plan_index: &str) -> String {
    let active = regex::Regex::new(r#"<ACTIVE_PHASE>([^<]+)</ACTIVE_PHASE>"#)
        .ok()
        .and_then(|re| re.captures(plan_index).map(|c| c[1].to_string()))
        .unwrap_or_else(|| "unknown".into());
    if active == "unknown" || active == "none" {
        return active;
    }
    let active_entry = format!(
        r#"<PHASE\s+id="{}"[^>]*status="([^"]+)""#,
        regex::escape(&active)
    );
    match regex::Regex::new(&active_entry)
        .ok()
        .and_then(|re| re.captures(plan_index).map(|c| c[1].to_string()))
        .as_deref()
    {
        Some("done") => "none".into(),
        Some(_) => active,
        None => active,
    }
}
// END_active_phase_display_value

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_active_phase_display_value_hides_completed_phase
    // PURPOSE: Verify status does not report a completed ACTIVE_PHASE as active work.
    // OUTPUTS: { () }
    // START_test_active_phase_display_value_hides_completed_phase
    #[test]
    fn test_active_phase_display_value_hides_completed_phase() {
        let plan = r#"<PLAN_INDEX>
  <META><ACTIVE_PHASE>Phase-11</ACTIVE_PHASE></META>
  <PHASES>
    <PHASE id="Phase-11" path="docs/phases/Phase-11.xml" status="done" />
  </PHASES>
</PLAN_INDEX>"#;
        assert_eq!(active_phase_display_value(plan), "none");
        assert_eq!(active_phase_label(plan), "No active phase");
        assert_eq!(active_phase_table_value(plan), "No active phase");
    }
    // END_test_active_phase_display_value_hides_completed_phase

    // START_CONTRACT_test_rtk_adoption_gaps_are_advisory
    // PURPOSE: Verify RTK adoption gaps are surfaced as advisory metadata without changing gates.
    // OUTPUTS: { () }
    // START_test_rtk_adoption_gaps_are_advisory
    #[test]
    fn test_rtk_adoption_gaps_are_advisory() {
        let tmp = tempfile::tempdir().unwrap();
        let adoption = collect_rtk_adoption(tmp.path(), 0);

        assert_eq!(adoption.discovery_capabilities, RTK_DISCOVERY_COUNT);
        assert!(!adoption.hooks_installed);
        assert_eq!(adoption.tracked_commands, 0);
        assert!(adoption.gaps.iter().any(|gap| gap.contains("syn discover")));
    }
    // END_test_rtk_adoption_gaps_are_advisory
}
// END_public_api
