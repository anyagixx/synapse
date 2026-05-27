// MODULE_CONTRACT
// MODULE_ID: M-GRACE-INVENTORY-PLAN
// PURPOSE: MyGRACE plan artifact writer — preserves active phase metadata and completed phase history while refreshing generated shards
// SCOPE: plan-index.xml generation and Phase-1 shard synchronization/history preservation
// DEPENDS: M-GRACE-INVENTORY-TYPES, M-GRACE-LAYOUT
// LINKS: docs/plan-index.xml, docs/phases/

// START_MODULE_MAP
// write_phase_index — Writes plan-index.xml while preserving active phase files
// write_phase_one — Rewrites or preserves Phase-1 module refs while preserving phase status
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.0.0 — Preserve no-active-phase state when all phase shards are done]
// END_CHANGE_SUMMARY

use crate::grace::inventory_types::CodeModule;
use crate::grace::layout::DocsLayout;

// START_public_api

// START_CONTRACT_write_phase_index
// PURPOSE: Write plan-index.xml from existing phase shards without forcing Phase-1 active
// INPUTS: { layout: &DocsLayout — docs layout resolver }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes docs/plan-index.xml
// START_write_phase_index
pub(crate) fn write_phase_index(layout: &DocsLayout) -> anyhow::Result<()> {
    let phases = phase_ids(layout);
    let active = active_phase(layout, &phases);
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<PLAN_INDEX>\n  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY><ACTIVE_PHASE>{}</ACTIVE_PHASE></META>\n  <PHASES>\n",
        active
    );
    for phase in phases {
        let status = if phase == active {
            "active".to_string()
        } else {
            non_active_status(layout, &phase)
        };
        xml.push_str(&format!(
            "    <PHASE id=\"{}\" path=\"docs/phases/{}.xml\" status=\"{}\" />\n",
            phase, phase, status
        ));
    }
    xml.push_str("  </PHASES>\n</PLAN_INDEX>\n");
    std::fs::write(layout.plan_index_path(), xml)?;
    Ok(())
}
// END_write_phase_index

// START_CONTRACT_write_phase_one
// PURPOSE: Rewrite Phase-1 shard module refs unless completed Phase-1 history should be preserved
// INPUTS: { layout: &DocsLayout }, { modules: &[CodeModule] }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes docs/phases/Phase-1.xml
// START_write_phase_one
pub(crate) fn write_phase_one(layout: &DocsLayout, modules: &[CodeModule]) -> anyhow::Result<()> {
    let phase = "Phase-1";
    let status = phase_status(layout, phase).unwrap_or_else(|| "active".into());
    let active = active_phase(layout, &phase_ids(layout));
    let existing_refs = phase_module_refs(layout, phase);
    let module_refs: Vec<String> =
        if status == "done" && active != phase && !existing_refs.is_empty() {
            existing_refs
        } else {
            modules.iter().map(|module| module.id.clone()).collect()
        };
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<PHASE id=\"{}\" status=\"{}\">\n  <NAME>MyGRACE Truth Restoration</NAME>\n  <GOAL>Keep code contracts, graph index, module shards, and verification shards synchronized</GOAL>\n  <MODULE_REFS>\n",
        phase, status
    );
    for module_id in module_refs {
        xml.push_str(&format!("    <MODULE_REF id=\"{}\" />\n", module_id));
    }
    xml.push_str("  </MODULE_REFS>\n</PHASE>\n");
    std::fs::write(layout.phases_dir().join("Phase-1.xml"), xml)?;
    Ok(())
}
// END_write_phase_one

// END_public_api

fn phase_ids(layout: &DocsLayout) -> Vec<String> {
    let mut ids: Vec<String> = std::fs::read_dir(layout.phases_dir())
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
    if !ids.iter().any(|id| id == "Phase-1") {
        ids.push("Phase-1".into());
    }
    ids.sort();
    ids
}

// START_CONTRACT_phase_module_refs
// PURPOSE: Parse module references from a phase shard while preserving their existing order
// INPUTS: { layout: &DocsLayout }, { phase: &str — phase id }
// OUTPUTS: { Vec<String> — module ids referenced by the phase shard }
// START_phase_module_refs
fn phase_module_refs(layout: &DocsLayout, phase: &str) -> Vec<String> {
    let path = layout.phases_dir().join(format!("{}.xml", phase));
    let content = std::fs::read_to_string(path).unwrap_or_default();
    content
        .lines()
        .filter(|line| line.contains("<MODULE_REF ") && line.contains("id="))
        .filter_map(|line| attr(line, "id"))
        .collect()
}
// END_phase_module_refs

fn active_phase(layout: &DocsLayout, phases: &[String]) -> String {
    let existing = std::fs::read_to_string(layout.plan_index_path())
        .ok()
        .and_then(|content| tag_value(&content, "ACTIVE_PHASE"));
    if existing.as_deref() == Some("none") {
        return "none".into();
    }
    if let Some(phase) = existing.filter(|phase| phases.iter().any(|candidate| candidate == phase))
    {
        if phase_status(layout, &phase).as_deref() != Some("done") {
            return phase;
        }
    }
    phases
        .iter()
        .find(|phase| {
            matches!(
                phase_status(layout, phase).as_deref(),
                Some("active") | Some("planned")
            )
        })
        .cloned()
        .unwrap_or_else(|| "none".into())
}

