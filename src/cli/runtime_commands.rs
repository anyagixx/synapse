// MODULE_CONTRACT
// MODULE_ID: M-CLI-RUNTIME-COMMANDS
// PURPOSE: CLI runtime, integration, and diagnostic command handlers with storage health and clean-bootstrap reporting
// SCOPE: GainCmd, ProxyCmd, CompressCmd, McpCmd, ConfigCmd, HooksCmd, DoctorCmd, clean config fallback diagnostics, index storage diagnostics, ServeCmd
// DEPENDS: M-CONFIG, M-TRACKING, M-PROXY, M-COMPRESS, M-MCP, M-HOOKS, M-DASHBOARD, M-INDEXER-STORAGE
// LINKS: docs/modules/M-CLI.xml

// START_MODULE_MAP
// GainCmd::run — Prints token savings
// print_savings_graph — Prints ASCII token-savings bars for top commands
// ProxyCmd::run — Runs proxied shell commands
// CompressCmd::run — Compresses or restores files
// McpCmd::run — Starts MCP server
// ConfigCmd::run — Prints or opens config
// HooksCmd::run — Manages hook installation
// DoctorCmd::run — Runs setup diagnostics
// ServeCmd::run — Starts dashboard
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.2.0 — Keep public MCP command stdio-only until HTTP transport exists]
// END_CHANGE_SUMMARY

use super::{CompressCmd, ConfigCmd, DoctorCmd, GainCmd, HooksCmd, McpCmd, ProxyCmd, ServeCmd};
use crate::config::Config;
use std::io::Write;

// START_public_api

impl GainCmd {
    // START_CONTRACT_GainCmd::run
    // PURPOSE: Print token savings analytics
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_gain_run
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
            let est_cost_saved = stats.total_saved_tokens as f64 * 0.000003;
            println!("Est. cost saved:     ${:.4}", est_cost_saved);
        }
        if !stats.top_commands.is_empty() {
            println!();
            println!("Top commands by token savings:");
            for item in &stats.top_commands {
                println!(
                    "  - {}: {} runs, {} tokens saved, avg {:.1}%",
                    item.command, item.count, item.saved_tokens, item.avg_savings_pct
                );
            }
        }
        if self.graph {
            print_savings_graph(&stats.top_commands);
        }
        Ok(())
    }
    // END_gain_run
}

// START_CONTRACT_print_savings_graph
// PURPOSE: Render token savings by command as compact ASCII bars
// INPUTS: { items: &[TrackingCommandStat] — top tracked command savings }
// OUTPUTS: { stdout graph lines }
// SIDE_EFFECTS: writes to stdout
// START_print_savings_graph
fn print_savings_graph(items: &[crate::tracking::TrackingCommandStat]) {
    if items.is_empty() {
        return;
    }
    let max_saved = items
        .iter()
        .map(|item| item.saved_tokens)
        .max()
        .unwrap_or(0);
    println!();
    println!("Savings graph:");
    for item in items {
        let width = if max_saved == 0 {
            0
        } else {
            ((item.saved_tokens as f64 / max_saved as f64) * 24.0).round() as usize
        };
        let visible_width = if item.saved_tokens > 0 {
            width.max(1)
        } else {
            0
        };
        println!(
            "  {:<16} | {:<24} {}",
            item.command,
            "#".repeat(visible_width),
            item.saved_tokens
        );
    }
}
// END_print_savings_graph

impl ProxyCmd {
    // START_CONTRACT_ProxyCmd::run
    // PURPOSE: Execute a command through the token-saving proxy
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: runs external command through proxy
    // START_proxy_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        if self.args.is_empty() {
            anyhow::bail!("Usage: syn proxy -- <command> [args...]");
        }
        let proxy = crate::proxy::Proxy::new(&config);
        let output = proxy.execute(&self.args).await?;
        {
            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "{}", output.text)?;
            stdout.flush()?;
        }
        if !output.success {
            std::process::exit(output.status_code);
        }
        Ok(())
    }
    // END_proxy_run
}

impl CompressCmd {
    // START_CONTRACT_CompressCmd::run
    // PURPOSE: Compress or restore one or more files
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes compressed or restored file contents
    // START_compress_run
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
    // END_compress_run
}

impl McpCmd {
    // START_CONTRACT_McpCmd::run
    // PURPOSE: Start MCP server over stdio
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: starts MCP server loop
    // START_mcp_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let server = crate::mcp::server::McpServer::new(config);
        server.start_stdio().await
    }
    // END_mcp_run
}

impl ConfigCmd {
    // START_CONTRACT_ConfigCmd::run
    // PURPOSE: Print config, print path, open editor, or acknowledge set command
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: may spawn configured editor
    // START_config_run
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
    // END_config_run
}

impl HooksCmd {
    // START_CONTRACT_HooksCmd::run
    // PURPOSE: Install, uninstall, or report Synapse hook status
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: may write or remove hook files
    // START_hooks_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let manager = crate::hooks::HookManager::new(&config);
        match self.action.as_str() {
            "install" => manager.install(&self.agent),
            "uninstall" => manager.uninstall(&self.agent),
            "status" => manager.status(),
            _ => anyhow::bail!("Usage: syn hooks install|uninstall|status [opencode|all]"),
        }
    }
    // END_hooks_run
}

impl DoctorCmd {
    // START_CONTRACT_DoctorCmd::run
    // PURPOSE: Run local setup diagnostics for config, index, hooks, docs, parsers, tracking, and sources
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_doctor_run
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

        match Config::path() {
            Ok(path) if path.exists() => match Config::load() {
                Ok(c) => {
                    check!("config", true, &format!("loaded ({})", path.display()), "");
                    let _ = c;
                }
                Err(e) => {
                    check!("config", false, "", &format!("cannot load: {}", e));
                }
            },
            Ok(path) => {
                check!(
                    "config",
                    true,
                    &format!(
                        "using built-in defaults (no user config at {})",
                        path.display()
                    ),
                    ""
                );
            }
            Err(e) => {
                check!("config", false, "", &format!("cannot resolve path: {}", e));
            }
        }

        let storage = crate::indexer::storage::Storage::new(&root);
        let indexed = !storage.is_empty();
        let index_failure = storage
            .load_error()
            .map(|error| format!("index storage unreadable — run `syn index` ({})", error))
            .unwrap_or_else(|| "not indexed — run `syn index`".to_string());
        check!(
            "index",
            indexed && storage.load_error().is_none(),
            &format!("{} blocks indexed", storage.count()),
            &index_failure
        );

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

        let parser = crate::indexer::parser::ParserEngine::new();
        let test_code = "fn test() {}";
        let blocks = parser.parse(test_code, "rust");
        check!(
            "tree-sitter",
            !blocks.is_empty(),
            &format!("Rust parser works ({} blocks)", blocks.len()),
            "parser failed — tree-sitter may be broken"
        );

        let db = crate::tracking::Tracker::db_path().ok();
        check!(
            "sqlite tracking",
            db.is_some(),
            "tracking DB available",
            "cannot find tracking DB path"
        );

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
    // END_doctor_run
}

impl ServeCmd {
    // START_CONTRACT_ServeCmd::run
    // PURPOSE: Start the web dashboard
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: starts HTTP server
    // START_serve_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        crate::dashboard::start_dashboard(&self.bind).await
    }
    // END_serve_run
}

// END_public_api
