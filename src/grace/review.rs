// MODULE_CONTRACT
// MODULE_ID: M-GRACE-REVIEW
// PURPOSE: GRACE integrity review — checks semantic markup, profile-aware contracts, canonical shards, naming, secrets
// SCOPE: Reviewer struct, ReviewReport, ReviewSection, review_with_profile, scoped_gate, wave_audit, full_integrity
// DEPENDS: M-GRACE-CONTRACT, M-GRACE-INVENTORY, M-GRACE-SEMANTIC, M-INDEXER-WALKER
// LINKS: docs/graph-index.xml, docs/verification-index.xml

// START_MODULE_MAP
// ReviewReport — Full review report with sections
// ReviewSection — Single review section with issues list
// Reviewer — GRACE integrity reviewer with scoped, wave-audit, full modes, and strictness profiles
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Added profile-aware contract review gates]
// END_CHANGE_SUMMARY

use crate::grace::contract::{ContractValidator, GraceProfile};
use crate::grace::inventory::MyGraceInventory;
use crate::grace::layout::DocsLayout;
use crate::grace::semantic::SemanticExtractor;
use std::path::Path;

// START_public_api

// START_ReviewReport
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReviewReport {
    pub mode: String,
    pub passed: bool,
    pub sections: Vec<ReviewSection>,
}
// END_ReviewReport

// START_ReviewSection
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReviewSection {
    pub name: String,
    pub passed: bool,
    pub details: String,
    pub issues: Vec<String>,
}
// END_ReviewSection

// START_Reviewer
pub struct Reviewer;
// END_Reviewer

impl Default for Reviewer {
    fn default() -> Self {
        Self::new()
    }
}

impl Reviewer {
    // START_CONTRACT_Reviewer::new
    // PURPOSE: Create a new Reviewer
    // OUTPUTS: { Self }
    // START_reviewer_new
    pub fn new() -> Self {
        Self
    }
    // END_reviewer_new

    // START_CONTRACT_Reviewer::review
    // PURPOSE: Run GRACE integrity review with the given mode
    // INPUTS: { root: &Path }, { mode: &str — scoped|wave-audit|full }
    // OUTPUTS: { anyhow::Result<ReviewReport> }
    // START_reviewer_review
    pub fn review(root: &Path, mode: &str) -> anyhow::Result<ReviewReport> {
        Self::review_with_profile(root, mode, GraceProfile::Strict)
    }
    // END_reviewer_review

    // START_CONTRACT_Reviewer::review_with_profile
    // PURPOSE: Run GRACE integrity review with the selected strictness profile
    // INPUTS: { root: &Path }, { mode: &str — scoped|wave-audit|full }, { profile: GraceProfile }
    // OUTPUTS: { anyhow::Result<ReviewReport> }
    // START_reviewer_review_with_profile
    pub fn review_with_profile(
        root: &Path,
        mode: &str,
        profile: GraceProfile,
    ) -> anyhow::Result<ReviewReport> {
        match mode {
            "scoped" => Self::scoped_gate_with_profile(root, profile),
            "wave-audit" => Self::wave_audit_with_profile(root, profile),
            "full" => Self::full_integrity_with_profile(root, profile),
            _ => Self::scoped_gate_with_profile(root, profile),
        }
    }
    // END_reviewer_review_with_profile

