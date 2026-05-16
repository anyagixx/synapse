// MODULE_CONTRACT
// MODULE_ID: M-CLI
// PURPOSE: CLI command definitions and handlers — clap-powered CLI structs and run() impls for all commands
// SCOPE: SynCli, Command enum, all Cmd structs (Init, Index, Search, View, etc.), cmd_run! macro, all command handlers
// DEPENDS: M-CONFIG, M-INDEXER, M-GRACE, M-PROXY, M-COMPRESS, M-TRACKING, M-GRAPHRAG, M-HOOKS, M-DASHBOARD, M-MCP
// LINKS: Cargo.toml

// START_MODULE_MAP
// SynCli — Top-level CLI struct (clap Parser)
// Command — CLI command enum with all subcommands
// InitCmd, IndexCmd, SearchCmd, ViewCmd, GrepCmd, VerifyCmd, ReviewCmd, FixCmd, ExplainCmd, StatusCmd — Command structs
// GraphRagCmd, GainCmd, ProxyCmd, CompressCmd, McpCmd, ConfigCmd, DoctorCmd, RefreshCmd, HistoryCmd, ServeCmd — Command structs
// HooksCmd, PlanCmd, ExecuteCmd, LogsCmd, TelemetryCmd, McpProxyCmd — Command structs
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "syn",
    version,
    about = "Synapse — Unified AI Agent Engineering Platform"
)]
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
    Verify(VerifyCmd),
    Review(ReviewCmd),
    Status(StatusCmd),
    Proxy(ProxyCmd),
    Gain(GainCmd),
    Compress(CompressCmd),
    Mcp(McpCmd),
    Config(ConfigCmd),
    #[command(name = "graphrag")]
    GraphRag(GraphRagCmd),
    Hooks(HooksCmd),
    Doctor(DoctorCmd),
    Refresh(RefreshCmd),
    #[command(name = "history")]
    History(HistoryCmd),
    Serve(ServeCmd),
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

#[derive(clap::Args)]
#[command(about = "Install Synapse hooks into current project (MCP, plugin, rules)")]
pub struct InitCmd;
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
#[command(about = "Manage Synapse hooks for AI agents")]
pub struct HooksCmd {
    pub action: String,
    #[arg(default_value = "opencode")]
    pub agent: String,
}

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

#[derive(clap::Args)]
#[command(about = "Run diagnostic checks on the Synapse setup")]
pub struct DoctorCmd;

#[derive(clap::Args)]
#[command(about = "Sync knowledge graph and verification plan with code")]
pub struct RefreshCmd {
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Args)]
#[command(about = "Search git history for code changes")]
pub struct HistoryCmd {
    pub query: Vec<String>,
    #[arg(long, default_value_t = 20)]
    pub max_results: u32,
}

#[derive(clap::Args)]
#[command(about = "Start web dashboard")]
pub struct ServeCmd {
    #[arg(long, default_value = "127.0.0.1:3100")]
    pub bind: String,
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

cmd_run!(LogsCmd, TelemetryCmd);

impl InitCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;

        // AGENTS.md — GRACE constitution at project root (OpenCode reads this automatically)
        let agents_path = root.join("AGENTS.md");
        if !agents_path.exists() {
            std::fs::write(&agents_path, include_str!("../.opencode/rules/synapse.md"))?;
        }

        let opencode_dir = root.join(".opencode");
        std::fs::create_dir_all(&opencode_dir)?;

