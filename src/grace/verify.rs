// MODULE_CONTRACT
// MODULE_ID: M-GRACE-VERIFY
// PURPOSE: 3-level verification facade — module-local, wave, and delegated phase checks for profile-aware GRACE compliance
// SCOPE: Verifier struct, typed LINKS, structured LOG, belief-state, requirements, technology, development-plan, mental-tests, traceability, non-human patterns and anchor syntax validation, verify_all, verify_all_with_profile, verify_module_local, verify_wave, delegated verify_phase
// DEPENDS: M-GRACE-ANCHOR, M-GRACE-BELIEF-STATE, M-GRACE-CONTRACT, M-GRACE-DEVELOPMENT-PLAN, M-GRACE-MENTAL-TEST, M-GRACE-TRACEABILITY, M-GRACE-NON-HUMAN-PATTERNS, M-GRACE-INVENTORY, M-GRACE-LOG, M-GRACE-REQUIREMENTS, M-GRACE-TECHNOLOGY, M-GRACE-SEMANTIC, M-GRACE-VERIFY-PHASE, M-GRACE-VERIFY-TYPES, M-INDEXER-WALKER
// LINKS: docs/requirements.xml, docs/technology.xml, docs/development-plan.xml, docs/verification-plan.xml, docs/knowledge-graph.xml

// START_MODULE_MAP
// Verifier — Runs 3-level GRACE verification with strict or lightweight profiles
// VerificationResult — Re-exported per-level verification result
// CheckResult — Re-exported single verification check result
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.20.0 — Added non-human programming pattern verification gates]
// END_CHANGE_SUMMARY

