// MODULE_CONTRACT
// MODULE_ID: M-GRACE-CASCADE-CHANGE
// PURPOSE: Cascade changelog model and XML writer for recorded GRACE cascade executions
// SCOPE: CascadeChangeLog, trigger/change entries, XML rendering, changelog persistence, and history scanning
// DEPENDS: M-GRACE-LAYOUT
// LINKS:
//   -> V-M-GRACE-CASCADE-CHANGE (verified_by) - changelog rendering and history scan tests

// START_MODULE_MAP
// CascadeTrigger - Trigger artifact and author metadata for one cascade
// CascadeChange - One applied or proposed downstream cascade change
// CascadeChangeLog - Complete cascade execution changelog
// CascadeHistoryReport - Compact status/review history summary
// write_changelog - Persist a changelog under docs/cascade/changelogs
// list_changelog_entries - Read existing changelog metadata
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added cascade changelog persistence]
// END_CHANGE_SUMMARY

use crate::grace::layout::DocsLayout;
use std::path::{Path, PathBuf};

// START_public_api

// START_CascadeTrigger
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CascadeTrigger {
    pub artifact: String,
    pub change: String,
    pub author: String,
}
// END_CascadeTrigger

// START_CascadeChange
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CascadeChange {
    pub artifact: String,
    pub change_type: String,
    pub action: String,
    pub before: String,
    pub after: String,
    pub verification_status: String,
}
// END_CascadeChange

// START_CascadeChangeLog
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CascadeChangeLog {
    pub id: String,
    pub timestamp: String,
    pub trigger: CascadeTrigger,
    pub changes: Vec<CascadeChange>,
}
// END_CascadeChangeLog

// START_CascadeHistoryEntry
#[derive(Debug, Clone, serde::Serialize)]
pub struct CascadeHistoryEntry {
    pub id: String,
    pub timestamp: String,
    pub artifact: String,
    pub path: String,
}
// END_CascadeHistoryEntry

// START_CascadeHistoryReport
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct CascadeHistoryReport {
    pub changelog_count: usize,
    pub latest: Option<CascadeHistoryEntry>,
    pub entries: Vec<CascadeHistoryEntry>,
}
// END_CascadeHistoryReport

// START_CONTRACT_new_change_log
// PURPOSE: Build a timestamped cascade changelog from trigger metadata and downstream changes
// INPUTS: { id: &str }, { artifact: &str }, { change: &str }, { author: &str }, { changes: Vec<CascadeChange> }
// OUTPUTS: { CascadeChangeLog }
// START_new_change_log
pub fn new_change_log(
    id: &str,
    artifact: &str,
    change: &str,
    author: &str,
    changes: Vec<CascadeChange>,
) -> CascadeChangeLog {
    CascadeChangeLog {
        id: id.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        trigger: CascadeTrigger {
            artifact: artifact.to_string(),
            change: change.to_string(),
            author: author.to_string(),
        },
        changes,
    }
}
// END_new_change_log

// START_CONTRACT_write_changelog
// PURPOSE: Persist one cascade changelog as XML under docs/cascade/changelogs
// INPUTS: { root: &Path }, { log: &CascadeChangeLog }
// OUTPUTS: { anyhow::Result<PathBuf> }
// SIDE_EFFECTS: creates docs/cascade/changelogs and writes a changelog XML file
// START_write_changelog
pub fn write_changelog(root: &Path, log: &CascadeChangeLog) -> anyhow::Result<PathBuf> {
    let layout = DocsLayout::new(root);
    std::fs::create_dir_all(layout.cascade_changelogs_dir())?;
    let path = layout
        .cascade_changelogs_dir()
        .join(format!("{}.xml", safe_id(&log.id)));
    std::fs::write(&path, render_changelog_xml(log))?;
    Ok(path)
}
// END_write_changelog

// START_CONTRACT_list_changelog_entries
// PURPOSE: Scan existing cascade changelogs and return compact history metadata
// INPUTS: { root: &Path }
// OUTPUTS: { anyhow::Result<CascadeHistoryReport> }
// START_list_changelog_entries
pub fn list_changelog_entries(root: &Path) -> anyhow::Result<CascadeHistoryReport> {
    let layout = DocsLayout::new(root);
    let mut entries = Vec::new();
    let Ok(read_dir) = std::fs::read_dir(layout.cascade_changelogs_dir()) else {
        return Ok(CascadeHistoryReport::default());
    };
    for entry in read_dir.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("xml") {
            continue;
        }
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        entries.push(CascadeHistoryEntry {
            id: attr(&content, "id").unwrap_or_else(|| {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".into())
            }),
            timestamp: attr(&content, "timestamp").unwrap_or_default(),
            artifact: tag_value(&content, "Artifact").unwrap_or_default(),
            path: path.display().to_string(),
        });
    }
    entries.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
    let latest = entries.last().cloned();
    Ok(CascadeHistoryReport {
        changelog_count: entries.len(),
        latest,
        entries,
    })
}
// END_list_changelog_entries

