use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "syn", version, about = "Synapse — Unified AI Agent Engineering Platform")]
#[command(help_template = "\
{syn}{name} {version}
{about}

{usage-heading} {usage}

{all-args}{after-help}")]
pub struct SynCli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Init(InitCmd),
    Index(IndexCmd),
    Search(SearchCmd),
    View(ViewCmd),
    Grep(GrepCmd),
    Plan(PlanCmd),
    Execute(ExecuteCmd),
    Verify(VerifyCmd),
    Review(ReviewCmd),
    Fix(FixCmd),
    Status(StatusCmd),
    Explain(ExplainCmd),
    Proxy(ProxyCmd),
    Gain(GainCmd),
    Compress(CompressCmd),
    Mcp(McpCmd),
    McpProxy(McpProxyCmd),
    Config(ConfigCmd),
    Logs(LogsCmd),
    Telemetry(TelemetryCmd),
    #[command(name = "graphrag")]
    GraphRag(GraphRagCmd),
}

macro_rules! cmd_struct {
    ($name:ident, $about:expr) => {
        #[derive(clap::Args)]
        #[command(about = $about)]
        pub struct $name;
    };
    ($name:ident, $about:expr, $($field:ident: $ty:ty),+ $(,)?) => {
        #[derive(clap::Args)]
        #[command(about = $about)]
        pub struct $name {
            $(pub $field: $ty),+
        }
    };
}

cmd_struct!(InitCmd, "Bootstrap a new Synapse project");
#[derive(clap::Args)]
#[command(about = "Index codebase for semantic search")]
pub struct IndexCmd {
    #[arg(long)]
    pub watch: bool,
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub no_git: bool,
}
cmd_struct!(PlanCmd, "Generate architecture plan from requirements");
cmd_struct!(ExecuteCmd, "Execute development plan",
    module: Option<String>,
);
cmd_struct!(VerifyCmd, "Run verification suite",
    level: Option<String>,
    r#mod: Option<String>,
);
cmd_struct!(ReviewCmd, "GRACE integrity review",
    mode: Option<String>,
);
cmd_struct!(FixCmd, "Debug via knowledge graph",
    description: Vec<String>,
);
#[derive(clap::Args)]
#[command(about = "Project health report")]
pub struct StatusCmd {
    #[arg(long)]
    pub json: bool,
}
cmd_struct!(ExplainCmd, "Explain code using indexed context",
    query: Vec<String>,
);
#[derive(clap::Args)]
#[command(name = "graphrag", about = "Query the code knowledge graph")]
pub struct GraphRagCmd {
    pub operation: Option<String>,
    pub args: Vec<String>,
}
cmd_struct!(GainCmd, "View token savings analytics");
cmd_struct!(LogsCmd, "View MCP server logs");
cmd_struct!(TelemetryCmd, "Manage telemetry consent",
    action: Option<String>,
);

#[derive(clap::Args)]
#[command(about = "Semantic code search")]
pub struct SearchCmd {
    pub query: Vec<String>,
    #[arg(long, default_value = "code")]
    pub mode: String,
    #[arg(long)]
    pub language: Option<String>,
    #[arg(long, default_value_t = 10)]
    pub max_results: u32,
}

#[derive(clap::Args)]
#[command(about = "View file signatures")]
pub struct ViewCmd {
    pub path: Vec<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Args)]
#[command(about = "AST structural search")]
pub struct GrepCmd {
    pub pattern: String,
    #[arg(long)]
    pub rewrite: Option<String>,
    #[arg(long)]
    pub language: Option<String>,
}

