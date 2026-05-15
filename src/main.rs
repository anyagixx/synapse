use clap::Parser;
use synapse::cli::SynCli;
use synapse::config::Config;

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
            synapse::cli::Command::Init(cmd) => cmd.run(config).await,
            synapse::cli::Command::Index(cmd) => cmd.run(config).await,
            synapse::cli::Command::Search(cmd) => cmd.run(config).await,
            synapse::cli::Command::View(cmd) => cmd.run(config).await,
            synapse::cli::Command::Grep(cmd) => cmd.run(config).await,
            synapse::cli::Command::Plan(cmd) => cmd.run(config).await,
            synapse::cli::Command::Execute(cmd) => cmd.run(config).await,
            synapse::cli::Command::Verify(cmd) => cmd.run(config).await,
            synapse::cli::Command::Review(cmd) => cmd.run(config).await,
            synapse::cli::Command::Fix(cmd) => cmd.run(config).await,
            synapse::cli::Command::Status(cmd) => cmd.run(config).await,
            synapse::cli::Command::Explain(cmd) => cmd.run(config).await,
            synapse::cli::Command::Proxy(cmd) => cmd.run(config).await,
            synapse::cli::Command::Gain(cmd) => cmd.run(config).await,
            synapse::cli::Command::Compress(cmd) => cmd.run(config).await,
            synapse::cli::Command::Mcp(cmd) => cmd.run(config).await,
            synapse::cli::Command::McpProxy(cmd) => cmd.run(config).await,
            synapse::cli::Command::Config(cmd) => cmd.run(config).await,
            synapse::cli::Command::Logs(cmd) => cmd.run(config).await,
            synapse::cli::Command::Telemetry(cmd) => cmd.run(config).await,
            synapse::cli::Command::GraphRag(cmd) => cmd.run(config).await,
        }
    })
}