    fn scoped_gate_with_profile(
        root: &Path,
        profile: GraceProfile,
    ) -> anyhow::Result<ReviewReport> {
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
        let report = ContractValidator::validate_project_with_profile(root, profile)?;
        sections.push(ReviewSection {
            name: "contract-compliance".into(),
            passed: report.invalid == 0,
            details: format!(
                "{}/{} valid contracts under {} profile",
                report.valid,
                report.with_contract,
                profile.as_str()
            ),
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

    fn wave_audit_with_profile(root: &Path, profile: GraceProfile) -> anyhow::Result<ReviewReport> {
        let mut sections = Self::scoped_gate_with_profile(root, profile)?.sections;

        // Cross-module import/dependency check
        let report = ContractValidator::validate_project_with_profile(root, profile)?;
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

    fn full_integrity_with_profile(
        root: &Path,
        profile: GraceProfile,
    ) -> anyhow::Result<ReviewReport> {
        let mut sections = Self::scoped_gate_with_profile(root, profile)?.sections;

        // 3. Verification integrity
        let layout = DocsLayout::new(root);
        let has_verification = layout.verification_index_path().exists();
        sections.push(ReviewSection {
            name: "verification-plan".into(),
            passed: has_verification,
            details: if has_verification {
                "Sharded verification index exists".into()
            } else {
                "No sharded verification index".into()
            },
            issues: if has_verification {
                vec![]
            } else {
                vec!["Missing docs/verification-index.xml".into()]
            },
        });

        // 4. Graph consistency
        let has_graph = layout.graph_index_path().exists();
        sections.push(ReviewSection {
            name: "knowledge-graph".into(),
            passed: has_graph,
            details: if has_graph {
                "Sharded graph index exists".into()
            } else {
                "No sharded graph index".into()
            },
            issues: if has_graph {
                vec![]
            } else {
                vec!["Missing docs/graph-index.xml".into()]
            },
        });

        let mut shard_issues = Vec::new();
        for required in [
            layout.modules_dir(),
            layout.phases_dir(),
            layout.verification_dir(),
        ] {
            if !required.exists() {
                shard_issues.push(format!("Missing {}", required.display()));
            }
        }
        if has_graph {
            let content = std::fs::read_to_string(layout.graph_index_path()).unwrap_or_default();
            let module_re = regex::Regex::new(r#"path=\"([^\"]+)\""#).map_err(|error| {
                anyhow::anyhow!(
                    "[Reviewer][full_integrity][REGEX] invalid module shard regex: {}",
                    error
                )
            })?;
            for cap in module_re.captures_iter(&content) {
                let shard = root.join(&cap[1]);
                if !shard.exists() {
                    shard_issues.push(format!("Missing shard {}", &cap[1]));
                }
            }
        }
        sections.push(ReviewSection {
            name: "sharded-artifacts".into(),
            passed: shard_issues.is_empty(),
            details: if shard_issues.is_empty() {
                "Primary shard directories and graph references are consistent".into()
            } else {
                format!("{} shard issues", shard_issues.len())
            },
            issues: shard_issues,
        });

        let drift = MyGraceInventory::drift(root)?;
        let drift_issues = canonical_drift_issues(&drift);
        sections.push(ReviewSection {
            name: "canonical-mygrace-drift".into(),
            passed: drift.is_clean(),
            details: if drift.is_clean() {
                format!(
                    "{} code modules are synchronized with graph and verification shards",
                    drift.total_code_modules
                )
            } else {
                format!("{} canonical drift issues", drift.issue_count())
            },
            issues: drift_issues,
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
            if f.path.ends_with("src/grace/review.rs")
                || f.path.ends_with("src/mcp/server.rs")
                || f.path.ends_with("AGENTS.md")
                || f.path.ends_with(".md")
            {
                continue;
            }
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
// END_public_api

fn canonical_drift_issues(drift: &crate::grace::inventory::ArtifactDrift) -> Vec<String> {
    let mut issues = Vec::new();
    issues.extend(
        drift
            .duplicate_graph_ids
            .iter()
            .map(|id| format!("Duplicate graph module id: {}", id)),
    );
    issues.extend(
        drift
            .duplicate_verification_ids
            .iter()
            .map(|id| format!("Duplicate verification id: {}", id)),
    );
    issues.extend(
        drift
            .code_not_in_graph
            .iter()
            .map(|id| format!("Code module missing from graph-index.xml: {}", id)),
    );
    issues.extend(
        drift
            .graph_not_in_code
            .iter()
            .map(|id| format!("Graph entry missing from code contracts: {}", id)),
    );
    issues.extend(
        drift
            .code_not_in_verification
            .iter()
            .map(|id| format!("Code module missing from verification-index.xml: {}", id)),
    );
    issues.extend(
        drift
            .verification_not_in_code
            .iter()
            .map(|id| format!("Verification entry missing from code contracts: {}", id)),
    );
    issues.extend(
        drift
            .files_without_contract
            .iter()
            .map(|path| format!("Governed source file lacks MODULE_CONTRACT: {}", path)),
    );
    issues.extend(
        drift
            .missing_module_shards
            .iter()
            .map(|path| format!("Missing module shard: {}", path)),
    );
    issues.extend(
        drift
            .missing_verification_shards
            .iter()
            .map(|path| format!("Missing verification shard: {}", path)),
    );
    issues.extend(
        drift
            .module_shard_mismatches
            .iter()
            .map(|issue| format!("Module shard mismatch: {}", issue)),
    );
    issues.extend(
        drift
            .verification_shard_mismatches
            .iter()
            .map(|issue| format!("Verification shard mismatch: {}", issue)),
    );
    issues.truncate(50);
    issues
}
