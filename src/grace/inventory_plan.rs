// MODULE_CONTRACT
// MODULE_ID: M-GRACE-INVENTORY-PLAN
// PURPOSE: MyGRACE plan artifact writer — preserves active phase metadata while refreshing generated shards
// SCOPE: plan-index.xml generation and Phase-1 shard synchronization
// DEPENDS: M-GRACE-INVENTORY-TYPES, M-GRACE-LAYOUT
// LINKS: docs/plan-index.xml, docs/phases/

// START_MODULE_MAP
// write_phase_index — Writes plan-index.xml while preserving active phase files
// write_phase_one — Rewrites Phase-1 module refs while preserving phase status
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.8.0 — Extracted plan artifact writing and preserved Phase-2 active status]
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
// PURPOSE: Rewrite Phase-1 shard module refs while preserving its status when later phases are active
// INPUTS: { layout: &DocsLayout }, { modules: &[CodeModule] }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes docs/phases/Phase-1.xml
// START_write_phase_one
pub(crate) fn write_phase_one(layout: &DocsLayout, modules: &[CodeModule]) -> anyhow::Result<()> {
    let phase = "Phase-1";
    let status = phase_status(layout, phase).unwrap_or_else(|| "active".into());
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<PHASE id=\"{}\" status=\"{}\">\n  <NAME>MyGRACE Truth Restoration</NAME>\n  <GOAL>Keep code contracts, graph index, module shards, and verification shards synchronized</GOAL>\n  <MODULE_REFS>\n",
        phase, status
    );
    for module in modules {
        xml.push_str(&format!("    <MODULE_REF id=\"{}\" />\n", module.id));
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

fn active_phase(layout: &DocsLayout, phases: &[String]) -> String {
    let existing = std::fs::read_to_string(layout.plan_index_path())
        .ok()
        .and_then(|content| tag_value(&content, "ACTIVE_PHASE"));
    existing
        .filter(|phase| phases.iter().any(|candidate| candidate == phase))
        .unwrap_or_else(|| {
            if phases.iter().any(|phase| phase == "Phase-2") {
                "Phase-2".into()
            } else {
                "Phase-1".into()
            }
        })
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
