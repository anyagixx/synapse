// MODULE_CONTRACT
// MODULE_ID: M-MAIN
// PURPOSE: Binary entry point — parses CLI, resolves runtime config, initializes telemetry-aware tracing, and delegates command dispatch through the CLI module
// SCOPE: CLI argument parsing, config loading, tokio runtime creation before telemetry init, CLI-owned command dispatch delegation, and runtime-entered telemetry shutdown
// DEPENDS: M-LIB, M-CLI, M-CONFIG, M-TELEMETRY
// LINKS: Cargo.toml

// START_MODULE_MAP
// main — Entry point: parse CLI, resolve config, init telemetry-aware tracing, delegate command dispatch, shut down telemetry
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v5.7.0 - Entered Tokio runtime context for OTLP telemetry init and shutdown]
// END_CHANGE_SUMMARY

use clap::Parser;
use syn::cli::{RunCommand, SynCli};
use syn::config::Config;

// START_CONTRACT_main
// PURPOSE: Parse CLI arguments, resolve config with default fallback, initialize tracing/telemetry, and delegate command execution to the CLI schema module
// OUTPUTS: { anyhow::Result — ok on success, error on failure }
// SIDE_EFFECTS: initializes tracing, executes selected CLI command
// START_main
fn main() -> anyhow::Result<()> {
    let cli = SynCli::parse();
    let config = Config::load_or_default();
    let runtime = tokio::runtime::Runtime::new()?;
    let telemetry = {
        let _runtime_guard = runtime.enter();
        syn::telemetry::init_telemetry(&config)?
    };
    let result = runtime.block_on(async { cli.command.run(config).await });
    {
        let _runtime_guard = runtime.enter();
        telemetry.shutdown()?;
    }
    result
}
// END_main