        // MCP auto-start config — in PROJECT ROOT
        let oc_config_path = root.join("opencode.jsonc");
        let mcp_config = serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "mcp": {
                "synapse": {
                    "type": "local",
                    "command": ["syn", "mcp"],
                    "enabled": true
                }
            }
        });
        if !oc_config_path.exists() {
            std::fs::write(&oc_config_path, &serde_json::to_string_pretty(&mcp_config)?)?;
        }

        // Plugin
        let plugins_dir = opencode_dir.join("plugins");
        std::fs::create_dir_all(&plugins_dir)?;
        let plugin_path = plugins_dir.join("synapse.ts");
        if !plugin_path.exists() {
            std::fs::write(
                &plugin_path,
                include_str!("../.opencode/plugins/synapse.ts"),
            )?;
        }

        // Rules
        let rules_dir = opencode_dir.join("rules");
        std::fs::create_dir_all(&rules_dir)?;
        let rules_path = rules_dir.join("synapse.md");
        if !rules_path.exists() {
            std::fs::write(&rules_path, include_str!("../.opencode/rules/synapse.md"))?;
        }

        // Package.json
        let pkg_path = opencode_dir.join("package.json");
        if !pkg_path.exists() {
            std::fs::write(
                &pkg_path,
                r#"{"dependencies":{"@opencode-ai/plugin":"^1.15"}}"#,
            )?;
        }

        // Phase 0: GRACE architecture templates in docs/
        let docs_dir = root.join("docs");
        std::fs::create_dir_all(&docs_dir)?;
        let phase0_templates: &[(&str, &str)] = &[
            (
                "docs/requirements.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<REQUIREMENTS>
  <META>
    <PROJECT>my-project</PROJECT>
    <DESCRIPTION>Describe what this project does in one sentence</DESCRIPTION>
    <LANGUAGE>rust</LANGUAGE>
  </META>
  <REQUIREMENT>
    <NAME>Core Feature</NAME>
    <PURPOSE>Describe the core purpose</PURPOSE>
    <DEPENDS></DEPENDS>
  </REQUIREMENT>
</REQUIREMENTS>
"#,
            ),
            (
                "docs/technology.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<TECHNOLOGY>
  <STACK>
    <LANGUAGE>rust</LANGUAGE>
    <FRAMEWORK></FRAMEWORK>
    <DATABASE></DATABASE>
  </STACK>
  <TOOLS>
    <TOOL purpose="build">cargo</TOOL>
    <TOOL purpose="testing">cargo test</TOOL>
    <TOOL purpose="lint">cargo clippy</TOOL>
  </TOOLS>
</TECHNOLOGY>
"#,
            ),
            (
                "docs/development-plan.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<DEVELOPMENT_PLAN>
  <META>
    <GENERATED_BY>syn init</GENERATED_BY>
  </META>
  <PHASES>
    <PHASE id="1" name="Foundation">
      <DESCRIPTION>Core data structures and module contracts</DESCRIPTION>
      <MODULES>
        <M-1>
          <ID>M-CORE</ID>
          <NAME>Core Module</NAME>
          <PURPOSE>Core application logic</PURPOSE>
          <FILES><FILE>src/core.rs</FILE></FILES>
        </M-1>
      </MODULES>
    </PHASE>
  </PHASES>
  <DEPENDENCIES></DEPENDENCIES>
</DEVELOPMENT_PLAN>
"#,
            ),
            (
                "docs/verification-plan.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<VERIFICATION_PLAN>
  <GLOBAL_POLICY>
    <DETERMINISTIC_FIRST>true</DETERMINISTIC_FIRST>
  </GLOBAL_POLICY>
  <MODULE_VERIFICATION id="V-M-CORE">
    <MODULE_ID>M-CORE</MODULE_ID>
    <UNIT_TESTS></UNIT_TESTS>
    <LOG_MARKERS></LOG_MARKERS>
  </MODULE_VERIFICATION>
  <PHASE_GATES>
    <PHASE id="1">
      <GATE>module-local verify on all M-1 modules</GATE>
    </PHASE>
  </PHASE_GATES>
</VERIFICATION_PLAN>
"#,
            ),
            (
                "docs/knowledge-graph.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<KNOWLEDGE_GRAPH>
  <NODES>
    <NODE id="M-CORE">
      <NAME>Core Module</NAME>
      <KIND>module</KIND>
      <PATH>src/core.rs</PATH>
    </NODE>
  </NODES>
  <RELATIONSHIPS></RELATIONSHIPS>
</KNOWLEDGE_GRAPH>
"#,
            ),
        ];
        for (path, content) in phase0_templates {
            let full = root.join(path);
            if !full.exists() {
                std::fs::write(&full, content)?;
            }
        }

        // Index existing sources
        let walker = crate::indexer::walker::Walker::new(&root);
        let files = walker.walk();
        if !files.is_empty() {
            let indexer = crate::indexer::Indexer::new(&_config);
            indexer.index_directory(&root).await?;
        }

        println!("Synapse hooks installed at {}", root.display());
        println!();
        println!("What was created:");
        println!("  AGENTS.md                    — GRACE constitution (read by every LLM session)");
        println!("  opencode.jsonc              — MCP auto-start (12 tools for LLM)");
        println!("  docs/                        — Phase 0 architecture templates (5 XML files)");
        println!("  .opencode/plugins/synapse.ts — auto-proxy + GRACE system context");
        println!("  .opencode/rules/synapse.md   — tool reference for LLM");
        println!();
        println!("Done. Now run: opencode");
        println!("The LLM will ask what you want to build and create everything.");
        Ok(())
    }
}
impl SearchCmd {
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let query = self.query.join(" ");
        if query.is_empty() {
            anyhow::bail!("Usage: syn search <query> [--mode code|file] [--language rust|py|...] [--max-results N]");
        }
        let indexer = crate::indexer::Indexer::new(&config);
        let results = indexer.search(&query, self.max_results as usize).await?;
        if results.is_empty() {
            println!(
                "No results for '{}'. Run `syn index` first if project is not indexed.",
                query
            );
            return Ok(());
        }
        for (i, r) in results.iter().enumerate() {
            let preview = if r.content.len() > 120 {
                format!("{}...", &r.content[..120].replace('\n', " "))
            } else {
                r.content.replace('\n', " ")
            };
            println!(
                "{}. {}:{} ({} {}) score={:.1}\n   {}",
                i + 1,
                r.path,
                r.start_line,
                r.language,
                r.kind,
                r.score,
                preview
            );
        }
        Ok(())
    }
}

