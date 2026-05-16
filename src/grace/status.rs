use crate::config::Config;
use crate::grace::contract::{ContractReport, ContractValidator};
use crate::grace::refresh::Refresher;
use crate::grace::semantic::{SemanticExtractor, SemanticReport};
use crate::grace::verify::Verifier;
use crate::tracking::Tracker;
use std::path::Path;

#[derive(Debug, serde::Serialize)]
pub struct StatusReport {
    pub contracts: ContractReport,
    pub semantic: SemanticReport,
    pub verification: Vec<super::verify::VerificationResult>,
    pub drift: Option<super::refresh::RefreshReport>,
    pub token_economy: TokenEconomy,
    pub system: SystemInfo,
    pub next_actions: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct TokenEconomy {
    pub total_commands: u64,
    pub total_saved: u64,
    pub avg_savings_pct: f64,
    pub db_size: String,
}

#[derive(Debug, serde::Serialize)]
pub struct SystemInfo {
    pub version: String,
    pub config_path: String,
    pub data_path: String,
    pub project_path: String,
}

pub struct StatusCollector;

impl Default for StatusCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusCollector {
    pub fn new() -> Self {
        Self
    }

    pub async fn collect(root: &Path) -> anyhow::Result<StatusReport> {
        let contracts = ContractValidator::validate_project(root)?;
        let semantic = SemanticExtractor::scan_project(root)?;
        let verification = Verifier::verify_all(root).await?;
        let drift = Refresher::refresh(root).ok();

        let config = Config::load().unwrap_or_default();
        let tracker = Tracker::new(&config);
        let stats = tracker.get_stats().await?;

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
        if !semantic.duplicate_name_blocks.is_empty() {
            next_actions.push(format!(
                "Fix {} duplicate block names",
                semantic.duplicate_name_blocks.len()
            ));
        }

        let all_verify_pass = verification.iter().all(|v| v.passed);
        if !all_verify_pass {
            next_actions.push("Run 'syn verify' to see failing checks".into());
        }

        let phase0_done = root.join("docs/requirements.xml").exists()
            && root.join("docs/technology.xml").exists()
            && root.join("docs/development-plan.xml").exists()
            && root.join("docs/verification-plan.xml").exists()
            && root.join("docs/knowledge-graph.xml").exists();
        if !phase0_done {
            next_actions.push("Complete Phase 0: create all 5 docs/ files".into());
        }

        if let Some(ref d) = drift {
            if !d.not_in_graph.is_empty() || !d.not_in_verification.is_empty() {
                next_actions.push("Run 'syn refresh' to sync artifacts".into());
            }
        }

        Ok(StatusReport {
            token_economy: TokenEconomy {
                total_commands: stats.total_commands,
                total_saved: stats.total_saved_tokens,
                avg_savings_pct: stats.avg_savings_pct,
                db_size,
            },
            system: SystemInfo {
                version: crate::VERSION.into(),
                config_path: Config::path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                data_path: db_path.display().to_string(),
                project_path: root.display().to_string(),
            },
            contracts,
            semantic,
            verification,
            drift,
            next_actions,
        })
    }

    pub fn print_report(report: &StatusReport) {
        println!("╔══════════════════════════════════════╗");
        println!("║       Synapse Project Health        ║");
        println!("╠══════════════════════════════════════╣");
        println!("║ Version: {:<33}║", report.system.version);
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
        println!("╚══════════════════════════════════════╝");

        if let Some(ref drift) = report.drift {
            if !drift.not_in_graph.is_empty()
                || !drift.not_in_verification.is_empty()
                || !drift.in_graph_not_in_code.is_empty()
            {
                println!();
                println!("DRIFT DETECTED (run 'syn refresh' for details):");
                if !drift.not_in_graph.is_empty() {
                    println!(
                        "  {} modules not in knowledge graph",
                        drift.not_in_graph.len()
                    );
                }
                if !drift.not_in_verification.is_empty() {
                    println!(
                        "  {} modules not in verification plan",
                        drift.not_in_verification.len()
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
}
