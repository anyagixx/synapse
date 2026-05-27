// MODULE_CONTRACT
// MODULE_ID: M-PROXY
// PURPOSE: Proxy executor — routes shell commands, applies trusted TOML filters, records telemetry span fields, and reports token tracking degradation
// SCOPE: Proxy struct, command-router integration, trusted FilterEngine integration, filter dry-run submodule declaration, async-friendly configurable CommandRunner integration, raw evidence metadata, token tracking with degraded-mode logging, and proxy.execute tracing fields
// DEPENDS: M-CONFIG, M-TRACKING, M-PROXY-ROUTER, M-PROXY-RUNNER, M-PROXY-FILTER, M-UTILS
// LINKS:
//   → M-PROXY-ROUTER (depends) - RTK-style command classification
//   → M-PROXY-FILTER (depends) - TOML output filters
//   → M-TRACKING (depends) - token economy persistence
//   → UC-002 (implements) - proxied command execution evidence
//   → NFR-003 (traces_to) - token-saving shell output reduction

// START_MODULE_MAP
// Proxy — Command proxy combining router, filter engine, command runner, and token tracker
// ProxyOutput — Filtered output plus original command exit status and raw evidence metadata
// filter_dry_run — Internal explain-mode pipeline for M-PROXY-FILTER
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v5.3.0 - Added proxy.execute telemetry fields]
// END_CHANGE_SUMMARY

mod filter_dry_run;
pub mod filter_trust;
pub mod router;
pub mod runner;
pub mod toml_filter;

use syn_core::config::Config;
use syn_core::tracking::Tracker;
use router::CommandRouter;
use runner::CommandRunner;
use toml_filter::FilterEngine;

// START_public_api

// START_ProxyOutput
pub struct ProxyOutput {
    pub text: String,
    pub status_code: i32,
    pub success: bool,
    pub evidence_path: Option<std::path::PathBuf>,
    pub raw_bytes: u64,
}
// END_ProxyOutput

// START_Proxy
pub struct Proxy {
    config: Config,
    router: CommandRouter,
    engine: FilterEngine,
    tracker: Tracker,
}
// END_Proxy

impl Proxy {
    // START_CONTRACT_Proxy::new
    // PURPOSE: Create a new Proxy with the given config
    // OUTPUTS: { Self }
    // LINKS:
    //   → M-PROXY-ROUTER (depends) - initialize route classifier
    //   → NFR-003 (traces_to) - routing enables command-specific savings
    // START_proxy_new
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            router: CommandRouter::new(),
            engine: FilterEngine::new(),
            tracker: Tracker::new(config),
        }
    }
    // END_proxy_new

    // START_CONTRACT_Proxy::execute
    // PURPOSE: Execute a shell command through the proxy: run with evidence tee, filter output, track tokens
    // INPUTS: { cmd_parts: &[String] — command and args }
    // OUTPUTS: { anyhow::Result<ProxyOutput> — filtered output and original exit status }
    // SIDE_EFFECTS: runs external command, writes tracking data
    // LINKS:
    //   → M-PROXY-ROUTER (depends) - classify command before filtering
    //   → M-PROXY-FILTER (depends) - apply command-specific output filters
    //   → M-TRACKING (depends) - persist route-aware token economy data
    //   → UC-002 (implements) - shell command execution is tracked as bounded evidence
    //   → NFR-003 (traces_to) - filtered output reduces LLM context load
    // START_proxy_execute
    #[tracing::instrument(name = "proxy.execute", skip(self, cmd_parts), fields(command = %cmd_parts.first().map(|value| value.as_str()).unwrap_or(""), adapter = tracing::field::Empty, route = tracing::field::Empty, input_tokens = tracing::field::Empty, output_tokens = tracing::field::Empty, saved_pct = tracing::field::Empty))]
    pub async fn execute(&self, cmd_parts: &[String]) -> anyhow::Result<ProxyOutput> {
        if cmd_parts.is_empty() {
            return Err(anyhow::anyhow!("No command specified"));
        }

        let full_cmd = cmd_parts.join(" ");
        let route = self.router.route(cmd_parts);
        tracing::Span::current()
            .record("adapter", tracing::field::display(&route.adapter))
            .record("route", tracing::field::display(&route.route_key));
        let runner = CommandRunner::from_config(cmd_parts, &self.config);

        // Execute the command
        let raw = runner.execute().await?;
        let raw_output = raw.text;
        let input_tokens = syn_core::utils::estimate_tokens(&raw_output);

        // Find and apply filter
        let output = match self
            .engine
            .find_filter(&full_cmd)
            .or_else(|| self.engine.find_filter(&route.filter_key))
        {
            Some(filter) => {
                let filtered = self.engine.apply(filter, &raw_output);
                if filtered.is_empty() {
                    raw_output.clone()
                } else {
                    filtered
                }
            }
            None => {
                // Passthrough: limit output size
                let max_chars = self.config.proxy.passthrough_max_chars as usize;
                if raw_output.chars().count() > max_chars {
                    format!(
                        "{}\n[output truncated at {} chars]",
                        syn_core::utils::truncate_chars(&raw_output, max_chars),
                        max_chars
                    )
                } else {
                    raw_output
                }
            }
        };

        let output_tokens = syn_core::utils::estimate_tokens(&output) as u32;
        tracing::Span::current()
            .record("input_tokens", u64::from(input_tokens))
            .record("output_tokens", u64::from(output_tokens))
            .record(
                "saved_pct",
                tracing::field::display(syn_core::utils::format_savings(input_tokens, output_tokens)),
            );

        if let Err(e) = self
            .tracker
            .record_routed(
                &full_cmd,
                input_tokens,
                output_tokens,
                &route.adapter,
                &route.route_key,
            )
            .await
        {
            tracing::warn!("[Proxy][execute][TRACKING] token tracking degraded: {}", e);
        }

        tracing::debug!(
            "Proxy: {} [{}:{}] — {} tokens → {} tokens ({}% saved)",
            full_cmd,
            route.adapter,
            route.route_key,
            input_tokens,
            output_tokens,
            syn_core::utils::format_savings(input_tokens, output_tokens),
        );

        Ok(ProxyOutput {
            text: output,
            status_code: raw.status_code,
            success: raw.success,
            evidence_path: raw.evidence_path,
            raw_bytes: raw.raw_bytes,
        })
    }
    // END_proxy_execute
}
// END_public_api
