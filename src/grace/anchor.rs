// MODULE_CONTRACT
// MODULE_ID: M-GRACE-ANCHOR
// PURPOSE: XML-like GRACE anchor normalization and syntax consistency reporting
// SCOPE: normalize_anchor_syntax, XML-like tag parsing, syntax style detection, project-level consistency report
// DEPENDS: M-INDEXER-WALKER
// LINKS:
//   → V-M-GRACE-ANCHOR (verified_by) — XML-like anchor normalization and syntax report tests

// START_MODULE_MAP
// AnchorSyntaxReport — Project-level legacy/XML-like syntax usage report
// normalize_anchor_syntax — Converts XML-like anchors into legacy START/END-compatible markers
// anchor_syntax_report — Scans source files for legacy/XML-like/mixed anchor styles
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Added XML-like anchor syntax normalization and consistency reporting]
// END_CHANGE_SUMMARY

use std::path::Path;

// START_public_api

// START_AnchorSyntaxReport
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct AnchorSyntaxReport {
    pub total_files: usize,
    pub legacy_files: usize,
    pub xml_files: usize,
    pub mixed_files: Vec<String>,
    pub legacy_anchor_count: usize,
    pub xml_anchor_count: usize,
}
// END_AnchorSyntaxReport

impl AnchorSyntaxReport {
    // START_CONTRACT_AnchorSyntaxReport::has_mixed_syntax
    // PURPOSE: Return true when at least one source file mixes legacy and XML-like anchor styles
    // OUTPUTS: { bool }
    // START_anchor_syntax_report_has_mixed_syntax
    pub fn has_mixed_syntax(&self) -> bool {
        !self.mixed_files.is_empty()
    }
    // END_anchor_syntax_report_has_mixed_syntax
}

// START_CONTRACT_normalize_anchor_syntax
// PURPOSE: Convert XML-like GRACE anchors into START/END-compatible markers for existing parsers
// INPUTS: { content: &str — source text }
// OUTPUTS: { String — source text with XML-like anchors normalized }
// START_normalize_anchor_syntax
pub fn normalize_anchor_syntax(content: &str) -> String {
    content
        .lines()
        .map(normalize_anchor_line)
        .collect::<Vec<_>>()
        .join("\n")
}
// END_normalize_anchor_syntax

// START_CONTRACT_anchor_syntax_report
// PURPOSE: Scan source files and report legacy/XML-like anchor style usage
// INPUTS: { root: &Path — project root }
// OUTPUTS: { anyhow::Result<AnchorSyntaxReport> }
// START_anchor_syntax_report
pub fn anchor_syntax_report(root: &Path) -> anyhow::Result<AnchorSyntaxReport> {
    let walker = crate::indexer::walker::Walker::new(root);
    let files = walker.walk();
    let mut report = AnchorSyntaxReport {
        total_files: files.len(),
        ..AnchorSyntaxReport::default()
    };

    for file in files {
        if file.language == "markdown" {
            continue;
        }
        let full_path = root.join(&file.path);
        let Ok(content) = std::fs::read_to_string(&full_path) else {
            continue;
        };
        let usage = detect_anchor_usage(&content);
        if usage.legacy_count > 0 {
            report.legacy_files += 1;
            report.legacy_anchor_count += usage.legacy_count;
        }
        if usage.xml_count > 0 {
            report.xml_files += 1;
            report.xml_anchor_count += usage.xml_count;
        }
        if usage.legacy_count > 0 && usage.xml_count > 0 {
            report.mixed_files.push(file.path);
        }
    }
    Ok(report)
}
// END_anchor_syntax_report

// END_public_api

#[derive(Debug, Clone, Copy)]
struct AnchorUsage {
    legacy_count: usize,
    xml_count: usize,
}

fn normalize_anchor_line(line: &str) -> String {
    let Some(comment) = split_comment_line(line) else {
        return line.to_string();
    };
    let marker = comment.marker.trim();
    if let Some(open) = parse_xml_open(marker) {
        if let Some(normalized) = normalized_open_marker(&open) {
            return format!("{}{} {}", comment.indent, comment.prefix, normalized);
        }
    }
    if let Some(tag) = parse_xml_close(marker) {
        if let Some(normalized) = normalized_close_marker(&tag) {
            return format!("{}{} {}", comment.indent, comment.prefix, normalized);
        }
    }
    line.to_string()
}

