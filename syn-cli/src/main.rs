// MODULE_CONTRACT
// MODULE_ID: M-WORKSPACE-CLI
// PURPOSE: Binary entry point — parses CLI, resolves runtime config, initializes telemetry-aware tracing, and delegates command dispatch
// DEPENDS: M-WORKSPACE-CORE
// LINKS: Cargo.toml

// START_MODULE_MAP
// main — Binary entry point: parses CLI, initializes telemetry-aware tracing, dispatches to command handler
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Moved to syn-cli workspace crate]
// END_CHANGE_SUMMARY

use clap::Parser;
use syn_cli::cli::{RunCommand, SynCli};
use syn_core::config::Config;

// START_main
fn main() -> anyhow::Result<()> {
    let cli = SynCli::parse();
    let config = Config::load_or_default();
    let runtime = tokio::runtime::Runtime::new()?;
    let telemetry = {
        let _runtime_guard = runtime.enter();
        syn_core::telemetry::init_telemetry(&config)?
    };
    let result = runtime.block_on(async { cli.command.run(config).await });
    {
        let _runtime_guard = runtime.enter();
        telemetry.shutdown()?;
    }
    result
}
// END_main
