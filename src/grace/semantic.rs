// MODULE_CONTRACT
// MODULE_ID: M-GRACE-SEMANTIC
// PURPOSE: Semantic block scanner — extracts language-aware START_/END_ and XML-like anchor block pairs from source files
// SCOPE: SemanticExtractor, SemanticBlock, SemanticReport, extract_blocks, scan_project, comment marker normalization, XML-like anchor normalization
// DEPENDS: M-GRACE-ANCHOR, M-INDEXER-WALKER
// LINKS: N/A

// START_MODULE_MAP
// SemanticBlock — A single START/END block with name, location, content, closure status
// SemanticReport — Aggregate semantic markup report across project
// SemanticExtractor — Scans project for semantic block markers after anchor syntax normalization
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.13.0 — Added XML-like anchor syntax support]
// END_CHANGE_SUMMARY

use std::path::Path;

// START_public_api

// START_SemanticBlock
#[derive(Debug, Clone, serde::Serialize)]
pub struct SemanticBlock {
    pub name: String,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub is_closed: bool,
}
// END_SemanticBlock

// START_SemanticReport
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct SemanticReport {
    pub total_files: usize,
    pub files_with_blocks: usize,
    pub total_blocks: usize,
    pub open_blocks: usize,
    pub closed_blocks: usize,
    pub unclosed_blocks: Vec<SemanticBlock>,
    pub duplicate_name_blocks: Vec<SemanticBlock>,
    pub blocks: Vec<SemanticBlock>,
}
// END_SemanticReport

// START_SemanticExtractor
pub struct SemanticExtractor;
// END_SemanticExtractor

impl Default for SemanticExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticExtractor {
    // START_CONTRACT_SemanticExtractor::new
    // PURPOSE: Create a new SemanticExtractor
    // OUTPUTS: { Self }
    // START_se_new
    pub fn new() -> Self {
        Self
    }
    // END_se_new

    // START_CONTRACT_SemanticExtractor::extract_blocks
    // PURPOSE: Extract all START_/END_ and XML-like block pairs from a single file
    // INPUTS: { path: &Path }, { content: &str }
    // OUTPUTS: { Vec<SemanticBlock> }
    // START_se_extract_blocks
    pub fn extract_blocks(path: &Path, content: &str) -> Vec<SemanticBlock> {
        let file_path = path.to_string_lossy().to_string();
        let mut blocks = Vec::new();
        let mut stack: Vec<(String, usize)> = Vec::new(); // name, start_line
        let normalized = crate::grace::anchor::normalize_anchor_syntax(content);
        let lines: Vec<&str> = normalized.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let line_num = i + 1;

            // START_BLOCK_NAME — skip CONTRACT markers (they pair with actual START)
            if let Some(name) = semantic_marker_name(trimmed, "START_") {
                if !name.is_empty() && !name.starts_with("CONTRACT") {
                    stack.push((name, line_num));
                }
            }

            // END_BLOCK_NAME or END_NAME — skip CONTRACT_ markers
            let is_end = semantic_marker_name(trimmed, "END_")
                .map(|name| !name.starts_with("CONTRACT"))
                .unwrap_or(false);

            if is_end {
                let Some((name, start_line)) = stack.pop() else {
                    continue;
                };
                // Extract content between start and end
                let content_slice = if start_line < line_num {
                    lines[start_line..line_num].join("\n")
                } else {
                    String::new()
                };
                blocks.push(SemanticBlock {
                    name,
                    file_path: file_path.clone(),
                    start_line,
                    end_line: line_num,
                    content: content_slice,
                    is_closed: true,
                });
            }
        }

        // Unclosed blocks remain on the stack
        for (name, start_line) in stack {
            blocks.push(SemanticBlock {
                name,
                file_path: file_path.clone(),
                start_line,
                end_line: lines.len(),
                content: lines[start_line..].join("\n"),
                is_closed: false,
            });
        }

        blocks
    }
    // END_se_extract_blocks

    // START_CONTRACT_SemanticExtractor::scan_project
    // PURPOSE: Scan entire project for semantic blocks
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<SemanticReport> }
    // START_se_scan_project
    pub fn scan_project(root: &Path) -> anyhow::Result<SemanticReport> {
        let mut report = SemanticReport::default();
        let walker = crate::indexer::walker::Walker::new(root);
        let files = walker.walk();
        report.total_files = files.len();

        for file in &files {
            if file.language == "markdown" {
                continue;
            }
            let full_path = root.join(&file.path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let blocks = Self::extract_blocks(&full_path, &content);
                if !blocks.is_empty() {
                    report.files_with_blocks += 1;
                    // Detect duplicate block names within the file
                    let mut seen: std::collections::HashMap<String, usize> =
                        std::collections::HashMap::new();
                    for block in &blocks {
                        *seen.entry(block.name.clone()).or_insert(0) += 1;
                    }
                    for (name, count) in &seen {
                        if *count > 1 {
                            if let Some(block) = blocks.iter().find(|b| b.name == *name) {
                                report.duplicate_name_blocks.push(block.clone());
                            }
                        }
                    }
                    for block in blocks {
                        if block.is_closed {
                            report.closed_blocks += 1;
                        } else {
                            report.open_blocks += 1;
                            report.unclosed_blocks.push(block.clone());
                        }
                        report.blocks.push(block);
                    }
                }
            }
        }

        report.total_blocks = report.closed_blocks + report.open_blocks;
        Ok(report)
    }
    // END_se_scan_project
}

