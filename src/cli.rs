// MODULE_CONTRACT
// MODULE_ID: M-CLI
// PURPOSE: CLI schema facade — clap-powered top-level parser, command enum, and command argument structs
// SCOPE: SynCli, Command enum, profile-aware command argument structs, run scenario/action flags, RTK route/economics flags, expanded first-class RTK shortcut commands including container shortcuts, local RTK adapters, local RTK system adapters, discovery/learning diagnostics, rewrite hook decisions, parity inventory gate, proxy evidence flag, filter lifecycle commands, CI action enum
// DEPENDS: M-CLI-SETUP-COMMANDS, M-CLI-CODE-COMMANDS, M-CLI-GRACE-COMMANDS, M-CLI-RUNTIME-COMMANDS, M-CLI-RTK-COMMANDS
// LINKS: Cargo.toml

// START_MODULE_MAP
// SynCli — Top-level CLI parser struct
// Command — Enum of all supported CLI commands
// *Cmd structs — Clap argument schemas for supported commands
// RtkProxyCmd — Shared schema for first-class RTK-style proxy shortcuts
// JsonCmd/DepsCmd/EnvCmd/WcCmd — Local RTK-style token-saving adapters
// PipeCmd/LogCmd/SmartCmd — Local RTK-style system adapters
// DiscoverCmd/LearnCmd — Bounded RTK discovery and learning diagnostics
// RtkParityCmd — Machine-checkable RTK parity inventory report
// RewriteCmd — Hook-facing command rewrite dry run
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v5.0.0 — Added bounded RTK discover/learn diagnostics]
// END_CHANGE_SUMMARY

mod code_commands;
mod grace_commands;
mod rtk_adapters;
mod rtk_commands;
mod rtk_discovery;
mod rtk_parity;
mod rtk_system_adapters;
mod runtime_commands;
mod setup_commands;

use clap::{Parser, Subcommand};

// START_public_api

// START_SynCli
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
// END_SynCli

