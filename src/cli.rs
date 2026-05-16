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
#[command(about = "Bootstrap a new Synapse project")]
pub struct InitCmd {
    #[arg(long)]
    pub interactive: bool,
}
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
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;

        // Check if already a Synapse project
        if root.join("synapsec.toml").exists() {
            println!("Already a Synapse project at {}", root.display());
            return Ok(());
        }

        let project_name = if self.interactive {
            println!();
            println!("=== Synapse Project Setup ===");
            println!();
            let name = ask(
                "Project name",
                root.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("my-project"),
            );
            println!();
            println!("What do you want to build? (describe in 1 sentence)");
            let description = ask_input("> ");
            println!();
            let language = ask_options(
                "Main language",
                &["Rust", "Python", "TypeScript", "JavaScript", "Go", "Other"],
            );
            println!();

            // Create initial requirements.xml from description
            let docs_dir = root.join("docs");
            std::fs::create_dir_all(&docs_dir)?;
            let req_content = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<REQUIREMENTS>
  <META>
    <PROJECT>{name}</PROJECT>
    <DESCRIPTION>{description}</DESCRIPTION>
    <LANGUAGE>{language}</LANGUAGE>
    <GENERATED_BY>syn init --interactive</GENERATED_BY>
  </META>

  <REQUIREMENT>
    <NAME>Core Module</NAME>
    <PURPOSE>{description}</PURPOSE>
    <DEPENDS></DEPENDS>
    <LINK>docs/technology.xml</LINK>
  </REQUIREMENT>
</REQUIREMENTS>
"#,
                name = name,
                description = description,
                language = language
            );
            if !docs_dir.join("requirements.xml").exists() {
                std::fs::write(docs_dir.join("requirements.xml"), &req_content)?;
            }
            name
        } else {
            root.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("my-project")
                .to_string()
        };

        // Create project config
        let config_path = Config::path()?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut project_config = config;
        project_config.project.name = project_name.clone();
        let config_toml = toml::to_string_pretty(&project_config)?;
        std::fs::write(&config_path, &config_toml)?;
        println!("Created configuration at {}", config_path.display());

        // Create .opencode directory structure
        let opencode_dir = root.join(".opencode");
        std::fs::create_dir_all(&opencode_dir)?;

        // Create OpenCode rules
        let rules_dir = opencode_dir.join("rules");
        std::fs::create_dir_all(&rules_dir)?;
        let rules_content = include_str!("../.opencode/rules/synapse.md");
        let rules_path = rules_dir.join("synapse.md");
        if !rules_path.exists() {
            std::fs::write(&rules_path, rules_content)?;
        }

        // Create OpenCode MCP config for auto-starting syn mcp
        let oc_config_path = opencode_dir.join("opencode.jsonc");
        let mcp_config = serde_json::json!({
            "mcpServers": {
                "synapse": {
                    "command": "syn",
                    "args": ["mcp"],
                    "env": {}
                }
            }
        });
        if !oc_config_path.exists() {
            std::fs::write(&oc_config_path, &serde_json::to_string_pretty(&mcp_config)?)?;
        }

        // Create OpenCode plugin for synapse
        let plugins_dir = opencode_dir.join("plugins");
        std::fs::create_dir_all(&plugins_dir)?;
        let plugin_path = plugins_dir.join("synapse.ts");
        if !plugin_path.exists() {
            let plugin_content = include_str!("../.opencode/plugins/synapse.ts");
            std::fs::write(&plugin_path, plugin_content)?;
        }

        // Create docs and GRACE templates
        let docs_dir = root.join("docs");
        std::fs::create_dir_all(&docs_dir)?;

        let templates = [
            (
                "docs/requirements.xml",
                include_str!("../templates/requirements.xml"),
            ),
            (
                "docs/technology.xml",
                include_str!("../templates/technology.xml"),
            ),
            (
                "docs/development-plan.xml",
                include_str!("../templates/development-plan.xml"),
            ),
            (
                "docs/verification-plan.xml",
                include_str!("../templates/verification-plan.xml"),
            ),
            (
                "docs/knowledge-graph.xml",
                include_str!("../templates/knowledge-graph.xml"),
            ),
        ];
        for (path, content) in &templates {
            let full = root.join(path);
            if !full.exists() {
                std::fs::write(&full, content)?;
            }
        }

        // Create .opencode/package.json for plugin dependencies
        let pkg_json_path = opencode_dir.join("package.json");
        if !pkg_json_path.exists() {
            std::fs::write(
                &pkg_json_path,
                r#"{"dependencies":{"@opencode-ai/plugin":"^1.15"}}"#,
            )?;
        }

        // Check for source files and auto-index
        let walker = crate::indexer::walker::Walker::new(&root);
        let files = walker.walk();
        if !files.is_empty() {
            let indexer = crate::indexer::Indexer::new(&project_config);
            println!("Found {} source files. Indexing...", files.len());
            indexer.index_directory(&root).await?;
            println!("Index complete.");
        } else {
            println!("No source files yet. Run `syn index` after adding code.");
        }

        println!();
        println!(
            "Synapse project '{}' ready at {}",
            project_config.project.name,
            root.display()
        );
        println!("   .opencode/rules/synapse.md   — AI command reference");
        println!("   .opencode/opencode.jsonc     — MCP auto-start config");
        println!("   .opencode/plugins/synapse.ts — Active plugin");
        println!("   docs/                         — GRACE templates");
        println!();
        if self.interactive {
            println!("Your requirements are in docs/requirements.xml");
            println!("Run: syn plan     (AI designs the architecture)");
            println!("Then: syn execute (creates source file skeletons)");
            println!("Then: opencode    (AI writes the code)");
        } else {
            println!("Run: opencode    (syn mcp starts automatically)");
        }
        Ok(())
    }
}

