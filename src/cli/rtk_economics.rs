// MODULE_CONTRACT
// MODULE_ID: M-CLI-RTK-COMMANDS
// PURPOSE: RTK-style session and economics analytics over Synapse's local token-savings tracker
// SCOPE: SessionCmd and CcEconomicsCmd execution, compact text/JSON/CSV rendering, local savings estimates, session-level token economy summaries, route adoption metrics, and missed-route candidates
// DEPENDS: M-CONFIG, M-TRACKING
// LINKS:
//   -> M-CLI (depends) - exposes session and cc-economics command schemas
//   -> M-TRACKING (depends) - provides route-aware command, adapter, and session statistics
//   -> Phase-56 (implements) - RTK economics and session analytics parity
//   -> NFR-003 (traces_to) - economics output proves token savings are measurable

// START_MODULE_MAP
// SessionCmd::run - Prints local RTK session adoption and savings summaries
// CcEconomicsCmd::run - Prints local Claude Code economics from Synapse tracking
// build_session_report - Builds a serializable session analytics report
// build_economics_report - Builds a serializable token economics report
// print_session_report - Renders compact session analytics text
// print_economics_report - Renders compact economics text or CSV
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 - Added tracking-backed route adoption analytics]
// END_CHANGE_SUMMARY

use super::{CcEconomicsCmd, SessionCmd};
use crate::config::Config;
use crate::tracking::{
    TrackingAdapterStat, TrackingAdoptionStats, TrackingCommandStat, TrackingMissedRouteStat,
    TrackingSessionStat, TrackingStats,
};
use serde::Serialize;

const EST_USD_PER_SAVED_TOKEN: f64 = 0.000003;

// START_public_api

impl SessionCmd {
    // START_CONTRACT_SessionCmd::run
    // PURPOSE: Print RTK session coverage from locally tracked Synapse command records
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads SQLite tracking data and writes stdout
    // LINKS:
    //   -> M-TRACKING (depends) - session stats source
    //   -> Phase-56 (implements) - session analytics parity
    // START_session_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let tracker = crate::tracking::Tracker::new(&config);
        let stats = tracker.get_stats().await?;
        let report = build_session_report(&stats);
        if self.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print_session_report(&report);
        }
        Ok(())
    }
    // END_session_cmd_run
}

impl CcEconomicsCmd {
    // START_CONTRACT_CcEconomicsCmd::run
    // PURPOSE: Print local token economics compatible with RTK cc-economics workflows
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads SQLite tracking data and writes stdout
    // LINKS:
    //   -> M-TRACKING (depends) - token savings source
    //   -> Phase-56 (implements) - economics analytics parity
    // START_cc_economics_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let tracker = crate::tracking::Tracker::new(&config);
        let stats = tracker.get_stats().await?;
        let report = build_economics_report(self, &stats);
        match self.format.as_str() {
            "json" => println!("{}", serde_json::to_string_pretty(&report)?),
            "csv" => print_economics_csv(&report),
            "text" => print_economics_report(&report),
            other => anyhow::bail!("unsupported format '{other}'; use text, json, or csv"),
        }
        Ok(())
    }
    // END_cc_economics_cmd_run
}

// END_public_api

// START_SessionReport
#[derive(Debug, Clone, Serialize)]
struct SessionReport {
    total_sessions: u64,
    total_commands: u64,
    total_saved_tokens: u64,
    avg_savings_pct: f64,
    adoption: TrackingAdoptionStats,
    sessions: Vec<TrackingSessionStat>,
}
// END_SessionReport

// START_EconomicsReport
#[derive(Debug, Clone, Serialize)]
struct EconomicsReport {
    scope: String,
    local_only: bool,
    total_commands: u64,
    total_input_tokens: u64,
    total_output_tokens: u64,
    total_saved_tokens: u64,
    avg_savings_pct: f64,
    adapter_groups: u64,
    session_groups: u64,
    route_adoption_pct: f64,
    estimated_cost_saved_usd: f64,
    top_commands: Vec<TrackingCommandStat>,
    top_adapters: Vec<TrackingAdapterStat>,
    missed_route_candidates: Vec<TrackingMissedRouteStat>,
}
// END_EconomicsReport

// START_CONTRACT_build_session_report
// PURPOSE: Convert aggregate tracking stats into a serializable session report
// INPUTS: { stats: &TrackingStats }
// OUTPUTS: { SessionReport }
// START_build_session_report
fn build_session_report(stats: &TrackingStats) -> SessionReport {
    SessionReport {
        total_sessions: stats.session_groups,
        total_commands: stats.total_commands,
        total_saved_tokens: stats.total_saved_tokens,
        avg_savings_pct: stats.avg_savings_pct,
        adoption: stats.adoption.clone(),
        sessions: stats.recent_sessions.clone(),
    }
}
// END_build_session_report

