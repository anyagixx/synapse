// MODULE_CONTRACT
// MODULE_ID: M-GRACE-INVENTORY
// PURPOSE: Canonical MyGRACE inventory — compares code contracts, shard indexes, and per-entity files
// SCOPE: ArtifactInventory, ArtifactDrift, MyGraceInventory collection, drift detection, and artifact sync
// DEPENDS: M-GRACE-CONTRACT, M-GRACE-LAYOUT
// LINKS: docs/graph-index.xml, docs/verification-index.xml, docs/modules/, docs/verification/

// START_MODULE_MAP
// ArtifactInventory — Raw facts collected from source contracts and sharded artifacts
// ArtifactDrift — Normalized drift report shared by refresh, verify, review, and status
// MyGraceInventory — Collects inventory, detects drift, and writes canonical artifacts
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.5.0 — Added canonical MyGRACE inventory and sync layer]
// END_CHANGE_SUMMARY

use crate::grace::contract::{ContractValidator, ModuleContract};
use crate::grace::layout::DocsLayout;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// START_public_api

// START_CodeModule
#[derive(Debug, Clone, serde::Serialize)]
pub struct CodeModule {
    pub id: String,
    pub source_path: String,
    pub purpose: String,
    pub scope: String,
    pub depends: Vec<String>,
    pub links: Vec<String>,
    pub contract_errors: Vec<String>,
}
// END_CodeModule

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
    // PURPOSE: Count all drift issue entries
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
        std::fs::create_dir_all(layout.modules_dir())?;
        std::fs::create_dir_all(layout.phases_dir())?;
        std::fs::create_dir_all(layout.verification_dir())?;
        std::fs::create_dir_all(layout.docs_dir().join("archive"))?;

        let inventory = Self::collect(root)?;
        let ids: BTreeSet<String> = inventory
            .code_modules
            .iter()
            .map(|m| m.id.clone())
            .collect();

        archive_stale_shards(root, &layout.modules_dir(), &ids, "modules")?;
        let verification_ids: BTreeSet<String> = ids.iter().map(|id| format!("V-{}", id)).collect();
        archive_stale_shards(
            root,
            &layout.verification_dir(),
            &verification_ids,
            "verification",
        )?;

        write_graph_index(&layout, &inventory.code_modules)?;
        write_verification_index(&layout, &inventory.code_modules)?;
        write_phase_index(&layout)?;
        write_phase_one(&layout, &inventory.code_modules)?;

        for module in &inventory.code_modules {
            write_module_shard(&layout, module)?;
            write_verification_shard(&layout, module)?;
        }

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
        links: clean_refs(&contract.links),
        contract_errors: contract.errors.clone(),
    }
}

