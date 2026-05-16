use crate::grace::contract::ContractValidator;
use crate::grace::semantic::SemanticExtractor;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReviewReport {
    pub mode: String,
    pub passed: bool,
    pub sections: Vec<ReviewSection>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReviewSection {
    pub name: String,
    pub passed: bool,
    pub details: String,
    pub issues: Vec<String>,
}

pub struct Reviewer;

impl Default for Reviewer {
    fn default() -> Self {
        Self::new()
    }
}

impl Reviewer {
    pub fn new() -> Self {
        Self
    }

    pub fn review(root: &Path, mode: &str) -> anyhow::Result<ReviewReport> {
        match mode {
            "scoped" => Self::scoped_gate(root),
            "wave-audit" => Self::wave_audit(root),
            "full" => Self::full_integrity(root),
            _ => Self::scoped_gate(root),
        }
    }

    fn scoped_gate(root: &Path) -> anyhow::Result<ReviewReport> {
        let mut sections = Vec::new();

        // 1. Semantic markup integrity
        let sem = SemanticExtractor::scan_project(root)?;
        let unclosed = sem.unclosed_blocks;
        sections.push(ReviewSection {
            name: "semantic-markup".into(),
            passed: unclosed.is_empty() && sem.duplicate_name_blocks.is_empty(),
            details: format!(
                "{} blocks, {} unclosed, {} duplicates",
                sem.total_blocks,
                unclosed.len(),
                sem.duplicate_name_blocks.len()
            ),
            issues: unclosed
                .iter()
                .map(|b| format!("Unclosed: {} at {}", b.name, b.file_path))
                .chain(
                    sem.duplicate_name_blocks
                        .iter()
                        .map(|b| format!("Duplicate: {} at {}", b.name, b.file_path)),
                )
                .collect(),
        });

        // 2. Contract compliance
        let report = ContractValidator::validate_project(root)?;
        sections.push(ReviewSection {
            name: "contract-compliance".into(),
            passed: report.invalid == 0,
            details: format!("{}/{} valid contracts", report.valid, report.with_contract),
            issues: report
                .contracts
                .iter()
                .filter(|c| !c.valid && c.has_contract)
                .map(|c| format!("Invalid contract: {}", c.file_path))
                .collect(),
        });

        let passed = sections.iter().all(|s| s.passed);
        Ok(ReviewReport {
            mode: "scoped".into(),
            passed,
            sections,
        })
    }

    fn wave_audit(root: &Path) -> anyhow::Result<ReviewReport> {
        let mut sections = Self::scoped_gate(root)?.sections;

        // Cross-module import/dependency check
        let report = ContractValidator::validate_project(root)?;
        let mut dep_issues = Vec::new();
        for c in &report.contracts {
            if !c.has_contract {
                continue;
            }
            // Check DEPENDS reference actual modules
            for dep in &c.depends {
                let found = report
                    .contracts
                    .iter()
                    .any(|other| other.module_id.as_deref() == Some(dep.as_str()));
                if !found && dep != "N/A" && !dep.is_empty() {
                    dep_issues.push(format!("{} depends on unknown module {}", c.file_path, dep));
                }
            }
        }
        sections.push(ReviewSection {
            name: "cross-module-deps".into(),
            passed: dep_issues.is_empty(),
            details: format!("{} dependency issues", dep_issues.len()),
            issues: dep_issues,
        });

        // Knowledge graph consistency with actual imports
        let kg_path = root.join("docs").join("knowledge-graph.xml");
        if kg_path.exists() {
            let kg_content = std::fs::read_to_string(&kg_path).unwrap_or_default();
            let mut kg_issues = Vec::new();
            for c in &report.contracts {
                if !c.has_contract || c.depends.is_empty() {
                    continue;
                }
                for dep in &c.depends {
                    if !kg_content.contains(dep.as_str()) {
                        kg_issues.push(format!("{}:{} not in knowledge-graph", c.file_path, dep));
                    }
                }
            }
            sections.push(ReviewSection {
                name: "graph-consistency".into(),
                passed: kg_issues.is_empty(),
                details: format!("{} graph mismatches", kg_issues.len()),
                issues: kg_issues,
            });
        }

        let passed = sections.iter().all(|s| s.passed);
        Ok(ReviewReport {
            mode: "wave-audit".into(),
            passed,
            sections,
        })
    }

    fn full_integrity(root: &Path) -> anyhow::Result<ReviewReport> {
        let mut sections = Self::scoped_gate(root)?.sections;

        // 3. Verification integrity
        let vp_path = root.join("docs").join("verification-plan.xml");
        let has_plan = vp_path.exists();
        sections.push(ReviewSection {
            name: "verification-plan".into(),
            passed: has_plan,
            details: if has_plan {
                "Verification plan exists".into()
            } else {
                "No verification plan".into()
            },
            issues: if has_plan {
                vec![]
            } else {
                vec!["Missing docs/verification-plan.xml".into()]
            },
        });

        // 4. Graph consistency
        let kg_path = root.join("docs").join("knowledge-graph.xml");
        let has_graph = kg_path.exists();
        sections.push(ReviewSection {
            name: "knowledge-graph".into(),
            passed: has_graph,
            details: if has_graph {
                "Knowledge graph exists".into()
            } else {
                "No knowledge graph".into()
            },
            issues: if has_graph {
                vec![]
            } else {
                vec!["Missing docs/knowledge-graph.xml".into()]
            },
        });

        // 5. Naming conventions (check for common anti-patterns)
        let walker = crate::indexer::walker::Walker::new(root);
        let files = walker.walk();
        let mut name_issues = Vec::new();
        for f in &files {
            if f.path.contains(' ') {
                name_issues.push(format!("Space in path: {}", f.path));
            }
        }
        sections.push(ReviewSection {
            name: "naming-conventions".into(),
            passed: name_issues.is_empty(),
            details: format!(
                "{} files checked, {} issues",
                files.len(),
                name_issues.len()
            ),
            issues: name_issues,
        });

        // 6. Security check (no secrets in code)
        let mut secret_issues = Vec::new();
        for f in &files {
            let full_path = root.join(&f.path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                for (i, line) in content.lines().enumerate() {
                    let l = line.to_lowercase();
                    if (l.contains("api_key") || l.contains("password") || l.contains("secret"))
                        && !l.trim_start().starts_with("//")
                        && !l.trim_start().starts_with('#')
                        && !l.trim_start().starts_with("/*")
                        && (line.contains('=') || line.contains(':'))
                    {
                        secret_issues.push(format!("{}:{} — possible secret", f.path, i + 1));
                    }
                }
            }
        }
        sections.push(ReviewSection {
            name: "secrets-check".into(),
            passed: secret_issues.is_empty(),
            details: format!("{} potential secrets found", secret_issues.len()),
            issues: secret_issues,
        });

        let passed = sections.iter().all(|s| s.passed);
        Ok(ReviewReport {
            mode: "full".into(),
            passed,
            sections,
        })
    }
}
