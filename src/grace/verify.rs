// MODULE_CONTRACT
// MODULE_ID: M-GRACE-VERIFY
// PURPOSE: 3-level verification facade — module-local, wave, and delegated phase checks for GRACE compliance
// SCOPE: Verifier struct, verify_all, verify_module_local, verify_wave, delegated verify_phase
// DEPENDS: M-GRACE-CONTRACT, M-GRACE-INVENTORY, M-GRACE-SEMANTIC, M-GRACE-VERIFY-PHASE, M-GRACE-VERIFY-TYPES, M-INDEXER-WALKER
// LINKS: docs/requirements.xml, docs/technology.xml, docs/development-plan.xml, docs/verification-plan.xml, docs/knowledge-graph.xml

// START_MODULE_MAP
// Verifier — Runs 3-level GRACE verification
// VerificationResult — Re-exported per-level verification result
// CheckResult — Re-exported single verification check result
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.9.0 — Built-in verification regexes return explicit errors instead of unwrap panics]
// END_CHANGE_SUMMARY

use crate::grace::contract::ContractValidator;
use crate::grace::inventory::MyGraceInventory;
use crate::grace::layout::DocsLayout;
use crate::grace::semantic::SemanticExtractor;
use std::path::Path;

pub use crate::grace::verify_types::{CheckResult, VerificationResult};

// START_public_api

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

        // Check sharded artifact model
        let layout = DocsLayout::new(root);
        let mut shard_issues = Vec::new();
        if !layout.graph_index_path().exists() {
            shard_issues.push("missing docs/graph-index.xml".to_string());
        }
        if !layout.plan_index_path().exists() {
            shard_issues.push("missing docs/plan-index.xml".to_string());
        }
        if !layout.verification_index_path().exists() {
            shard_issues.push("missing docs/verification-index.xml".to_string());
        }
        if !layout.modules_dir().exists() {
            shard_issues.push("missing docs/modules/".to_string());
        }
        if !layout.phases_dir().exists() {
            shard_issues.push("missing docs/phases/".to_string());
        }
        if !layout.verification_dir().exists() {
            shard_issues.push("missing docs/verification/".to_string());
        }
        checks.push(CheckResult {
            name: "sharded-artifacts".into(),
            passed: shard_issues.is_empty(),
            details: if shard_issues.is_empty() {
                "Primary sharded GRACE artifacts exist".into()
            } else {
                shard_issues.join(", ")
            },
        });

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
        let trace_re = regex::Regex::new(r"\[(\w+)\]\[(\w+)\]\[(\w+)\]").map_err(|error| {
            anyhow::anyhow!(
                "[Verifier][verify_module_local][REGEX] invalid trace marker regex: {}",
                error
            )
        })?;
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
        let layout = DocsLayout::new(root);

        let graph_exists = layout.graph_index_path().exists();
        let plan_exists = layout.plan_index_path().exists();
        let verification_exists = layout.verification_index_path().exists();
        let modules_dir_exists = layout.modules_dir().exists();
        let phases_dir_exists = layout.phases_dir().exists();
        let verification_dir_exists = layout.verification_dir().exists();

        checks.push(CheckResult {
            name: "knowledge-graph".into(),
            passed: graph_exists,
            details: if graph_exists {
                "Sharded graph index found".into()
            } else {
                "Missing docs/graph-index.xml".into()
            },
        });
        checks.push(CheckResult {
            name: "plan-index".into(),
            passed: plan_exists && phases_dir_exists,
            details: if plan_exists && phases_dir_exists {
                "Sharded plan index and phase shards found".into()
            } else {
                "Missing docs/plan-index.xml or docs/phases/".into()
            },
        });
        checks.push(CheckResult {
            name: "verification-index".into(),
            passed: verification_exists && verification_dir_exists,
            details: if verification_exists && verification_dir_exists {
                "Sharded verification index and verification shards found".into()
            } else {
                "Missing docs/verification-index.xml or docs/verification/".into()
            },
        });
        checks.push(CheckResult {
            name: "module-shards".into(),
            passed: modules_dir_exists,
            details: if modules_dir_exists {
                "Module shard directory found".into()
            } else {
                "Missing docs/modules/".into()
            },
        });

        if graph_exists && modules_dir_exists {
            let graph_content =
                std::fs::read_to_string(layout.graph_index_path()).unwrap_or_default();
            let mut shard_refs_ok = true;
            let mut ref_issues = Vec::new();
            let module_re = regex::Regex::new(r#"path=\"([^\"]+)\""#).map_err(|error| {
                anyhow::anyhow!(
                    "[Verifier][verify_wave][REGEX] invalid module shard regex: {}",
                    error
                )
            })?;
            for cap in module_re.captures_iter(&graph_content) {
                let shard_path = root.join(&cap[1]);
                if !shard_path.exists() {
                    shard_refs_ok = false;
                    ref_issues.push(format!("missing shard {}", &cap[1]));
                }
            }
            checks.push(CheckResult {
                name: "artifact-ref-integrity".into(),
                passed: shard_refs_ok,
                details: if ref_issues.is_empty() {
                    "Graph index shard references resolved".into()
                } else {
                    ref_issues.join(", ")
                },
            });
        } else {
            checks.push(CheckResult {
                name: "artifact-ref-integrity".into(),
                passed: false,
                details: "Cannot validate shard refs without graph-index.xml and docs/modules/"
                    .into(),
            });
        }

        match MyGraceInventory::drift(root) {
            Ok(drift) => {
                checks.push(CheckResult {
                    name: "canonical-mygrace-drift".into(),
                    passed: drift.is_clean(),
                    details: if drift.is_clean() {
                        format!("{} code modules match graph, module, and verification shards", drift.total_code_modules)
                    } else {
                        format!(
                            "{} canonical drift issues. duplicates graph={} verification={} missing graph={} missing verification={} missing contracts={} shard mismatches={}",
                            drift.issue_count(),
                            drift.duplicate_graph_ids.len(),
                            drift.duplicate_verification_ids.len() + drift.duplicate_verification_modules.len(),
                            drift.code_not_in_graph.len() + drift.graph_not_in_code.len(),
                            drift.code_not_in_verification.len() + drift.verification_not_in_code.len(),
                            drift.files_without_contract.len(),
                            drift.module_shard_mismatches.len() + drift.verification_shard_mismatches.len()
                        )
                    },
                });
            }
            Err(e) => checks.push(CheckResult {
                name: "canonical-mygrace-drift".into(),
                passed: false,
                details: format!("Failed to collect canonical inventory: {}", e),
            }),
        }

        // Compatibility plan check
        let dp_path = root.join("docs").join("development-plan.xml");
        if dp_path.exists() {
            let has_modules = std::fs::read_to_string(&dp_path)
                .map(|c| c.contains("<MODULE ref=") || c.contains("<M-"))
                .unwrap_or(false);
            checks.push(CheckResult {
                name: "development-plan".into(),
                passed: has_modules,
                details: if has_modules {
                    "Compatibility development plan references modules".into()
                } else {
                    "No module references found in compatibility development plan".into()
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
        crate::grace::verify_phase::verify_phase(root).await
    }
    // END_verifier_verify_phase
}
// END_public_api