// START_Command
#[derive(Subcommand)]
pub enum Command {
    Init(InitCmd),
    Index(IndexCmd),
    Search(SearchCmd),
    View(ViewCmd),
    Verify(VerifyCmd),
    Review(ReviewCmd),
    Status(StatusCmd),
    #[command(about = "Read files through the token-saving proxy")]
    Read(RtkProxyCmd),
    #[command(about = "List directory contents through the token-saving proxy")]
    Ls(RtkProxyCmd),
    #[command(about = "Show directory tree through the token-saving proxy")]
    Tree(RtkProxyCmd),
    #[command(about = "Find files through the token-saving proxy")]
    Find(RtkProxyCmd),
    #[command(
        name = "rg",
        about = "Search with ripgrep through the token-saving proxy"
    )]
    Rg(RtkProxyCmd),
    #[command(about = "Search with grep through the token-saving proxy")]
    Grep(RtkProxyCmd),
    #[command(about = "Run git through the token-saving proxy")]
    Git(RtkProxyCmd),
    #[command(about = "Run cargo through the token-saving proxy")]
    Cargo(RtkProxyCmd),
    #[command(about = "Run npm through the token-saving proxy")]
    Npm(RtkProxyCmd),
    #[command(about = "Run pnpm through the token-saving proxy")]
    Pnpm(RtkProxyCmd),
    #[command(about = "Run npx through the token-saving proxy")]
    Npx(RtkProxyCmd),
    #[command(about = "Run pytest through the token-saving proxy")]
    Pytest(RtkProxyCmd),
    #[command(name = "ruff", about = "Run ruff through the token-saving proxy")]
    Ruff(RtkProxyCmd),
    #[command(name = "mypy", about = "Run mypy through the token-saving proxy")]
    Mypy(RtkProxyCmd),
    #[command(
        name = "basedpyright",
        about = "Run basedpyright through the token-saving proxy"
    )]
    Basedpyright(RtkProxyCmd),
    #[command(name = "pip", about = "Run pip through the token-saving proxy")]
    Pip(RtkProxyCmd),
    #[command(name = "uv", about = "Run uv through the token-saving proxy")]
    Uv(RtkProxyCmd),
    #[command(
        name = "next",
        about = "Run Next.js tooling through the token-saving proxy"
    )]
    Next(RtkProxyCmd),
    #[command(
        name = "playwright",
        about = "Run Playwright through the token-saving proxy"
    )]
    Playwright(RtkProxyCmd),
    #[command(
        name = "prettier",
        about = "Run Prettier through the token-saving proxy"
    )]
    Prettier(RtkProxyCmd),
    #[command(name = "prisma", about = "Run Prisma through the token-saving proxy")]
    Prisma(RtkProxyCmd),
    #[command(
        name = "tsc",
        about = "Run TypeScript compiler through the token-saving proxy"
    )]
    Tsc(RtkProxyCmd),
    #[command(name = "vitest", about = "Run Vitest through the token-saving proxy")]
    Vitest(RtkProxyCmd),
    #[command(name = "gh", about = "Run GitHub CLI through the token-saving proxy")]
    Gh(RtkProxyCmd),
    #[command(name = "glab", about = "Run GitLab CLI through the token-saving proxy")]
    Glab(RtkProxyCmd),
    #[command(name = "aws", about = "Run AWS CLI through the token-saving proxy")]
    Aws(RtkProxyCmd),
    #[command(name = "psql", about = "Run psql through the token-saving proxy")]
    Psql(RtkProxyCmd),
    #[command(name = "curl", about = "Run curl through the token-saving proxy")]
    Curl(RtkProxyCmd),
    #[command(name = "wget", about = "Run wget through the token-saving proxy")]
    Wget(RtkProxyCmd),
    #[command(name = "jq", about = "Run jq through the token-saving proxy")]
    Jq(RtkProxyCmd),
    #[command(name = "go", about = "Run Go tooling through the token-saving proxy")]
    Go(RtkProxyCmd),
    #[command(
        name = "golangci",
        about = "Run golangci-lint through the token-saving proxy"
    )]
    Golangci(RtkProxyCmd),
    #[command(name = "dotnet", about = "Run dotnet through the token-saving proxy")]
    Dotnet(RtkProxyCmd),
    #[command(name = "rake", about = "Run rake through the token-saving proxy")]
    Rake(RtkProxyCmd),
    #[command(name = "rspec", about = "Run rspec through the token-saving proxy")]
    Rspec(RtkProxyCmd),
    #[command(name = "rubocop", about = "Run rubocop through the token-saving proxy")]
    Rubocop(RtkProxyCmd),
    #[command(name = "gradle", about = "Run Gradle through the token-saving proxy")]
    Gradle(RtkProxyCmd),
    #[command(
        name = "gradlew",
        about = "Run local Gradle wrapper through the token-saving proxy"
    )]
    Gradlew(RtkProxyCmd),
    #[command(name = "make", about = "Run make through the token-saving proxy")]
    Make(RtkProxyCmd),
    #[command(name = "just", about = "Run just through the token-saving proxy")]
    Just(RtkProxyCmd),
    #[command(name = "helm", about = "Run Helm through the token-saving proxy")]
    Helm(RtkProxyCmd),
    #[command(name = "kubectl", about = "Run kubectl through the token-saving proxy")]
    Kubectl(RtkProxyCmd),
    #[command(name = "docker", about = "Run Docker through the token-saving proxy")]
    Docker(RtkProxyCmd),
    #[command(name = "podman", about = "Run Podman through the token-saving proxy")]
    Podman(RtkProxyCmd),
    #[command(about = "Inspect JSON with compact values or keys-only schema")]
    Json(JsonCmd),
    #[command(about = "Summarize dependency manifests without dumping full files")]
    Deps(DepsCmd),
    #[command(about = "Show filtered environment variables with secrets masked")]
    Env(EnvCmd),
    #[command(about = "Count text locally with compact wc-style output")]
    Wc(WcCmd),
    #[command(about = "Filter stdin through Synapse RTK filters")]
    Pipe(PipeCmd),
    #[command(about = "Deduplicate and summarize log output from a file or stdin")]
    Log(LogCmd),
    #[command(about = "Summarize source file structure without printing full code")]
    Smart(SmartCmd),
    #[command(about = "Discover routeable token-heavy commands and Synapse replacements")]
    Discover(DiscoverCmd),
    #[command(about = "Show bounded RTK learning guidance for recurring misses")]
    Learn(LearnCmd),
    #[command(
        name = "rtk-parity",
        about = "Report machine-checkable RTK parity inventory"
    )]
    RtkParity(RtkParityCmd),
    #[command(about = "Rewrite a shell command to its Synapse proxy form for agent hooks")]
    Rewrite(RewriteCmd),
    Run(RunCmd),
    Proxy(ProxyCmd),
    Filters(FiltersCmd),
    Gain(GainCmd),
    Compress(CompressCmd),
    Mcp(McpCmd),
    Config(ConfigCmd),
    #[command(name = "graphrag")]
    GraphRag(GraphRagCmd),
    Hooks(HooksCmd),
    Doctor(DoctorCmd),
    Refresh(RefreshCmd),
    Skills(SkillsCmd),
    #[command(name = "ci")]
    Ci(CiCmd),
    #[command(name = "history")]
    History(HistoryCmd),
    Serve(ServeCmd),
}
// END_Command

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