fn detect_anchor_usage(content: &str) -> AnchorUsage {
    let mut usage = AnchorUsage {
        legacy_count: 0,
        xml_count: 0,
    };
    for line in content.lines() {
        let Some(comment) = split_comment_line(line) else {
            continue;
        };
        let marker = comment.marker.trim();
        if is_legacy_anchor(marker) {
            usage.legacy_count += 1;
        }
        if parse_xml_open(marker).is_some() || parse_xml_close(marker).is_some() {
            usage.xml_count += 1;
        }
    }
    usage
}

#[derive(Debug, Clone)]
struct CommentLine<'a> {
    indent: &'a str,
    prefix: &'static str,
    marker: &'a str,
}

fn split_comment_line(line: &str) -> Option<CommentLine<'_>> {
    let trimmed_start = line.trim_start();
    let indent_len = line.len().saturating_sub(trimmed_start.len());
    let indent = &line[..indent_len];
    if let Some(marker) = trimmed_start.strip_prefix("//") {
        Some(CommentLine {
            indent,
            prefix: "//",
            marker,
        })
    } else if let Some(marker) = trimmed_start.strip_prefix('#') {
        Some(CommentLine {
            indent,
            prefix: "#",
            marker,
        })
    } else if let Some(marker) = trimmed_start.strip_prefix("--") {
        Some(CommentLine {
            indent,
            prefix: "--",
            marker,
        })
    } else if let Some(marker) = trimmed_start.strip_prefix("/*") {
        Some(CommentLine {
            indent,
            prefix: "/*",
            marker,
        })
    } else if let Some(marker) = trimmed_start.strip_prefix('*') {
        Some(CommentLine {
            indent,
            prefix: "*",
            marker,
        })
    } else {
        trimmed_start
            .strip_prefix("<!--")
            .map(|marker| CommentLine {
                indent,
                prefix: "<!--",
                marker,
            })
    }
}

#[derive(Debug, Clone)]
struct XmlOpen {
    tag: String,
    name: Option<String>,
    id: Option<String>,
    module: Option<String>,
    ref_block: Option<String>,
}

fn parse_xml_open(marker: &str) -> Option<XmlOpen> {
    let marker = clean_marker_tail(marker);
    if !marker.starts_with('<') || marker.starts_with("</") {
        return None;
    }
    let inner = marker
        .strip_prefix('<')?
        .trim_end_matches('>')
        .trim_end_matches('/')
        .trim();
    let mut parts = inner.splitn(2, char::is_whitespace);
    let tag = parts.next()?.trim().to_ascii_uppercase();
    if !is_supported_tag(&tag) {
        return None;
    }
    let attrs = parts.next().unwrap_or_default();
    Some(XmlOpen {
        tag,
        name: attr_value(attrs, "name"),
        id: attr_value(attrs, "id"),
        module: attr_value(attrs, "module"),
        ref_block: attr_value(attrs, "ref"),
    })
}

fn parse_xml_close(marker: &str) -> Option<String> {
    let marker = clean_marker_tail(marker);
    let inner = marker.strip_prefix("</")?.trim_end_matches('>').trim();
    let tag = inner.to_ascii_uppercase();
    if is_supported_tag(&tag) {
        Some(tag)
    } else {
        None
    }
}

fn normalized_open_marker(open: &XmlOpen) -> Option<String> {
    match open.tag.as_str() {
        "MODULE_CONTRACT" => Some("MODULE_CONTRACT".into()),
        "MODULE_MAP" => Some("START_MODULE_MAP".into()),
        "CHANGE_SUMMARY" => Some("START_CHANGE_SUMMARY".into()),
        "FUNCTION_CONTRACT" => Some(format!(
            "START_CONTRACT_{}",
            anchor_name(open.name.as_deref(), "anonymous")
        )),
        "LINKS" => Some("LINKS:".into()),
        "MODULE" => Some(format!(
            "START_MODULE_{}",
            anchor_name(open.name.as_deref(), "root")
        )),
        "CLASS" => Some(format!(
            "START_CLASS_{}",
            anchor_name(open.name.as_deref(), "anonymous")
        )),
        "BLOCK" | "METHOD" => Some(format!(
            "START_{}",
            anchor_name(open.name.as_deref(), open.tag.as_str())
        )),
        "LOG" => Some(format!(
            "START_LOG_{}",
            anchor_name(
                open.id
                    .as_deref()
                    .or(open.ref_block.as_deref())
                    .or(open.name.as_deref()),
                "entry"
            )
        )),
        "BELIEF_STATE" => Some(format!(
            "START_BELIEF_STATE_{}",
            anchor_name(open.module.as_deref().or(open.name.as_deref()), "module")
        )),
        _ => None,
    }
}

