// MODULE_CONTRACT
// MODULE_ID: M-CLI-RUNTIME-COMMANDS
// PURPOSE: CLI runtime, integration, and diagnostic command handlers with storage health, dependency, filter lifecycle, and clean-bootstrap reporting
// SCOPE: RunCmd autonomous scenario/action queue smoke, GainCmd with graph/session/adapter output, ProxyCmd with route preview, optional raw evidence hint, and wrapped exit-code propagation, FiltersCmd verify/trust/status controls, CompressCmd, McpCmd, ConfigCmd, HooksCmd multi-agent install/status/audit, DoctorCmd, dependency diagnostics, clean config fallback diagnostics, index storage diagnostics, ServeCmd
// DEPENDS: M-CONFIG, M-RUNNER, M-GRACE-STATUS, M-TRACKING, M-PROXY, M-PROXY-ROUTER, M-PROXY-FILTER, M-COMPRESS, M-MCP, M-HOOKS, M-DASHBOARD, M-INDEXER-STORAGE, M-INDEXER-WALKER
// LINKS:
//   → M-PROXY-ROUTER (depends) - route preview for proxied commands
//   → M-PROXY-FILTER (depends) - filter verification and project trust lifecycle
//   → M-TRACKING (depends) - token economy analytics
//   → UC-002 (implements) - CLI exposes bounded execution diagnostics
//   → NFR-003 (traces_to) - command analytics quantify token savings

// START_MODULE_MAP
// RunCmd::run — Runs a bounded autonomous E2E scenario or action queue command
// run_action_command — Plans, steps, loops, retries, or replays a persisted run
// latest_run_id — Resolves the most recently updated run when --run-id is omitted
// print_run_scenario_result — Prints scenario gate/action/replay summary
// print_run_action_value — Prints compact action command output
// GainCmd::run — Prints token savings
// print_savings_graph — Prints ASCII token-savings bars for top commands
// print_adapter_stats — Prints adapter-level savings
// print_session_stats — Prints session-level savings
// ProxyCmd::run — Runs proxied shell commands
// print_route_decision — Prints command-router decision without execution
// FiltersCmd::run — Verifies, trusts, untrusts, and reports proxy filter status
// CompressCmd::run — Compresses or restores files
// McpCmd::run — Starts MCP server
// ConfigCmd::run — Prints or opens config
// HooksCmd::run — Manages hook installation
// DoctorCmd::run — Runs setup and dependency diagnostics
// check_project_dependencies — Checks Python dependency readiness when manifests are present
// ServeCmd::run — Starts dashboard
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v5.3.0 — Routed hooks status to multi-agent targets]
// END_CHANGE_SUMMARY

use super::{
    CompressCmd, ConfigCmd, DoctorCmd, FiltersAction, FiltersCmd, GainCmd, HooksCmd, McpCmd,
    ProxyCmd, RunCmd, ServeCmd,
};
use crate::config::Config;
use crate::run::scenario::{RunScenarioMode, RunScenarioRequest, RunScenarioResult};
use crate::run::RunManager;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const CONFIG_SINGLE_ARG_COUNT: usize = 1;
const CONFIG_SET_ARG_COUNT: usize = 3;

// START_public_api