fn drift_from_inventory(root: &Path, inventory: &ArtifactInventory) -> ArtifactDrift {
    let layout = DocsLayout::new(root);
    let code_ids: BTreeSet<String> = inventory
        .code_modules
        .iter()
        .map(|m| m.id.clone())
        .collect();
    let graph_ids: Vec<String> = inventory
        .graph_entries
        .iter()
        .map(|e| e.id.clone())
        .collect();
    let verification_ids: Vec<String> = inventory
        .verification_entries
        .iter()
        .map(|e| e.id.clone())
        .collect();
    let verification_modules: Vec<String> = inventory
        .verification_entries
        .iter()
        .map(|e| e.module.clone())
        .collect();
    let graph_set: BTreeSet<String> = graph_ids.iter().cloned().collect();
    let verification_module_set: BTreeSet<String> = verification_modules.iter().cloned().collect();

    let mut drift = ArtifactDrift {
        total_code_modules: code_ids.len(),
        duplicate_graph_ids: duplicates(&graph_ids),
        duplicate_verification_ids: duplicates(&verification_ids),
        duplicate_verification_modules: duplicates(&verification_modules),
        files_without_contract: inventory.files_without_contract.clone(),
        ..Default::default()
    };

    for id in &code_ids {
        if !graph_set.contains(id) {
            drift.code_not_in_graph.push(id.clone());
        }
        if !verification_module_set.contains(id) {
            drift.code_not_in_verification.push(id.clone());
        }
    }
    for id in &graph_set {
        if !code_ids.contains(id) {
            drift.graph_not_in_code.push(id.clone());
        }
    }
    for id in &verification_module_set {
        if !code_ids.contains(id) {
            drift.verification_not_in_code.push(id.clone());
        }
    }

    for entry in &inventory.graph_entries {
        let expected = module_shard_path(&entry.id);
        if entry.path != expected {
            drift.graph_path_mismatches.push(format!(
                "{} path={} expected={}",
                entry.id, entry.path, expected
            ));
        }
        if !root.join(&entry.path).exists() {
            drift.missing_module_shards.push(entry.path.clone());
        }
    }
    for entry in &inventory.verification_entries {
        let expected_id = format!("V-{}", entry.module);
        let expected_path = verification_shard_path(&entry.module);
        if entry.id != expected_id || entry.path != expected_path {
            drift.verification_path_mismatches.push(format!(
                "{} module={} path={} expected_id={} expected_path={}",
                entry.id, entry.module, entry.path, expected_id, expected_path
            ));
        }
        if !root.join(&entry.path).exists() {
            drift.missing_verification_shards.push(entry.path.clone());
        }
    }

    for shard in &inventory.module_shards {
        if !graph_set.contains(shard) {
            drift.orphan_module_shards.push(shard.clone());
        }
    }
    let verification_entry_ids: BTreeSet<String> = verification_ids.into_iter().collect();
    for shard in &inventory.verification_shards {
        if !verification_entry_ids.contains(shard) {
            drift.orphan_verification_shards.push(shard.clone());
        }
    }

    let module_by_id: BTreeMap<String, &CodeModule> = inventory
        .code_modules
        .iter()
        .map(|m| (m.id.clone(), m))
        .collect();
    for (id, module) in &module_by_id {
        let path = layout.modules_dir().join(format!("{}.xml", id));
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                if !content.contains(&format!(r#"<MODULE id="{}""#, id))
                    || !content.contains(&format!("<FILE>{}</FILE>", module.source_path))
                    || !content.contains(&format!("<VERIFICATION_REF>V-{}</VERIFICATION_REF>", id))
                {
                    drift.module_shard_mismatches.push(format!(
                        "{} content does not match {}",
                        path.display(),
                        id
                    ));
                }
            }
            Err(_) => drift
                .missing_module_shards
                .push(module_shard_path(id).to_string()),
        }

        let vpath = layout.verification_dir().join(format!("V-{}.xml", id));
        match std::fs::read_to_string(&vpath) {
            Ok(content) => {
                if !content.contains(&format!(r#"<VERIFICATION id="V-{}" module="{}""#, id, id)) {
                    drift.verification_shard_mismatches.push(format!(
                        "{} content does not match V-{}",
                        vpath.display(),
                        id
                    ));
                }
            }
            Err(_) => drift
                .missing_verification_shards
                .push(verification_shard_path(id).to_string()),
        }

        for issue in &module.contract_errors {
            drift
                .contract_issues
                .push(format!("{}: {}", module.source_path, issue));
        }
    }

    drift.suggested_actions = suggested_actions(&drift);
    drift
}

fn parse_graph_index(path: &Path) -> Vec<GraphEntry> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    content
        .lines()
        .filter(|line| line.contains("<MODULE ") && line.contains("id="))
        .filter_map(|line| {
            Some(GraphEntry {
                id: attr(line, "id")?,
                path: attr(line, "path").unwrap_or_default(),
                status: attr(line, "status").unwrap_or_else(|| "planned".into()),
            })
        })
        .collect()
}

fn parse_verification_index(path: &Path) -> Vec<VerificationEntry> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    content
        .lines()
        .filter(|line| line.contains("<VERIFICATION ") && line.contains("module="))
        .filter_map(|line| {
            Some(VerificationEntry {
                id: attr(line, "id")?,
                module: attr(line, "module")?,
                path: attr(line, "path").unwrap_or_default(),
                priority: attr(line, "priority").unwrap_or_else(|| "normal".into()),
                status: attr(line, "status").unwrap_or_else(|| "planned".into()),
            })
        })
        .collect()
}

