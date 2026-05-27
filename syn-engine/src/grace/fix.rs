// MODULE_CONTRACT
// MODULE_ID: M-GRACE-FIX
// PURPOSE: Debug/fix module — diagnoses issues via knowledge graph navigation, indexed search, and structured failure reports
// SCOPE: Debugger struct, FixResult, diagnose via semantic search, diagnose_failure_report via enhanced failure diagnosis, and Unicode-safe snippet previews
// DEPENDS: M-GRACE-FAILURE-DIAGNOSIS, M-INDEXER-STORAGE, M-UTILS
// LINKS:
//   -> UC-002 (implements) - diagnosis supports verified bounded changes
//   -> NFR-002 (traces_to) - diagnosis failures must degrade explicitly

// START_MODULE_MAP
// FixResult — Debug/fix result with related modules, suggested blocks, diagnosis
// Debugger — Diagnoses issues using indexed code and structured failure reports
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.2.0 - Added structured failure report diagnosis]
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
                    syn_core::utils::truncate_chars(&b.content, 200)
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

    // START_CONTRACT_Debugger::diagnose_failure_report
    // PURPOSE: Diagnose tester-agent XML or plain text failure with exact search and repair suggestions
    // INPUTS: { report: &str }, { root: &Path }
    // OUTPUTS: { anyhow::Result<EnhancedFixResult> }
    // LINKS:
    //   -> UC-002 (implements) - convert failure evidence into bounded repair context
    // START_debugger_diagnose_failure_report
    pub async fn diagnose_failure_report(
        report: &str,
        root: &Path,
    ) -> anyhow::Result<crate::grace::failure_diagnosis::EnhancedFixResult> {
        crate::grace::failure_diagnosis::diagnose_failure_text(root, report)
    }
    // END_debugger_diagnose_failure_report
}
// END_public_api
