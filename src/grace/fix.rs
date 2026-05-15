use std::path::Path;
use crate::indexer::storage::Storage;

#[derive(Debug, serde::Serialize)]
pub struct FixResult {
    pub description: String,
    pub related_modules: Vec<String>,
    pub suggested_blocks: Vec<String>,
    pub diagnosis: String,
}

pub struct Debugger;

impl Debugger {
    pub fn new() -> Self {
        Self
    }

    /// Debug via knowledge graph navigation
    pub async fn diagnose(description: &str, root: &Path) -> anyhow::Result<FixResult> {
        let storage = Storage::new(root);

        // 1. Find related modules via semantic search
        let related = storage.search(description, 10);

        // 2. Build diagnosis
        let mut modules: Vec<String> = related.iter()
            .map(|b| format!("{}:{} — {} ('{}')", b.path, b.start_line, b.kind, b.name))
            .collect();
        modules.sort();
        modules.dedup();

        let blocks: Vec<String> = related.iter()
            .take(5)
            .map(|b| format!("{}:{} ({})\n  {}", b.path, b.start_line, b.name, truncate(&b.content, 200)))
            .collect();

        let diagnosis = if modules.is_empty() {
            format!("No indexed code relates to '{}'. Run `syn index` first.", description)
        } else {
            format!("Found {} relevant code blocks across {} modules.", related.len(), modules.len())
        };

        Ok(FixResult {
            description: description.to_string(),
            related_modules: modules,
            suggested_blocks: blocks,
            diagnosis,
        })
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max]) }
}
