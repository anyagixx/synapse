// MODULE_CONTRACT
// MODULE_ID: M-CLI-RTK-COMMANDS
// PURPOSE: Bounded RTK discover/learn diagnostics for routeable token-heavy commands
// SCOPE: DiscoverCmd, LearnCmd, route-aware missed-opportunity reports including Graphite shortcuts, tracking-backed missed-route history, measured learning signals, JSON/text rendering
// DEPENDS: M-CONFIG, M-PROXY-ROUTER, M-CAPABILITIES, M-TRACKING
// LINKS:
//   -> M-PROXY-ROUTER (depends) - uses route decisions as the source of truth for discovery
//   -> M-CAPABILITIES (depends) - shares discover/learn capability metadata
//   -> Phase-49 (implements) - RTK Discovery And Learning Parity
//   -> Phase-56 (implements) - measured RTK learn/discover analytics
//   -> NFR-003 (traces_to) - discovery reduces missed token-saving opportunities

// START_MODULE_MAP
// DiscoverCmd::run - Prints routeable command opportunities and suggested Synapse replacements
// LearnCmd::run - Prints bounded RTK learning guidance without reading private session history
// build_discovery_report - Builds a JSON-serializable discovery report
// build_learn_report - Builds a JSON-serializable learning report
// suggestion_for_command - Maps routeable commands to first-class shortcuts or syn proxy fallback
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.2.0 - Added tracking-backed discovery and learning signals]
// END_CHANGE_SUMMARY

use super::{DiscoverCmd, LearnCmd};
use crate::config::Config;
use crate::proxy::router::CommandRouter;
use crate::tracking::{TrackingAdoptionStats, TrackingMissedRouteStat};
use serde::Serialize;

const DEFAULT_DISCOVER_LIMIT: usize = 20;
const PYTHON_PYTEST_PREFIX_LEN: usize = 3;
const LEARN_TRIGGER_MISSED_ROUTE: &str =
    "A raw shell command is routeable but was run without syn proxy or a first-class shortcut.";
const LEARN_RECOMMENDATION_MISSED_ROUTE: &str =
    "Run syn discover <cmd> or syn rewrite <cmd>, then prefer the suggested syn shortcut in agent instructions.";
const LEARN_RECOMMENDATION_HOOK_ADOPTION: &str =
    "Run syn hooks install opencode and keep .opencode/hooks/synapse-proxy.sh sourced for shell-level auto-proxying.";
const LEARN_RECOMMENDATION_ECONOMICS: &str =
    "Run syn gain --sessions --adapters and compare high-volume adapters with syn discover catalogue suggestions.";
const LEARN_RECOMMENDATION_MEASURED_MISSES: &str =
    "Run syn discover --json and promote repeated routeable suggestions into agent instructions or hook setup.";

// START_public_api

impl DiscoverCmd {
    // START_CONTRACT_DiscoverCmd::run
    // PURPOSE: Print RTK-style missed-opportunity diagnostics without scanning untrusted session files
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes stdout
    // LINKS:
    //   -> M-PROXY-ROUTER (depends) - route catalogue and route previews drive recommendations
    // START_discover_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let tracker = crate::tracking::Tracker::new(&config);
        let adoption = tracker.get_adoption_stats(self.limit).await?;
        let report = build_discovery_report(&self.command, self.limit, &adoption);
        if self.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print_discovery_report(&report);
        }
        Ok(())
    }
    // END_discover_cmd_run
}

impl LearnCmd {
    // START_CONTRACT_LearnCmd::run
    // PURPOSE: Print bounded learning guidance for recurring token-saving misses
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes stdout
    // LINKS:
    //   -> M-CAPABILITIES (depends) - exposes learn capability metadata
    // START_learn_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let tracker = crate::tracking::Tracker::new(&config);
        let adoption = tracker.get_adoption_stats(DEFAULT_DISCOVER_LIMIT).await?;
        let report = build_learn_report(&adoption);
        if self.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print_learn_report(&report);
        }
        Ok(())
    }
    // END_learn_cmd_run
}

// END_public_api

// START_DiscoveryReport
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DiscoveryReport {
    mode: String,
    analyzed_command: Option<String>,
    opportunities: Vec<DiscoveryOpportunity>,
    history: Vec<DiscoveryHistoryOpportunity>,
    notes: Vec<String>,
}
// END_DiscoveryReport

// START_DiscoveryOpportunity
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DiscoveryOpportunity {
    command: String,
    suggestion: String,
    adapter: String,
    family: String,
    route_key: String,
    reason: String,
}
// END_DiscoveryOpportunity

// START_DiscoveryHistoryOpportunity
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DiscoveryHistoryOpportunity {
    command: String,
    suggestion: String,
    adapter: String,
    family: String,
    route_key: String,
    count: u64,
    last_seen: String,
}
// END_DiscoveryHistoryOpportunity

