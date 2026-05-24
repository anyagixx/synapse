// MODULE_CONTRACT
// MODULE_ID: M-PROXY
// PURPOSE: Proxy executor — routes shell commands, applies TOML filters, and reports token tracking degradation
// SCOPE: Proxy struct, command-router integration, FilterEngine integration, CommandRunner integration, token tracking with degraded-mode logging
// DEPENDS: M-CONFIG, M-TRACKING, M-PROXY-ROUTER, M-PROXY-RUNNER, M-PROXY-FILTER, M-UTILS
// LINKS:
//   → M-PROXY-ROUTER (depends) - RTK-style command classification
//   → M-PROXY-FILTER (depends) - TOML output filters
//   → M-TRACKING (depends) - token economy persistence
//   → UC-002 (implements) - proxied command execution evidence
//   → NFR-003 (traces_to) - token-saving shell output reduction

// START_MODULE_MAP
// Proxy — Command proxy combining router, filter engine, command runner, and token tracker
// ProxyOutput — Filtered output plus original command exit status
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.0.0 — Added RTK-style command routing and adapter-aware tracking]
// END_CHANGE_SUMMARY

pub mod router;
pub mod runner;
pub mod toml_filter;

use crate::config::Config;
use crate::tracking::Tracker;
use router::CommandRouter;
use runner::CommandRunner;
use toml_filter::FilterEngine;

// START_public_api

// START_ProxyOutput
pub struct ProxyOutput {
    pub text: String,
    pub status_code: i32,
    pub success: bool,
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
    // PURPOSE: Execute a shell command through the proxy: run, filter output, track tokens
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
    pub async fn execute(&self, cmd_parts: &[String]) -> anyhow::Result<ProxyOutput> {
        if cmd_parts.is_empty() {
            return Err(anyhow::anyhow!("No command specified"));
        }

        let full_cmd = cmd_parts.join(" ");
        let route = self.router.route(cmd_parts);
        let runner = CommandRunner::new(cmd_parts);

        // Execute the command
        let raw = runner.execute()?;
        let raw_output = raw.text;
        let input_tokens = crate::utils::estimate_tokens(&raw_output);

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
                        crate::utils::truncate_chars(&raw_output, max_chars),
                        max_chars
                    )
                } else {
                    raw_output
                }
            }
        };

        let output_tokens = crate::utils::estimate_tokens(&output) as u32;

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
            crate::utils::format_savings(input_tokens, output_tokens),
        );

        Ok(ProxyOutput {
            text: output,
            status_code: raw.status_code,
            success: raw.success,
        })
    }
    // END_proxy_execute
}
// END_public_api
