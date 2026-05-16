pub mod runner;
pub mod toml_filter;

use crate::config::Config;
use crate::tracking::Tracker;
use runner::CommandRunner;
use toml_filter::FilterEngine;

pub struct Proxy {
    config: Config,
    engine: FilterEngine,
    tracker: Tracker,
}

impl Proxy {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            engine: FilterEngine::new(),
            tracker: Tracker::new(config),
        }
    }

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
}
