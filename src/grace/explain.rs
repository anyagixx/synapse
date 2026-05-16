use crate::indexer::storage::Storage;
use std::path::Path;

#[derive(Debug, serde::Serialize)]
pub struct ExplainResult {
    pub query: String,
    pub answer: String,
    pub sources: Vec<String>,
}

pub struct Explainer;

impl Default for Explainer {
    fn default() -> Self {
        Self::new()
    }
}

impl Explainer {
    pub fn new() -> Self {
        Self
    }

    pub async fn explain(query: &str, root: &Path) -> anyhow::Result<ExplainResult> {
        let storage = Storage::new(root);
        let results = storage.search(query, 8);

        let mut sources: Vec<String> = results
            .iter()
            .map(|b| format!("{}:{} — {}", b.path, b.start_line, b.name))
            .collect();
        sources.sort();
        sources.dedup();

        let answer = if sources.is_empty() {
            format!(
                "No relevant code found for '{}'. Run `syn index` first.",
                query
            )
        } else {
            let mut ans = format!("Found {} relevant code sections:\n\n", sources.len());
            for (i, src) in sources.iter().enumerate() {
                ans.push_str(&format!("{}. {}\n", i + 1, src));
            }
            ans
        };

        Ok(ExplainResult {
            query: query.to_string(),
            answer,
            sources,
        })
    }
}
