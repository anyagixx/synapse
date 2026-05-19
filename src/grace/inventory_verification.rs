// MODULE_CONTRACT
// MODULE_ID: M-GRACE-INVENTORY-VERIFICATION
// PURPOSE: MyGRACE verification artifact writer — rewrites verification index and shards while preserving custom shard content
// SCOPE: sync_verification_artifacts, write_verification_index, write_verification_shard, preserve_existing_verification_body
// DEPENDS: M-GRACE-INVENTORY-TYPES, M-GRACE-LAYOUT
// LINKS: docs/verification-index.xml, docs/verification/

// START_MODULE_MAP
// sync_verification_artifacts — Rewrites verification index and shards from code modules
// preserve_existing_verification_body — Keeps custom verification commands, markers, assertions, and phase gates
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Extracted verification shard preservation from inventory artifacts]
// END_CHANGE_SUMMARY

use crate::grace::inventory_types::{CodeModule, VerificationEntry};
use crate::grace::layout::DocsLayout;
use std::collections::BTreeMap;
use std::path::Path;

// START_public_api

// START_CONTRACT_sync_verification_artifacts
// PURPOSE: Rewrite verification index and module shards while preserving existing verification detail
// INPUTS: { layout: &DocsLayout — docs path helper }, { modules: &[CodeModule] — canonical code modules }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes docs/verification-index.xml and docs/verification/V-*.xml
// START_sync_verification_artifacts
pub(crate) fn sync_verification_artifacts(
    layout: &DocsLayout,
    modules: &[CodeModule],
) -> anyhow::Result<()> {
    let existing_entries: BTreeMap<String, VerificationEntry> =
        parse_verification_index(&layout.verification_index_path())
            .into_iter()
            .map(|entry| (entry.module.clone(), entry))
            .collect();

    write_verification_index(layout, modules, &existing_entries)?;
    for module in modules {
        write_verification_shard(layout, module, existing_entries.get(&module.id))?;
    }
    Ok(())
}
// END_sync_verification_artifacts

// END_public_api

// START_CONTRACT_write_verification_index
// PURPOSE: Write canonical verification index entries while preserving existing priority and status
// INPUTS: { layout: &DocsLayout }, { modules: &[CodeModule] }, { existing_entries: &BTreeMap<String, VerificationEntry> }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes docs/verification-index.xml
// START_write_verification_index
fn write_verification_index(
    layout: &DocsLayout,
    modules: &[CodeModule],
    existing_entries: &BTreeMap<String, VerificationEntry>,
) -> anyhow::Result<()> {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VERIFICATION_INDEX>\n  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY></META>\n  <VERIFICATIONS>\n",
    );
    for module in modules {
        let existing = existing_entries.get(&module.id);
        let priority = existing
            .map(|entry| entry.priority.as_str())
            .unwrap_or("normal");
        let status = existing
            .map(|entry| entry.status.as_str())
            .unwrap_or("active");
        xml.push_str(&format!(
            "    <VERIFICATION id=\"V-{}\" module=\"{}\" path=\"{}\" priority=\"{}\" status=\"{}\" />\n",
            module.id,
            module.id,
            verification_shard_path(&module.id),
            xml_escape(priority),
            xml_escape(status)
        ));
    }
    xml.push_str("  </VERIFICATIONS>\n</VERIFICATION_INDEX>\n");
    std::fs::write(layout.verification_index_path(), xml)?;
    Ok(())
}
// END_write_verification_index

// START_CONTRACT_write_verification_shard
// PURPOSE: Write a canonical verification shard root while preserving body content for matching existing shards
// INPUTS: { layout: &DocsLayout }, { module: &CodeModule }, { existing_entry: Option<&VerificationEntry> }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes docs/verification/V-MODULE.xml
// START_write_verification_shard
fn write_verification_shard(
    layout: &DocsLayout,
    module: &CodeModule,
    existing_entry: Option<&VerificationEntry>,
) -> anyhow::Result<()> {
    let path = layout
        .verification_dir()
        .join(format!("V-{}.xml", module.id));
    let body = preserve_existing_verification_body(&path, &module.id)
        .unwrap_or_else(default_verification_body);
    let priority = existing_entry
        .map(|entry| entry.priority.as_str())
        .unwrap_or("normal");
    let status = existing_entry
        .map(|entry| entry.status.as_str())
        .unwrap_or("active");
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VERIFICATION id=\"V-{}\" module=\"{}\" priority=\"{}\" status=\"{}\">{}</VERIFICATION>\n",
        module.id,
        module.id,
        xml_escape(priority),
        xml_escape(status),
        body
    );
    std::fs::write(path, xml)?;
    Ok(())
}
// END_write_verification_shard