// START_CONTRACT_render_changelog_xml
// PURPOSE: Render one cascade changelog as deterministic XML
// INPUTS: { log: &CascadeChangeLog }
// OUTPUTS: { String }
// START_render_changelog_xml
pub fn render_changelog_xml(log: &CascadeChangeLog) -> String {
    let mut output = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<CascadeChange id=\"{}\" timestamp=\"{}\">\n",
        xml_attr(&log.id),
        xml_attr(&log.timestamp)
    );
    output.push_str("  <Trigger>\n");
    output.push_str(&format!(
        "    <Artifact>{}</Artifact>\n",
        xml_text(&log.trigger.artifact)
    ));
    output.push_str(&format!(
        "    <Change>{}</Change>\n",
        xml_text(&log.trigger.change)
    ));
    output.push_str(&format!(
        "    <Author>{}</Author>\n",
        xml_text(&log.trigger.author)
    ));
    output.push_str("  </Trigger>\n  <Changes>\n");
    for change in &log.changes {
        output.push_str(&format!(
            "    <Change artifact=\"{}\" type=\"{}\" action=\"{}\">\n",
            xml_attr(&change.artifact),
            xml_attr(&change.change_type),
            xml_attr(&change.action)
        ));
        output.push_str(&format!(
            "      <Before>{}</Before>\n",
            xml_text(&change.before)
        ));
        output.push_str(&format!(
            "      <After>{}</After>\n",
            xml_text(&change.after)
        ));
        output.push_str(&format!(
            "      <VerificationStatus>{}</VerificationStatus>\n",
            xml_text(&change.verification_status)
        ));
        output.push_str("    </Change>\n");
    }
    output.push_str("  </Changes>\n</CascadeChange>\n");
    output
}
// END_render_changelog_xml

// END_public_api

// START_CONTRACT_safe_id
// PURPOSE: Sanitize a cascade id for use as a local XML filename
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_safe_id
fn safe_id(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        }
    }
    if out.is_empty() {
        "CSC-UNKNOWN".into()
    } else {
        out
    }
}
// END_safe_id

// START_CONTRACT_attr
// PURPOSE: Extract one XML attribute value from a short tag string
// INPUTS: { content: &str }, { name: &str }
// OUTPUTS: { Option<String> }
// START_attr
fn attr(content: &str, name: &str) -> Option<String> {
    let needle = format!(r#"{}=""#, name);
    let start = content.find(&needle)? + needle.len();
    let rest = &content[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
// END_attr

// START_CONTRACT_tag_value
// PURPOSE: Extract one simple XML tag value from changelog content
// INPUTS: { content: &str }, { tag: &str }
// OUTPUTS: { Option<String> }
// START_tag_value
fn tag_value(content: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = content.find(&open)? + open.len();
    let end = content[start..].find(&close)? + start;
    Some(content[start..end].to_string())
}
// END_tag_value

// START_CONTRACT_xml_text
// PURPOSE: Escape XML text node content
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_xml_text
fn xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
// END_xml_text

// START_CONTRACT_xml_attr
// PURPOSE: Escape XML attribute content
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_xml_attr
fn xml_attr(value: &str) -> String {
    xml_text(value).replace('"', "&quot;")
}
// END_xml_attr

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_write_changelog_creates_history_entry
    // PURPOSE: Verify changelog XML persistence can be listed back through history scanning
    // OUTPUTS: { () }
    // START_test_write_changelog_creates_history_entry
    #[test]
    fn test_write_changelog_creates_history_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = CascadeChangeLog {
            id: "CSC-TEST".into(),
            timestamp: "2026-05-21T00:00:00Z".into(),
            trigger: CascadeTrigger {
                artifact: "UC-001".into(),
                change: "Address step".into(),
                author: "test".into(),
            },
            changes: vec![CascadeChange {
                artifact: "M-ORDER".into(),
                change_type: "contract".into(),
                action: "updated".into(),
                before: "old".into(),
                after: "new".into(),
                verification_status: "pending".into(),
            }],
        };

        write_changelog(dir.path(), &log).expect("write changelog");
        let history = list_changelog_entries(dir.path()).expect("history");
        assert_eq!(history.changelog_count, 1);
        assert_eq!(
            history.latest.as_ref().map(|entry| entry.id.as_str()),
            Some("CSC-TEST")
        );
    }
    // END_test_write_changelog_creates_history_entry
}
