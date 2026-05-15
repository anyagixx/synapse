use clap::Parser;
use syn::cli::SynCli;
use syn::config::Config;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = SynCli::parse();
    let config = Config::load()?;

    tokio::runtime::Runtime::new()?.block_on(async {
        match cli.command {
            syn::cli::Command::Init(cmd) => cmd.run(config).await,
            syn::cli::Command::Index(cmd) => cmd.run(config).await,
            syn::cli::Command::Search(cmd) => cmd.run(config).await,
            syn::cli::Command::View(cmd) => cmd.run(config).await,
            syn::cli::Command::Grep(cmd) => cmd.run(config).await,
            syn::cli::Command::Plan(cmd) => cmd.run(config).await,
            syn::cli::Command::Execute(cmd) => cmd.run(config).await,
            syn::cli::Command::Verify(cmd) => cmd.run(config).await,
            syn::cli::Command::Review(cmd) => cmd.run(config).await,
            syn::cli::Command::Fix(cmd) => cmd.run(config).await,
            syn::cli::Command::Status(cmd) => cmd.run(config).await,
            syn::cli::Command::Explain(cmd) => cmd.run(config).await,
            syn::cli::Command::Proxy(cmd) => cmd.run(config).await,
            syn::cli::Command::Gain(cmd) => cmd.run(config).await,
            syn::cli::Command::Compress(cmd) => cmd.run(config).await,
            syn::cli::Command::Mcp(cmd) => cmd.run(config).await,
            syn::cli::Command::McpProxy(cmd) => cmd.run(config).await,
            syn::cli::Command::Config(cmd) => cmd.run(config).await,
            syn::cli::Command::Logs(cmd) => cmd.run(config).await,
            syn::cli::Command::Telemetry(cmd) => cmd.run(config).await,
            syn::cli::Command::GraphRag(cmd) => cmd.run(config).await,
        }
    })
}
