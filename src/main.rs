// MODULE_CONTRACT
// MODULE_ID: M-MAIN
// PURPOSE: Binary entry point — parses CLI, resolves runtime config, initializes stderr tracing, and delegates command dispatch through the CLI module
// SCOPE: CLI argument parsing, stderr tracing init, config loading, tokio runtime creation, and CLI-owned command dispatch delegation
// DEPENDS: M-LIB, M-CLI, M-CONFIG
// LINKS: Cargo.toml

// START_MODULE_MAP
// main — Entry point: init tracing, parse CLI, resolve config, delegate command dispatch
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v5.5.0 — Delegated command dispatch to CLI RunCommand]
// END_CHANGE_SUMMARY

use clap::Parser;
use syn::cli::{RunCommand, SynCli};
use syn::config::Config;

// START_CONTRACT_main
// PURPOSE: Initialize tracing subscriber, parse CLI arguments, resolve config with default fallback, and delegate command execution to the CLI schema module
// OUTPUTS: { anyhow::Result — ok on success, error on failure }
// SIDE_EFFECTS: initializes tracing, executes selected CLI command
// START_main
fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .json()
        .init();

    let cli = SynCli::parse();
    let config = Config::load_or_default();

    tokio::runtime::Runtime::new()?.block_on(async { cli.command.run(config).await })
}
// END_main
