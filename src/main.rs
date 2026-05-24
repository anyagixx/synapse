// MODULE_CONTRACT
// MODULE_ID: M-MAIN
// PURPOSE: Binary entry point — parses CLI, resolves runtime config, dispatches commands via tokio runtime
// SCOPE: CLI argument parsing, stderr tracing init, command dispatch including RTK proxy shortcuts and hook rewrites
// DEPENDS: M-LIB, M-CLI, M-CONFIG
// LINKS: Cargo.toml

// START_MODULE_MAP
// main — Entry point: init tracing, parse CLI, resolve config, dispatch command
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v4.2.0 — Dispatch hook-facing syn rewrite command]
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
            syn::cli::Command::Read(cmd) => {
                cmd.run_as(config, "cat", true, "Usage: syn read <file>...")
                    .await
            }
            syn::cli::Command::Ls(cmd) => {
                cmd.run_as(config, "ls", false, "Usage: syn ls [args...]")
                    .await
            }
            syn::cli::Command::Tree(cmd) => {
                cmd.run_as(config, "tree", false, "Usage: syn tree [args...]")
                    .await
            }
            syn::cli::Command::Find(cmd) => {
                cmd.run_as(config, "find", false, "Usage: syn find [args...]")
                    .await
            }
            syn::cli::Command::Rg(cmd) => {
                cmd.run_as(config, "rg", true, "Usage: syn rg <pattern> [path...]")
                    .await
            }
            syn::cli::Command::Grep(cmd) => {
                cmd.run_as(config, "grep", true, "Usage: syn grep <pattern> [path...]")
                    .await
            }
            syn::cli::Command::Git(cmd) => {
                cmd.run_as(config, "git", false, "Usage: syn git [args...]")
                    .await
            }
            syn::cli::Command::Cargo(cmd) => {
                cmd.run_as(config, "cargo", false, "Usage: syn cargo [args...]")
                    .await
            }
            syn::cli::Command::Npm(cmd) => {
                cmd.run_as(config, "npm", false, "Usage: syn npm [args...]")
                    .await
            }
            syn::cli::Command::Pnpm(cmd) => {
                cmd.run_as(config, "pnpm", false, "Usage: syn pnpm [args...]")
                    .await
            }
            syn::cli::Command::Npx(cmd) => {
                cmd.run_as(config, "npx", false, "Usage: syn npx [args...]")
                    .await
            }
            syn::cli::Command::Pytest(cmd) => {
                cmd.run_as(config, "pytest", false, "Usage: syn pytest [args...]")
                    .await
            }
            syn::cli::Command::Rewrite(cmd) => cmd.run(config).await,
            syn::cli::Command::Run(cmd) => cmd.run(config).await,
            syn::cli::Command::Proxy(cmd) => cmd.run(config).await,
            syn::cli::Command::Filters(cmd) => cmd.run(config).await,
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