fn attr(line: &str, name: &str) -> Option<String> {
    let needle = format!(r#"{}=""#, name);
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn list_xml_stems(dir: &Path) -> Vec<String> {
    let mut stems: Vec<String> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok()))
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("xml") {
                path.file_stem().map(|s| s.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    stems.sort();
    stems
}

fn duplicates(values: &[String]) -> Vec<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for value in values {
        *counts.entry(value.clone()).or_default() += 1;
    }
    counts
        .into_iter()
        .filter_map(|(value, count)| if count > 1 { Some(value) } else { None })
        .collect()
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

fn relative_to_root(root: &Path, path: &str) -> String {
    let path = PathBuf::from(path);
    let rel = path.strip_prefix(root).unwrap_or(&path);
    rel.to_string_lossy().replace('\\', "/")
}

fn module_shard_path(id: &str) -> String {
    format!("docs/modules/{}.xml", id)
}

fn verification_shard_path(id: &str) -> String {
    format!("docs/verification/V-{}.xml", id)
}

fn suggested_actions(drift: &ArtifactDrift) -> Vec<String> {
    let mut actions = Vec::new();
    if drift.issue_count() > 0 {
        actions.push("Run `syn refresh --fix` to rewrite canonical MyGRACE artifacts".into());
    }
    if !drift.files_without_contract.is_empty() {
        actions
            .push("Add MODULE_CONTRACT/MODULE_MAP/CHANGE_SUMMARY to governed source files".into());
    }
    if !drift.contract_issues.is_empty() {
        actions.push("Add missing function contracts or decide explicit advisory scope".into());
    }
    if !drift.duplicate_graph_ids.is_empty() || !drift.duplicate_verification_ids.is_empty() {
        actions.push("Remove duplicate index IDs; lazy navigation requires unique IDs".into());
    }
    actions.sort();
    actions.dedup();
    actions
}

fn write_graph_index(layout: &DocsLayout, modules: &[CodeModule]) -> anyhow::Result<()> {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<GRAPH_INDEX>\n  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY></META>\n  <MODULES>\n",
    );
    for module in modules {
        xml.push_str(&format!(
            "    <MODULE id=\"{}\" path=\"{}\" status=\"active\" />\n",
            module.id,
            module_shard_path(&module.id)
        ));
    }
    xml.push_str("  </MODULES>\n  <RELATIONSHIPS>\n");
    for module in modules {
        for dep in &module.depends {
            xml.push_str(&format!(
                "    <REL source=\"{}\" target=\"{}\" type=\"DEPENDS\" />\n",
                module.id, dep
            ));
        }
    }
    xml.push_str("  </RELATIONSHIPS>\n</GRAPH_INDEX>\n");
    std::fs::write(layout.graph_index_path(), xml)?;
    Ok(())
}

fn write_verification_index(layout: &DocsLayout, modules: &[CodeModule]) -> anyhow::Result<()> {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VERIFICATION_INDEX>\n  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY></META>\n  <VERIFICATIONS>\n",
    );
    for module in modules {
        xml.push_str(&format!(
            "    <VERIFICATION id=\"V-{}\" module=\"{}\" path=\"{}\" priority=\"normal\" status=\"active\" />\n",
            module.id,
            module.id,
            verification_shard_path(&module.id)
        ));
    }
    xml.push_str("  </VERIFICATIONS>\n</VERIFICATION_INDEX>\n");
    std::fs::write(layout.verification_index_path(), xml)?;
    Ok(())
}

fn write_phase_index(layout: &DocsLayout) -> anyhow::Result<()> {
    let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<PLAN_INDEX>\n  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY><ACTIVE_PHASE>Phase-1</ACTIVE_PHASE></META>\n  <PHASES>\n    <PHASE id=\"Phase-1\" path=\"docs/phases/Phase-1.xml\" status=\"active\" />\n  </PHASES>\n</PLAN_INDEX>\n";
    std::fs::write(layout.plan_index_path(), xml)?;
    Ok(())
}

