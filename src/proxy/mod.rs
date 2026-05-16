// MODULE_CONTRACT
// MODULE_ID: M-PROXY
// PURPOSE: Proxy executor — intercepts shell commands, applies TOML filters, tracks token savings
// SCOPE: Proxy struct, FilterEngine integration, CommandRunner integration, token tracking
// DEPENDS: M-CONFIG, M-TRACKING, M-PROXY-RUNNER, M-PROXY-FILTER, M-UTILS
// LINKS: filters.toml

// START_MODULE_MAP
// Proxy — Command proxy combining filter engine, command runner, and token tracker
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY

pub mod runner;
pub mod toml_filter;

use crate::config::Config;
use crate::tracking::Tracker;
use runner::CommandRunner;
use toml_filter::FilterEngine;

// START_public_api

// START_Proxy
pub struct Proxy {
    config: Config,
    engine: FilterEngine,
    tracker: Tracker,
}
// END_Proxy

impl Proxy {
    // START_CONTRACT_Proxy::new
    // PURPOSE: Create a new Proxy with the given config
    // OUTPUTS: { Self }
    // START_proxy_new
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            engine: FilterEngine::new(),
            tracker: Tracker::new(config),
        }
    }
    // END_proxy_new

    // START_CONTRACT_Proxy::execute
    // PURPOSE: Execute a shell command through the proxy: run, filter output, track tokens
    // INPUTS: { cmd_parts: &[String] — command and args }
    // OUTPUTS: { anyhow::Result<String> — filtered output }
    // SIDE_EFFECTS: runs external command, writes tracking data
    // START_proxy_execute
    pub async fn execute(&self, cmd_parts: &[String]) -> anyhow::Result<String> {
        if cmd_parts.is_empty() {
            return Err(anyhow::anyhow!("No command specified"));
        }

        let full_cmd = cmd_parts.join(" ");
        let runner = CommandRunner::new(cmd_parts);

        // Execute the command
        let raw_output = runner.execute()?;
        let input_tokens = crate::utils::estimate_tokens(&raw_output);

        // Find and apply filter
        let output = match self.engine.find_filter(&full_cmd) {
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
                if raw_output.len() > max_chars {
                    format!(
                        "{}...\n[output truncated at {} chars]",
                        &raw_output[..max_chars],
                        max_chars
                    )
                } else {
                    raw_output
                }
            }
        };

        let output_tokens = crate::utils::estimate_tokens(&output) as u32;

        // Track savings
        self.tracker
            .record(&full_cmd, input_tokens, output_tokens)
            .await
            .ok();

        tracing::debug!(
            "Proxy: {} — {} tokens → {} tokens ({}% saved)",
            full_cmd,
            input_tokens,
            output_tokens,
            crate::utils::format_savings(input_tokens, output_tokens),
        );

        Ok(output)
    }
    // END_proxy_execute
}
// END_public_api
