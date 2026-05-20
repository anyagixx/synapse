// MODULE_CONTRACT
// MODULE_ID: M-GRACE-INVENTORY
// PURPOSE: Canonical MyGRACE inventory facade — compares code contracts, shard indexes, and per-entity files
// SCOPE: MyGraceInventory collection, drift detection, and artifact sync orchestration
// DEPENDS: M-GRACE-CONTRACT, M-GRACE-INVENTORY-ARTIFACTS, M-GRACE-INVENTORY-TYPES, M-GRACE-LAYOUT
// LINKS: docs/graph-index.xml, docs/verification-index.xml, docs/modules/, docs/verification/

// START_MODULE_MAP
// MyGraceInventory — Collects inventory, detects drift, and writes canonical artifacts
// ArtifactInventory — Re-exported raw facts model
// ArtifactDrift — Re-exported normalized drift report
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Split inventory types and artifact IO into dedicated modules]
// END_CHANGE_SUMMARY

use crate::grace::contract::{ContractValidator, ModuleContract, TypedLink};
use crate::grace::inventory_artifacts::{
    drift_from_inventory, list_xml_stems, parse_graph_index, parse_verification_index,
    sync_inventory_artifacts,
};
use crate::grace::layout::DocsLayout;
use std::path::{Path, PathBuf};

pub use crate::grace::inventory_types::{
    ArtifactDrift, ArtifactInventory, CodeModule, GraphEntry, VerificationEntry,
};

// START_public_api

// START_MyGraceInventory
pub struct MyGraceInventory;
// END_MyGraceInventory

impl MyGraceInventory {
    // START_CONTRACT_MyGraceInventory::collect
    // PURPOSE: Collect source contract and sharded artifact facts for a project
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<ArtifactInventory> }
    // START_inventory_collect
    pub fn collect(root: &Path) -> anyhow::Result<ArtifactInventory> {
        let layout = DocsLayout::new(root);
        let contracts = ContractValidator::validate_project(root)?;
        let mut code_modules = Vec::new();
        let mut files_without_contract = Vec::new();

        for contract in &contracts.contracts {
            if !contract.has_contract {
                files_without_contract.push(relative_to_root(root, &contract.file_path));
                continue;
            }
            if let Some(id) = contract.module_id.as_deref() {
                code_modules.push(code_module_from_contract(root, contract, id));
            }
        }
        code_modules.sort_by(|a, b| a.id.cmp(&b.id));

        let graph_entries = parse_graph_index(&layout.graph_index_path());
        let verification_entries = parse_verification_index(&layout.verification_index_path());
        let module_shards = list_xml_stems(&layout.modules_dir());
        let verification_shards = list_xml_stems(&layout.verification_dir());

        Ok(ArtifactInventory {
            code_modules,
            graph_entries,
            verification_entries,
            module_shards,
            verification_shards,
            files_without_contract,
        })
    }
    // END_inventory_collect

    // START_CONTRACT_MyGraceInventory::drift
    // PURPOSE: Detect canonical MyGRACE drift across code contracts, indexes, and shards
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<ArtifactDrift> }
    // START_inventory_drift
    pub fn drift(root: &Path) -> anyhow::Result<ArtifactDrift> {
        let inventory = Self::collect(root)?;
        Ok(drift_from_inventory(root, &inventory))
    }
    // END_inventory_drift

    // START_CONTRACT_MyGraceInventory::sync
    // PURPOSE: Rewrite graph, module, verification, and Phase-1 shards from canonical source contracts
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<ArtifactDrift> — drift after sync }
    // SIDE_EFFECTS: writes docs/ indexes and shard files, archives stale generated shards
    // START_inventory_sync
    pub fn sync(root: &Path) -> anyhow::Result<ArtifactDrift> {
        let layout = DocsLayout::new(root);
        let inventory = Self::collect(root)?;
        sync_inventory_artifacts(root, &layout, &inventory.code_modules)?;
        Self::drift(root)
    }
    // END_inventory_sync
}

// END_public_api

fn code_module_from_contract(root: &Path, contract: &ModuleContract, id: &str) -> CodeModule {
    CodeModule {
        id: id.to_string(),
        source_path: relative_to_root(root, &contract.file_path),
        purpose: contract.purpose.clone().unwrap_or_default(),
        scope: contract.scope.clone().unwrap_or_default(),
        depends: clean_refs(&contract.depends),
        links: clean_link_targets(&contract.links),
        contract_errors: contract.errors.clone(),
    }
}

fn clean_refs(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter(|value| {
            let value = value.trim();
            !value.is_empty() && value != "N/A"
        })
        .cloned()
        .collect()
}

fn clean_link_targets(values: &[TypedLink]) -> Vec<String> {
    values
        .iter()
        .map(|link| link.target.trim())
        .filter(|value| !value.is_empty() && *value != "N/A")
        .map(ToOwned::to_owned)
        .collect()
}

fn relative_to_root(root: &Path, path: &str) -> String {
    let path = PathBuf::from(path);
    let rel = path.strip_prefix(root).unwrap_or(&path);
    rel.to_string_lossy().replace('\\', "/")
}