// START_CONTRACT_build_economics_report
// PURPOSE: Convert aggregate tracking stats into a local token economics report
// INPUTS: { cmd: &CcEconomicsCmd }, { stats: &TrackingStats }
// OUTPUTS: { EconomicsReport }
// START_build_economics_report
fn build_economics_report(cmd: &CcEconomicsCmd, stats: &TrackingStats) -> EconomicsReport {
    EconomicsReport {
        scope: economics_scope(cmd),
        local_only: true,
        total_commands: stats.total_commands,
        total_input_tokens: stats.total_input_tokens,
        total_output_tokens: stats.total_output_tokens,
        total_saved_tokens: stats.total_saved_tokens,
        avg_savings_pct: stats.avg_savings_pct,
        adapter_groups: stats.adapter_groups,
        session_groups: stats.session_groups,
        route_adoption_pct: stats.adoption.route_adoption_pct,
        estimated_cost_saved_usd: estimate_cost_saved(stats.total_saved_tokens),
        top_commands: stats.top_commands.clone(),
        top_adapters: stats.top_adapters.clone(),
        missed_route_candidates: stats.adoption.missed_route_candidates.clone(),
    }
}
// END_build_economics_report

// START_CONTRACT_economics_scope
// PURPOSE: Render the requested RTK economics scope from command flags
// INPUTS: { cmd: &CcEconomicsCmd }
// OUTPUTS: { String }
// START_economics_scope
fn economics_scope(cmd: &CcEconomicsCmd) -> String {
    if cmd.all {
        return "all".into();
    }
    let mut scopes = Vec::new();
    if cmd.daily {
        scopes.push("daily");
    }
    if cmd.weekly {
        scopes.push("weekly");
    }
    if cmd.monthly {
        scopes.push("monthly");
    }
    if scopes.is_empty() {
        "summary".into()
    } else {
        scopes.join(",")
    }
}
// END_economics_scope

// START_CONTRACT_estimate_cost_saved
// PURPOSE: Estimate local token-cost savings using Synapse's existing gain heuristic
// INPUTS: { saved_tokens: u64 }
// OUTPUTS: { f64 }
// START_estimate_cost_saved
fn estimate_cost_saved(saved_tokens: u64) -> f64 {
    saved_tokens as f64 * EST_USD_PER_SAVED_TOKEN
}
// END_estimate_cost_saved

// START_CONTRACT_print_session_report
// PURPOSE: Render compact session analytics text
// INPUTS: { report: &SessionReport }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes stdout
// START_print_session_report
fn print_session_report(report: &SessionReport) {
    println!("Synapse RTK Session Overview");
    println!("sessions: {}", report.total_sessions);
    println!("commands: {}", report.total_commands);
    println!("tokens saved: {}", report.total_saved_tokens);
    println!("average savings: {:.1}%", report.avg_savings_pct);
    println!("route adoption: {:.1}%", report.adoption.route_adoption_pct);
    if report.sessions.is_empty() {
        println!("No tracked Synapse RTK sessions yet.");
        return;
    }
    println!();
    println!(
        "{:<18} {:>6} {:>12} {:>10} {:>20}",
        "Session", "Cmds", "Saved", "Avg", "Last seen"
    );
    for session in &report.sessions {
        println!(
            "{:<18} {:>6} {:>12} {:>9.1}% {:>20}",
            compact_label(&session.session_id, 18),
            session.count,
            session.saved_tokens,
            session.avg_savings_pct,
            compact_label(&session.last_seen, 20)
        );
    }
    if !report.adoption.missed_route_candidates.is_empty() {
        println!();
        println!("Missed-route candidates:");
        for item in &report.adoption.missed_route_candidates {
            println!(
                "  - {}: {} passthrough runs",
                compact_label(&item.command, 32),
                item.count
            );
        }
    }
}
// END_print_session_report