// START_LearnReport
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LearnReport {
    mode: String,
    rules: Vec<LearnRule>,
    signals: Vec<LearnSignal>,
    notes: Vec<String>,
}
// END_LearnReport

// START_LearnRule
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LearnRule {
    name: String,
    trigger: String,
    recommendation: String,
}
// END_LearnRule

// START_LearnSignal
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LearnSignal {
    name: String,
    value: String,
}
// END_LearnSignal

// START_CONTRACT_build_discovery_report
// PURPOSE: Build a discovery report for one command or for the supported router catalogue
// INPUTS: { command: &[String] }, { limit: usize }, { adoption: &TrackingAdoptionStats }
// OUTPUTS: { DiscoveryReport }
// START_build_discovery_report
fn build_discovery_report(
    command: &[String],
    limit: usize,
    adoption: &TrackingAdoptionStats,
) -> DiscoveryReport {
    let normalized_limit = if limit == 0 {
        DEFAULT_DISCOVER_LIMIT
    } else {
        limit
    };
    let router = CommandRouter::new();
    let command_text = command.join(" ").trim().to_string();
    let opportunities = if command_text.is_empty() {
        router
            .supported_adapters()
            .into_iter()
            .flat_map(|adapter| adapter.examples.iter().copied())
            .filter_map(|example| opportunity_for(&router, example))
            .take(normalized_limit)
            .collect()
    } else {
        opportunity_for(&router, &command_text)
            .into_iter()
            .take(normalized_limit)
            .collect()
    };
    DiscoveryReport {
        mode: if command_text.is_empty() {
            "catalog".into()
        } else {
            "command".into()
        },
        analyzed_command: (!command_text.is_empty()).then_some(command_text),
        opportunities,
        history: history_opportunities(&router, adoption, normalized_limit),
        notes: vec![
            "Discovery is bounded: Synapse classifies supplied commands, built-in examples, and local tracking records; it does not read private agent session history.".into(),
            "Use syn rewrite <cmd> for hook-exact rewriting and syn gain --sessions --adapters for measured economics.".into(),
        ],
    }
}
// END_build_discovery_report

// START_CONTRACT_history_opportunities
// PURPOSE: Convert locally tracked passthrough commands into routeable missed-route history opportunities
// INPUTS: { router: &CommandRouter }, { adoption: &TrackingAdoptionStats }, { limit: usize }
// OUTPUTS: { Vec<DiscoveryHistoryOpportunity> }
// LINKS:
//   -> M-TRACKING (depends) - reads measured passthrough command candidates
//   -> Phase-56 (implements) - tracking-backed discover history
// START_history_opportunities
fn history_opportunities(
    router: &CommandRouter,
    adoption: &TrackingAdoptionStats,
    limit: usize,
) -> Vec<DiscoveryHistoryOpportunity> {
    adoption
        .missed_route_candidates
        .iter()
        .filter_map(|candidate| history_opportunity_for(router, candidate))
        .take(limit)
        .collect()
}
// END_history_opportunities

// START_CONTRACT_history_opportunity_for
// PURPOSE: Convert one tracked passthrough command into a routeable missed-route history item
// INPUTS: { router: &CommandRouter }, { candidate: &TrackingMissedRouteStat }
// OUTPUTS: { Option<DiscoveryHistoryOpportunity> }
// START_history_opportunity_for
fn history_opportunity_for(
    router: &CommandRouter,
    candidate: &TrackingMissedRouteStat,
) -> Option<DiscoveryHistoryOpportunity> {
    let opportunity = opportunity_for(router, &candidate.command)?;
    Some(DiscoveryHistoryOpportunity {
        command: opportunity.command,
        suggestion: opportunity.suggestion,
        adapter: opportunity.adapter,
        family: opportunity.family,
        route_key: opportunity.route_key,
        count: candidate.count,
        last_seen: candidate.last_seen.clone(),
    })
}
// END_history_opportunity_for

// START_CONTRACT_opportunity_for
// PURPOSE: Convert one shell command into a routeable discovery opportunity when possible
// INPUTS: { router: &CommandRouter }, { command: &str }
// OUTPUTS: { Option<DiscoveryOpportunity> }
// START_opportunity_for
fn opportunity_for(router: &CommandRouter, command: &str) -> Option<DiscoveryOpportunity> {
    let tokens = split_simple_command(command);
    if tokens.is_empty() {
        return None;
    }
    let decision = router.route(&tokens);
    if !decision.should_proxy {
        return None;
    }
    Some(DiscoveryOpportunity {
        command: command.to_string(),
        suggestion: suggestion_for_tokens(&tokens),
        adapter: decision.adapter,
        family: decision.family,
        route_key: decision.route_key,
        reason: decision.reason,
    })
}
// END_opportunity_for