use crate::grace::contract::{ContractValidator, GraceProfile};
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
        Self::verify_all_with_profile(root, GraceProfile::Strict).await
    }
    // END_verifier_verify_all

    // START_CONTRACT_Verifier::verify_all_with_profile
    // PURPOSE: Run all verification levels using a selected GRACE strictness profile
    // INPUTS: { root: &Path — project root }, { profile: GraceProfile }
    // OUTPUTS: { anyhow::Result<Vec<VerificationResult>> }
    // START_verifier_verify_all_with_profile
    pub async fn verify_all_with_profile(
        root: &Path,
        profile: GraceProfile,
    ) -> anyhow::Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        results.push(Self::verify_module_local_with_profile(root, profile).await?);
        results.push(Self::verify_wave(root).await?);
        results.push(Self::verify_phase(root).await?);

        Ok(results)
    }
    // END_verifier_verify_all_with_profile

    // START_CONTRACT_Verifier::verify_module_local
    // PURPOSE: Level 1 — per-module checks (contracts, semantic, 500-token, traces)
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<VerificationResult> }
    // START_verifier_verify_module_local
    pub async fn verify_module_local(root: &Path) -> anyhow::Result<VerificationResult> {
        Self::verify_module_local_with_profile(root, GraceProfile::Strict).await
    }
    // END_verifier_verify_module_local

    // START_CONTRACT_Verifier::verify_module_local_with_profile
    // PURPOSE: Level 1 module checks using the selected GRACE strictness profile
    // INPUTS: { root: &Path }, { profile: GraceProfile }
    // OUTPUTS: { anyhow::Result<VerificationResult> }
    // START_verifier_verify_module_local_with_profile
    pub async fn verify_module_local_with_profile(
        root: &Path,
        profile: GraceProfile,
    ) -> anyhow::Result<VerificationResult> {
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
        let report = ContractValidator::validate_project_with_profile(root, profile)?;
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
                    "All contracts have valid PURPOSE and MODULE_ID".into()
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
                passed: total_fn > 0 || profile != GraceProfile::Strict,
                details: if total_fn > 0 {
                    format!(
                        "{} function contracts found under {} profile",
                        total_fn,
                        profile.as_str()
                    )
                } else if profile == GraceProfile::Strict {
                    "No function contracts (START_CONTRACT_name)".into()
                } else {
                    format!(
                        "No function contracts found; {} profile allows module-level contracts for small/non-critical helpers",
                        profile.as_str()
                    )
                },
            });

            let link_report = ContractValidator::validate_links(root, &report);
            checks.push(CheckResult {
                name: "links-valid-types".into(),
                passed: link_report.invalid_type_issues.is_empty(),
                details: if link_report.invalid_type_issues.is_empty() {
                    "All typed LINKS use allowed relationship types".into()
                } else {
                    format!(
                        "{} typed LINKS have invalid relationship types: {:?}",
                        link_report.invalid_type_issues.len(),
                        link_report.invalid_type_issues
                    )
                },
            });
            checks.push(CheckResult {
                name: "links-targets-exist".into(),
                passed: link_report.missing_target_issues.is_empty(),
                details: if link_report.missing_target_issues.is_empty() {
                    "All LINKS declare non-empty targets".into()
                } else {
                    format!(
                        "{} LINKS have empty targets: {:?}",
                        link_report.missing_target_issues.len(),
                        link_report.missing_target_issues
                    )
                },
            });
            checks.push(CheckResult {
                name: "links-no-dangling".into(),
                passed: link_report.dangling_target_issues.is_empty(),
                details: if link_report.dangling_target_issues.is_empty() {
                    "No typed LINKS point to missing known artifacts".into()
                } else {
                    format!(
                        "{} dangling LINKS targets: {:?}",
                        link_report.dangling_target_issues.len(),
                        link_report.dangling_target_issues
                    )
                },
            });
            checks.push(CheckResult {
                name: "links-format".into(),
                passed: true,
                details: if link_report.legacy_format_warnings.is_empty() {
                    "All LINKS use typed directional format".into()
                } else {
                    format!(
                        "{} legacy LINKS entries parsed as depends; migrate to directional typed LINKS",
                        link_report.legacy_format_warnings.len()
                    )
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

        let anchor_report = crate::grace::anchor::anchor_syntax_report(root)?;
        checks.push(CheckResult {
            name: "anchor-syntax-consistent".into(),
            passed: true,
            details: if anchor_report.has_mixed_syntax() {
                format!(
                    "{} files mix XML-like and START/END anchors: {:?}",
                    anchor_report.mixed_files.len(),
                    anchor_report.mixed_files
                )
            } else if anchor_report.xml_anchor_count > 0 {
                format!(
                    "XML-like anchor style used consistently in {} files",
                    anchor_report.xml_files
                )
            } else {
                format!(
                    "Legacy START/END anchor style used consistently in {} files",
                    anchor_report.legacy_files
                )
            },
        });

        // Check profile-aware artifact size guidance (tokens ≈ chars/4).
        // DevelopmentPlan is a structured blueprint with DataFlows and GenerationOrder,
        // so it has a larger budget and is validated by dedicated structural checks below.
        let templates = [
            ("requirements.xml", 2000usize),
            ("technology.xml", 2000usize),
            ("development-plan.xml", 3500usize),
            ("verification-plan.xml", 2000usize),
            ("knowledge-graph.xml", 2000usize),
        ];
        let mut passing = true;
        let mut details = Vec::new();
        for (tpl, token_budget) in &templates {
            let path = root.join("docs").join(tpl);
            if let Ok(content) = std::fs::read_to_string(&path) {
                let token_estimate = content.len() / 4;
                let ok = token_estimate <= *token_budget;
                if !ok {
                    passing = false;
                    details.push(format!(
                        "{} exceeds artifact token budget {} (est. {} tokens)",
                        tpl, token_budget, token_estimate
                    ));
                }
            }
        }
        checks.push(CheckResult {
            name: "500-token-rule".into(),
            passed: passing,
            details: if details.is_empty() {
                format!(
                    "Architecture artifacts within profile-aware size guidance ({})",
                    profile.as_str()
                )
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

        let log_report = crate::grace::log::scan_source_logs(root)?;
        checks.push(CheckResult {
            name: "structured-log-format".into(),
            passed: log_report.passed(),
            details: if log_report.total_logs == 0 {
                "No structured LOG markers found; optional until LDD adoption.".into()
            } else if log_report.passed() {
                format!("{} structured LOG markers valid", log_report.valid_logs)
            } else {
                format!(
                    "{} invalid structured LOG markers, duplicates={:?}, issues={:?}",
                    log_report.invalid_logs, log_report.duplicate_ids, log_report.issues
                )
            },
        });

        let belief_report = crate::grace::belief_state::scan_project_belief_states(root)?;
        checks.push(CheckResult {
            name: "belief-state-exists".into(),
            passed: belief_report.passed(),
            details: if belief_report.total_modules == 0 {
                "No MODULE_CONTRACT modules found for belief state coverage".into()
            } else if belief_report.invalid_states > 0 {
                format!(
                    "{} invalid BELIEF_STATE blocks: {:?}",
                    belief_report.invalid_states, belief_report.issues
                )
            } else if belief_report.states_found == belief_report.total_modules {
                format!(
                    "Belief states cover all {} modules",
                    belief_report.total_modules
                )
            } else {
                format!(
                    "{}/{} modules have belief states ({:.1}%); adoption pending for {} modules",
                    belief_report.states_found,
                    belief_report.total_modules,
                    belief_report.coverage_pct,
                    belief_report.missing_modules.len()
                )
            },
        });

        let requirements = crate::grace::requirements::validate_requirements(root)?;
        checks.push(CheckResult {
            name: "requirements-entities-defined".into(),
            passed: requirements.has_entities_defined(),
            details: format!(
                "{} entities parsed from RequirementsAnalysis",
                requirements.entities.len()
            ),
        });
        checks.push(CheckResult {
            name: "requirements-use-cases".into(),
            passed: requirements.has_use_cases(),
            details: format!(
                "{} use cases parsed; AAG complete={}",
                requirements.use_cases.len(),
                requirements.has_use_cases()
            ),
        });
        checks.push(CheckResult {
            name: "requirements-glossary".into(),
            passed: requirements.has_glossary(),
            details: format!(
                "{} glossary terms; entity terms matched={}",
                requirements.glossary_terms.len(),
                requirements.has_glossary()
            ),
        });
        checks.push(CheckResult {
            name: "requirements-no-empty-sections".into(),
            passed: requirements.has_no_empty_sections(),
            details: if requirements.valid {
                "RequirementsAnalysis sections are populated".into()
            } else {
                format!("RequirementsAnalysis errors: {:?}", requirements.errors)
            },
        });

        let technology = crate::grace::technology::validate_technology(root)?;
        checks.push(CheckResult {
            name: "technology-language-defined".into(),
            passed: technology.has_language_defined(),
            details: format!(
                "{} language entries parsed from Technology",
                technology.languages.len()
            ),
        });
        checks.push(CheckResult {
            name: "technology-dependencies-compatible".into(),
            passed: technology.dependencies_compatible(),
            details: if technology.dependencies_compatible() {
                format!(
                    "{} compatibility checks passed",
                    technology.compatibility_checks.len()
                )
            } else {
                format!(
                    "Technology incompatibilities: {:?}",
                    technology.incompatible_checks
                )
            },
        });
        checks.push(CheckResult {
            name: "technology-no-version-guessing".into(),
            passed: technology.has_no_version_guessing(),
            details: if technology.has_no_version_guessing() {
                format!(
                    "{} versioned components use exact versions",
                    technology.components.len()
                )
            } else {
                format!(
                    "Technology version issues: {:?}",
                    technology.missing_versions
                )
            },
        });
        checks.push(CheckResult {
            name: "technology-known-issues".into(),
            passed: technology.has_known_issues(),
            details: format!("{} known issue entries documented", technology.known_issues),
        });

        let plan = crate::grace::development_plan::validate_development_plan(root)?;
        checks.push(CheckResult {
            name: "dataflow-sources-exist".into(),
            passed: plan.dataflow_sources_exist(),
            details: if plan.dataflow_sources_exist() {
                format!(
                    "{} DataFlows have resolvable endpoints",
                    plan.data_flows.len()
                )
            } else {
                format!(
                    "DataFlow endpoint issues: {:?}",
                    plan.missing_dataflow_sources
                )
            },
        });
        checks.push(CheckResult {
            name: "dataflow-contracts-exist".into(),
            passed: plan.dataflow_contracts_exist(),
            details: if plan.dataflow_contracts_exist() {
                format!("{} DataFlow contract refs resolved", plan.data_flows.len())
            } else {
                format!(
                    "DataFlow contract issues: {:?}",
                    plan.missing_dataflow_contracts
                )
            },
        });
        checks.push(CheckResult {
            name: "dataflow-protocol-consistent".into(),
            passed: plan.dataflow_protocol_consistent(),
            details: if plan.dataflow_protocol_consistent() {
                "All DataFlows declare protocols".into()
            } else {
                format!("DataFlow protocol issues: {:?}", plan.protocol_issues)
            },
        });
        checks.push(CheckResult {
            name: "dataflow-errors-documented".into(),
            passed: plan.dataflow_errors_documented(),
            details: if plan.dataflow_errors_documented() {
                "All DataFlows document error handling".into()
            } else {
                format!(
                    "DataFlow error handling issues: {:?}",
                    plan.error_handling_issues
                )
            },
        });
        checks.push(CheckResult {
            name: "genorder-topology-correct".into(),
            passed: plan.genorder_topology_correct(),
            details: if plan.genorder_topology_correct() {
                format!(
                    "{} GenerationOrder modules are topologically ordered",
                    plan.generation_modules.len()
                )
            } else {
                format!("GenerationOrder issues: {:?}", plan.generation_order_issues)
            },
        });
        checks.push(CheckResult {
            name: "genorder-all-modules".into(),
            passed: plan.genorder_all_modules(),
            details: format!(
                "{} architecture modules, {} generated modules",
                plan.architecture_modules.len(),
                plan.generation_modules.len()
            ),
        });
        checks.push(CheckResult {
            name: "genorder-no-dangling-deps".into(),
            passed: plan.genorder_no_dangling_deps(),
            details: if plan.genorder_no_dangling_deps() {
                "All GenerationOrder DependsOn refs resolve to known modules".into()
            } else {
                format!(
                    "GenerationOrder dependency issues: {:?}",
                    plan.generation_order_issues
                )
            },
        });

        let mental_tests = crate::grace::mental_test::scan_project_mental_tests(root)?;
        checks.push(CheckResult {
            name: "mental-tests-defined".into(),
            passed: mental_tests.mental_tests_defined(),
            details: if mental_tests.mental_tests_defined() {
                format!(
                    "{} mental tests cover {} required targets",
                    mental_tests.total,
                    mental_tests.required_targets.len()
                )
            } else {
                format!(
                    "Missing mental tests for critical targets: {:?}",
                    mental_tests.missing_required_targets
                )
            },
        });
        checks.push(CheckResult {
            name: "mental-tests-passed".into(),
            passed: mental_tests.mental_tests_passed(),
            details: if mental_tests.mental_tests_passed() {
                format!(
                    "{} passed, {} total",
                    mental_tests.passed, mental_tests.total
                )
            } else {
                format!(
                    "Mental test failures: failed={} not_run={} needs_clarification={} errors={:?}",
                    mental_tests.failed,
                    mental_tests.not_run,
                    mental_tests.needs_clarification,
                    mental_tests.errors
                )
            },
        });
        checks.push(CheckResult {
            name: "mental-test-no-drift".into(),
            passed: mental_tests.mental_test_no_drift(),
            details: if mental_tests.mental_test_no_drift() {
                "Mental test targets resolve to current source modules".into()
            } else {
                format!("Mental test drift issues: {:?}", mental_tests.drift_issues)
            },
        });

        let traceability = crate::grace::traceability::scan_project_traceability(root)?;
        checks.push(CheckResult {
            name: "traceability-requirements-implemented".into(),
            passed: traceability.requirements_implemented_gate(),
            details: if traceability.untraced_requirements.is_empty() {
                format!(
                    "{} requirements have implementing trace links",
                    traceability.requirements_total
                )
            } else {
                format!(
                    "{} untraced requirements under {} enforcement: {:?}",
                    traceability.untraced_requirements.len(),
                    traceability.enforcement_mode,
                    traceability.untraced_requirements
                )
            },
        });
        checks.push(CheckResult {
            name: "traceability-code-traced".into(),
            passed: traceability.code_traced_gate(),
            details: if traceability.untraced_functions.is_empty() {
                format!(
                    "{}/{} function contracts trace to requirements or use cases",
                    traceability.functions_with_traceability, traceability.total_functions
                )
            } else {
                format!(
                    "{}/{} functions traced; {} gaps under {} enforcement",
                    traceability.functions_with_traceability,
                    traceability.total_functions,
                    traceability.untraced_functions.len(),
                    traceability.enforcement_mode
                )
            },
        });
        checks.push(CheckResult {
            name: "traceability-logs-traced".into(),
            passed: traceability.logs_traced_gate(),
            details: if traceability.total_logs == 0 {
                "No structured LOG markers found; TRACEABILITY evidence adoption pending".into()
            } else {
                format!(
                    "{}/{} structured LOG markers include TRACEABILITY fields",
                    traceability.logs_with_traceability, traceability.total_logs
                )
            },
        });
        checks.push(CheckResult {
            name: "traceability-no-dangling".into(),
            passed: traceability.no_dangling_gate(),
            details: if traceability.no_dangling_gate() {
                format!(
                    "Traceability links resolve across {} chains; score {:.1}%",
                    traceability.chains.len(),
                    traceability.traceability_score * 100.0
                )
            } else {
                let dangling = traceability
                    .gaps
                    .iter()
                    .filter(|gap| gap.gap_type == "dangling_traceability_link")
                    .count();
                format!("{} dangling traceability links detected", dangling)
            },
        });

        let patterns = crate::grace::non_human_patterns::check_project_patterns(root, profile)?;
        for check_name in [
            "non-human-explicit-typing",
            "non-human-explicit-flow",
            "non-human-explicit-null",
            "non-human-no-magic-values",
            "non-human-deterministic-iter",
        ] {
            let enabled = patterns
                .patterns_checked
                .iter()
                .any(|check| check.pattern_name == check_name);
            let pattern_violations = patterns.pattern_violations(check_name);
            let errors = pattern_violations
                .iter()
                .filter(|violation| violation.severity.is_error())
                .count();
            let warnings = pattern_violations.len().saturating_sub(errors);
            checks.push(CheckResult {
                name: check_name.into(),
                passed: !enabled || errors == 0,
                details: if enabled {
                    format!(
                        "profile={} errors={} warnings={} score={:.1}%",
                        patterns.profile,
                        errors,
                        warnings,
                        patterns.score * 100.0
                    )
                } else {
                    format!(
                        "profile={} does not enforce {}; check skipped",
                        patterns.profile, check_name
                    )
                },
            });
        }

        let passed = checks.iter().all(|c| c.passed);
        Ok(VerificationResult {
            level: "module-local".into(),
            passed,
            checks,
        })
    }
    // END_verifier_verify_module_local_with_profile

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
                .map(|c| {
                    c.contains("<MODULE ref=") || c.contains("<Module id=\"M-") || c.contains("<M-")
                })
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