#[derive(clap::Args)]
#[command(about = "Run command through token-saving proxy")]
pub struct ProxyCmd {
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

#[derive(clap::Args)]
#[command(about = "Compress files for AI context")]
pub struct CompressCmd {
    pub path: Vec<String>,
    #[arg(long, default_value = "full")]
    pub level: String,
    #[arg(long)]
    pub restore: bool,
}

#[derive(clap::Args)]
#[command(about = "Start MCP server")]
pub struct McpCmd {
    #[arg(long)]
    pub http: bool,
    #[arg(long)]
    pub bind: Option<String>,
    #[arg(long)]
    pub with_lsp: Option<String>,
}

#[derive(clap::Args)]
#[command(about = "Start multi-repo MCP proxy")]
pub struct McpProxyCmd {
    #[arg(long)]
    pub config: Option<String>,
}

#[derive(clap::Args)]
#[command(about = "Manage configuration")]
pub struct ConfigCmd {
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

use crate::config::Config;

macro_rules! cmd_run {
    ($($cmd:ty),+ $(,)?) => {
        $(impl $cmd {
            pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
                anyhow::bail!("Command not yet implemented: {}",
                    stringify!($cmd).trim_end_matches("Cmd"))
            }
        })+
    };
}

cmd_run!(InitCmd, PlanCmd, ExecuteCmd, LogsCmd, TelemetryCmd);

// Commands with args that have placeholder implementations
impl SearchCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        anyhow::bail!("Search not yet implemented. Run `syn index` first.")
    }
}

impl ViewCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        anyhow::bail!("View not yet implemented.")
    }
}

impl GrepCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        anyhow::bail!("Grep not yet implemented.")
    }
}

impl VerifyCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let results = crate::grace::GraceEngine::verify_project(&root).await?;
        let all_pass = results.iter().all(|r| r.passed);
        for r in &results {
            let status = if r.passed { "✓ PASS" } else { "✗ FAIL" };
            println!("[{}] {}", status, r.level);
            for c in &r.checks {
                println!("  {} {} — {}", if c.passed { "✓" } else { "✗" }, c.name, c.details);
            }
        }
        if !all_pass {
            anyhow::bail!("Verification failed. Fix issues above and re-run.");
        }
        println!("All checks passed.");
        Ok(())
    }
}

impl ReviewCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let mode = self.mode.as_deref().unwrap_or("scoped");
        let report = crate::grace::review::Reviewer::review(&root, mode)?;
        println!("=== GRACE Integrity Review ({}) ===", report.mode);
        for s in &report.sections {
            let status = if s.passed { "✓" } else { "✗" };
            println!("{} {} — {}", status, s.name, s.details);
            for issue in &s.issues {
                println!("    ⚠ {}", issue);
            }
        }
        if report.passed {
            println!("Review passed.");
        } else {
            anyhow::bail!("Review found issues.");
        }
        Ok(())
    }
}

impl FixCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let desc = self.description.join(" ");
        if desc.is_empty() {
            anyhow::bail!("Usage: syn fix <description of the bug>");
        }
        let result = crate::grace::fix::Debugger::diagnose(&desc, &root).await?;
        println!("=== Diagnosis ===");
        println!("Bug: {}", result.description);
        println!("{}", result.diagnosis);
        if !result.related_modules.is_empty() {
            println!("\nRelated modules:");
            for m in &result.related_modules {
                println!("  • {}", m);
            }
        }
        if !result.suggested_blocks.is_empty() {
            println!("\nRelevant code:");
            for b in &result.suggested_blocks {
                println!("{}\n", b);
            }
        }
        Ok(())
    }
}

impl ExplainCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let query = self.query.join(" ");
        if query.is_empty() {
            anyhow::bail!("Usage: syn explain <query>");
        }
        let result = crate::grace::explain::Explainer::explain(&query, &root).await?;
        println!("{}", result.answer);
        Ok(())
    }
}

impl StatusCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let report = crate::grace::status::StatusCollector::collect(&root).await?;
        crate::grace::status::StatusCollector::print_report(&report);
        if self.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Ok(())
    }
}