// START_CONTRACT_preserve_existing_verification_body
// PURPOSE: Preserve a matching verification shard body while allowing refresh to canonicalize root metadata
// INPUTS: { path: &Path — existing verification shard path }, { module_id: &str — module id expected in the shard root }
// OUTPUTS: { Option<String> — normalized shard body including surrounding newlines }
// START_preserve_existing_verification_body
fn preserve_existing_verification_body(path: &Path, module_id: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let open_start = content.find("<VERIFICATION ")?;
    let open_end = content[open_start..].find('>')? + open_start + 1;
    let open_tag = &content[open_start..open_end];
    if !open_tag.contains(&format!(r#"id="V-{}""#, module_id))
        || !open_tag.contains(&format!(r#"module="{}""#, module_id))
    {
        return None;
    }
    let close_start = content.rfind("</VERIFICATION>")?;
    if close_start <= open_end {
        return None;
    }
    let body = content[open_end..close_start].trim_matches('\n');
    if body.trim().is_empty() {
        None
    } else {
        Some(format!("\n{}\n", body))
    }
}
// END_preserve_existing_verification_body

// START_CONTRACT_default_verification_body
// PURPOSE: Return the default generated verification shard body for modules without existing verification detail
// OUTPUTS: { String — default verification XML body with all-target tests and Phase-1 gate }
// START_default_verification_body
fn default_verification_body() -> String {
    "\n  <UNIT_TESTS>\n    <COMMAND>cargo test --all-targets</COMMAND>\n  </UNIT_TESTS>\n  <REQUIRED_LOG_MARKERS></REQUIRED_LOG_MARKERS>\n  <TRACE_ASSERTIONS></TRACE_ASSERTIONS>\n  <PHASE_GATE>Phase-1</PHASE_GATE>\n".to_string()
}
// END_default_verification_body

// START_CONTRACT_parse_verification_index
// PURPOSE: Parse existing verification index entries needed to preserve priority and status
// INPUTS: { path: &Path — verification index path }
// OUTPUTS: { Vec<VerificationEntry> }
// START_parse_verification_index
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
// END_parse_verification_index

fn attr(line: &str, name: &str) -> Option<String> {
    let needle = format!(r#"{}=""#, name);
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn verification_shard_path(id: &str) -> String {
    format!("docs/verification/V-{}.xml", id)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_sync_preserves_custom_verification_shard_body
    // PURPOSE: Verify refresh sync keeps custom verification commands, markers, assertions, and phase gate
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temporary MyGRACE docs
    // START_test_sync_preserves_custom_verification_shard_body
    #[test]
    fn test_sync_preserves_custom_verification_shard_body() {
        let dir = tempfile::tempdir().unwrap();
        let layout = DocsLayout::new(dir.path());
        std::fs::create_dir_all(layout.verification_dir()).unwrap();
        std::fs::write(
            layout.verification_index_path(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VERIFICATION_INDEX>\n  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY></META>\n  <VERIFICATIONS>\n    <VERIFICATION id=\"V-M-TEST\" module=\"M-TEST\" path=\"docs/verification/V-M-TEST.xml\" priority=\"critical\" status=\"wip\" />\n  </VERIFICATIONS>\n</VERIFICATION_INDEX>\n",
        )
        .unwrap();
        std::fs::write(
            layout.verification_dir().join("V-M-TEST.xml"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VERIFICATION id=\"V-M-TEST\" module=\"M-TEST\" priority=\"critical\" status=\"wip\">\n  <UNIT_TESTS>\n    <COMMAND>cargo test custom_refresh_preserve --lib</COMMAND>\n  </UNIT_TESTS>\n  <REQUIRED_LOG_MARKERS>\n    <MARKER>[Refresh][sync][PRESERVE]</MARKER>\n  </REQUIRED_LOG_MARKERS>\n  <TRACE_ASSERTIONS>\n    <ASSERTION>Custom verification detail survives refresh.</ASSERTION>\n  </TRACE_ASSERTIONS>\n  <PHASE_GATE>Phase-3</PHASE_GATE>\n</VERIFICATION>\n",
        )
        .unwrap();

        let module = CodeModule {
            id: "M-TEST".into(),
            source_path: "src/main.rs".into(),
            purpose: "Test module".into(),
            scope: "Refresh preservation".into(),
            depends: Vec::new(),
            links: Vec::new(),
            contract_errors: Vec::new(),
        };

        sync_verification_artifacts(&layout, &[module]).unwrap();

        let shard =
            std::fs::read_to_string(layout.verification_dir().join("V-M-TEST.xml")).unwrap();
        assert!(shard.contains("priority=\"critical\" status=\"wip\""));
        assert!(shard.contains("cargo test custom_refresh_preserve --lib"));
        assert!(shard.contains("[Refresh][sync][PRESERVE]"));
        assert!(shard.contains("Custom verification detail survives refresh."));
        assert!(shard.contains("<PHASE_GATE>Phase-3</PHASE_GATE>"));

        let index = std::fs::read_to_string(layout.verification_index_path()).unwrap();
        assert!(index.contains("priority=\"critical\" status=\"wip\""));
    }
    // END_test_sync_preserves_custom_verification_shard_body
}
