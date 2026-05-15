use std::path::Path;
use crate::grace::contract::{ContractValidator, ContractReport};
use crate::grace::semantic::{SemanticExtractor, SemanticReport};
use crate::grace::verify::Verifier;
use crate::tracking::Tracker;
use crate::config::Config;

#[derive(Debug, serde::Serialize)]
pub struct StatusReport {
    pub contracts: ContractReport,
    pub semantic: SemanticReport,
    pub verification: Vec<super::verify::VerificationResult>,
    pub token_economy: TokenEconomy,
    pub system: SystemInfo,
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

impl StatusCollector {
    pub fn new() -> Self {
        Self
    }

    pub async fn collect(root: &Path) -> anyhow::Result<StatusReport> {
        let contracts = ContractValidator::validate_project(root)?;
        let semantic = SemanticExtractor::scan_project(root)?;
        let verification = Verifier::verify_all(root).await?;

        let config = Config::load().unwrap_or_default();
        let tracker = Tracker::new(&config);
        let stats = tracker.get_stats().await?;

        let db_path = Tracker::db_path().unwrap_or_default();
        let db_size = std::fs::metadata(&db_path)
            .map(|m| format!("{:.1} KB", m.len() as f64 / 1024.0))
            .unwrap_or_else(|_| "N/A".into());

        Ok(StatusReport {
            token_economy: TokenEconomy {
                total_commands: stats.total_commands,
                total_saved: stats.total_saved_tokens,
                avg_savings_pct: stats.avg_savings_pct,
                db_size,
            },
            system: SystemInfo {
                version: crate::VERSION.into(),
                config_path: Config::path().map(|p| p.display().to_string()).unwrap_or_default(),
                data_path: db_path.display().to_string(),
                project_path: root.display().to_string(),
            },
            contracts,
            semantic,
            verification,
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
        println!("║  Files with contract:    {:<12}║", report.contracts.with_contract);
        println!("║  Files without contract: {:<12}║", report.contracts.without_contract);
        println!("║  Valid contracts:       {:<12}║", report.contracts.valid);
        println!("╠══════════════════════════════════════╣");
        println!("║ SEMANTIC MARKUP                      ║");
        println!("║  Total blocks:    {:<19}║", report.semantic.total_blocks);
        println!("║  Unclosed blocks: {:<19}║", report.semantic.open_blocks);
        println!("╠══════════════════════════════════════╣");
        println!("║ VERIFICATION                         ║");
        for v in &report.verification {
            let status = if v.passed { "PASS" } else { "FAIL" };
            println!("║  {:<19} {:<10}║", v.level, status);
        }
        println!("╠══════════════════════════════════════╣");
        println!("║ TOKEN ECONOMY                        ║");
        println!("║  Commands tracked:  {:<15}║", report.token_economy.total_commands);
        println!("║  Tokens saved:      {:<15}║", report.token_economy.total_saved);
        println!("║  Avg savings:       {:<15.1}║", report.token_economy.avg_savings_pct);
        println!("║  DB size:           {:<15}║", report.token_economy.db_size);
        println!("╚══════════════════════════════════════╝");
    }
}