fn write_phase_one(layout: &DocsLayout, modules: &[CodeModule]) -> anyhow::Result<()> {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<PHASE id=\"Phase-1\" status=\"active\">\n  <NAME>MyGRACE Truth Restoration</NAME>\n  <GOAL>Keep code contracts, graph index, module shards, and verification shards synchronized</GOAL>\n  <MODULE_REFS>\n",
    );
    for module in modules {
        xml.push_str(&format!("    <MODULE_REF id=\"{}\" />\n", module.id));
    }
    xml.push_str("  </MODULE_REFS>\n</PHASE>\n");
    std::fs::write(layout.phases_dir().join("Phase-1.xml"), xml)?;
    Ok(())
}

fn write_module_shard(layout: &DocsLayout, module: &CodeModule) -> anyhow::Result<()> {
    let deps = if module.depends.is_empty() {
        String::new()
    } else {
        module.depends.join(",")
    };
    let links = if module.links.is_empty() {
        String::new()
    } else {
        module.links.join(",")
    };
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<MODULE id=\"{}\" type=\"{}\" status=\"active\">\n  <NAME>{}</NAME>\n  <PURPOSE>{}</PURPOSE>\n  <SCOPE>{}</SCOPE>\n  <FILES>\n    <FILE>{}</FILE>\n  </FILES>\n  <DEPENDS>{}</DEPENDS>\n  <LINKS>{}</LINKS>\n  <VERIFICATION_REF>V-{}</VERIFICATION_REF>\n</MODULE>\n",
        module.id,
        module_type(module),
        xml_escape(&module.id),
        xml_escape(&module.purpose),
        xml_escape(&module.scope),
        xml_escape(&module.source_path),
        xml_escape(&deps),
        xml_escape(&links),
        module.id
    );
    std::fs::write(layout.modules_dir().join(format!("{}.xml", module.id)), xml)?;
    Ok(())
}

fn write_verification_shard(layout: &DocsLayout, module: &CodeModule) -> anyhow::Result<()> {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VERIFICATION id=\"V-{}\" module=\"{}\" priority=\"normal\" status=\"active\">\n  <UNIT_TESTS>\n    <COMMAND>cargo test --all-targets</COMMAND>\n  </UNIT_TESTS>\n  <REQUIRED_LOG_MARKERS></REQUIRED_LOG_MARKERS>\n  <TRACE_ASSERTIONS></TRACE_ASSERTIONS>\n  <PHASE_GATE>Phase-1</PHASE_GATE>\n</VERIFICATION>\n",
        module.id, module.id
    );
    std::fs::write(
        layout
            .verification_dir()
            .join(format!("V-{}.xml", module.id)),
        xml,
    )?;
    Ok(())
}

fn archive_stale_shards(
    root: &Path,
    dir: &Path,
    live_stems: &BTreeSet<String>,
    group: &str,
) -> anyhow::Result<()> {
    let archive_dir = root.join("docs").join("archive").join(group);
    std::fs::create_dir_all(&archive_dir)?;
    for stem in list_xml_stems(dir) {
        if live_stems.contains(&stem) {
            continue;
        }
        let source = dir.join(format!("{}.xml", stem));
        let mut target = archive_dir.join(format!("{}.xml", stem));
        let mut i = 1usize;
        while target.exists() {
            target = archive_dir.join(format!("{}-{}.xml", stem, i));
            i += 1;
        }
        std::fs::rename(source, target)?;
    }
    Ok(())
}

fn module_type(module: &CodeModule) -> &'static str {
    let path = module.source_path.as_str();
    if path == "src/main.rs" || path == "src/cli.rs" || path.contains("plugin") {
        "ENTRY_POINT"
    } else if path.contains("storage") || path.contains("tracking") {
        "DATA_LAYER"
    } else if path.contains("dashboard") {
        "UI_COMPONENT"
    } else if path.contains("mcp")
        || path.contains("proxy")
        || path.contains("hooks")
        || path.contains("lsp")
    {
        "INTEGRATION"
    } else if path.contains("tests/") || path.ends_with("build.rs") || path.contains("utils") {
        "UTILITY"
    } else {
        "CORE_LOGIC"
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
