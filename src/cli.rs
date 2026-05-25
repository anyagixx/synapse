// MODULE_CONTRACT
// MODULE_ID: M-CLI
// PURPOSE: CLI schema facade — clap-powered top-level parser, command enum, command dispatch, and command argument structs
// SCOPE: SynCli, Command enum, CLI-owned command dispatch, profile-aware command argument structs, run scenario/action flags, RTK route/economics flags, expanded first-class RTK shortcut commands including ecosystem, Graphite, and container shortcuts, local RTK adapters, local RTK system adapters, .NET artifact adapters, core RTK adapters, session/economics analytics, discovery/learning diagnostics, hooks audit JSON flag, hook processor schemas, rewrite hook decisions, parity inventory and full parity flags, proxy evidence flag, filter lifecycle commands, CI action enum
// DEPENDS: M-CLI-SETUP-COMMANDS, M-CLI-CODE-COMMANDS, M-CLI-GRACE-COMMANDS, M-CLI-RUNTIME-COMMANDS, M-CLI-RTK-COMMANDS, M-RTK-FULL-PARITY
// LINKS: Cargo.toml

// START_MODULE_MAP
// SynCli — Top-level CLI parser struct
// Command — Enum of all supported CLI commands
// RunCommand — Dispatch trait implemented by Command next to the CLI schema
// *Cmd structs — Clap argument schemas for supported commands
// RtkProxyCmd — Shared schema for first-class RTK-style proxy shortcuts
// JsonCmd/DepsCmd/EnvCmd/WcCmd — Local RTK-style token-saving adapters
// PipeCmd/LogCmd/SmartCmd — Local RTK-style system adapters
// ConfigAction — Explicit config get, set, list, unset, path, and edit operations
// BinlogCmd/DotnetFormatReportCmd/DotnetTrxCmd — Local .NET artifact summarizers
// ErrCmd/TestCmd/DiffCmd/SummaryCmd — Core RTK-style adapters
// SessionCmd/CcEconomicsCmd — Local RTK session and economics analytics
// DiscoverCmd/LearnCmd — Bounded RTK discovery and learning diagnostics
// HooksCmd — Agent hook install/status/audit schema
// HookCmd/HookProcessorAction — RTK-style hook processor schema
// RtkParityCmd — Machine-checkable RTK parity inventory and full parity matrix report
// RewriteCmd — Hook-facing command rewrite dry run
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v5.9.0 — Added explicit config subcommands]
// END_CHANGE_SUMMARY

mod code_commands;
mod config_commands;
mod grace_commands;
mod rtk_adapters;
mod rtk_commands;
mod rtk_core_adapters;
mod rtk_discovery;
mod rtk_dotnet_artifacts;
mod rtk_economics;
mod rtk_full_parity;
mod rtk_hook_processors;
mod rtk_parity;
mod rtk_system_adapters;
mod runtime_commands;
mod setup_commands;

use crate::config::Config;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    #[command(name = "gt", about = "Run Graphite CLI through the token-saving proxy")]
    Gt(RtkProxyCmd),
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
    #[command(name = "jest", about = "Run Jest through the token-saving proxy")]
    Jest(RtkProxyCmd),
    #[command(name = "lint", about = "Run ESLint through the token-saving proxy")]
    Lint(RtkProxyCmd),
    #[command(name = "format", about = "Run Prettier through the token-saving proxy")]
    Format(RtkProxyCmd),
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
    #[command(about = "Run a command and show only errors and warnings")]
    Err(ErrCmd),
    #[command(about = "Run tests and show compact failure output")]
    Test(TestCmd),
    #[command(about = "Summarize file or unified diff output")]
    Diff(DiffCmd),
    #[command(about = "Summarize text from a file or stdin")]
    Summary(SummaryCmd),
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
    #[command(name = "binlog", about = "Summarize MSBuild binary log diagnostics")]
    Binlog(BinlogCmd),
    #[command(
        name = "dotnet-format-report",
        about = "Summarize dotnet format JSON reports"
    )]
    DotnetFormatReport(DotnetFormatReportCmd),
    #[command(name = "dotnet-trx", about = "Summarize dotnet TRX test result files")]
    DotnetTrx(DotnetTrxCmd),
    #[command(about = "Show RTK adoption and token savings by tracked Synapse session")]
    Session(SessionCmd),
    #[command(
        name = "cc-economics",
        about = "Show local Claude Code token economics from Synapse tracking"
    )]
    CcEconomics(CcEconomicsCmd),
    #[command(
        name = "rtk-parity",
        about = "Report machine-checkable RTK parity inventory"
    )]
    RtkParity(RtkParityCmd),
    #[command(about = "Rewrite a shell command to its Synapse proxy form for agent hooks")]
    Rewrite(RewriteCmd),
    #[command(about = "Process RTK-style agent hook JSON or dry-run hook rewrites")]
    Hook(HookCmd),
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