impl ViewCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let indexer = crate::indexer::Indexer::new(&_config);
        for path in &self.path {
            if self.path.len() > 1 {
                println!("=== {} ===", path);
            }
            match indexer.view_signatures(path).await {
                Ok(sigs) => {
                    if self.json {
                        println!("{}", serde_json::to_string_pretty(&sigs)?);
                    } else {
                        for s in &sigs {
                            println!("  {}", s);
                        }
                        if sigs.is_empty() {
                            println!("  (no signatures found)");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading {}: {}", path, e);
                }
            }
        }
        Ok(())
    }
}

impl GrepCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        use crate::indexer::parser::ParserEngine;
        let root = std::env::current_dir()?;
        let walker = crate::indexer::walker::Walker::new(&root);
        let files = walker.walk();
        let parser = ParserEngine::new();
        let pattern_lower = self.pattern.to_lowercase();
        let mut found = 0usize;

        // Validate --rewrite requires capture groups
        if self.rewrite.is_some() && !self.pattern.contains('(') {
            anyhow::bail!("--rewrite requires a pattern with capture groups, e.g. 'fn (\\w+)'");
        }

        for file in &files {
            if let Some(ref lang_filter) = self.language {
                if file.language != *lang_filter {
                    continue;
                }
            }
            let full_path = root.join(&file.path);
            let code = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if code.len() > 100_000 {
                continue;
            }

            let blocks = parser.parse(&code, &file.language);
            for block in &blocks {
                let kind_match = block.kind.to_lowercase().contains(&pattern_lower);
                let name_match = block.name.to_lowercase().contains(&pattern_lower);
                let content_match = block.content.to_lowercase().contains(&pattern_lower);

                if kind_match || name_match || content_match {
                    found += 1;
                    let preview = if block.content.len() > 200 {
                        format!("{}...", &block.content[..200].replace('\n', " "))
                    } else {
                        block.content.replace('\n', " ")
                    };
                    println!(
                        "{}:{} — {} ({})",
                        file.path, block.start_line, block.name, block.kind
                    );
                    if !content_match {
                        println!("  {}", preview);
                    }

                    if let Some(ref rewrite_pattern) = self.rewrite {
                        if let Ok(re) = regex::Regex::new(rewrite_pattern) {
                            let rewritten =
                                re.replace_all(&block.content, |caps: &regex::Captures| {
                                    caps.iter()
                                        .enumerate()
                                        .filter_map(|(i, m)| {
                                            if i == 0 {
                                                None
                                            } else {
                                                m.map(|m| m.as_str().to_string())
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                });
                            println!("  => {}", rewritten);
                        }
                    }
                }
            }
        }

        if found == 0 {
            println!("No AST nodes matching '{}'", self.pattern);
        } else {
            eprintln!("\n{} matches in {} files", found, files.len());
        }
        Ok(())
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
                println!(
                    "  {} {} — {}",
                    if c.passed { "✓" } else { "✗" },
                    c.name,
                    c.details
                );
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
                    println!(
                        "Nodes: {}, Relationships: {}",
                        ov.total_nodes, ov.total_relationships
                    );
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
        let indexer = crate::indexer::Indexer::new(&config);
        let root = std::env::current_dir()?;

        if self.force {
            println!("Force re-indexing...");
        }
        if self.no_git {
            println!("Indexing without gitignore rules");
        }

        indexer.index_directory(&root).await?;

        let total = {
            let guard = indexer.storage.read().unwrap();
            guard.as_ref().map(|s| s.count()).unwrap_or(0)
        };
        println!("Index complete: {} code blocks", total);

        if self.watch {
            println!("Watching for changes... (Ctrl+C to stop)");
            watch_and_reindex(root).await?;
        }

        Ok(())
    }
}

async fn watch_and_reindex(root: std::path::PathBuf) -> anyhow::Result<()> {
    use notify::{Event, EventKind, RecursiveMode, Watcher};
    use std::time::Duration;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<notify::Result<Event>>(32);

    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.blocking_send(res);
    })?;

    watcher.watch(&root, RecursiveMode::Recursive)?;

    let mut last_index = tokio::time::Instant::now();

    while let Some(event) = rx.recv().await {
        match event {
            Ok(e) => {
                let is_modify = matches!(
                    e.kind,
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                );
                if !is_modify {
                    continue;
                }

                let source_files: Vec<_> = e
                    .paths
                    .iter()
                    .filter(|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| {
                                matches!(
                                    ext,
                                    "rs" | "py"
                                        | "ts"
                                        | "tsx"
                                        | "js"
                                        | "jsx"
                                        | "go"
                                        | "rb"
                                        | "php"
                                        | "java"
                                        | "cpp"
                                        | "h"
                                        | "hpp"
                                        | "css"
                                        | "scss"
                                        | "lua"
                                        | "sh"
                                )
                            })
                            .unwrap_or(false)
                    })
                    .collect();

                if source_files.is_empty() {
                    continue;
                }

                // Debounce: don't re-index more than once per 2 seconds
                if last_index.elapsed() < Duration::from_secs(2) {
                    continue;
                }
                last_index = tokio::time::Instant::now();

                let config = crate::config::Config::load()?;
                let indexer = crate::indexer::Indexer::new(&config);
                indexer.index_directory(&root).await?;

                let guard = indexer.storage.read().unwrap();
                let total = guard.as_ref().map(|s| s.count()).unwrap_or(0);
                drop(guard);

                let changed: Vec<String> = source_files
                    .iter()
                    .filter_map(|p| p.strip_prefix(&root).ok())
                    .map(|p| p.display().to_string())
                    .collect();
                eprintln!(
                    "Re-indexed ({} blocks total) — files changed: {}",
                    total,
                    changed.join(", ")
                );
            }
            Err(e) => {
                eprintln!("Watch error: {}", e);
            }
        }
    }

    Ok(())
}