fn normalized_close_marker(tag: &str) -> Option<String> {
    match tag {
        "MODULE_CONTRACT" => Some("END_MODULE_CONTRACT".into()),
        "MODULE_MAP" => Some("END_MODULE_MAP".into()),
        "CHANGE_SUMMARY" => Some("END_CHANGE_SUMMARY".into()),
        "FUNCTION_CONTRACT" => Some("END_CONTRACT".into()),
        "LINKS" => Some("END_LINKS".into()),
        "MODULE" => Some("END_MODULE".into()),
        "CLASS" => Some("END_CLASS".into()),
        "BLOCK" | "METHOD" => Some("END_BLOCK".into()),
        "LOG" => Some("END_LOG".into()),
        "BELIEF_STATE" => Some("END_BELIEF_STATE".into()),
        _ => None,
    }
}

fn is_legacy_anchor(marker: &str) -> bool {
    let clean = clean_marker_tail(marker)
        .trim_matches('=')
        .trim()
        .to_ascii_uppercase();
    clean == "MODULE_CONTRACT"
        || clean.starts_with("START_")
        || clean.starts_with("END_")
        || clean == "MODULE_MAP"
        || clean == "CHANGE_SUMMARY"
}

fn is_supported_tag(tag: &str) -> bool {
    matches!(
        tag,
        "MODULE"
            | "CLASS"
            | "BLOCK"
            | "METHOD"
            | "LOG"
            | "BELIEF_STATE"
            | "FUNCTION_CONTRACT"
            | "LINKS"
            | "MODULE_CONTRACT"
            | "MODULE_MAP"
            | "CHANGE_SUMMARY"
    )
}

fn attr_value(attrs: &str, name: &str) -> Option<String> {
    let pattern = format!(r#"{}=""#, name);
    let start = attrs.find(&pattern)? + pattern.len();
    let rest = &attrs[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn anchor_name(value: Option<&str>, fallback: &str) -> String {
    let source = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback);
    let mut out = String::new();
    let mut last_sep = false;
    for ch in source.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' || ch == '-' || ch == '.' {
            out.push(ch);
            last_sep = false;
        } else if !last_sep && !out.is_empty() {
            out.push('-');
            last_sep = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

fn clean_marker_tail(marker: &str) -> &str {
    marker
        .trim()
        .trim_end_matches("*/")
        .trim_end_matches("-->")
        .trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_normalize_anchor_syntax_converts_xml_blocks
    // PURPOSE: Verify XML-like BLOCK anchors normalize to START/END syntax
    // OUTPUTS: { () }
    // START_test_normalize_anchor_syntax_converts_xml_blocks
    #[test]
    fn test_normalize_anchor_syntax_converts_xml_blocks() {
        let normalized = normalize_anchor_syntax(concat!(
            "// <BLOCK name=\"stock-validation\">\n",
            "let ok = true;\n",
            "// </BLOCK>\n",
        ));
        assert!(normalized.contains("// START_stock-validation"));
        assert!(normalized.contains("// END_BLOCK"));
    }
    // END_test_normalize_anchor_syntax_converts_xml_blocks

    // START_CONTRACT_test_normalize_anchor_syntax_converts_function_contract
    // PURPOSE: Verify XML-like FUNCTION_CONTRACT anchors normalize to START_CONTRACT syntax
    // OUTPUTS: { () }
    // START_test_normalize_anchor_syntax_converts_function_contract
    #[test]
    fn test_normalize_anchor_syntax_converts_function_contract() {
        let normalized = normalize_anchor_syntax(concat!(
            "# <FUNCTION_CONTRACT name=\"parse_date\">\n",
            "# PURPOSE: Parse date\n",
            "# </FUNCTION_CONTRACT>\n",
        ));
        assert!(normalized.contains("# START_CONTRACT_parse_date"));
        assert!(normalized.contains("# END_CONTRACT"));
    }
    // END_test_normalize_anchor_syntax_converts_function_contract

    // START_CONTRACT_test_anchor_syntax_report_detects_mixed_file
    // PURPOSE: Verify syntax report records files that mix legacy and XML-like anchors
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temp source file
    // START_test_anchor_syntax_report_detects_mixed_file
    #[test]
    fn test_anchor_syntax_report_detects_mixed_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("mixed.rs"),
            concat!(
                "// MODULE_CONTRACT\n",
                "// <BLOCK name=\"x\">\n",
                "fn x() {}\n",
                "// </BLOCK>\n",
            ),
        )
        .expect("write source");
        let report = anchor_syntax_report(dir.path()).expect("report");
        assert_eq!(report.mixed_files, vec!["mixed.rs".to_string()]);
        assert!(report.has_mixed_syntax());
    }
    // END_test_anchor_syntax_report_detects_mixed_file
}
// END_public_api