fn ask(prompt: &str, default: &str) -> String {
    use std::io::{self, Write};
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let input = input.trim();
    if input.is_empty() {
        default.to_string()
    } else {
        input.to_string()
    }
}

fn ask_input(prompt: &str) -> String {
    use std::io::{self, Write};
    print!("{}", prompt);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    input.trim().to_string()
}

fn ask_options(prompt: &str, options: &[&str]) -> String {
    println!("{}:", prompt);
    for (i, opt) in options.iter().enumerate() {
        println!("  {}. {}", i + 1, opt);
    }
    let default = options[0];
    let answer = ask("Choose", &format!("1 ({})", default));
    if let Ok(n) = answer.trim().parse::<usize>() {
        if n > 0 && n <= options.len() {
            return options[n - 1].to_string();
        }
    }
    default.to_string()
}

impl PlanCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let docs = root.join("docs");
        std::fs::create_dir_all(&docs)?;

        let req_path = docs.join("requirements.xml");
        let plan_path = docs.join("development-plan.xml");

        if !req_path.exists() {
            println!("{} not found. Creating template...", req_path.display());
            std::fs::write(&req_path, include_str!("../templates/requirements.xml"))?;
            println!(
                "Edit {} with your requirements, then run `syn plan` again.",
                req_path.display()
            );
            return Ok(());
        }

        let req_content = std::fs::read_to_string(&req_path)?;
        let modules = extract_requirements(&req_content);
        if modules.is_empty() {
            anyhow::bail!(
                "No <REQUIREMENT> entries found in {}. Add requirements first.",
                req_path.display()
            );
        }

        let mut plan = String::new();
        plan.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        plan.push_str("<DEVELOPMENT_PLAN>\n");
        plan.push_str("  <META>\n");
        plan.push_str("    <GENERATED_BY>syn plan</GENERATED_BY>\n");
        plan.push_str(&format!(
            "    <TIMESTAMP>{}</TIMESTAMP>\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ));
        plan.push_str("  </META>\n\n");
        plan.push_str("  <PHASES>\n");
        plan.push_str("    <PHASE id=\"1\" name=\"Foundation\">\n");
        plan.push_str(
            "      <DESCRIPTION>Core data structures and module contracts</DESCRIPTION>\n",
        );
        plan.push_str("      <MODULES>\n");

        for (i, m) in modules.iter().enumerate() {
            let module_id = format!("M-{}", m.name.to_uppercase().replace(' ', "_"));
            plan.push_str(&format!("        <M-{}>\n", i + 1));
            plan.push_str(&format!("          <ID>{}</ID>\n", module_id));
            plan.push_str(&format!("          <NAME>{}</NAME>\n", m.name));
            plan.push_str(&format!("          <PURPOSE>{}</PURPOSE>\n", m.purpose));
            plan.push_str("          <FILES>\n");
            let filename = m.name.to_lowercase().replace(' ', "_");
            plan.push_str(&format!("            <FILE>src/{}.rs</FILE>\n", filename));
            plan.push_str("          </FILES>\n");
            plan.push_str(&format!("        </M-{}>\n", i + 1));
        }

        plan.push_str("      </MODULES>\n");
        plan.push_str("    </PHASE>\n");
        plan.push_str("  </PHASES>\n\n");
        plan.push_str("  <DEPENDENCIES>\n");
        for m in &modules {
            for dep in &m.depends {
                plan.push_str(&format!("    <DEP from=\"{}\" to=\"{}\" />\n", m.name, dep));
            }
        }
        plan.push_str("  </DEPENDENCIES>\n");
        plan.push_str("</DEVELOPMENT_PLAN>\n");

        std::fs::write(&plan_path, &plan)?;
        println!("Development plan: {}", plan_path.display());
        println!();
        for (i, m) in modules.iter().enumerate() {
            println!("  {}. {} — {}", i + 1, m.name, m.purpose);
        }
        println!("\nNext: syn execute    (creates source file skeletons)");

        Ok(())
    }
}

struct ReqModule {
    name: String,
    purpose: String,
    depends: Vec<String>,
}