impl HooksCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let manager = crate::hooks::HookManager::new(&_config);
        match self.action.as_str() {
            "install" => manager.install(&self.agent),
            "uninstall" => manager.uninstall(&self.agent),
            "status" => manager.status(),
            _ => anyhow::bail!("Usage: syn hooks install|uninstall|status [opencode|all]"),
        }
    }
}

impl DoctorCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        use colored::Colorize;
        let root = std::env::current_dir()?;
        println!("Synapse Doctor — checking your setup...\n");

        let mut issues = 0u32;

        macro_rules! check {
            ($label:expr, $cond:expr, $ok:expr, $fail:expr) => {
                if $cond {
                    println!(
                        "  {} {}",
                        "PASS".green().bold(),
                        format!("{} — {}", $label, $ok).green()
                    );
                } else {
                    println!(
                        "  {} {}",
                        "FAIL".red().bold(),
                        format!("{} — {}", $label, $fail).red()
                    );
                    issues += 1;
                }
            };
        }

        // Check config
        match Config::load() {
            Ok(c) => {
                check!(
                    "config",
                    true,
                    &format!(
                        "loaded ({})",
                        Config::path()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()
                    ),
                    ""
                );
                let _ = c;
            }
            Err(e) => {
                check!("config", false, "", &format!("cannot load: {}", e));
            }
        }

        // Check index
        let storage = crate::indexer::storage::Storage::new(&root);
        let indexed = !storage.is_empty();
        check!(
            "index",
            indexed,
            &format!("{} blocks indexed", storage.count()),
            "not indexed — run `syn index`"
        );

        // Check OpenCode integration
        let oc_agents = root.join("AGENTS.md").exists();
        check!(
            "agents.md",
            oc_agents,
            "AGENTS.md (GRACE constitution)",
            "missing — run `syn init`"
        );

        let oc_rules = root.join(".opencode/rules/synapse.md").exists();
        check!(
            "opencode rules",
            oc_rules,
            ".opencode/rules/synapse.md",
            "missing — run `syn init`"
        );

        let oc_plugin = root.join(".opencode/plugins/synapse.ts").exists();
        check!(
            "opencode plugin",
            oc_plugin,
            ".opencode/plugins/synapse.ts",
            "missing — run `syn init`"
        );

        let oc_mcp = root.join("opencode.jsonc").exists() || root.join("opencode.json").exists();
        check!(
            "opencode mcp",
            oc_mcp,
            "MCP config present",
            "missing — run `syn init`"
        );

        // Check Phase 0 docs
        let phase0_done = root.join("docs/requirements.xml").exists()
            && root.join("docs/technology.xml").exists()
            && root.join("docs/development-plan.xml").exists()
            && root.join("docs/verification-plan.xml").exists()
            && root.join("docs/knowledge-graph.xml").exists();
        check!(
            "phase 0 docs",
            phase0_done,
            "All 5 GRACE docs present",
            "missing — run `syn init` to create templates"
        );

        // Check tree-sitter grammars
        let parser = crate::indexer::parser::ParserEngine::new();
        let test_code = "fn test() {}";
        let blocks = parser.parse(test_code, "rust");
        check!(
            "tree-sitter",
            !blocks.is_empty(),
            &format!("Rust parser works ({} blocks)", blocks.len()),
            "parser failed — tree-sitter may be broken"
        );

        // Check SQLite
        let db = crate::tracking::Tracker::db_path().ok();
        check!(
            "sqlite tracking",
            db.is_some(),
            "tracking DB available",
            "cannot find tracking DB path"
        );

        // Check project structure
        let has_sources = !crate::indexer::walker::Walker::new(&root).walk().is_empty();
        check!(
            "sources",
            has_sources,
            "source files found",
            "no source files in project"
        );

        println!();
        if issues == 0 {
            println!("{}", "Synapse is ready! All checks passed.".green().bold());
            println!();
            println!("Next steps:");
            println!("  syn index              Index the codebase");
            println!("  syn status             View project health");
            println!("  syn search <query>     Search indexed code");
            println!("  opencode               Start AI development");
        } else {
            println!(
                "{} {} issue(s) found. Run `syn init` to fix setup.",
                "WARN".yellow().bold(),
                issues
            );
            if !indexed {
                println!("  • Run: syn index");
            }
            if !oc_rules || !oc_plugin || !oc_mcp {
                println!("  • Run: syn init");
            }
        }

        Ok(())
    }
}

