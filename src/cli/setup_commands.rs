// MODULE_CONTRACT
// MODULE_ID: M-CLI-SETUP-COMMANDS
// PURPOSE: CLI setup and indexing command handlers with guarded index storage status reporting and safe OpenCode config merge
// SCOPE: InitCmd, IndexCmd, watch_and_reindex, guarded index storage counts, gitignore toggle, OpenCode MCP merge
// DEPENDS: M-CONFIG, M-GRACE-BOOTSTRAP, M-GRACE-LAYOUT, M-INDEXER, M-HOOKS
// LINKS: docs/modules/M-CLI.xml

// START_MODULE_MAP
// InitCmd::run — Initializes project hooks and MyGRACE docs
// IndexCmd::run — Indexes the current project
// watch_and_reindex — Re-indexes source files after filesystem changes
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.6.0 — Updated install summary for 32 MCP tools]
// END_CHANGE_SUMMARY

use super::{IndexCmd, InitCmd};
use crate::config::Config;
use crate::grace::bootstrap::bootstrap_existing_repo;
use crate::grace::layout::DocsLayout;

// START_public_api

impl InitCmd {
    // START_CONTRACT_InitCmd::run
    // PURPOSE: Initialize project hooks, sharded docs, and optional existing-repo artifacts
    // INPUTS: { config: Config — runtime config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes project integration files, docs, and index storage
    // START_init_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;

        let agents_path = root.join("AGENTS.md");
        if !agents_path.exists() {
            std::fs::write(
                &agents_path,
                include_str!("../../.opencode/rules/synapse.md"),
            )?;
        }

        let opencode_dir = root.join(".opencode");
        std::fs::create_dir_all(&opencode_dir)?;

        let oc_config_path = root.join("opencode.jsonc");
        let existing_config = std::fs::read_to_string(&oc_config_path).unwrap_or_default();
        let merged_config = crate::hooks::merge_synapse_mcp_config(&existing_config)?;
        if existing_config != merged_config {
            std::fs::write(&oc_config_path, merged_config)?;
        }

        let plugins_dir = opencode_dir.join("plugins");
        std::fs::create_dir_all(&plugins_dir)?;
        let plugin_path = plugins_dir.join("synapse.ts");
        if !plugin_path.exists() {
            std::fs::write(
                &plugin_path,
                include_str!("../../.opencode/plugins/synapse.ts"),
            )?;
        }

        let rules_dir = opencode_dir.join("rules");
        std::fs::create_dir_all(&rules_dir)?;
        let rules_path = rules_dir.join("synapse.md");
        if !rules_path.exists() {
            std::fs::write(
                &rules_path,
                include_str!("../../.opencode/rules/synapse.md"),
            )?;
        }

        let pkg_path = opencode_dir.join("package.json");
        if !pkg_path.exists() {
            std::fs::write(
                &pkg_path,
                r#"{"dependencies":{"@opencode-ai/plugin":"^1.15"}}"#,
            )?;
        }

        let layout = DocsLayout::new(&root);
        layout.ensure_initialized()?;
        if self.from_existing {
            bootstrap_existing_repo(&root)?;
        }

        let walker = crate::indexer::walker::Walker::new(&root);
        let files = walker.walk();
        if !files.is_empty() {
            let indexer = crate::indexer::Indexer::new(&config);
            indexer.index_directory(&root).await?;
        }

        println!("Synapse hooks installed at {}", root.display());
        println!();
        println!("What was created:");
        println!("  AGENTS.md                    — GRACE constitution (read by every LLM session)");
        println!("  opencode.jsonc              — MCP auto-start (32 tools: 17 core + 15 GRACE)");
        println!(
            "  docs/                        — Sharded architecture layout + compatibility XML docs"
        );
        println!("  .opencode/plugins/synapse.ts — auto-proxy + GRACE system context");
        println!("  .opencode/rules/synapse.md   — tool reference for LLM");
        println!();
        println!("Done. Now run: opencode");
        if self.from_existing {
            println!("Existing repo bootstrap: seeded sharded docs from current source tree.");
        }
        println!("The LLM will ask what you want to build and create everything.");
        Ok(())
    }
    // END_init_run
}

impl IndexCmd {
    // START_CONTRACT_IndexCmd::run
    // PURPOSE: Index the current directory and optionally watch for source changes
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes index storage and may watch filesystem events
    // START_index_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let indexer = crate::indexer::Indexer::new(&config);
        let root = std::env::current_dir()?;

        if self.force {
            println!("Force re-indexing...");
        }
        if self.no_git {
            println!("Indexing without gitignore rules");
        }

        indexer
            .index_directory_with_gitignore(&root, !self.no_git)
            .await?;

        let total = indexer.storage_count()?;
        println!("Index complete: {} code blocks", total);

        if self.watch {
            println!("Watching for changes... (Ctrl+C to stop)");
            watch_and_reindex(root).await?;
        }

        Ok(())
    }
    // END_index_run
}

// START_CONTRACT_watch_and_reindex
// PURPOSE: Watch source files and re-run indexing after debounced changes
// INPUTS: { root: PathBuf — project root }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: watches filesystem and rewrites index storage
// START_watch_and_reindex
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

                if source_files.is_empty() || last_index.elapsed() < Duration::from_secs(2) {
                    continue;
                }
                last_index = tokio::time::Instant::now();

                let config = crate::config::Config::load_or_default();
                let indexer = crate::indexer::Indexer::new(&config);
                indexer.index_directory(&root).await?;

                let total = indexer.storage_count()?;

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
// END_watch_and_reindex

// END_public_api