impl GraphRagCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let mut graphrag = crate::graphrag::GraphRag::new();
        graphrag.build(&root)?;
        match &self.operation {
            Some(op) if op == "overview" => {
                if let Some(ov) = graphrag.overview() {
                    println!("Graph Overview:");
                    println!("  Nodes: {}", ov.total_nodes);
                    println!("  Relationships: {}", ov.total_relationships);
                    for t in &ov.node_types {
                        println!("  Type: {}", t);
                    }
                }
            }
            Some(op) if op == "search" => {
                let q = self.args.join(" ");
                let nodes = graphrag.search_nodes(&q);
                for n in &nodes {
                    println!("• {} — {} ({}:{})", n.name, n.kind, n.path, n.size_lines);
                }
            }
            _ => {
                if let Some(ov) = graphrag.overview() {
                    println!("Nodes: {}, Relationships: {}", ov.total_nodes, ov.total_relationships);
                }
            }
        }
        Ok(())
    }
}

impl GainCmd {
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let tracker = crate::tracking::Tracker::new(&config);
        let stats = tracker.get_stats().await?;
        println!("=== Token Savings Report ===");
        println!("Commands tracked:    {}", stats.total_commands);
        println!("Input tokens:        {}", stats.total_input_tokens);
        println!("Output tokens:       {}", stats.total_output_tokens);
        println!("Tokens saved:        {}", stats.total_saved_tokens);
        println!("Average savings:     {:.1}%", stats.avg_savings_pct);
        if stats.total_commands > 0 {
            let est_cost_saved = stats.total_saved_tokens as f64 * 0.000003; // ~$3/M input tokens
            println!("Est. cost saved:     ${:.4}", est_cost_saved);
        }
        Ok(())
    }
}

impl ProxyCmd {
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        if self.args.is_empty() {
            anyhow::bail!("Usage: syn proxy -- <command> [args...]");
        }
        let proxy = crate::proxy::Proxy::new(&config);
        let output = proxy.execute(&self.args).await?;
        println!("{}", output);
        Ok(())
    }
}

impl CompressCmd {
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let compressor = crate::compress::Compressor::new(&config);
        for path in &self.path {
            let p = std::path::Path::new(path);
            if self.restore {
                compressor.restore_file(p).await?;
            } else {
                compressor.compress_file(p).await?;
            }
        }
        Ok(())
    }
}

impl McpCmd {
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let server = crate::mcp::server::McpServer::new(config);
        if self.http {
            let bind = self.bind.as_deref().unwrap_or("127.0.0.1:3100");
            server.start_http(bind).await
        } else {
            server.start_stdio().await
        }
    }
}

impl McpProxyCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        anyhow::bail!("MCP proxy not yet implemented.")
    }
}

impl ConfigCmd {
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        if self.args.is_empty() {
            println!("{}", toml::to_string_pretty(&config)?);
        } else if self.args.len() == 1 && self.args[0] == "path" {
            println!("{}", Config::path()?.display());
        } else if self.args.len() == 1 && self.args[0] == "edit" {
            let path = Config::path()?;
            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| "vim".into());
            std::process::Command::new(editor).arg(&path).status()?;
        } else if self.args.len() == 3 && self.args[0] == "set" {
            let key = &self.args[1];
            let value = &self.args[2];
            println!("Set {} = {} (not yet persisted)", key, value);
        } else {
            anyhow::bail!("Usage: syn config [path|edit|set <key> <value>]")
        }
        Ok(())
    }
}

impl IndexCmd {
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let _ = self.watch;
        let _ = self.no_git;
        let indexer = crate::indexer::Indexer::new(&config);
        let root = std::env::current_dir()?;
        indexer.index_directory(&root).await?;

        let guard = indexer.storage.lock().unwrap();
        match guard.as_ref() {
            Some(s) => {
                println!("Index complete: {} code blocks total", s.count());
            }
            None => {
                println!("Indexing completed but no data stored");
            }
        }
        Ok(())
    }
}
