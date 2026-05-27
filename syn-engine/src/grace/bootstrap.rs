// MODULE_CONTRACT
// MODULE_ID: M-GRACE-BOOTSTRAP
// PURPOSE: Existing-repository bootstrap helpers — create canonical MyGRACE artifacts from current source files
// SCOPE: module scope resolution, init --from-existing artifact generation, path-derived module IDs
// DEPENDS: M-GRACE-CONTRACT, M-GRACE-LAYOUT, M-INDEXER-WALKER
// LINKS: docs/graph-index.xml, docs/modules/, docs/verification/

// START_MODULE_MAP
// resolve_module_scope — Resolve a module id to source and artifact paths
// bootstrap_existing_repo — Generate indexes and shards for an existing source tree
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.5.0 — Extracted existing-repo bootstrap from CLI]
// END_CHANGE_SUMMARY

use crate::grace::contract::ContractValidator;
use crate::grace::layout::DocsLayout;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// START_public_api

// START_CONTRACT_resolve_module_scope
// PURPOSE: Resolve source and artifact paths that belong to a module id
// INPUTS: { root: &Path — project root }, { module: &str — module id }
// OUTPUTS: { HashSet<PathBuf> — matching source/index/shard paths }
// START_resolve_module_scope
pub fn resolve_module_scope(root: &Path, module: &str) -> HashSet<PathBuf> {
    let layout = DocsLayout::new(root);
    let mut paths = HashSet::new();
    let graph = std::fs::read_to_string(layout.graph_index_path()).unwrap_or_default();
    let verification =
        std::fs::read_to_string(layout.verification_index_path()).unwrap_or_default();
    let contract_report = ContractValidator::validate_project(root).ok();
    let module_files: Vec<String> = contract_report
        .as_ref()
        .map(|report| {
            report
                .contracts
                .iter()
                .filter(|c| c.module_id.as_deref() == Some(module))
                .map(|c| c.file_path.clone())
                .collect()
        })
        .unwrap_or_default();

    if let Ok(module_re) = regex::Regex::new(&format!(
        r#"<MODULE id=\"{}\" path=\"([^\"]+)\""#,
        regex::escape(module)
    )) {
        for cap in module_re.captures_iter(&graph) {
            paths.insert(root.join(&cap[1]));
        }
    }
    if let Ok(verification_re) =
        regex::Regex::new(&format!(r#"module=\"{}\""#, regex::escape(module)))
    {
        if verification_re.is_match(&verification) {
            paths.insert(layout.graph_index_path());
            paths.insert(layout.plan_index_path());
            paths.insert(layout.verification_index_path());
            paths.insert(layout.modules_dir().join(format!("{}.xml", module)));
            paths.insert(layout.verification_dir().join(format!("V-{}.xml", module)));
        }
    }
    for path in module_files {
        paths.insert(PathBuf::from(path));
    }
    paths
}
// END_resolve_module_scope

// START_CONTRACT_bootstrap_existing_repo
// PURPOSE: Generate canonical sharded MyGRACE artifacts for an existing source tree
// INPUTS: { root: &Path — project root }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes docs/graph-index.xml, docs/plan-index.xml, module shards, and verification shards
// START_bootstrap_existing_repo
pub fn bootstrap_existing_repo(root: &Path) -> anyhow::Result<()> {
    let layout = DocsLayout::new(root);
    let walker = crate::indexer::walker::Walker::new(root);
    let files = walker.walk();
    let mut module_lines = Vec::new();

    for file in files {
        if !file.path.ends_with(".rs") && !file.path.ends_with(".ts") && !file.path.ends_with(".py")
        {
            continue;
        }
        let full = root.join(&file.path);
        let content = std::fs::read_to_string(&full).unwrap_or_default();
        let contract = ContractValidator::scan_file(&full, &content);
        let module_id = contract
            .module_id
            .clone()
            .unwrap_or_else(|| module_id_from_path(&file.path));
        let purpose = contract
            .purpose
            .clone()
            .unwrap_or_else(|| format!("Source module for {}", file.path));
        module_lines.push((module_id, file.path, purpose));
    }
    module_lines.sort_by(|a, b| a.0.cmp(&b.0));
    module_lines.dedup_by(|a, b| a.0 == b.0);

    if module_lines.is_empty() {
        return Ok(());
    }

    let mut graph = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<GRAPH_INDEX>\n  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY></META>\n  <MODULES>\n");
    for (mid, _, _) in &module_lines {
        graph.push_str(&format!(
            "    <MODULE id=\"{}\" path=\"docs/modules/{}.xml\" status=\"planned\" />\n",
            mid, mid
        ));
    }
    graph.push_str("  </MODULES>\n  <RELATIONSHIPS></RELATIONSHIPS>\n</GRAPH_INDEX>\n");
    std::fs::write(layout.graph_index_path(), graph)?;

    let plan = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<PLAN_INDEX>\n  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY><ACTIVE_PHASE>Phase-1</ACTIVE_PHASE></META>\n  <PHASES>\n    <PHASE id=\"Phase-1\" path=\"docs/phases/Phase-1.xml\" status=\"active\" />\n  </PHASES>\n</PLAN_INDEX>\n");
    std::fs::write(layout.plan_index_path(), plan)?;

    let mut verification = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VERIFICATION_INDEX>\n  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY></META>\n  <VERIFICATIONS>\n");
    for (mid, _, _) in &module_lines {
        verification.push_str(&format!("    <VERIFICATION id=\"V-{}\" module=\"{}\" path=\"docs/verification/V-{}.xml\" priority=\"normal\" status=\"planned\" />\n", mid, mid, mid));
    }
    verification.push_str("  </VERIFICATIONS>\n</VERIFICATION_INDEX>\n");
    std::fs::write(layout.verification_index_path(), verification)?;

    for (mid, path, purpose) in &module_lines {
        let module_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<MODULE id=\"{}\" type=\"CORE_LOGIC\" status=\"planned\">\n  <NAME>{}</NAME>\n  <PURPOSE>{}</PURPOSE>\n  <FILES>\n    <FILE>{}</FILE>\n  </FILES>\n  <VERIFICATION_REF>V-{}</VERIFICATION_REF>\n</MODULE>\n",
            mid, mid, purpose, path, mid
        );
        std::fs::write(
            layout.modules_dir().join(format!("{}.xml", mid)),
            module_xml,
        )?;

        let verification_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VERIFICATION id=\"V-{}\" module=\"{}\" priority=\"normal\" status=\"planned\">\n  <UNIT_TESTS></UNIT_TESTS>\n  <REQUIRED_LOG_MARKERS></REQUIRED_LOG_MARKERS>\n  <TRACE_ASSERTIONS></TRACE_ASSERTIONS>\n  <PHASE_GATE>Phase-1</PHASE_GATE>\n</VERIFICATION>\n",
            mid, mid
        );
        std::fs::write(
            layout.verification_dir().join(format!("V-{}.xml", mid)),
            verification_xml,
        )?;
    }

    Ok(())
}
// END_bootstrap_existing_repo

// END_public_api

fn module_id_from_path(path: &str) -> String {
    let mut parts: Vec<String> = path
        .trim_end_matches(".rs")
        .trim_end_matches(".ts")
        .trim_end_matches(".py")
        .split('/')
        .filter(|part| *part != "src" && *part != "tests")
        .map(|part| part.to_uppercase().replace(['-', '.'], "_"))
        .collect();
    if parts.last().map(|part| part.as_str()) == Some("MOD") && parts.len() > 1 {
        parts.pop();
    }
    if parts.is_empty() {
        parts.push("MODULE".into());
    }
    format!("M-{}", parts.join("-"))
}
