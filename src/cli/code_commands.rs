// MODULE_CONTRACT
// MODULE_ID: M-CLI-CODE-COMMANDS
// PURPOSE: CLI code navigation command handlers
// SCOPE: SearchCmd, ViewCmd, GrepCmd, GraphRagCmd, HistoryCmd
// DEPENDS: M-CONFIG, M-INDEXER, M-GRAPHRAG
// LINKS: docs/modules/M-CLI.xml

// START_MODULE_MAP
// SearchCmd::run — Runs semantic code search
// ViewCmd::run — Prints file signatures
// GrepCmd::run — Runs AST structural search
// GraphRagCmd::run — Queries code graph
// HistoryCmd::run — Searches git history summaries
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Extracted code navigation CLI handlers from M-CLI]
// END_CHANGE_SUMMARY

use super::{GraphRagCmd, GrepCmd, HistoryCmd, SearchCmd, ViewCmd};
use crate::config::Config;

// START_public_api

impl SearchCmd {
    // START_CONTRACT_SearchCmd::run
    // PURPOSE: Search indexed code blocks and print ranked results
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_search_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let query = self.query.join(" ");
        if query.is_empty() {
            anyhow::bail!(
                "Usage: syn search <query> [--mode code|file] [--language rust|py|...] [--max-results N]"
            );
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
    // END_search_run
}

impl ViewCmd {
    // START_CONTRACT_ViewCmd::run
    // PURPOSE: Print indexed signatures for one or more files
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_view_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let indexer = crate::indexer::Indexer::new(&config);
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
    // END_view_run
}

impl GrepCmd {
    // START_CONTRACT_GrepCmd::run
    // PURPOSE: Search parsed code blocks by kind, name, or content and optionally print rewrite output
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_grep_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        use crate::indexer::parser::ParserEngine;
        let root = std::env::current_dir()?;
        let walker = crate::indexer::walker::Walker::new(&root);
        let files = walker.walk();
        let parser = ParserEngine::new();
        let pattern_lower = self.pattern.to_lowercase();
        let mut found = 0usize;

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
    // END_grep_run
}

impl GraphRagCmd {
    // START_CONTRACT_GraphRagCmd::run
    // PURPOSE: Build the code graph and run overview or search operation
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_graphrag_run
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
    // END_graphrag_run
}

impl HistoryCmd {
    // START_CONTRACT_HistoryCmd::run
    // PURPOSE: Search recent git history summaries for a query
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_history_run
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
    // END_history_run
}

// END_public_api
