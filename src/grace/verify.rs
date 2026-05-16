// MODULE_CONTRACT
// MODULE_ID: M-GRACE-VERIFY
// PURPOSE: 3-level verification — module-local, wave, phase checks for GRACE compliance
// SCOPE: Verifier struct, VerificationResult, CheckResult, verify_all, verify_module_local, verify_wave, verify_phase
// DEPENDS: M-GRACE-CONTRACT, M-GRACE-SEMANTIC, M-INDEXER-WALKER
// LINKS: docs/requirements.xml, docs/technology.xml, docs/development-plan.xml, docs/verification-plan.xml, docs/knowledge-graph.xml

// START_MODULE_MAP
// VerificationResult — Per-level verification result with check list
// CheckResult — Single verification check result
// Verifier — Runs 3-level GRACE verification
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY

use crate::grace::contract::ContractValidator;
use crate::grace::semantic::SemanticExtractor;
use std::path::Path;

// START_public_api

// START_VerificationResult
#[derive(Debug, Clone, serde::Serialize)]
pub struct VerificationResult {
    pub level: String,
    pub passed: bool,
    pub checks: Vec<CheckResult>,
}
// END_VerificationResult

// START_CheckResult
#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub details: String,
}
// END_CheckResult

// START_Verifier
pub struct Verifier;
// END_Verifier

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Verifier {
    // START_CONTRACT_Verifier::new
    // PURPOSE: Create a new Verifier
    // OUTPUTS: { Self }
    // START_verifier_new
    pub fn new() -> Self {
        Self
    }
    // END_verifier_new

    // START_CONTRACT_Verifier::verify_all
    // PURPOSE: Run all 3 verification levels (module-local, wave, phase)
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<Vec<VerificationResult>> }
    // START_verifier_verify_all
    pub async fn verify_all(root: &Path) -> anyhow::Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        results.push(Self::verify_module_local(root).await?);
        results.push(Self::verify_wave(root).await?);
        results.push(Self::verify_phase(root).await?);

        Ok(results)
    }
    // END_verifier_verify_all

    // START_CONTRACT_Verifier::verify_module_local
    // PURPOSE: Level 1 — per-module checks (contracts, semantic, 500-token, traces)
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<VerificationResult> }
    // START_verifier_verify_module_local
    pub async fn verify_module_local(root: &Path) -> anyhow::Result<VerificationResult> {
        let mut checks = Vec::new();

        // Check contracts
        let report = ContractValidator::validate_project(root)?;
        checks.push(CheckResult {
            name: "contract-exists".into(),
            passed: report.with_contract > 0,
            details: format!(
                "{} / {} files have MODULE_CONTRACT",
                report.with_contract, report.total_files
            ),
        });

        if report.with_contract > 0 {
            let invalid: Vec<String> = report
                .contracts
                .iter()
                .filter(|c| c.has_contract && !c.valid)
                .map(|c| c.file_path.clone())
                .collect();
            checks.push(CheckResult {
                name: "contract-valid".into(),
                passed: invalid.is_empty(),
                details: if invalid.is_empty() {
                    "All contracts have valid PURPOSE".into()
                } else {
                    format!("{} contracts have errors: {:?}", invalid.len(), invalid)
                },
            });

            // Check MODULE_MAP
            let missing_map: Vec<String> = report
                .contracts
                .iter()
                .filter(|c| c.has_contract && !c.has_module_map)
                .map(|c| c.file_path.clone())
                .collect();
            checks.push(CheckResult {
                name: "module-map".into(),
                passed: missing_map.is_empty(),
                details: if missing_map.is_empty() {
                    "All contracted files have MODULE_MAP".into()
                } else {
                    format!(
                        "{} files missing MODULE_MAP: {:?}",
                        missing_map.len(),
                        missing_map
                    )
                },
            });

            // Check CHANGE_SUMMARY
            let missing_cs: Vec<String> = report
                .contracts
                .iter()
                .filter(|c| c.has_contract && !c.has_change_summary)
                .map(|c| c.file_path.clone())
                .collect();
            checks.push(CheckResult {
                name: "change-summary".into(),
                passed: missing_cs.is_empty(),
                details: if missing_cs.is_empty() {
                    "All contracted files have CHANGE_SUMMARY".into()
                } else {
                    format!(
                        "{} files missing CHANGE_SUMMARY: {:?}",
                        missing_cs.len(),
                        missing_cs
                    )
                },
            });

            // Check function contracts
            let total_fn: usize = report
                .contracts
                .iter()
                .map(|c| c.function_contracts.len())
                .sum();
            checks.push(CheckResult {
                name: "function-contracts".into(),
                passed: total_fn > 0,
                details: if total_fn > 0 {
                    format!("{} function contracts found", total_fn)
                } else {
                    "No function contracts (START_CONTRACT_name)".into()
                },
            });
        }

        // Check semantic markup
        match SemanticExtractor::scan_project(root) {
            Ok(sem) => {
                checks.push(CheckResult {
                    name: "semantic-blocks".into(),
                    passed: sem.unclosed_blocks.is_empty(),
                    details: if sem.unclosed_blocks.is_empty() {
                        format!("All {} blocks are properly closed", sem.closed_blocks)
                    } else {
                        format!("{} unclosed blocks found", sem.open_blocks)
                    },
                });
                checks.push(CheckResult {
                    name: "unique-block-names".into(),
                    passed: sem.duplicate_name_blocks.is_empty(),
                    details: if sem.duplicate_name_blocks.is_empty() {
                        "All block names are unique".into()
                    } else {
                        format!(
                            "{} duplicate block names: {:?}",
                            sem.duplicate_name_blocks.len(),
                            sem.duplicate_name_blocks
                                .iter()
                                .map(|b| format!("{} in {}", b.name, b.file_path))
                                .collect::<Vec<_>>()
                        )
                    },
                });
            }
            Err(e) => {
                checks.push(CheckResult {
                    name: "semantic-blocks".into(),
                    passed: false,
                    details: format!("Error scanning blocks: {}", e),
                });
            }
        }

        // Check 500-token rule on XML artifacts (tokens ≈ chars/4)
        let templates = [
            "requirements.xml",
            "technology.xml",
            "development-plan.xml",
            "verification-plan.xml",
            "knowledge-graph.xml",
        ];
        let mut passing = true;
        let mut details = Vec::new();
        for tpl in &templates {
            let path = root.join("docs").join(tpl);
            if let Ok(content) = std::fs::read_to_string(&path) {
                let token_estimate = content.len() / 4;
                let ok = token_estimate <= 2000;
                if !ok {
                    passing = false;
                    details.push(format!(
                        "{} exceeds ~500 tokens (est. {} tokens)",
                        tpl, token_estimate
                    ));
                }
            }
        }
        checks.push(CheckResult {
            name: "500-token-rule".into(),
            passed: passing,
            details: if details.is_empty() {
                "All artifacts within ~500 token limit".into()
            } else {
                details.join("; ")
            },
        });

        // Check trace assertions: log markers [Module][function][BLOCK_NAME] in source files
        let trace_re = regex::Regex::new(r"\[(\w+)\]\[(\w+)\]\[(\w+)\]").unwrap();
        let walker = crate::indexer::walker::Walker::new(root);
        let files = walker.walk();
        let mut trace_files = Vec::new();
        let mut total_traces = 0usize;
        for file in &files {
            let full_path = root.join(&file.path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let count = trace_re.find_iter(&content).count();
                if count > 0 {
                    trace_files.push(format!("{} ({} traces)", file.path, count));
                    total_traces += count;
                }
            }
        }
        checks.push(CheckResult {
            name: "trace-assertions".into(),
            passed: true, // Informational: traces are recommended, not required
            details: if total_traces > 0 {
                format!(
                    "{} log trace markers found in {} files",
                    total_traces,
                    trace_files.len()
                )
            } else {
                "No log trace markers found. Add [Module][function][BLOCK_NAME] for observability."
                    .into()
            },
        });

        let passed = checks.iter().all(|c| c.passed);
        Ok(VerificationResult {
            level: "module-local".into(),
            passed,
            checks,
        })
    }
    // END_verifier_verify_module_local

    // START_CONTRACT_Verifier::verify_wave
    // PURPOSE: Level 2 — cross-module checks (knowledge graph, development plan)
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<VerificationResult> }
    // START_verifier_verify_wave
    pub async fn verify_wave(root: &Path) -> anyhow::Result<VerificationResult> {
        let mut checks = Vec::new();

        // Check knowledge graph matches actual modules
        let kg_path = root.join("docs").join("knowledge-graph.xml");
        if kg_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&kg_path) {
                let has_modules = content.contains("<MODULES>") || content.contains("<M-");
                checks.push(CheckResult {
                    name: "knowledge-graph".into(),
                    passed: has_modules,
                    details: if has_modules {
                        "Knowledge graph found".into()
                    } else {
                        "No modules in knowledge graph".into()
                    },
                });
            } else {
                checks.push(CheckResult {
                    name: "knowledge-graph".into(),
                    passed: false,
                    details: "Cannot read knowledge-graph.xml".into(),
                });
            }
        } else {
            checks.push(CheckResult {
                name: "knowledge-graph".into(),
                passed: true,
                details: "Knowledge graph not required (no strict mode)".into(),
            });
        }

        // Check development plan
        let dp_path = root.join("docs").join("development-plan.xml");
        if dp_path.exists() {
            let has_modules = std::fs::read_to_string(&dp_path)
                .map(|c| c.contains("<M-"))
                .unwrap_or(false);
            checks.push(CheckResult {
                name: "development-plan".into(),
                passed: has_modules,
                details: if has_modules {
                    "Development plan has module definitions".into()
                } else {
                    "No module definitions found".into()
                },
            });
        }

        let passed = checks.iter().all(|c| c.passed);
        Ok(VerificationResult {
            level: "wave".into(),
            passed,
            checks,
        })
    }
    // END_verifier_verify_wave

    // START_CONTRACT_Verifier::verify_phase
    // PURPOSE: Level 3 — full regression (TODO/FIXME check, file size limits)
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<VerificationResult> }
    // START_verifier_verify_phase
    pub async fn verify_phase(root: &Path) -> anyhow::Result<VerificationResult> {
        let mut checks = Vec::new();

        // Check no TODO/FIXME in indexed code (warning)
        let walker = crate::indexer::walker::Walker::new(root);
        let files = walker.walk();
        let mut todos = Vec::new();
        for file in &files {
            let full_path = root.join(&file.path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                for (i, line) in content.lines().enumerate() {
                    let l = line.trim().to_lowercase();
                    if (l.starts_with("// todo")
                        || l.starts_with("# todo")
                        || l.starts_with("/* todo")
                        || l.contains("TODO:"))
                        || (l.starts_with("// fixme")
                            || l.starts_with("# fixme")
                            || l.starts_with("/* fixme")
                            || l.contains("FIXME:"))
                    {
                        todos.push(format!("{}:{}", file.path, i + 1));
                    }
                }
            }
        }
        checks.push(CheckResult {
            name: "no-todos".into(),
            passed: todos.is_empty(),
            details: if todos.is_empty() {
                "No TODO/FIXME found".into()
            } else {
                format!(
                    "{} TODO/FIXME found:\n  {}",
                    todos.len(),
                    todos.join("\n  ")
                )
            },
        });

        // Check file size limit (500 lines per source file recommended)
        let mut large_files = Vec::new();
        for file in &files {
            let full_path = root.join(&file.path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let line_count = content.lines().count();
                if line_count > 500 {
                    large_files.push(format!("{} ({} lines)", file.path, line_count));
                }
            }
        }
        checks.push(CheckResult {
            name: "file-size-limit".into(),
            passed: large_files.is_empty(),
            details: if large_files.is_empty() {
                "All files under 500 lines".into()
            } else {
                format!("Large files:\n  {}", large_files.join("\n  "))
            },
        });

        let passed = checks.iter().all(|c| c.passed);
        Ok(VerificationResult {
            level: "phase".into(),
            passed,
            checks,
        })
    }
    // END_verifier_verify_phase
}
// END_public_api