// START_CONTRACT_RunCommand
// PURPOSE: Provide one CLI-owned dispatch entry point for executable command enums
// INPUTS: { self: Command — parsed command variant }, { config: Config — resolved runtime configuration }
// OUTPUTS: { anyhow::Result<()> — asynchronous command execution result }
// SIDE_EFFECTS: executes the selected command implementation and any command-specific IO
// START_RunCommand
#[allow(async_fn_in_trait)]
pub trait RunCommand {
    async fn run(self, config: Config) -> anyhow::Result<()>;
}
// END_RunCommand

// START_CONTRACT_Command_run
// PURPOSE: Route every top-level CLI command variant to its owned command handler without exposing dispatch logic in main.rs
// INPUTS: { self: Command — parsed top-level command }, { config: Config — resolved runtime configuration }
// OUTPUTS: { anyhow::Result<()> — selected command result }
// SIDE_EFFECTS: executes proxy commands, local RTK adapters, GRACE operations, MCP server, dashboard server, or CI grouped commands
// START_Command_run
impl RunCommand for Command {
    async fn run(self, config: Config) -> anyhow::Result<()> {
        match self {
            Command::Init(cmd) => cmd.run(config).await,
            Command::Index(cmd) => cmd.run(config).await,
            Command::Search(cmd) => cmd.run(config).await,
            Command::View(cmd) => cmd.run(config).await,
            Command::Verify(cmd) => cmd.run(config).await,
            Command::Review(cmd) => cmd.run(config).await,
            Command::Status(cmd) => cmd.run(config).await,
            Command::Read(cmd) => {
                cmd.run_as(config, "cat", true, "Usage: syn read <file>...")
                    .await
            }
            Command::Ls(cmd) => {
                cmd.run_as(config, "ls", false, "Usage: syn ls [args...]")
                    .await
            }
            Command::Tree(cmd) => {
                cmd.run_as(config, "tree", false, "Usage: syn tree [args...]")
                    .await
            }
            Command::Find(cmd) => {
                cmd.run_as(config, "find", false, "Usage: syn find [args...]")
                    .await
            }
            Command::Rg(cmd) => {
                cmd.run_as(config, "rg", true, "Usage: syn rg <pattern> [path...]")
                    .await
            }
            Command::Grep(cmd) => {
                cmd.run_as(config, "grep", true, "Usage: syn grep <pattern> [path...]")
                    .await
            }
            Command::Git(cmd) => {
                cmd.run_as(config, "git", false, "Usage: syn git [args...]")
                    .await
            }
            Command::Gt(cmd) => {
                cmd.run_as(config, "gt", false, "Usage: syn gt [args...]")
                    .await
            }
            Command::Cargo(cmd) => {
                cmd.run_as(config, "cargo", false, "Usage: syn cargo [args...]")
                    .await
            }
            Command::Npm(cmd) => {
                cmd.run_as(config, "npm", false, "Usage: syn npm [args...]")
                    .await
            }
            Command::Pnpm(cmd) => {
                cmd.run_as(config, "pnpm", false, "Usage: syn pnpm [args...]")
                    .await
            }
            Command::Npx(cmd) => {
                cmd.run_as(config, "npx", false, "Usage: syn npx [args...]")
                    .await
            }
            Command::Pytest(cmd) => {
                cmd.run_as(config, "pytest", false, "Usage: syn pytest [args...]")
                    .await
            }
            Command::Ruff(cmd) => {
                cmd.run_as(config, "ruff", false, "Usage: syn ruff [args...]")
                    .await
            }
            Command::Mypy(cmd) => {
                cmd.run_as(config, "mypy", false, "Usage: syn mypy [args...]")
                    .await
            }
            Command::Basedpyright(cmd) => {
                cmd.run_as(
                    config,
                    "basedpyright",
                    false,
                    "Usage: syn basedpyright [args...]",
                )
                .await
            }
            Command::Pip(cmd) => {
                cmd.run_as(config, "pip", false, "Usage: syn pip [args...]")
                    .await
            }
            Command::Uv(cmd) => {
                cmd.run_as(config, "uv", false, "Usage: syn uv [args...]")
                    .await
            }
            Command::Next(cmd) => {
                cmd.run_as(config, "next", false, "Usage: syn next [args...]")
                    .await
            }
            Command::Playwright(cmd) => {
                cmd.run_as(
                    config,
                    "playwright",
                    false,
                    "Usage: syn playwright [args...]",
                )
                .await
            }
            Command::Prettier(cmd) => {
                cmd.run_as(config, "prettier", false, "Usage: syn prettier [args...]")
                    .await
            }
            Command::Prisma(cmd) => {
                cmd.run_as(config, "prisma", false, "Usage: syn prisma [args...]")
                    .await
            }
            Command::Tsc(cmd) => {
                cmd.run_as(config, "tsc", false, "Usage: syn tsc [args...]")
                    .await
            }
            Command::Vitest(cmd) => {
                cmd.run_as(config, "vitest", false, "Usage: syn vitest [args...]")
                    .await
            }
            Command::Jest(cmd) => {
                cmd.run_as(config, "jest", false, "Usage: syn jest [args...]")
                    .await
            }
            Command::Lint(cmd) => {
                cmd.run_as(config, "eslint", false, "Usage: syn lint [args...]")
                    .await
            }
            Command::Format(cmd) => {
                cmd.run_as(config, "prettier", false, "Usage: syn format [args...]")
                    .await
            }
            Command::Gh(cmd) => {
                cmd.run_as(config, "gh", false, "Usage: syn gh [args...]")
                    .await
            }
            Command::Glab(cmd) => {
                cmd.run_as(config, "glab", false, "Usage: syn glab [args...]")
                    .await
            }
            Command::Aws(cmd) => {
                cmd.run_as(config, "aws", false, "Usage: syn aws [args...]")
                    .await
            }
            Command::Psql(cmd) => {
                cmd.run_as(config, "psql", false, "Usage: syn psql [args...]")
                    .await
            }
            Command::Curl(cmd) => {
                cmd.run_as(config, "curl", true, "Usage: syn curl <url-or-args>...")
                    .await
            }
            Command::Wget(cmd) => {
                cmd.run_as(config, "wget", true, "Usage: syn wget <url-or-args>...")
                    .await
            }
            Command::Jq(cmd) => {
                cmd.run_as(config, "jq", false, "Usage: syn jq [args...]")
                    .await
            }
            Command::Go(cmd) => {
                cmd.run_as(config, "go", false, "Usage: syn go [args...]")
                    .await
            }
            Command::Golangci(cmd) => {
                cmd.run_as(
                    config,
                    "golangci-lint",
                    false,
                    "Usage: syn golangci [args...]",
                )
                .await
            }
            Command::Dotnet(cmd) => {
                cmd.run_as(config, "dotnet", false, "Usage: syn dotnet [args...]")
                    .await
            }
            Command::Rake(cmd) => {
                cmd.run_as(config, "rake", false, "Usage: syn rake [args...]")
                    .await
            }
            Command::Rspec(cmd) => {
                cmd.run_as(config, "rspec", false, "Usage: syn rspec [args...]")
                    .await
            }
            Command::Rubocop(cmd) => {
                cmd.run_as(config, "rubocop", false, "Usage: syn rubocop [args...]")
                    .await
            }
            Command::Gradle(cmd) => {
                cmd.run_as(config, "gradle", false, "Usage: syn gradle [args...]")
                    .await
            }
            Command::Gradlew(cmd) => {
                cmd.run_as(config, "./gradlew", false, "Usage: syn gradlew [args...]")
                    .await
            }
            Command::Make(cmd) => {
                cmd.run_as(config, "make", false, "Usage: syn make [args...]")
                    .await
            }
            Command::Just(cmd) => {
                cmd.run_as(config, "just", false, "Usage: syn just [args...]")
                    .await
            }
            Command::Helm(cmd) => {
                cmd.run_as(config, "helm", false, "Usage: syn helm [args...]")
                    .await
            }
            Command::Kubectl(cmd) => {
                cmd.run_as(config, "kubectl", false, "Usage: syn kubectl [args...]")
                    .await
            }
            Command::Docker(cmd) => {
                cmd.run_as(config, "docker", false, "Usage: syn docker [args...]")
                    .await
            }
            Command::Podman(cmd) => {
                cmd.run_as(config, "podman", false, "Usage: syn podman [args...]")
                    .await
            }
            Command::Json(cmd) => cmd.run(config).await,
            Command::Deps(cmd) => cmd.run(config).await,
            Command::Env(cmd) => cmd.run(config).await,
            Command::Wc(cmd) => cmd.run(config).await,
            Command::Err(cmd) => cmd.run(config).await,
            Command::Test(cmd) => cmd.run(config).await,
            Command::Diff(cmd) => cmd.run(config).await,
            Command::Summary(cmd) => cmd.run(config).await,
            Command::Pipe(cmd) => cmd.run(config).await,
            Command::Log(cmd) => cmd.run(config).await,
            Command::Smart(cmd) => cmd.run(config).await,
            Command::Discover(cmd) => cmd.run(config).await,
            Command::Learn(cmd) => cmd.run(config).await,
            Command::Binlog(cmd) => cmd.run(config).await,
            Command::DotnetFormatReport(cmd) => cmd.run(config).await,
            Command::DotnetTrx(cmd) => cmd.run(config).await,
            Command::Session(cmd) => cmd.run(config).await,
            Command::CcEconomics(cmd) => cmd.run(config).await,
            Command::RtkParity(cmd) => cmd.run(config).await,
            Command::Rewrite(cmd) => cmd.run(config).await,
            Command::Hook(cmd) => cmd.run(config).await,
            Command::Run(cmd) => cmd.run(config).await,
            Command::Proxy(cmd) => cmd.run(config).await,
            Command::Filters(cmd) => cmd.run(config).await,
            Command::Gain(cmd) => cmd.run(config).await,
            Command::Compress(cmd) => cmd.run(config).await,
            Command::Mcp(cmd) => cmd.run(config).await,
            Command::Config(cmd) => cmd.run(config).await,
            Command::GraphRag(cmd) => cmd.run(config).await,
            Command::Hooks(cmd) => cmd.run(config).await,
            Command::Doctor(cmd) => cmd.run(config).await,
            Command::Refresh(cmd) => cmd.run(config).await,
            Command::Ci(cmd) => match cmd.action {
                CiAction::Verify(inner) => inner.run(config).await,
                CiAction::Review(inner) => inner.run(config).await,
                CiAction::Status(inner) => inner.run(config).await,
            },
            Command::Skills(cmd) => cmd.run(config).await,
            Command::History(cmd) => cmd.run(config).await,
            Command::Serve(cmd) => cmd.run(config).await,
        }
    }
}
// END_Command_run

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