// START_CONTRACT_suggestion_for_tokens
// PURPOSE: Suggest a first-class Synapse shortcut when available, otherwise fall back to syn proxy
// INPUTS: { tokens: &[String] }
// OUTPUTS: { String }
// START_suggestion_for_tokens
fn suggestion_for_tokens(tokens: &[String]) -> String {
    if tokens.is_empty() {
        return "syn proxy --".into();
    }
    if tokens.len() >= PYTHON_PYTEST_PREFIX_LEN
        && tokens[0] == "python"
        && tokens[1] == "-m"
        && tokens[2] == "pytest"
    {
        return render_shortcut("pytest", &tokens[PYTHON_PYTEST_PREFIX_LEN..]);
    }
    let (shortcut, skip) = match tokens[0].as_str() {
        "cat" => ("read", 1),
        "golangci-lint" => ("golangci", 1),
        "./gradlew" => ("gradlew", 1),
        "pip3" => ("pip", 1),
        "ls" | "tree" | "find" | "rg" | "grep" | "git" | "gt" | "cargo" | "npm" | "pnpm"
        | "npx" | "pytest" | "ruff" | "mypy" | "basedpyright" | "pip" | "uv" | "next"
        | "playwright" | "prettier" | "prisma" | "tsc" | "vitest" | "gh" | "glab" | "aws"
        | "psql" | "curl" | "wget" | "jq" | "go" | "dotnet" | "rake" | "rspec" | "rubocop"
        | "gradle" | "make" | "just" | "helm" | "kubectl" | "docker" | "podman" | "wc" => {
            (tokens[0].as_str(), 1)
        }
        _ => return format!("syn proxy -- {}", tokens.join(" ")),
    };
    render_shortcut(shortcut, &tokens[skip..])
}
// END_suggestion_for_tokens

// START_CONTRACT_render_shortcut
// PURPOSE: Render a shell-safe simple Synapse shortcut suggestion for already tokenized examples
// INPUTS: { shortcut: &str }, { rest: &[String] }
// OUTPUTS: { String }
// START_render_shortcut
fn render_shortcut(shortcut: &str, rest: &[String]) -> String {
    if rest.is_empty() {
        format!("syn {shortcut}")
    } else {
        format!("syn {shortcut} {}", rest.join(" "))
    }
}
// END_render_shortcut

// START_CONTRACT_split_simple_command
// PURPOSE: Split bounded diagnostic examples and simple user input into route tokens
// INPUTS: { command: &str }
// OUTPUTS: { Vec<String> }
// START_split_simple_command
fn split_simple_command(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
        .collect()
}
// END_split_simple_command

// START_CONTRACT_build_learn_report
// PURPOSE: Build learning guidance from Synapse adoption patterns and local tracking signals
// INPUTS: { adoption: &TrackingAdoptionStats }
// OUTPUTS: { LearnReport }
// START_build_learn_report
fn build_learn_report(adoption: &TrackingAdoptionStats) -> LearnReport {
    let mut rules = vec![
        LearnRule {
            name: "missed-route".into(),
            trigger: LEARN_TRIGGER_MISSED_ROUTE.into(),
            recommendation: LEARN_RECOMMENDATION_MISSED_ROUTE.into(),
        },
        LearnRule {
            name: "hook-adoption".into(),
            trigger: "Routeable commands keep appearing as raw shell commands in an agent session."
                .into(),
            recommendation: LEARN_RECOMMENDATION_HOOK_ADOPTION.into(),
        },
        LearnRule {
            name: "economics-review".into(),
            trigger: "Token savings are unclear or adapter coverage looks uneven.".into(),
            recommendation: LEARN_RECOMMENDATION_ECONOMICS.into(),
        },
    ];
    if !adoption.missed_route_candidates.is_empty() {
        rules.push(LearnRule {
            name: "measured-missed-routes".into(),
            trigger: format!(
                "{} passthrough command groups were found in local Synapse tracking.",
                adoption.missed_route_candidates.len()
            ),
            recommendation: LEARN_RECOMMENDATION_MEASURED_MISSES.into(),
        });
    }
    LearnReport {
        mode: "measured-guidance".into(),
        rules,
        signals: vec![
            LearnSignal {
                name: "route-adoption-pct".into(),
                value: format!("{:.1}", adoption.route_adoption_pct),
            },
            LearnSignal {
                name: "passthrough-commands".into(),
                value: adoption.passthrough_commands.to_string(),
            },
            LearnSignal {
                name: "missed-route-candidates".into(),
                value: adoption.missed_route_candidates.len().to_string(),
            },
        ],
        notes: vec![
            "Synapse learn is advisory; it uses local tracking aggregates and does not mutate rules or scrape private session files.".into(),
            "Use MyGRACE verification and review gates before turning learning suggestions into persistent project rules.".into(),
        ],
    }
}
// END_build_learn_report