// START_CONTRACT_semantic_marker_name
// PURPOSE: Extract a START_ or END_ semantic marker name from a language-aware comment line
// INPUTS: { trimmed: &str — source line without surrounding whitespace }, { prefix: &str — START_ or END_ }
// OUTPUTS: { Option<String> }
// START_semantic_marker_name
fn semantic_marker_name(trimmed: &str, prefix: &str) -> Option<String> {
    let marker = normalize_comment_line(trimmed)?;
    let rest = marker.strip_prefix(prefix)?;
    let name = rest
        .trim()
        .trim_end_matches("*/")
        .trim_end_matches("-->")
        .trim()
        .to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}
// END_semantic_marker_name

// START_CONTRACT_normalize_comment_line
// PURPOSE: Strip supported language comment syntax from one semantic marker line
// INPUTS: { trimmed: &str — source line without surrounding whitespace }
// OUTPUTS: { Option<String> }
// START_normalize_comment_line
fn normalize_comment_line(trimmed: &str) -> Option<String> {
    let value = if let Some(rest) = trimmed.strip_prefix("//") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix('#') {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("--") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("/*") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix('*') {
        rest
    } else {
        trimmed.strip_prefix("<!--")?
    };
    Some(
        value
            .trim()
            .trim_end_matches("*/")
            .trim_end_matches("-->")
            .trim()
            .trim_matches('=')
            .trim()
            .to_string(),
    )
}
// END_normalize_comment_line

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_blocks_ignores_orphan_end_marker() {
        let blocks = SemanticExtractor::extract_blocks(
            Path::new("src/orphan.rs"),
            "// END_missing\nfn main() {}\n",
        );

        assert!(blocks.is_empty());
    }

    #[test]
    fn test_extract_blocks_supports_sql_markers() {
        let blocks = SemanticExtractor::extract_blocks(
            Path::new("schema.sql"),
            "-- START_create_users\nCREATE TABLE users(id INTEGER);\n-- END_create_users\n",
        );

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].name, "create_users");
        assert!(blocks[0].is_closed);
    }

    #[test]
    fn test_extract_blocks_supports_html_markers() {
        let blocks = SemanticExtractor::extract_blocks(
            Path::new("template.html"),
            "<!-- START_content -->\n<section></section>\n<!-- END_content -->\n",
        );

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].name, "content");
        assert!(blocks[0].is_closed);
    }

    #[test]
    fn test_extract_blocks_supports_xml_like_anchors() {
        let blocks = SemanticExtractor::extract_blocks(
            Path::new("src/order.rs"),
            concat!(
                "// <BLOCK name=\"stock-validation\">\n",
                "let valid = true;\n",
                "// </BLOCK>\n",
            ),
        );

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].name, "stock-validation");
        assert!(blocks[0].is_closed);
    }

    #[test]
    fn test_extract_blocks_supports_nested_xml_like_anchors() {
        let blocks = SemanticExtractor::extract_blocks(
            Path::new("src/order.rs"),
            concat!(
                "// <MODULE name=\"OrderService\">\n",
                "// <BLOCK name=\"stock-validation\">\n",
                "let valid = true;\n",
                "// </BLOCK>\n",
                "// </MODULE>\n",
            ),
        );

        assert_eq!(blocks.len(), 2);
        assert!(blocks.iter().any(|block| block.name == "stock-validation"));
        assert!(blocks
            .iter()
            .any(|block| block.name == "MODULE_OrderService"));
    }

    #[test]
    fn test_scan_project_skips_markdown_examples() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("README.md"),
            concat!(
                "```rust\n",
                "// <BLOCK name=\"same\">\n",
                "// </BLOCK>\n",
                "// START_same\n",
                "// END_same\n",
                "```\n",
            ),
        )
        .expect("write markdown");
        let report = SemanticExtractor::scan_project(dir.path()).expect("scan");
        assert!(report.blocks.is_empty());
        assert!(report.duplicate_name_blocks.is_empty());
    }
}
// END_public_api