// START_ErrCmd
#[derive(clap::Args)]
#[command(about = "Run a command and show only errors and warnings")]
pub struct ErrCmd {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}
// END_ErrCmd

// START_TestCmd
#[derive(clap::Args)]
#[command(about = "Run tests and show compact failure output")]
pub struct TestCmd {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}
// END_TestCmd

// START_DiffCmd
#[derive(clap::Args)]
#[command(about = "Summarize file or unified diff output")]
pub struct DiffCmd {
    pub file1: Option<PathBuf>,
    pub file2: Option<PathBuf>,
}
// END_DiffCmd

// START_SummaryCmd
#[derive(clap::Args)]
#[command(about = "Summarize text from a file or stdin")]
pub struct SummaryCmd {
    pub input: Option<PathBuf>,
}
// END_SummaryCmd

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

// START_BinlogCmd
#[derive(clap::Args)]
#[command(about = "Summarize MSBuild binary log diagnostics")]
pub struct BinlogCmd {
    pub path: PathBuf,
}
// END_BinlogCmd

// START_DotnetFormatReportCmd
#[derive(clap::Args)]
#[command(about = "Summarize dotnet format JSON reports")]
pub struct DotnetFormatReportCmd {
    pub path: PathBuf,
}
// END_DotnetFormatReportCmd