// START_InitCmd
#[derive(clap::Args)]
#[command(about = "Install Synapse hooks into current project (MCP, plugin, rules)")]
pub struct InitCmd {
    #[arg(long)]
    pub from_existing: bool,
}
// END_InitCmd

// START_IndexCmd
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
// END_IndexCmd

// START_VerifyCmd
#[derive(clap::Args)]
#[command(about = "Run verification suite")]
pub struct VerifyCmd {
    #[arg(long)]
    pub level: Option<String>,
    #[arg(long = "mod")]
    pub r#mod: Option<String>,
    #[arg(long, default_value = "strict")]
    pub profile: String,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub ci: bool,
}
// END_VerifyCmd

// START_ReviewCmd
#[derive(clap::Args)]
#[command(about = "GRACE integrity review")]
pub struct ReviewCmd {
    #[arg(long)]
    pub mode: Option<String>,
    #[arg(long = "mod")]
    pub r#mod: Option<String>,
    #[arg(long, default_value = "strict")]
    pub profile: String,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub ci: bool,
}
// END_ReviewCmd

// START_StatusCmd
#[derive(clap::Args)]
#[command(about = "Project health report")]
pub struct StatusCmd {
    #[arg(long)]
    pub json: bool,
    #[arg(long = "mod")]
    pub r#mod: Option<String>,
    #[arg(long)]
    pub ci: bool,
}
// END_StatusCmd

// START_RtkProxyCmd
#[derive(clap::Args)]
pub struct RtkProxyCmd {
    #[arg(long)]
    pub route: bool,
    #[arg(long)]
    pub evidence: bool,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}
// END_RtkProxyCmd

// START_RewriteCmd
#[derive(clap::Args)]
pub struct RewriteCmd {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}
// END_RewriteCmd

// START_JsonCmd
#[derive(clap::Args)]
#[command(about = "Inspect JSON with compact values or keys-only schema")]
pub struct JsonCmd {
    pub file: String,
    #[arg(short = 'd', long, default_value_t = 5)]
    pub depth: usize,
    #[arg(long)]
    pub keys_only: bool,
}
// END_JsonCmd

// START_DepsCmd
#[derive(clap::Args)]
#[command(about = "Summarize dependency manifests")]
pub struct DepsCmd {
    #[arg(default_value = ".")]
    pub path: String,
}
// END_DepsCmd