fn extract_requirements(xml: &str) -> Vec<ReqModule> {
    let mut modules = Vec::new();
    let re = regex::Regex::new(r"(?s)<REQUIREMENT>(.*?)</REQUIREMENT>").unwrap();
    let name_re = regex::Regex::new(r"<NAME>(.*?)</NAME>").unwrap();
    let purpose_re = regex::Regex::new(r"<PURPOSE>(.*?)</PURPOSE>").unwrap();
    let depends_re = regex::Regex::new(r"<DEPENDS>(.*?)</DEPENDS>").unwrap();
    for cap in re.captures_iter(xml) {
        let block = &cap[1];
        let name = name_re
            .captures(block)
            .map(|c| c[1].trim().to_string())
            .unwrap_or_else(|| "Unnamed".into());
        let purpose = purpose_re
            .captures(block)
            .map(|c| c[1].trim().to_string())
            .unwrap_or_else(|| "TBD".into());
        let deps: Vec<String> = depends_re
            .captures_iter(block)
            .flat_map(|c| {
                c[1].split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .collect();
        modules.push(ReqModule {
            name,
            purpose,
            depends: deps,
        });
    }
    modules
}

impl ExecuteCmd {
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let plan_path = root.join("docs").join("development-plan.xml");

        if !plan_path.exists() {
            anyhow::bail!("No development plan found. Run `syn plan` first.");
        }

        let plan = std::fs::read_to_string(&plan_path)?;
        let module_re = regex::Regex::new(r"(?s)<M-\d+>(.*?)</M-\d+>").unwrap();
        let _id_re = regex::Regex::new(r"<ID>(.*?)</ID>").unwrap();
        let name_re = regex::Regex::new(r"<NAME>(.*?)</NAME>").unwrap();
        let purpose_re = regex::Regex::new(r"<PURPOSE>(.*?)</PURPOSE>").unwrap();
        let file_re = regex::Regex::new(r"<FILE>(.*?)</FILE>").unwrap();

        let mut created = Vec::new();
        for cap in module_re.captures_iter(&plan) {
            let block = &cap[1];
            let module_name = name_re
                .captures(block)
                .map(|c| c[1].to_string())
                .unwrap_or_default();
            let purpose = purpose_re
                .captures(block)
                .map(|c| c[1].to_string())
                .unwrap_or_default();

            for fcap in file_re.captures_iter(block) {
                let file_path = root.join(&fcap[1]);
                if file_path.exists() {
                    println!("  {} — already exists, skipping", file_path.display());
                    continue;
                }
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let ext = file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("rs");
                let comment = match ext {
                    "py" => "#",
                    _ => "//",
                };
                let module_id = module_name.to_uppercase().replace(' ', "_");

                let mut content = String::new();
                content.push_str(&format!("{} MODULE_CONTRACT\n", comment));
                content.push_str(&format!("{} MODULE_ID: {}\n", comment, module_id));
                content.push_str(&format!("{} PURPOSE: {}\n", comment, purpose));
                content.push_str(&format!("{} SCOPE: {}\n", comment, module_name));
                content.push_str(&format!("{} DEPENDS:\n", comment));
                content.push_str(&format!("{} LINKS:\n", comment));
                content.push_str(&format!("{}\n", comment));
                content.push_str(&format!("{}\n", comment));

                match ext {
                    "rs" => {
                        content.push_str(&format!(
                            "pub mod {}_mod {{\n",
                            module_name.to_lowercase().replace(' ', "_")
                        ));
                        content.push_str("    // START_public_api\n");
                        content.push_str("    // END_public_api\n");
                        content.push_str("}\n");
                    }
                    "py" => {
                        content.push_str(&format!("# Module: {}\n", module_name));
                        content.push_str("# START_public_api\n");
                        content.push_str("# END_public_api\n");
                    }
                    "ts" | "tsx" | "js" | "jsx" => {
                        content.push_str(&format!("// Module: {}\n", module_name));
                        content.push_str("// START_public_api\n");
                        content.push_str("// END_public_api\n");
                    }
                    _ => {
                        content.push_str(&format!("// Module: {} ({})\n", module_name, ext));
                    }
                }

                std::fs::write(&file_path, &content)?;
                created.push(fcap[1].to_string());
            }
        }

        if created.is_empty() {
            println!("All files already exist. Nothing to create.");
        } else {
            println!("Created {} source files:", created.len());
            for f in &created {
                println!("  {}", f);
            }
            println!("\nEach file has a MODULE_CONTRACT header.");
            println!("Next: opencode   (AI fills in the implementation)");
        }

        Ok(())
    }
}

// Commands with args that have placeholder implementations
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
            let guard = indexer.storage.lock().unwrap();
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

                let guard = indexer.storage.lock().unwrap();
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

        let oc_mcp = root.join(".opencode/opencode.jsonc").exists()
            || root.join(".opencode/opencode.json").exists();
        check!(
            "opencode mcp",
            oc_mcp,
            "MCP config present",
            "missing — run `syn init`"
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
