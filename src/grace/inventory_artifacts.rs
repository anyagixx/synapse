// MODULE_CONTRACT
// MODULE_ID: M-GRACE-INVENTORY-ARTIFACTS
// PURPOSE: MyGRACE artifact IO — parses indexes, computes drift, and rewrites sharded XML artifacts
// SCOPE: graph/verification index parsing, shard listing, drift calculation, XML artifact synchronization
// DEPENDS: M-GRACE-INVENTORY-TYPES, M-GRACE-LAYOUT
// LINKS: docs/graph-index.xml, docs/verification-index.xml, docs/modules/, docs/verification/

// START_MODULE_MAP
// parse_graph_index — Reads graph index entries
// drift_from_inventory — Compares code contracts against sharded artifacts
// sync_inventory_artifacts — Rewrites canonical MyGRACE indexes and shards
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Extracted artifact IO from M-GRACE-INVENTORY]
// END_CHANGE_SUMMARY

use crate::grace::inventory_types::{
    ArtifactDrift, ArtifactInventory, CodeModule, GraphEntry, VerificationEntry,
};
use crate::grace::layout::DocsLayout;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// START_public_api
// START_CONTRACT_drift_from_inventory
// PURPOSE: Detect canonical MyGRACE drift across code contracts, indexes, and shards
// INPUTS: { root: &Path — project root }, { inventory: &ArtifactInventory — collected facts }
// OUTPUTS: { ArtifactDrift }
// START_drift_from_inventory
pub(crate) fn drift_from_inventory(root: &Path, inventory: &ArtifactInventory) -> ArtifactDrift {
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

    check_index_paths(root, inventory, &mut drift);
    check_orphan_shards(inventory, &graph_set, verification_ids, &mut drift);
    check_shard_content(&layout, inventory, &mut drift);

    drift.suggested_actions = suggested_actions(&drift);
    drift
}
// END_drift_from_inventory
// START_CONTRACT_parse_graph_index
// PURPOSE: Parse module entries from docs/graph-index.xml
// INPUTS: { path: &Path — graph index path }
// OUTPUTS: { Vec<GraphEntry> }
// START_parse_graph_index
pub(crate) fn parse_graph_index(path: &Path) -> Vec<GraphEntry> {
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
// END_parse_graph_index
// START_CONTRACT_parse_verification_index
// PURPOSE: Parse verification entries from docs/verification-index.xml
// INPUTS: { path: &Path — verification index path }
// OUTPUTS: { Vec<VerificationEntry> }
// START_parse_verification_index
pub(crate) fn parse_verification_index(path: &Path) -> Vec<VerificationEntry> {
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
// END_parse_verification_index
// START_CONTRACT_list_xml_stems
// PURPOSE: Return sorted XML file stems from a shard directory
// INPUTS: { dir: &Path — shard directory }
// OUTPUTS: { Vec<String> }
// START_list_xml_stems
pub(crate) fn list_xml_stems(dir: &Path) -> Vec<String> {
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
// END_list_xml_stems
// START_CONTRACT_sync_inventory_artifacts
// PURPOSE: Rewrite graph, plan, module, verification, and Phase-1 shards from code modules
// INPUTS: { root: &Path — project root }, { layout: &DocsLayout }, { modules: &[CodeModule] }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes docs indexes and shard files, archives stale generated shards
// START_sync_inventory_artifacts
pub(crate) fn sync_inventory_artifacts(
    root: &Path,
    layout: &DocsLayout,
    modules: &[CodeModule],
) -> anyhow::Result<()> {
    std::fs::create_dir_all(layout.modules_dir())?;
    std::fs::create_dir_all(layout.phases_dir())?;
    std::fs::create_dir_all(layout.verification_dir())?;
    std::fs::create_dir_all(layout.docs_dir().join("archive"))?;

    let ids: BTreeSet<String> = modules.iter().map(|m| m.id.clone()).collect();
    archive_stale_shards(root, &layout.modules_dir(), &ids, "modules")?;
    let verification_ids: BTreeSet<String> = ids.iter().map(|id| format!("V-{}", id)).collect();
    archive_stale_shards(
        root,
        &layout.verification_dir(),
        &verification_ids,
        "verification",
    )?;

    write_graph_index(layout, modules)?;
    write_verification_index(layout, modules)?;
    write_phase_index(layout)?;
    write_phase_one(layout, modules)?;

    for module in modules {
        write_module_shard(layout, module)?;
        write_verification_shard(layout, module)?;
    }

    Ok(())
}
// END_sync_inventory_artifacts

// END_public_api
fn check_index_paths(root: &Path, inventory: &ArtifactInventory, drift: &mut ArtifactDrift) {
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
}

fn check_orphan_shards(
    inventory: &ArtifactInventory,
    graph_set: &BTreeSet<String>,
    verification_ids: Vec<String>,
    drift: &mut ArtifactDrift,
) {
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
}

fn check_shard_content(
    layout: &DocsLayout,
    inventory: &ArtifactInventory,
    drift: &mut ArtifactDrift,
) {
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
}

fn attr(line: &str, name: &str) -> Option<String> {
    let needle = format!(r#"{}=""#, name);
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
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