// START_EnvCmd
#[derive(clap::Args)]
#[command(about = "Show filtered environment variables with secrets masked")]
pub struct EnvCmd {
    #[arg(short, long)]
    pub filter: Option<String>,
    #[arg(long)]
    pub show_all: bool,
}
// END_EnvCmd

// START_WcCmd
#[derive(clap::Args)]
#[command(about = "Count text locally with compact wc-style output")]
pub struct WcCmd {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}
// END_WcCmd

// START_PipeCmd
#[derive(clap::Args)]
#[command(about = "Filter stdin through Synapse RTK filters")]
pub struct PipeCmd {
    #[arg(short = 'f', long)]
    pub filter: Option<String>,
    #[arg(long)]
    pub passthrough: bool,
}
// END_PipeCmd

// START_LogCmd
#[derive(clap::Args)]
#[command(about = "Deduplicate and summarize log output from a file or stdin")]
pub struct LogCmd {
    #[arg(default_value = "-")]
    pub source: String,
}
// END_LogCmd

// START_SmartCmd
#[derive(clap::Args)]
#[command(about = "Summarize source file structure without printing full code")]
pub struct SmartCmd {
    pub file: String,
    #[arg(short = 'n', long, default_value_t = 12)]
    pub items: usize,
}
// END_SmartCmd

// START_DiscoverCmd
#[derive(clap::Args)]
#[command(about = "Discover routeable token-heavy commands and Synapse replacements")]
pub struct DiscoverCmd {
    #[arg(long)]
    pub json: bool,
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}
// END_DiscoverCmd

// START_LearnCmd
#[derive(clap::Args)]
#[command(about = "Show bounded RTK learning guidance for recurring misses")]
pub struct LearnCmd {
    #[arg(long)]
    pub json: bool,
}
// END_LearnCmd

// START_RtkParityCmd
#[derive(clap::Args)]
#[command(about = "Report RTK parity inventory against an optional rtk-develop source tree")]
pub struct RtkParityCmd {
    #[arg(long)]
    pub source: Option<String>,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub ci: bool,
}
// END_RtkParityCmd

// START_RunCmd
#[derive(clap::Args)]
#[command(about = "Run bounded autonomous workflow scenario or action queue")]
pub struct RunCmd {
    #[arg(long)]
    pub action: Option<String>,
    #[arg(long = "run-id")]
    pub run_id: Option<String>,
    #[arg(long, default_value_t = 8)]
    pub max_actions: usize,
    #[arg(long, default_value = "happy")]
    pub scenario: String,
    #[arg(long, default_value = "Release-grade autonomous runtime")]
    pub goal: String,
    #[arg(long, default_value = "Phase-22")]
    pub phase: String,
    #[arg(long = "mod", default_value = "M-RUNNER")]
    pub module_id: String,
    #[arg(long, default_value = "bounded objective to replay")]
    pub objective: String,
    #[arg(long)]
    pub json: bool,
}
// END_RunCmd

cmd_struct!(ExplainCmd, "Explain code using indexed context",
    query: Vec<String>,
);

// START_GraphRagCmd
#[derive(clap::Args)]
#[command(name = "graphrag", about = "Query the code knowledge graph")]
pub struct GraphRagCmd {
    pub operation: Option<String>,
    pub args: Vec<String>,
}
// END_GraphRagCmd

// START_GainCmd
#[derive(clap::Args)]
#[command(about = "View token savings analytics")]
pub struct GainCmd {
    #[arg(long)]
    pub graph: bool,
    #[arg(long)]
    pub sessions: bool,
    #[arg(long)]
    pub adapters: bool,
}
// END_GainCmd

// START_FiltersCmd
#[derive(clap::Args)]
#[command(about = "Verify and trust token-saving proxy filters")]
pub struct FiltersCmd {
    #[command(subcommand)]
    pub action: FiltersAction,
}

#[derive(Subcommand)]
pub enum FiltersAction {
    Verify(FiltersVerifyCmd),
    Trust,
    Untrust,
    Status,
}