impl RefreshCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let report = crate::grace::refresh::Refresher::refresh(&root)?;

        if self.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
            return Ok(());
        }

        println!("=== GRACE Refresh Report ===");
        println!();
        println!("Code modules with contracts: {}", report.total_modules);
        println!("  In knowledge-graph:      {}", report.in_graph);
        println!("  In verification-plan:    {}", report.in_verification);
        println!();

        if !report.not_in_graph.is_empty() {
            println!("Modules NOT in knowledge-graph.xml:");
            for m in &report.not_in_graph {
                println!("  ✗ {}", m);
            }
            println!();
        }

        if !report.in_graph_not_in_code.is_empty() {
            println!("Stale entries in knowledge-graph.xml (not in code):");
            for m in &report.in_graph_not_in_code {
                println!("  ✗ {}", m);
            }
            println!();
        }

        if !report.not_in_verification.is_empty() {
            println!("Modules NOT in verification-plan.xml:");
            for m in &report.not_in_verification {
                println!("  ✗ {}", m);
            }
            println!();
        }

        if !report.contract_issues.is_empty() {
            println!("Contract issues:");
            for i in &report.contract_issues {
                println!("  ✗ {}", i);
            }
            println!();
        }

        if !report.suggested_actions.is_empty() {
            println!("Suggested actions:");
            for a in &report.suggested_actions {
                println!("  → {}", a);
            }
            println!();
        }

        if report.not_in_graph.is_empty()
            && report.not_in_verification.is_empty()
            && report.contract_issues.is_empty()
        {
            println!("All modules synced. No drift detected.");
        }

        Ok(())
    }
}

impl ServeCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        crate::dashboard::start_dashboard(&self.bind).await
    }
}

impl HistoryCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let query = self.query.join(" ");
        if query.is_empty() {
            anyhow::bail!("Usage: syn history <query> [--max-results N]");
        }
        let query_lower = query.to_lowercase();

        let output = std::process::Command::new("git")
            .args(["log", "--oneline", "--all", "-n", "100", "--no-merges"])
            .current_dir(&root)
            .output();
        let mut results = Vec::new();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.to_lowercase().contains(&query_lower) {
                    results.push(line.to_string());
                }
            }
        }

        if results.is_empty() {
            println!("No git history matching '{}'", query);
            return Ok(());
        }

        results.truncate(self.max_results as usize);
        println!("=== Git History: '{}' ===", query);
        for r in &results {
            println!("  {}", r);
        }
        Ok(())
    }
}
