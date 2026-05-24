// MODULE_CONTRACT
// MODULE_ID: M-CLI
// PURPOSE: CLI schema facade — clap-powered top-level parser, command enum, and command argument structs
// SCOPE: SynCli, Command enum, profile-aware command argument structs, run scenario/action flags, RTK route/economics flags, proxy evidence flag, filter lifecycle commands, CI action enum
// DEPENDS: M-CLI-SETUP-COMMANDS, M-CLI-CODE-COMMANDS, M-CLI-GRACE-COMMANDS, M-CLI-RUNTIME-COMMANDS
// LINKS: Cargo.toml

// START_MODULE_MAP
// SynCli — Top-level CLI parser struct
// Command — Enum of all supported CLI commands
// *Cmd structs — Clap argument schemas for supported commands
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v4.1.0 — Added proxy raw evidence flag]
// END_CHANGE_SUMMARY

mod code_commands;
mod grace_commands;
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
