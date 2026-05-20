// MODULE_CONTRACT
// MODULE_ID: M-GRACE-FIX
// PURPOSE: Debug/fix module — diagnoses issues via knowledge graph navigation and indexed search
// SCOPE: Debugger struct, FixResult, diagnose via semantic search
// DEPENDS: M-INDEXER-STORAGE, M-UTILS
// LINKS: N/A

// START_MODULE_MAP
// FixResult — Debug/fix result with related modules, suggested blocks, diagnosis
// Debugger — Diagnoses issues using indexed code
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.1.0 — Use Unicode-safe truncation for suggested fix snippets]
// END_CHANGE_SUMMARY

use crate::indexer::storage::Storage;
use std::path::Path;

// START_public_api

// START_FixResult
#[derive(Debug, serde::Serialize)]
pub struct FixResult {
    pub description: String,
    pub related_modules: Vec<String>,
    pub suggested_blocks: Vec<String>,
    pub diagnosis: String,
}
// END_FixResult

// START_Debugger
pub struct Debugger;
// END_Debugger

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}

impl Debugger {
    // START_CONTRACT_Debugger::new
    // PURPOSE: Create a new Debugger
    // OUTPUTS: { Self }
    // START_debugger_new
    pub fn new() -> Self {
        Self
    }
    // END_debugger_new

    // START_CONTRACT_Debugger::diagnose
    // PURPOSE: Diagnose a bug description via knowledge graph navigation
    // INPUTS: { description: &str — bug description }, { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<FixResult> }
    // START_debugger_diagnose
    pub async fn diagnose(description: &str, root: &Path) -> anyhow::Result<FixResult> {
        let storage = Storage::new(root);

        // 1. Find related modules via semantic search
        let related = storage.search(description, 10);

        // 2. Build diagnosis
        let mut modules: Vec<String> = related
            .iter()
            .map(|b| format!("{}:{} — {} ('{}')", b.path, b.start_line, b.kind, b.name))
            .collect();
        modules.sort();
        modules.dedup();

        let blocks: Vec<String> = related
            .iter()
            .take(5)
            .map(|b| {
                format!(
                    "{}:{} ({})\n  {}",
                    b.path,
                    b.start_line,
                    b.name,
                    crate::utils::truncate_chars(&b.content, 200)
                )
            })
            .collect();

        let diagnosis = if modules.is_empty() {
            format!(
                "No indexed code relates to '{}'. Run `syn index` first.",
                description
            )
        } else {
            format!(
                "Found {} relevant code blocks across {} modules.",
                related.len(),
                modules.len()
            )
        };

        Ok(FixResult {
            description: description.to_string(),
            related_modules: modules,
            suggested_blocks: blocks,
            diagnosis,
        })
    }
    // END_debugger_diagnose
}
// END_public_api