impl RunCmd {
    // START_CONTRACT_RunCmd::run
    // PURPOSE: Run a bounded autonomous workflow scenario or operate on a persisted run action queue
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes docs/runs/*.json when the scenario run is created
    // LINKS:
    //   → M-RUNNER (depends) - creates durable run scenario state
    //   → M-GRACE-STATUS (depends) - builds scenario gate policy from project health
    // START_run_cmd_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        if let Some(action) = &self.action {
            return run_action_command(self, action);
        }
        let mode = RunScenarioMode::parse(&self.scenario)?;
        let root = std::env::current_dir()?;
        let report = crate::grace::status::StatusCollector::collect(&root).await?;
        let manager = RunManager::new(&root);
        let request = RunScenarioRequest::new(
            &self.goal,
            &self.phase,
            &self.module_id,
            &self.objective,
            mode,
        );
        let result = manager.run_scenario_from_report(request, &report)?;
        if self.json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            print_run_scenario_result(&result);
        }
        Ok(())
    }
    // END_run_cmd_run
}

// START_CONTRACT_run_action_command
// PURPOSE: Execute a run action command against a persisted bounded run
// INPUTS: { cmd: &RunCmd }, { action: &str }
// OUTPUTS: { anyhow::Result<()> }
// LINKS:
//   → M-RUNNER (depends) - action queue planner and executor
// START_run_action_command
fn run_action_command(cmd: &RunCmd, action: &str) -> anyhow::Result<()> {
    let root = std::env::current_dir()?;
    let manager = RunManager::new(&root);
    let run_id = resolve_run_id(&manager, cmd.run_id.as_deref())?;
    let normalized = action.trim().to_ascii_lowercase().replace('_', "-");
    match normalized.as_str() {
        "plan" => {
            let plan = manager.plan_run_actions(&run_id)?;
            print_run_action_value(cmd.json, "Action Plan", &plan)
        }
        "next" => {
            let execution = manager.execute_next_action(&run_id)?;
            print_run_action_value(cmd.json, "Action Execution", &execution)
        }
        "loop" => {
            let result = manager.run_action_loop(&run_id, cmd.max_actions)?;
            print_run_action_value(cmd.json, "Action Loop", &result)
        }
        "retry" => {
            let recovery = manager.attempt_recovery(&run_id)?;
            let run = manager.load(&run_id)?;
            let plan = manager.plan_run_actions(&run_id)?;
            let value = serde_json::json!({
                "recovery": recovery,
                "run": run,
                "next_plan": plan,
            });
            print_run_action_value(cmd.json, "Action Retry", &value)
        }
        "replay" => {
            let run = manager.load(&run_id)?;
            print_run_action_value(cmd.json, "Action Replay", &run.replay())
        }
        other => anyhow::bail!(
            "unknown run action '{}'; expected plan, next, loop, retry, or replay",
            other
        ),
    }
}
// END_run_action_command

// START_CONTRACT_resolve_run_id
// PURPOSE: Resolve explicit run id or newest persisted run id for action commands
// INPUTS: { manager: &RunManager }, { run_id: Option<&str> }
// OUTPUTS: { anyhow::Result<String> }
// START_resolve_run_id
fn resolve_run_id(manager: &RunManager, run_id: Option<&str>) -> anyhow::Result<String> {
    if let Some(run_id) = run_id.filter(|value| !value.trim().is_empty()) {
        return Ok(run_id.to_string());
    }
    manager
        .list()?
        .into_iter()
        .max_by(|left, right| left.updated_at.cmp(&right.updated_at))
        .map(|run| run.run_id)
        .ok_or_else(|| anyhow::anyhow!("no persisted runs found; pass --run-id or create a run"))
}
// END_resolve_run_id

// START_CONTRACT_print_run_scenario_result
// PURPOSE: Print a compact autonomous scenario summary suitable for release smoke checks
// INPUTS: { result: &RunScenarioResult }
// OUTPUTS: { stdout summary }
// START_print_run_scenario_result
fn print_run_scenario_result(result: &RunScenarioResult) {
    println!("=== Run Scenario ===");
    println!("mode:                  {}", result.mode.as_str());
    println!("run_id:                {}", result.run.run_id);
    println!("status:                {:?}", result.run.status);
    println!("phase:                 {}", result.run.phase);
    println!("module:                {}", result.run.module_id);
    println!("objective:             {}", result.run.objective);
    println!(
        "gate_decision:         {}",
        if result.decision.blocked {
            "blocked"
        } else {
            "passed"
        }
    );
    if let Some(reason) = &result.decision.reason {
        println!("blocked_reason:        {}", reason);
    }
    if let Some(action) = &result.decision.next_action {
        println!("next_action:           {}", action);
    }
    println!("blocked_before_review: {}", result.blocked_before_review);
    println!(
        "latest_review:         {}",
        result
            .run
            .latest_review_status()
            .map(|status| format!("{:?}", status))
            .unwrap_or_else(|| "none".into())
    );
    println!("evidence_refs:         {}", result.evidence_refs.len());
    for evidence in result.evidence_refs.iter().take(8) {
        println!("  - {}", evidence);
    }
    println!("replay_events:         {}", result.replay.events.len());
}
// END_print_run_scenario_result

// START_CONTRACT_print_run_action_value
// PURPOSE: Print action command output as JSON or compact text
// INPUTS: { json: bool }, { title: &str }, { value: &T }
// OUTPUTS: { anyhow::Result<()> }
// START_print_run_action_value
fn print_run_action_value<T>(json: bool, title: &str, value: &T) -> anyhow::Result<()>
where
    T: serde::Serialize,
{
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
        return Ok(());
    }
    let value = serde_json::to_value(value)?;
    println!("=== {} ===", title);
    if let Some(plan) = value
        .as_object()
        .and_then(|object| object.get("actions"))
        .and_then(|actions| actions.as_array())
    {
        println!("actions: {}", plan.len());
        for action in plan.iter().take(8) {
            println!(
                "  - {} {:?}",
                action["id"].as_str().unwrap_or("unknown"),
                action["kind"]
            );
        }
    } else {
        println!("{}", serde_json::to_string_pretty(&value)?);
    }
    Ok(())
}
// END_print_run_action_value

impl GainCmd {
    // START_CONTRACT_GainCmd::run
    // PURPOSE: Print token savings analytics
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // LINKS:
    //   → M-TRACKING (depends) - reads command, adapter, and session economy stats
    //   → NFR-003 (traces_to) - displays token savings to users and agents
    // START_gain_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let tracker = crate::tracking::Tracker::new(&config);
        let stats = tracker.get_stats().await?;
        println!("=== Token Savings Report ===");
        println!("Commands tracked:    {}", stats.total_commands);
        println!("Input tokens:        {}", stats.total_input_tokens);
        println!("Output tokens:       {}", stats.total_output_tokens);
        println!("Tokens saved:        {}", stats.total_saved_tokens);
        println!("Average savings:     {:.1}%", stats.avg_savings_pct);
        if self.adapters || self.sessions {
            println!("Adapter groups:      {}", stats.adapter_groups);
            println!("Session groups:      {}", stats.session_groups);
        }
        if stats.total_commands > 0 {
            let est_cost_saved = stats.total_saved_tokens as f64 * 0.000003;
            println!("Est. cost saved:     ${:.4}", est_cost_saved);
        }
        if !stats.top_commands.is_empty() {
            println!();
            println!("Top commands by token savings:");
            for item in &stats.top_commands {
                println!(
                    "  - {}: {} runs, {} tokens saved, avg {:.1}%",
                    item.command, item.count, item.saved_tokens, item.avg_savings_pct
                );
            }
        }
        if self.adapters || !stats.top_adapters.is_empty() {
            print_adapter_stats(&stats.top_adapters);
        }
        if self.sessions {
            print_session_stats(&stats.recent_sessions);
        }
        if self.graph {
            print_savings_graph(&stats.top_commands);
        }
        Ok(())
    }
    // END_gain_run
}

// START_CONTRACT_print_savings_graph
// PURPOSE: Render token savings by command as compact ASCII bars
// INPUTS: { items: &[TrackingCommandStat] — top tracked command savings }
// OUTPUTS: { stdout graph lines }
// SIDE_EFFECTS: writes to stdout
// LINKS:
//   → NFR-003 (traces_to) - graph summarizes token savings compactly
// START_print_savings_graph
fn print_savings_graph(items: &[crate::tracking::TrackingCommandStat]) {
    if items.is_empty() {
        return;
    }
    let max_saved = items
        .iter()
        .map(|item| item.saved_tokens)
        .max()
        .unwrap_or(0);
    println!();
    println!("Savings graph:");
    for item in items {
        let width = if max_saved == 0 {
            0
        } else {
            ((item.saved_tokens as f64 / max_saved as f64) * 24.0).round() as usize
        };
        let visible_width = if item.saved_tokens > 0 {
            width.max(1)
        } else {
            0
        };
        println!(
            "  {:<16} | {:<24} {}",
            item.command,
            "#".repeat(visible_width),
            item.saved_tokens
        );
    }
}
// END_print_savings_graph

// START_CONTRACT_print_adapter_stats
// PURPOSE: Render token savings grouped by routed adapter
// INPUTS: { items: &[TrackingAdapterStat] }
// OUTPUTS: { stdout adapter summary lines }
// SIDE_EFFECTS: writes to stdout
// LINKS:
//   → M-PROXY-ROUTER (depends) - adapter names come from route decisions
//   → NFR-003 (traces_to) - adapter economics highlight high-impact routing
// START_print_adapter_stats
fn print_adapter_stats(items: &[crate::tracking::TrackingAdapterStat]) {
    if items.is_empty() {
        return;
    }
    println!();
    println!("Top adapters by token savings:");
    for item in items {
        println!(
            "  - {}: {} runs, {} tokens saved, avg {:.1}%",
            item.adapter, item.count, item.saved_tokens, item.avg_savings_pct
        );
    }
}
// END_print_adapter_stats

// START_CONTRACT_print_session_stats
// PURPOSE: Render token savings grouped by session id
// INPUTS: { items: &[TrackingSessionStat] }
// OUTPUTS: { stdout session summary lines }
// SIDE_EFFECTS: writes to stdout
// LINKS:
//   → M-TRACKING (depends) - session stats are loaded from tracking database
//   → NFR-003 (traces_to) - session economics show per-agent-run savings
// START_print_session_stats
fn print_session_stats(items: &[crate::tracking::TrackingSessionStat]) {
    if items.is_empty() {
        return;
    }
    println!();
    println!("Session economics:");
    for item in items {
        println!(
            "  - {}: {} runs, {} → {} tokens, saved {}, avg {:.1}%",
            item.session_id,
            item.count,
            item.input_tokens,
            item.output_tokens,
            item.saved_tokens,
            item.avg_savings_pct
        );
    }
}
// END_print_session_stats

impl ProxyCmd {
    // START_CONTRACT_ProxyCmd::run
    // PURPOSE: Execute or preview a command through the token-saving proxy with optional raw evidence hint
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: may run external command through proxy
    // LINKS:
    //   → M-PROXY-ROUTER (depends) - --route previews command classification
    //   → M-PROXY (depends) - command execution path
    //   → UC-002 (implements) - shell execution remains observable
    // START_proxy_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        if self.args.is_empty() {
            anyhow::bail!("Usage: syn proxy [--route] -- <command> [args...]");
        }
        if self.route {
            let router = crate::proxy::router::CommandRouter::new();
            let decision = router.route(&self.args);
            print_route_decision(&decision);
            return Ok(());
        }
        let proxy = crate::proxy::Proxy::new(&config);
        let output = proxy.execute(&self.args).await?;
        {
            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "{}", output.text)?;
            if self.evidence {
                if let Some(path) = &output.evidence_path {
                    writeln!(
                        stdout,
                        "Raw output evidence: {} ({} bytes)",
                        path.display(),
                        output.raw_bytes
                    )?;
                } else {
                    writeln!(stdout, "Raw output evidence: none")?;
                }
            }
            stdout.flush()?;
        }
        if !output.success {
            std::process::exit(output.status_code);
        }
        Ok(())
    }
    // END_proxy_run
}

impl FiltersCmd {
    // START_CONTRACT_FiltersCmd::run
    // PURPOSE: Verify inline filter tests and manage trust for project-local proxy filters
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: may write or remove filter trust metadata
    // LINKS:
    //   → M-PROXY-FILTER (depends) - delegates filter verification and trust state
    //   → Phase-24 (implements) - RTK-style filter lifecycle management
    // START_filters_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        match &self.action {
            FiltersAction::Verify(cmd) => {
                let engine = crate::proxy::toml_filter::FilterEngine::new();
                let results = engine
                    .verify(cmd.filter.as_deref(), cmd.require_all)
                    .map_err(|err| anyhow::anyhow!(err))?;
                print_filter_verify_results(&results, cmd.require_all);
                if !results.passed(cmd.require_all) {
                    anyhow::bail!("filter verification failed");
                }
                Ok(())
            }
            FiltersAction::Trust => {
                let entry = crate::proxy::filter_trust::trust_project_filters()?;
                println!("Trusted project filters:");
                println!("path:   {}", entry.path);
                println!("sha256: {}", entry.sha256);
                Ok(())
            }
            FiltersAction::Untrust => {
                let removed = crate::proxy::filter_trust::untrust_project_filters()?;
                if removed {
                    println!("Project filter trust removed");
                } else {
                    println!("No project filter trust entry found");
                }
                Ok(())
            }
            FiltersAction::Status => {
                let status = crate::proxy::filter_trust::project_filter_status()?;
                println!("Project filter status: {}", status.label());
                if let crate::proxy::filter_trust::TrustStatus::ContentChanged {
                    trusted_sha256,
                    current_sha256,
                } = status
                {
                    println!("trusted_sha256: {}", trusted_sha256);
                    println!("current_sha256: {}", current_sha256);
                }
                Ok(())
            }
        }
    }
    // END_filters_run
}

// START_CONTRACT_print_filter_verify_results
// PURPOSE: Render filter inline-test verification results for CLI users and agents
// INPUTS: { results: &FilterVerifyResults }, { require_all: bool }
// OUTPUTS: { stdout verification summary }
// SIDE_EFFECTS: writes to stdout
// LINKS:
//   → M-PROXY-FILTER (depends) - renders verification outcomes
// START_print_filter_verify_results
fn print_filter_verify_results(
    results: &crate::proxy::toml_filter::FilterVerifyResults,
    require_all: bool,
) {
    println!("=== Synapse Filter Verification ===");
    for warning in &results.warnings {
        println!("WARN {}", warning);
    }
    if results.outcomes.is_empty() {
        println!("No inline filter tests found.");
    } else {
        for outcome in &results.outcomes {
            let status = if outcome.passed { "PASS" } else { "FAIL" };
            println!("{} {}::{}", status, outcome.filter_name, outcome.test_name);
            if !outcome.passed {
                println!("  expected: {}", outcome.expected);
                println!("  actual:   {}", outcome.actual);
            }
        }
    }
    if require_all && !results.filters_without_tests.is_empty() {
        println!(
            "Filters without tests: {}",
            results.filters_without_tests.join(", ")
        );
    }
    if results.passed(require_all) {
        println!("Filter verification passed");
    } else {
        println!("Filter verification failed");
    }
}
// END_print_filter_verify_results

// START_CONTRACT_print_route_decision
// PURPOSE: Render a command-router decision for dry-run diagnostics
// INPUTS: { decision: &RouteDecision }
// OUTPUTS: { stdout route lines }
// SIDE_EFFECTS: writes to stdout
// LINKS:
//   → M-PROXY-ROUTER (depends) - displays route decision metadata
//   → NFR-003 (traces_to) - route preview helps agents choose proxied commands
// START_print_route_decision
fn print_route_decision(decision: &crate::proxy::router::RouteDecision) {
    println!("=== Synapse Proxy Route ===");
    println!("should_proxy: {}", decision.should_proxy);
    println!("adapter:      {}", decision.adapter);
    println!("family:       {}", decision.family);
    println!("route_key:    {}", decision.route_key);
    println!("filter_key:   {}", decision.filter_key);
    println!("reason:       {}", decision.reason);
}
// END_print_route_decision

impl CompressCmd {
    // START_CONTRACT_CompressCmd::run
    // PURPOSE: Compress or restore one or more files
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes compressed or restored file contents
    // START_compress_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let compressor = crate::compress::Compressor::new(&config);
        for path in &self.path {
            let p = std::path::Path::new(path);
            if self.restore {
                compressor.restore_file(p).await?;
            } else {
                compressor.compress_file(p).await?;
            }
        }
        Ok(())
    }
    // END_compress_run
}

impl McpCmd {
    // START_CONTRACT_McpCmd::run
    // PURPOSE: Start MCP server over stdio
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: starts MCP server loop
    // START_mcp_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let server = crate::mcp::server::McpServer::new(config);
        server.start_stdio().await
    }
    // END_mcp_run
}

impl ConfigCmd {
    // START_CONTRACT_ConfigCmd::run
    // PURPOSE: Print config, print path, open editor, or acknowledge set command
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: may spawn configured editor
    // START_config_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        if self.args.is_empty() {
            println!("{}", toml::to_string_pretty(&config)?);
        } else if self.args.len() == CONFIG_SINGLE_ARG_COUNT && self.args[0] == "path" {
            println!("{}", Config::path()?.display());
        } else if self.args.len() == CONFIG_SINGLE_ARG_COUNT && self.args[0] == "edit" {
            let path = Config::path()?;
            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| "vim".into());
            std::process::Command::new(editor).arg(&path).status()?;
        } else if self.args.len() == CONFIG_SET_ARG_COUNT && self.args[0] == "set" {
            let key = &self.args[1];
            let value = &self.args[2];
            println!("Set {} = {} (not yet persisted)", key, value);
        } else {
            anyhow::bail!("Usage: syn config [path|edit|set <key> <value>]")
        }
        Ok(())
    }
    // END_config_run
}

impl HooksCmd {
    // START_CONTRACT_HooksCmd::run
    // PURPOSE: Install, uninstall, audit, or report Synapse hook status
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: may write or remove hook files
    // START_hooks_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let manager = crate::hooks::HookManager::new(&config);
        match self.action.as_str() {
            "install" => manager.install(&self.agent),
            "uninstall" => manager.uninstall(&self.agent),
            "status" => manager.status(&self.agent),
            "audit" | "check" => manager.audit(&self.agent, self.json),
            _ => anyhow::bail!(
                "Usage: syn hooks install|uninstall|status|audit|check [opencode|claude|cursor|gemini|copilot|all] [--json]"
            ),
        }
    }
    // END_hooks_run
}

impl DoctorCmd {
    // START_CONTRACT_DoctorCmd::run
    // PURPOSE: Run local setup diagnostics for config, index, hooks, docs, parsers, tracking, sources, and dependencies
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_doctor_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        use colored::Colorize;
        let root = std::env::current_dir()?;
        println!("Synapse Doctor — checking your setup...\n");

        let mut issues = 0u32;

        macro_rules! check {
            ($label:expr, $cond:expr, $ok:expr, $fail:expr) => {
                if $cond {
                    println!(
                        "  {} {}",
                        "PASS".green().bold(),
                        format!("{} — {}", $label, $ok).green()
                    );
                } else {
                    println!(
                        "  {} {}",
                        "FAIL".red().bold(),
                        format!("{} — {}", $label, $fail).red()
                    );
                    issues += 1;
                }
            };
        }

        match Config::path() {
            Ok(path) if path.exists() => match Config::load() {
                Ok(c) => {
                    check!("config", true, &format!("loaded ({})", path.display()), "");
                    let _ = c;
                }
                Err(e) => {
                    check!("config", false, "", &format!("cannot load: {}", e));
                }
            },
            Ok(path) => {
                check!(
                    "config",
                    true,
                    &format!(
                        "using built-in defaults (no user config at {})",
                        path.display()
                    ),
                    ""
                );
            }
            Err(e) => {
                check!("config", false, "", &format!("cannot resolve path: {}", e));
            }
        }

        let storage = crate::indexer::storage::Storage::new(&root);
        let indexed = !storage.is_empty();
        let index_failure = storage
            .load_error()
            .map(|error| format!("index storage unreadable — run `syn index` ({})", error))
            .unwrap_or_else(|| "not indexed — run `syn index`".to_string());
        check!(
            "index",
            indexed && storage.load_error().is_none(),
            &format!("{} blocks indexed", storage.count()),
            &index_failure
        );

        let oc_agents = root.join("AGENTS.md").exists();
        check!(
            "agents.md",
            oc_agents,
            "AGENTS.md (GRACE constitution)",
            "missing — run `syn init`"
        );

        let oc_rules = root.join(".opencode/rules/synapse.md").exists();
        check!(
            "opencode rules",
            oc_rules,
            ".opencode/rules/synapse.md",
            "missing — run `syn init`"
        );

        let oc_plugin = root.join(".opencode/plugins/synapse.ts").exists();
        check!(
            "opencode plugin",
            oc_plugin,
            ".opencode/plugins/synapse.ts",
            "missing — run `syn init`"
        );

        let oc_mcp = root.join("opencode.jsonc").exists() || root.join("opencode.json").exists();
        check!(
            "opencode mcp",
            oc_mcp,
            "MCP config present",
            "missing — run `syn init`"
        );

        let phase0_done = root.join("docs/requirements.xml").exists()
            && root.join("docs/technology.xml").exists()
            && root.join("docs/development-plan.xml").exists()
            && root.join("docs/verification-plan.xml").exists()
            && root.join("docs/knowledge-graph.xml").exists();
        check!(
            "phase 0 docs",
            phase0_done,
            "All 5 GRACE docs present",
            "missing — run `syn init` to create templates"
        );

        let parser = crate::indexer::parser::ParserEngine::new();
        let test_code = "fn test() {}";
        let blocks = parser.parse(test_code, "rust");
        check!(
            "tree-sitter",
            !blocks.is_empty(),
            &format!("Rust parser works ({} blocks)", blocks.len()),
            "parser failed — tree-sitter may be broken"
        );

        let db = crate::tracking::Tracker::db_path().ok();
        check!(
            "sqlite tracking",
            db.is_some(),
            "tracking DB available",
            "cannot find tracking DB path"
        );

        let has_sources = !crate::indexer::walker::Walker::new(&root).walk().is_empty();
        check!(
            "sources",
            has_sources,
            "source files found",
            "no source files in project"
        );

        issues += check_project_dependencies(&root, self.deps)?;

        println!();
        if issues == 0 {
            println!("{}", "Synapse is ready! All checks passed.".green().bold());
            println!();
            println!("Next steps:");
            println!("  syn index              Index the codebase");
            println!("  syn status             View project health");
            println!("  syn search <query>     Search indexed code");
            println!("  opencode               Start AI development");
        } else {
            println!(
                "{} {} issue(s) found. Run `syn init` to fix setup.",
                "WARN".yellow().bold(),
                issues
            );
            if !indexed {
                println!("  • Run: syn index");
            }
            if !oc_rules || !oc_plugin || !oc_mcp {
                println!("  • Run: syn init");
            }
        }

        Ok(())
    }
    // END_doctor_run
}

// START_CONTRACT_check_project_dependencies
// PURPOSE: Check Python dependency readiness when dependency manifests are present or explicitly requested
// INPUTS: { root: &Path — project root }, { force: bool — run even without a manifest }
// OUTPUTS: { anyhow::Result<u32> — number of dependency issues }
// SIDE_EFFECTS: runs local python/pip probes and prints diagnostic lines
// START_check_project_dependencies
fn check_project_dependencies(root: &Path, force: bool) -> anyhow::Result<u32> {
    let requirements_path = root.join("requirements.txt");
    let pyproject_path = root.join("pyproject.toml");
    let has_requirements = requirements_path.exists();
    let has_pyproject = pyproject_path.exists();
    let has_python_files = crate::indexer::walker::Walker::new(root)
        .walk()
        .iter()
        .any(|file| file.language == "python");

    if !force && !has_requirements && !has_pyproject {
        return Ok(0);
    }

    println!();
    println!("Dependency checks:");

    let mut issues = 0u32;
    let Some(python) = find_python_command() else {
        print_doctor_fail(
            "python",
            "python3/python not found — install Python before running dependency checks",
        );
        return Ok(1);
    };
    print_doctor_pass("python", &format!("{} available", python));

    let pip_available = command_success(&python, &["-m", "pip", "--version"]);
    if has_requirements || has_pyproject || force {
        if pip_available {
            print_doctor_pass("pip", "python -m pip available");
        } else {
            print_doctor_fail("pip", "python -m pip unavailable");
            issues += 1;
        }
    }

    if has_requirements && pip_available {
        let content = std::fs::read_to_string(&requirements_path)?;
        let imports = parse_requirement_imports(&content);
        let missing: Vec<String> = imports
            .iter()
            .filter(|import_name| !python_import_available(&python, import_name))
            .cloned()
            .collect();
        if missing.is_empty() {
            print_doctor_pass(
                "requirements",
                &format!("{} import checks passed", imports.len()),
            );
        } else {
            print_doctor_fail(
                "requirements",
                &format!(
                    "{} missing imports: {}. Run `{} -m pip install -r requirements.txt`",
                    missing.len(),
                    missing
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                    python
                ),
            );
            issues += 1;
        }
    } else if has_pyproject && pip_available {
        if command_success(&python, &["-m", "pip", "check"]) {
            print_doctor_pass("pyproject", "pip environment has no broken requirements");
        } else {
            print_doctor_fail(
                "pyproject",
                "pip check found broken requirements — run `python -m pip install -e .` or `uv sync`",
            );
            issues += 1;
        }
    } else if force && has_python_files {
        print_doctor_pass(
            "python dependencies",
            "Python files found but no requirements.txt or pyproject.toml; manifest audit skipped",
        );
    }

    Ok(issues)
}
// END_check_project_dependencies

// START_CONTRACT_find_python_command
// PURPOSE: Locate an executable Python command for dependency diagnostics
// OUTPUTS: { Option<String> — python command name }
// START_find_python_command
fn find_python_command() -> Option<String> {
    ["python3", "python"]
        .iter()
        .find(|candidate| command_success(candidate, &["--version"]))
        .map(|candidate| candidate.to_string())
}
// END_find_python_command

// START_CONTRACT_command_success
// PURPOSE: Run a local command probe and return whether it exited successfully
// INPUTS: { program: &str }, { args: &[&str] }
// OUTPUTS: { bool }
// START_command_success
fn command_success(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
// END_command_success

// START_CONTRACT_python_import_available
// PURPOSE: Check whether a Python import target is importable in the current environment
// INPUTS: { python: &str — python command }, { import_name: &str — import module name }
// OUTPUTS: { bool }
// START_python_import_available
fn python_import_available(python: &str, import_name: &str) -> bool {
    Command::new(python)
        .args([
            "-c",
            "import importlib.util, sys; sys.exit(0 if importlib.util.find_spec(sys.argv[1]) else 1)",
            import_name,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
// END_python_import_available

// START_CONTRACT_parse_requirement_imports
// PURPOSE: Convert simple requirements.txt entries to best-effort Python import names
// INPUTS: { content: &str — requirements.txt content }
// OUTPUTS: { Vec<String> — import names to probe }
// START_parse_requirement_imports
fn parse_requirement_imports(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(parse_requirement_import)
        .collect()
}
// END_parse_requirement_imports

// START_CONTRACT_parse_requirement_import
// PURPOSE: Parse one requirements.txt line into a best-effort import name
// INPUTS: { line: &str — raw requirement line }
// OUTPUTS: { Option<String> }
// START_parse_requirement_import
fn parse_requirement_import(line: &str) -> Option<String> {
    let clean = line.split('#').next()?.trim();
    if clean.is_empty()
        || clean.starts_with('-')
        || clean.starts_with("git+")
        || clean.contains("://")
        || clean.starts_with('.')
    {
        return None;
    }
    let package = clean
        .split(['=', '<', '>', '!', '~', '[', ';'])
        .next()?
        .trim();
    if package.is_empty() {
        None
    } else {
        Some(python_import_name(package))
    }
}
// END_parse_requirement_import

// START_CONTRACT_python_import_name
// PURPOSE: Map common package distribution names to their import module names
// INPUTS: { package: &str — distribution package name }
// OUTPUTS: { String — import module candidate }
// START_python_import_name
fn python_import_name(package: &str) -> String {
    match package.to_ascii_lowercase().as_str() {
        "beautifulsoup4" => "bs4".into(),
        "pillow" => "PIL".into(),
        "pyyaml" => "yaml".into(),
        "python-dotenv" => "dotenv".into(),
        "python-telegram-bot" => "telegram".into(),
        _ => package.replace('-', "_"),
    }
}
// END_python_import_name

// START_CONTRACT_print_doctor_pass
// PURPOSE: Print a standardized PASS diagnostic line outside the DoctorCmd macro scope
// INPUTS: { label: &str }, { message: &str }
// OUTPUTS: { stdout line }
// START_print_doctor_pass
fn print_doctor_pass(label: &str, message: &str) {
    use colored::Colorize;
    println!(
        "  {} {}",
        "PASS".green().bold(),
        format!("{} — {}", label, message).green()
    );
}
// END_print_doctor_pass

// START_CONTRACT_print_doctor_fail
// PURPOSE: Print a standardized FAIL diagnostic line outside the DoctorCmd macro scope
// INPUTS: { label: &str }, { message: &str }
// OUTPUTS: { stdout line }
// START_print_doctor_fail
fn print_doctor_fail(label: &str, message: &str) {
    use colored::Colorize;
    println!(
        "  {} {}",
        "FAIL".red().bold(),
        format!("{} — {}", label, message).red()
    );
}
// END_print_doctor_fail

impl ServeCmd {
    // START_CONTRACT_ServeCmd::run
    // PURPOSE: Start the web dashboard
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: starts HTTP server
    // START_serve_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        crate::dashboard::start_dashboard(&self.bind).await
    }
    // END_serve_run
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // START_CONTRACT_test_parse_requirement_imports_skips_non_packages
    // PURPOSE: Verify dependency diagnostics parse simple requirements without following nested or remote specs
    // START_test_parse_requirement_imports_skips_non_packages
    fn test_parse_requirement_imports_skips_non_packages() {
        let imports = parse_requirement_imports(
            "requests==2.32.0\n-r dev.txt\ngit+https://example/repo.git\npython-dotenv>=1\nbeautifulsoup4\n",
        );

        assert_eq!(
            imports,
            vec![
                "requests".to_string(),
                "dotenv".to_string(),
                "bs4".to_string()
            ]
        );
    }
    // END_test_parse_requirement_imports_skips_non_packages
}

// END_public_api
