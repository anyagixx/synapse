// MODULE_CONTRACT
// MODULE_ID: M-GRACE-EXPLAIN
// PURPOSE: Code explainer (simple) — searches indexed code for a query and returns relevant sections
// SCOPE: Explainer struct, ExplainResult, explain via indexed search
// DEPENDS: M-INDEXER-STORAGE
// LINKS: N/A

// START_MODULE_MAP
// ExplainResult — Explanation result with query, answer text, and source references
// Explainer — Searches indexed code and builds explanations
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY

use crate::indexer::storage::Storage;
use std::path::Path;

// START_public_api

// START_ExplainResult
#[derive(Debug, serde::Serialize)]
pub struct ExplainResult {
    pub query: String,
    pub answer: String,
    pub sources: Vec<String>,
}
// END_ExplainResult

// START_Explainer
pub struct Explainer;
// END_Explainer

impl Default for Explainer {
    fn default() -> Self {
        Self::new()
    }
}

impl Explainer {
    // START_CONTRACT_Explainer::new
    // PURPOSE: Create a new Explainer
    // OUTPUTS: { Self }
    // START_explainer_new
    pub fn new() -> Self {
        Self
    }
    // END_explainer_new

    // START_CONTRACT_Explainer::explain
    // PURPOSE: Search indexed code for a query and return an explanation
    // INPUTS: { query: &str }, { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<ExplainResult> }
    // START_explainer_explain
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
    // END_explainer_explain
}
// END_public_api
