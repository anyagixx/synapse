// MODULE_CONTRACT
// MODULE_ID: M-GRACE-SEMANTIC
// PURPOSE: Semantic block scanner — extracts START_/END_ block pairs from source files
// SCOPE: SemanticExtractor, SemanticBlock, SemanticReport, extract_blocks, scan_project
// DEPENDS: M-INDEXER-WALKER
// LINKS: N/A

// START_MODULE_MAP
// SemanticBlock — A single START/END block with name, location, content, closure status
// SemanticReport — Aggregate semantic markup report across project
// SemanticExtractor — Scans project for semantic block markers
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
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
    // PURPOSE: Extract all START_/END_ block pairs from a single file
    // INPUTS: { path: &Path }, { content: &str }
    // OUTPUTS: { Vec<SemanticBlock> }
    // START_se_extract_blocks
    pub fn extract_blocks(path: &Path, content: &str) -> Vec<SemanticBlock> {
        let file_path = path.to_string_lossy().to_string();
        let mut blocks = Vec::new();
        let mut stack: Vec<(String, usize)> = Vec::new(); // name, start_line
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let line_num = i + 1;

            // START_BLOCK_NAME — skip CONTRACT_ markers (they pair with actual START)
            if let Some(name) = trimmed
                .strip_prefix("// START_")
                .or_else(|| trimmed.strip_prefix("# START_"))
                .or_else(|| trimmed.strip_prefix("/* START_"))
                .or_else(|| trimmed.strip_prefix("* START_"))
            {
                let name = name.trim().trim_end_matches("*/").trim().to_string();
                if !name.is_empty() && !name.starts_with("CONTRACT_") {
                    stack.push((name, line_num));
                }
            }

            // END_BLOCK_NAME or END_NAME — skip CONTRACT_ markers
            let is_end = (trimmed.starts_with("// END_")
                || trimmed.starts_with("# END_")
                || trimmed.starts_with("/* END_")
                || trimmed.starts_with("* END_"))
                && !trimmed.contains("END_CONTRACT_");

            if is_end && !stack.is_empty() {
                let (name, start_line) = stack.pop().unwrap();
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
// END_public_api