fn phase_status(layout: &DocsLayout, phase: &str) -> Option<String> {
    let path = layout.phases_dir().join(format!("{}.xml", phase));
    let content = std::fs::read_to_string(path).ok()?;
    attr(&content, "status")
}

fn non_active_status(layout: &DocsLayout, phase: &str) -> String {
    match phase_status(layout, phase).as_deref() {
        Some("planned") => "planned".into(),
        Some("done") => "done".into(),
        _ => "done".into(),
    }
}

fn tag_value(content: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = content.find(&open)? + open.len();
    let end = content[start..].find(&close)? + start;
    Some(content[start..end].to_string())
}

fn attr(content: &str, name: &str) -> Option<String> {
    let needle = format!(r#"{}=""#, name);
    let start = content.find(&needle)? + needle.len();
    let end = content[start..].find('"')? + start;
    Some(content[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_write_phase_one_preserves_done_history_when_phase_three_active
    // PURPOSE: Verify refresh does not append new modules to completed Phase-1 while Phase-3 is active
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temporary MyGRACE plan shards
    // START_test_write_phase_one_preserves_done_history_when_phase_three_active
    #[test]
    fn test_write_phase_one_preserves_done_history_when_phase_three_active() {
        let dir = tempfile::tempdir().unwrap();
        let layout = DocsLayout::new(dir.path());
        std::fs::create_dir_all(layout.phases_dir()).unwrap();
        std::fs::write(
            layout.plan_index_path(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<PLAN_INDEX>\n  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY><ACTIVE_PHASE>Phase-3</ACTIVE_PHASE></META>\n  <PHASES>\n    <PHASE id=\"Phase-1\" path=\"docs/phases/Phase-1.xml\" status=\"done\" />\n    <PHASE id=\"Phase-3\" path=\"docs/phases/Phase-3.xml\" status=\"active\" />\n  </PHASES>\n</PLAN_INDEX>\n",
        )
        .unwrap();
        std::fs::write(
            layout.phases_dir().join("Phase-1.xml"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<PHASE id=\"Phase-1\" status=\"done\">\n  <NAME>MyGRACE Truth Restoration</NAME>\n  <GOAL>Existing history</GOAL>\n  <MODULE_REFS>\n    <MODULE_REF id=\"M-OLD\" />\n  </MODULE_REFS>\n</PHASE>\n",
        )
        .unwrap();
        std::fs::write(
            layout.phases_dir().join("Phase-3.xml"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<PHASE id=\"Phase-3\" status=\"active\"></PHASE>\n",
        )
        .unwrap();

        let modules = vec![
            CodeModule {
                id: "M-OLD".into(),
                source_path: "src/old.rs".into(),
                source_paths: vec!["src/old.rs".into()],
                purpose: "Old module".into(),
                scope: "Old scope".into(),
                depends: Vec::new(),
                links: Vec::new(),
                contract_errors: Vec::new(),
            },
            CodeModule {
                id: "M-NEW".into(),
                source_path: "src/new.rs".into(),
                source_paths: vec!["src/new.rs".into()],
                purpose: "New module".into(),
                scope: "New scope".into(),
                depends: Vec::new(),
                links: Vec::new(),
                contract_errors: Vec::new(),
            },
        ];

        write_phase_one(&layout, &modules).unwrap();

        let phase_one = std::fs::read_to_string(layout.phases_dir().join("Phase-1.xml")).unwrap();
        assert!(phase_one.contains("<MODULE_REF id=\"M-OLD\" />"));
        assert!(!phase_one.contains("<MODULE_REF id=\"M-NEW\" />"));
    }
    // END_test_write_phase_one_preserves_done_history_when_phase_three_active

    // START_CONTRACT_test_write_phase_index_preserves_no_active_phase
    // PURPOSE: Verify refresh keeps ACTIVE_PHASE none when all phase shards are done.
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temporary MyGRACE plan shards
    // START_test_write_phase_index_preserves_no_active_phase
    #[test]
    fn test_write_phase_index_preserves_no_active_phase() {
        let dir = tempfile::tempdir().unwrap();
        let layout = DocsLayout::new(dir.path());
        std::fs::create_dir_all(layout.phases_dir()).unwrap();
        std::fs::write(
            layout.plan_index_path(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<PLAN_INDEX>\n  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY><ACTIVE_PHASE>none</ACTIVE_PHASE></META>\n  <PHASES>\n    <PHASE id=\"Phase-11\" path=\"docs/phases/Phase-11.xml\" status=\"done\" />\n  </PHASES>\n</PLAN_INDEX>\n",
        )
        .unwrap();
        std::fs::write(
            layout.phases_dir().join("Phase-11.xml"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<PHASE id=\"Phase-11\" status=\"done\"></PHASE>\n",
        )
        .unwrap();

        write_phase_index(&layout).unwrap();

        let plan_index = std::fs::read_to_string(layout.plan_index_path()).unwrap();
        assert!(plan_index.contains("<ACTIVE_PHASE>none</ACTIVE_PHASE>"));
        assert!(plan_index.contains("status=\"done\""));
        assert!(!plan_index.contains("status=\"active\""));
    }
    // END_test_write_phase_index_preserves_no_active_phase
}