// START_DotnetTrxCmd
#[derive(clap::Args)]
#[command(about = "Summarize dotnet TRX test result files")]
pub struct DotnetTrxCmd {
    pub path: PathBuf,
}
// END_DotnetTrxCmd

// START_SessionCmd
#[derive(clap::Args)]
#[command(about = "Show RTK adoption and token savings by tracked Synapse session")]
pub struct SessionCmd {
    #[arg(long)]
    pub json: bool,
}
// END_SessionCmd

// START_CcEconomicsCmd
#[derive(clap::Args)]
#[command(about = "Show local Claude Code token economics from Synapse tracking")]
pub struct CcEconomicsCmd {
    #[arg(short, long)]
    pub daily: bool,
    #[arg(short, long)]
    pub weekly: bool,
    #[arg(short, long)]
    pub monthly: bool,
    #[arg(short, long)]
    pub all: bool,
    #[arg(short, long, default_value = "text")]
    pub format: String,
}
// END_CcEconomicsCmd

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
    #[arg(long)]
    pub full: bool,
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
    #[arg(long)]
    pub json: bool,
}
// END_HooksCmd

// START_HookCmd
#[derive(clap::Args)]
#[command(about = "Process RTK-style agent hook JSON or dry-run hook rewrites")]
pub struct HookCmd {
    #[command(subcommand)]
    pub command: HookProcessorAction,
}
// END_HookCmd

// START_HookProcessorAction
#[derive(Subcommand)]
pub enum HookProcessorAction {
    Claude,
    Cursor,
    Gemini,
    Copilot,
    Check {
        #[arg(long, default_value = "claude")]
        agent: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
}
// END_HookProcessorAction

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
    #[command(subcommand)]
    pub action: Option<ConfigAction>,
}
// END_ConfigCmd

// START_ConfigAction
#[derive(Subcommand)]
pub enum ConfigAction {
    #[command(about = "Print the config file path")]
    Path,
    #[command(about = "Open the config file in $EDITOR or $VISUAL")]
    Edit,
    #[command(about = "List supported scalar config keys and current values")]
    List,
    #[command(about = "Get one supported scalar config value")]
    Get { key: String },
    #[command(about = "Set and persist one supported scalar config value")]
    Set { key: String, value: String },
    #[command(about = "Reset and persist one supported scalar config value to its default")]
    Unset { key: String },
}
// END_ConfigAction

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
