use std::path::Path;

/// A semantic block defined by START/END markers.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SemanticBlock {
    pub name: String,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub is_closed: bool,
}

/// Report of semantic markup across the project.
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct SemanticReport {
    pub total_files: usize,
    pub files_with_blocks: usize,
    pub total_blocks: usize,
    pub open_blocks: usize,
    pub closed_blocks: usize,
    pub unclosed_blocks: Vec<SemanticBlock>,
    pub blocks: Vec<SemanticBlock>,
}

pub struct SemanticExtractor;

impl Default for SemanticExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract all START_BLOCK/END_BLOCK pairs from a file
    pub fn extract_blocks(path: &Path, content: &str) -> Vec<SemanticBlock> {
        let file_path = path.to_string_lossy().to_string();
        let mut blocks = Vec::new();
        let mut stack: Vec<(String, usize)> = Vec::new(); // name, start_line
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let line_num = i + 1;

            // START_BLOCK_NAME
            if let Some(name) = trimmed
                .strip_prefix("// START_")
                .or_else(|| trimmed.strip_prefix("# START_"))
                .or_else(|| trimmed.strip_prefix("/* START_"))
                .or_else(|| trimmed.strip_prefix("* START_"))
            {
                let name = name.trim().trim_end_matches("*/").trim().to_string();
                if !name.is_empty() {
                    stack.push((name, line_num));
                }
            }
            // START_NAME without BLOCK_ prefix
            else if let Some(name) = trimmed
                .strip_prefix("// START_")
                .or_else(|| trimmed.strip_prefix("# START_"))
            {
                let name = name.trim().to_string();
                if !name.is_empty() {
                    stack.push((name.clone(), line_num));
                }
            }

            // END_BLOCK_NAME or END_NAME
            let is_end = trimmed.starts_with("// END_")
                || trimmed.starts_with("# END_")
                || trimmed.starts_with("/* END_")
                || trimmed.starts_with("* END_");

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

    /// Scan entire project for semantic markup
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
}
