// MODULE_CONTRACT
// MODULE_ID: M-MAIN
// PURPOSE: Binary entry point — parses CLI, resolves runtime config, dispatches commands via tokio runtime
// SCOPE: CLI argument parsing, stderr tracing init, command dispatch
// DEPENDS: M-LIB, M-CLI, M-CONFIG
// LINKS: Cargo.toml

// START_MODULE_MAP
// main — Entry point: init tracing, parse CLI, resolve config, dispatch command
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.9.0 — Route tracing output to stderr so MCP stdio stdout remains protocol-clean]
// END_CHANGE_SUMMARY

use clap::Parser;
use syn::cli::SynCli;
use syn::config::Config;

// START_CONTRACT_main
// PURPOSE: Initialize tracing subscriber, parse CLI arguments, resolve config with default fallback, dispatch command via tokio
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

    tokio::runtime::Runtime::new()?.block_on(async {
        match cli.command {
            syn::cli::Command::Init(cmd) => cmd.run(config).await,
            syn::cli::Command::Index(cmd) => cmd.run(config).await,
            syn::cli::Command::Search(cmd) => cmd.run(config).await,
            syn::cli::Command::View(cmd) => cmd.run(config).await,
            syn::cli::Command::Verify(cmd) => cmd.run(config).await,
            syn::cli::Command::Review(cmd) => cmd.run(config).await,
            syn::cli::Command::Status(cmd) => cmd.run(config).await,
            syn::cli::Command::Proxy(cmd) => cmd.run(config).await,
            syn::cli::Command::Gain(cmd) => cmd.run(config).await,
            syn::cli::Command::Compress(cmd) => cmd.run(config).await,
            syn::cli::Command::Mcp(cmd) => cmd.run(config).await,
            syn::cli::Command::Config(cmd) => cmd.run(config).await,
            syn::cli::Command::GraphRag(cmd) => cmd.run(config).await,
            syn::cli::Command::Hooks(cmd) => cmd.run(config).await,
            syn::cli::Command::Doctor(cmd) => cmd.run(config).await,
            syn::cli::Command::Refresh(cmd) => cmd.run(config).await,
            syn::cli::Command::Ci(cmd) => match cmd.action {
                syn::cli::CiAction::Verify(inner) => inner.run(config).await,
                syn::cli::CiAction::Review(inner) => inner.run(config).await,
                syn::cli::CiAction::Status(inner) => inner.run(config).await,
            },
            syn::cli::Command::Skills(cmd) => cmd.run(config).await,
            syn::cli::Command::History(cmd) => cmd.run(config).await,
            syn::cli::Command::Serve(cmd) => cmd.run(config).await,
        }
    })
}
// END_main