#[derive(clap::Args)]
pub struct FiltersVerifyCmd {
    #[arg(long)]
    pub filter: Option<String>,
    #[arg(long)]
    pub require_all: bool,
}
// END_FiltersCmd

// START_HooksCmd
#[derive(clap::Args)]
#[command(about = "Manage Synapse hooks for AI agents")]
pub struct HooksCmd {
    pub action: String,
    #[arg(default_value = "opencode")]
    pub agent: String,
}
// END_HooksCmd

// START_SearchCmd
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
// END_SearchCmd

// START_ViewCmd
#[derive(clap::Args)]
#[command(about = "View file signatures")]
pub struct ViewCmd {
    pub path: Vec<String>,
    #[arg(long)]
    pub json: bool,
}
// END_ViewCmd

// START_GrepCmd
#[derive(clap::Args)]
#[command(about = "AST structural search")]
pub struct GrepCmd {
    pub pattern: String,
    #[arg(long)]
    pub rewrite: Option<String>,
    #[arg(long)]
    pub language: Option<String>,
}
// END_GrepCmd

// START_ProxyCmd
#[derive(clap::Args)]
#[command(about = "Run command through token-saving proxy")]
pub struct ProxyCmd {
    #[arg(long)]
    pub route: bool,
    #[arg(long)]
    pub evidence: bool,
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}
// END_ProxyCmd

// START_CompressCmd
#[derive(clap::Args)]
#[command(about = "Compress files for AI context")]
pub struct CompressCmd {
    pub path: Vec<String>,
    #[arg(long, default_value = "full")]
    pub level: String,
    #[arg(long)]
    pub restore: bool,
}
// END_CompressCmd

// START_McpCmd
#[derive(clap::Args)]
#[command(about = "Start MCP server")]
pub struct McpCmd {}
// END_McpCmd

// START_ConfigCmd
#[derive(clap::Args)]
#[command(about = "Manage configuration")]
pub struct ConfigCmd {
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}
// END_ConfigCmd

// START_DoctorCmd
#[derive(clap::Args)]
#[command(about = "Run diagnostic checks on the Synapse setup")]
pub struct DoctorCmd {
    #[arg(long)]
    pub deps: bool,
}
// END_DoctorCmd

// START_RefreshCmd
#[derive(clap::Args)]
#[command(about = "Sync knowledge graph and verification plan with code")]
pub struct RefreshCmd {
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub fix: bool,
}
// END_RefreshCmd

// START_SkillsCmd
#[derive(clap::Args)]
#[command(about = "Run or inspect GRACE workflow skills")]
pub struct SkillsCmd {
    pub action: String,
    pub name: Option<String>,
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}
// END_SkillsCmd

// START_CiAction
#[derive(Subcommand)]
pub enum CiAction {
    Verify(VerifyCmd),
    Review(ReviewCmd),
    Status(StatusCmd),
}
// END_CiAction

// START_CiCmd
#[derive(clap::Args)]
#[command(about = "CI-friendly grouped commands")]
pub struct CiCmd {
    #[command(subcommand)]
    pub action: CiAction,
}
// END_CiCmd

// START_HistoryCmd
#[derive(clap::Args)]
#[command(about = "Search git history for code changes")]
pub struct HistoryCmd {
    pub query: Vec<String>,
    #[arg(long, default_value_t = 20)]
    pub max_results: u32,
}
// END_HistoryCmd

// START_ServeCmd
#[derive(clap::Args)]
#[command(about = "Start web dashboard")]
pub struct ServeCmd {
    #[arg(long, default_value = "127.0.0.1:3100")]
    pub bind: String,
}
// END_ServeCmd

// START_CONTRACT_public_api
// PURPOSE: Export the top-level CLI parser, command enum, and clap argument structs
// OUTPUTS: { SynCli }, { Command }, { command argument structs }
// END_public_api