// START_CONTRACT_print_economics_report
// PURPOSE: Render compact local economics text
// INPUTS: { report: &EconomicsReport }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes stdout
// START_print_economics_report
fn print_economics_report(report: &EconomicsReport) {
    println!("Synapse RTK Economics");
    println!("scope: {}", report.scope);
    println!("local-only: {}", report.local_only);
    println!("commands: {}", report.total_commands);
    println!("input tokens: {}", report.total_input_tokens);
    println!("output tokens: {}", report.total_output_tokens);
    println!("tokens saved: {}", report.total_saved_tokens);
    println!("average savings: {:.1}%", report.avg_savings_pct);
    println!("adapter groups: {}", report.adapter_groups);
    println!("session groups: {}", report.session_groups);
    println!("route adoption: {:.1}%", report.route_adoption_pct);
    println!(
        "estimated cost saved: ${:.4}",
        report.estimated_cost_saved_usd
    );
    if !report.top_adapters.is_empty() {
        println!();
        println!("Top adapters:");
        for adapter in &report.top_adapters {
            println!(
                "  - {}: {} runs, {} tokens saved, avg {:.1}%",
                adapter.adapter, adapter.count, adapter.saved_tokens, adapter.avg_savings_pct
            );
        }
    }
    if !report.missed_route_candidates.is_empty() {
        println!();
        println!("Missed-route candidates:");
        for item in &report.missed_route_candidates {
            println!("  - {}: {} passthrough runs", item.command, item.count);
        }
    }
}
// END_print_economics_report

// START_CONTRACT_print_economics_csv
// PURPOSE: Render one CSV row for automation-friendly local economics export
// INPUTS: { report: &EconomicsReport }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes stdout
// START_print_economics_csv
fn print_economics_csv(report: &EconomicsReport) {
    println!("scope,total_commands,total_input_tokens,total_output_tokens,total_saved_tokens,avg_savings_pct,adapter_groups,session_groups,route_adoption_pct,estimated_cost_saved_usd");
    println!(
        "{},{},{},{},{},{:.1},{},{},{:.1},{:.4}",
        report.scope,
        report.total_commands,
        report.total_input_tokens,
        report.total_output_tokens,
        report.total_saved_tokens,
        report.avg_savings_pct,
        report.adapter_groups,
        report.session_groups,
        report.route_adoption_pct,
        report.estimated_cost_saved_usd
    );
}
// END_print_economics_csv

// START_CONTRACT_compact_label
// PURPOSE: Truncate labels for table rendering without shifting columns
// INPUTS: { value: &str }, { max: usize }
// OUTPUTS: { String }
// START_compact_label
fn compact_label(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    value
        .chars()
        .take(max.saturating_sub(3))
        .collect::<String>()
        + "..."
}
// END_compact_label

#[cfg(test)]
mod tests {
    use super::*;

    fn stats_fixture() -> TrackingStats {
        TrackingStats {
            total_commands: 3,
            total_input_tokens: 1000,
            total_output_tokens: 250,
            total_saved_tokens: 750,
            avg_savings_pct: 75.0,
            adapter_groups: 2,
            session_groups: 1,
            top_commands: vec![TrackingCommandStat {
                command: "cargo test".into(),
                count: 2,
                saved_tokens: 500,
                avg_savings_pct: 80.0,
            }],
            top_adapters: vec![TrackingAdapterStat {
                adapter: "rust-cargo".into(),
                count: 2,
                saved_tokens: 500,
                avg_savings_pct: 80.0,
            }],
            recent_sessions: vec![TrackingSessionStat {
                session_id: "session-1".into(),
                count: 3,
                input_tokens: 1000,
                output_tokens: 250,
                saved_tokens: 750,
                avg_savings_pct: 75.0,
                last_seen: "2026-05-25 10:00:00".into(),
            }],
            adoption: TrackingAdoptionStats {
                total_commands: 3,
                routed_commands: 2,
                passthrough_commands: 1,
                route_adoption_pct: 66.7,
                missed_route_candidates: vec![TrackingMissedRouteStat {
                    command: "git status".into(),
                    count: 1,
                    last_seen: "2026-05-25 10:00:00".into(),
                }],
            },
        }
    }

    #[test]
    fn session_report_uses_tracking_session_groups() {
        let report = build_session_report(&stats_fixture());

        assert_eq!(report.total_sessions, 1);
        assert_eq!(report.total_saved_tokens, 750);
        assert_eq!(report.adoption.passthrough_commands, 1);
        assert_eq!(report.sessions[0].session_id, "session-1");
    }

    #[test]
    fn economics_report_computes_local_cost_estimate() {
        let cmd = CcEconomicsCmd {
            daily: true,
            weekly: false,
            monthly: false,
            all: false,
            format: "text".into(),
        };
        let report = build_economics_report(&cmd, &stats_fixture());

        assert_eq!(report.scope, "daily");
        assert!(report.local_only);
        assert_eq!(report.missed_route_candidates[0].command, "git status");
        assert!((report.estimated_cost_saved_usd - 0.00225).abs() < f64::EPSILON);
    }

    #[test]
    fn compact_label_truncates_long_values() {
        assert_eq!(compact_label("short", 10), "short");
        assert_eq!(compact_label("abcdefghijkl", 8), "abcde...");
    }
}