// START_CONTRACT_print_discovery_report
// PURPOSE: Render discovery report as compact human-readable text
// INPUTS: { report: &DiscoveryReport }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes stdout
// START_print_discovery_report
fn print_discovery_report(report: &DiscoveryReport) {
    println!("RTK discovery ({})", report.mode);
    if let Some(command) = &report.analyzed_command {
        println!("command: {command}");
    }
    if report.opportunities.is_empty() {
        println!("No routeable token-heavy command detected.");
    } else {
        for item in &report.opportunities {
            println!(
                "- {} -> {} [{}:{}]",
                item.command, item.suggestion, item.family, item.adapter
            );
        }
    }
    if !report.history.is_empty() {
        println!("Measured missed-route history:");
        for item in &report.history {
            println!(
                "- {} -> {} [{} runs, last {}]",
                item.command, item.suggestion, item.count, item.last_seen
            );
        }
    }
    for note in &report.notes {
        println!("note: {note}");
    }
}
// END_print_discovery_report

// START_CONTRACT_print_learn_report
// PURPOSE: Render learning guidance as compact human-readable text
// INPUTS: { report: &LearnReport }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes stdout
// START_print_learn_report
fn print_learn_report(report: &LearnReport) {
    println!("RTK learn ({})", report.mode);
    for rule in &report.rules {
        println!("- {}: {}", rule.name, rule.recommendation);
    }
    for signal in &report.signals {
        println!("signal: {}={}", signal.name, signal.value);
    }
    for note in &report.notes {
        println!("note: {note}");
    }
}
// END_print_learn_report

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_discovery_for_single_command_suggests_shortcut
    // PURPOSE: Verify single-command discovery suggests a first-class shortcut for container commands.
    // OUTPUTS: { () }
    // START_discovery_for_single_command_suggests_shortcut
    #[test]
    fn discovery_for_single_command_suggests_shortcut() {
        let report = build_discovery_report(
            &["docker".into(), "ps".into()],
            20,
            &TrackingAdoptionStats::default(),
        );
        assert_eq!(report.mode, "command");
        assert_eq!(report.opportunities.len(), 1);
        assert_eq!(report.opportunities[0].suggestion, "syn docker ps");
        assert_eq!(report.opportunities[0].family, "infrastructure");
    }
    // END_discovery_for_single_command_suggests_shortcut

    // START_CONTRACT_discovery_catalog_contains_routeable_examples
    // PURPOSE: Verify catalogue discovery includes representative RTK shortcut families.
    // OUTPUTS: { () }
    // START_discovery_catalog_contains_routeable_examples
    #[test]
    fn discovery_catalog_contains_routeable_examples() {
        let report = build_discovery_report(&[], 50, &TrackingAdoptionStats::default());
        let suggestions = report
            .opportunities
            .iter()
            .map(|item| item.suggestion.as_str())
            .collect::<Vec<_>>();
        assert!(suggestions.contains(&"syn cargo test"));
        assert!(suggestions.contains(&"syn docker compose logs"));
        assert!(suggestions.contains(&"syn pytest"));
    }
    // END_discovery_catalog_contains_routeable_examples

    // START_CONTRACT_learn_report_contains_bounded_guidance
    // PURPOSE: Verify learn diagnostics stay bounded and advisory.
    // OUTPUTS: { () }
    // START_learn_report_contains_bounded_guidance
    #[test]
    fn learn_report_contains_bounded_guidance() {
        let report = build_learn_report(&TrackingAdoptionStats::default());
        assert_eq!(report.mode, "measured-guidance");
        assert!(report.rules.iter().any(|rule| rule.name == "hook-adoption"));
        assert!(report
            .signals
            .iter()
            .any(|signal| signal.name == "route-adoption-pct"));
        assert!(report
            .notes
            .iter()
            .any(|note| note.contains("does not mutate")));
    }
    // END_learn_report_contains_bounded_guidance

    // START_CONTRACT_discovery_history_uses_tracking_candidates
    // PURPOSE: Verify tracked passthrough candidates become measured routeable history.
    // OUTPUTS: { () }
    // START_discovery_history_uses_tracking_candidates
    #[test]
    fn discovery_history_uses_tracking_candidates() {
        let adoption = TrackingAdoptionStats {
            missed_route_candidates: vec![TrackingMissedRouteStat {
                command: "git status".into(),
                count: 2,
                last_seen: "2026-05-25 10:00:00".into(),
            }],
            ..TrackingAdoptionStats::default()
        };
        let report = build_discovery_report(&[], 20, &adoption);

        assert_eq!(report.history.len(), 1);
        assert_eq!(report.history[0].suggestion, "syn git status");
        assert_eq!(report.history[0].count, 2);
    }
    // END_discovery_history_uses_tracking_candidates
}
// END_public_api
