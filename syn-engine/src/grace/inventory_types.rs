// MODULE_CONTRACT
// MODULE_ID: M-GRACE-INVENTORY-TYPES
// PURPOSE: Canonical MyGRACE inventory data types shared by drift, refresh, review, and status flows
// SCOPE: CodeModule, GraphEntry, VerificationEntry, ArtifactInventory, ArtifactDrift models
// DEPENDS: N/A
// LINKS: docs/modules/M-GRACE-INVENTORY.xml

// START_MODULE_MAP
// CodeModule — Source module facts extracted from MODULE_CONTRACT
// canonical_code_modules — Coalesces duplicate MODULE_ID contracts into one module fact
// ArtifactInventory — Raw facts collected from source contracts and sharded artifacts
// ArtifactDrift — Normalized drift report shared by refresh, verify, review, and status
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.8.0 — Added canonical source path aggregation for duplicate MODULE_ID contracts]
// END_CHANGE_SUMMARY

use std::collections::{BTreeMap, BTreeSet};

// START_public_api

// START_CodeModule
#[derive(Debug, Clone, serde::Serialize)]
pub struct CodeModule {
    pub id: String,
    pub source_path: String,
    pub source_paths: Vec<String>,
    pub purpose: String,
    pub scope: String,
    pub depends: Vec<String>,
    pub links: Vec<String>,
    pub contract_errors: Vec<String>,
}
// END_CodeModule

// START_CONTRACT_canonical_code_modules
// PURPOSE: Return one deterministic CodeModule per MODULE_ID while preserving all source files and merged references
// INPUTS: { modules: Vec<CodeModule> — raw contract-derived module facts }
// OUTPUTS: { Vec<CodeModule> — sorted canonical module facts }
// START_canonical_code_modules
pub(crate) fn canonical_code_modules(modules: Vec<CodeModule>) -> Vec<CodeModule> {
    let mut by_id: BTreeMap<String, CodeModule> = BTreeMap::new();
    for mut module in modules {
        module.source_paths = normalized_sources(&module);
        module.source_path = module.source_paths.first().cloned().unwrap_or_default();
        module.depends = unique_nonempty(module.depends);
        module.links = unique_nonempty(module.links);
        module.contract_errors = unique_nonempty(module.contract_errors);

        if let Some(existing) = by_id.get_mut(&module.id) {
            merge_code_module(existing, module);
        } else {
            by_id.insert(module.id.clone(), module);
        }
    }
    by_id.into_values().collect()
}
// END_canonical_code_modules

// START_GraphEntry
#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphEntry {
    pub id: String,
    pub path: String,
    pub status: String,
}
// END_GraphEntry

// START_VerificationEntry
#[derive(Debug, Clone, serde::Serialize)]
pub struct VerificationEntry {
    pub id: String,
    pub module: String,
    pub path: String,
    pub priority: String,
    pub status: String,
}
// END_VerificationEntry

// START_ArtifactInventory
#[derive(Debug, Clone, serde::Serialize)]
pub struct ArtifactInventory {
    pub code_modules: Vec<CodeModule>,
    pub graph_entries: Vec<GraphEntry>,
    pub verification_entries: Vec<VerificationEntry>,
    pub module_shards: Vec<String>,
    pub verification_shards: Vec<String>,
    pub files_without_contract: Vec<String>,
}
// END_ArtifactInventory

// START_ArtifactDrift
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct ArtifactDrift {
    pub total_code_modules: usize,
    pub duplicate_graph_ids: Vec<String>,
    pub duplicate_verification_ids: Vec<String>,
    pub duplicate_verification_modules: Vec<String>,
    pub code_not_in_graph: Vec<String>,
    pub graph_not_in_code: Vec<String>,
    pub code_not_in_verification: Vec<String>,
    pub verification_not_in_code: Vec<String>,
    pub graph_path_mismatches: Vec<String>,
    pub verification_path_mismatches: Vec<String>,
    pub missing_module_shards: Vec<String>,
    pub missing_verification_shards: Vec<String>,
    pub orphan_module_shards: Vec<String>,
    pub orphan_verification_shards: Vec<String>,
    pub module_shard_mismatches: Vec<String>,
    pub verification_shard_mismatches: Vec<String>,
    pub files_without_contract: Vec<String>,
    pub contract_issues: Vec<String>,
    pub suggested_actions: Vec<String>,
}
// END_ArtifactDrift

impl ArtifactDrift {
    // START_CONTRACT_ArtifactDrift::is_clean
    // PURPOSE: Return true when no canonical MyGRACE drift is present
    // OUTPUTS: { bool }
    // START_artifact_drift_is_clean
    pub fn is_clean(&self) -> bool {
        self.issue_count() == 0
    }
    // END_artifact_drift_is_clean

    // START_CONTRACT_ArtifactDrift::issue_count
    // PURPOSE: Count all canonical drift issue entries
    // OUTPUTS: { usize }
    // START_artifact_drift_issue_count
    pub fn issue_count(&self) -> usize {
        self.duplicate_graph_ids.len()
            + self.duplicate_verification_ids.len()
            + self.duplicate_verification_modules.len()
            + self.code_not_in_graph.len()
            + self.graph_not_in_code.len()
            + self.code_not_in_verification.len()
            + self.verification_not_in_code.len()
            + self.graph_path_mismatches.len()
            + self.verification_path_mismatches.len()
            + self.missing_module_shards.len()
            + self.missing_verification_shards.len()
            + self.orphan_module_shards.len()
            + self.orphan_verification_shards.len()
            + self.module_shard_mismatches.len()
            + self.verification_shard_mismatches.len()
            + self.files_without_contract.len()
    }
    // END_artifact_drift_issue_count
}

fn merge_code_module(existing: &mut CodeModule, incoming: CodeModule) {
    existing.source_paths = unique_nonempty(
        existing
            .source_paths
            .iter()
            .cloned()
            .chain(incoming.source_paths)
            .collect(),
    );
    existing.source_path = existing.source_paths.first().cloned().unwrap_or_default();

    if incoming.purpose.len() > existing.purpose.len() {
        existing.purpose = incoming.purpose;
    }
    if incoming.scope.len() > existing.scope.len() {
        existing.scope = incoming.scope;
    }

    existing.depends = unique_nonempty(
        existing
            .depends
            .iter()
            .cloned()
            .chain(incoming.depends)
            .collect(),
    );
    existing.links = unique_nonempty(
        existing
            .links
            .iter()
            .cloned()
            .chain(incoming.links)
            .collect(),
    );
    existing.contract_errors = unique_nonempty(
        existing
            .contract_errors
            .iter()
            .cloned()
            .chain(incoming.contract_errors)
            .collect(),
    );
}

fn normalized_sources(module: &CodeModule) -> Vec<String> {
    let mut paths = module.source_paths.clone();
    paths.push(module.source_path.clone());
    unique_nonempty(paths)
}

fn unique_nonempty(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && value != "N/A")
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

// END_public_api
