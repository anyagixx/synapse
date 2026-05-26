// MODULE_CONTRACT
// MODULE_ID: M-TEST-COVERAGE-MATRIX
// PURPOSE: Build index-first evidence and code coverage reports for Synapse test visibility.
// SCOPE: Index-based module discovery, evidence counting, cargo-tarpaulin JSON execution/parsing, module source mapping, table output, JSON output, threshold reporting, and uncovered module reporting.
// DEPENDS: M-GRACE-LOG, M-GRACE-MENTAL-TEST, M-GRACE-VERIFY, M-GRACE-LAYOUT
// LINKS:
//   -> Phase-79 (implements) - coverage matrix
//   -> NFR-003 (traces_to) - coverage reporting must stay index-first and token-bounded
//   <- V-M-TEST-COVERAGE-MATRIX (verified_by) - coverage evidence verification

// START_MODULE_MAP
// CoverageMatrix - Project coverage report with summary and per-module rows
// ModuleCoverage - Per-module evidence and coverage status
// CoverageEvidence - Evidence booleans for contract, verification, mental tests, guides, LOGs, and verify status
// CodeCoverageReport - cargo-tarpaulin code coverage summary mapped to modules
// ModuleCodeCoverage - Per-module line/function/branch coverage row
// build_coverage_matrix - Builds the coverage matrix using graph-index first
// measure_code_coverage - Runs cargo tarpaulin and parses JSON output
// render_coverage_output - Renders evidence-only or combined evidence/code coverage output
// render_coverage_table - Renders a bounded human-readable coverage table
// render_coverage_json - Renders stable JSON for automation
// tarpaulin_path_value - Normalizes real cargo-tarpaulin path field variants
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 - Added Phase-94 cargo-tarpaulin code coverage integration]
// END_CHANGE_SUMMARY

use crate::grace::inventory_artifacts::{parse_graph_index, parse_verification_index};
use crate::grace::inventory_types::{GraphEntry, VerificationEntry};
use crate::grace::layout::DocsLayout;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const COVERED_EVIDENCE_THRESHOLD: usize = 3;
const DEFAULT_CODE_COVERAGE_THRESHOLD: f64 = 65.0;

// START_public_api

// START_CoverageMatrix
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CoverageMatrix {
    pub root: String,
    pub summary: CoverageSummary,
    pub modules: Vec<ModuleCoverage>,
}
// END_CoverageMatrix

// START_CoverageSummary
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CoverageSummary {
    pub total_modules: usize,
    pub covered_modules: usize,
    pub partial_modules: usize,
    pub uncovered_modules: usize,
    pub evidence_threshold: usize,
    pub evidence_types: Vec<&'static str>,
}
// END_CoverageSummary

// START_ModuleCoverage
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ModuleCoverage {
    pub module_id: String,
    pub graph_status: String,
    pub module_shard: String,
    pub source_files: Vec<String>,
    pub evidence: CoverageEvidence,
    pub evidence_count: usize,
    pub status: CoverageStatus,
}
// END_ModuleCoverage

// START_CoverageEvidence
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct CoverageEvidence {
    pub contract: bool,
    pub verification: bool,
    pub verify_status: bool,
    pub mental_test: bool,
    pub test_guide: bool,
    pub log_marker: bool,
}
// END_CoverageEvidence

// START_CoverageStatus
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CoverageStatus {
    Covered,
    Partial,
    Uncovered,
}
// END_CoverageStatus

// START_CodeCoverageReport
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CodeCoverageReport {
    pub overall_pct: f64,
    pub by_module: Vec<ModuleCodeCoverage>,
    pub threshold_pct: f64,
    pub passed: bool,
}
// END_CodeCoverageReport

// START_ModuleCodeCoverage
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ModuleCodeCoverage {
    pub module_id: String,
    pub source_file: String,
    pub line_pct: f64,
    pub function_pct: f64,
    pub branch_pct: f64,
    pub uncovered_functions: Vec<String>,
    pub below_threshold: bool,
}
// END_ModuleCodeCoverage

impl CoverageEvidence {
    // START_CONTRACT_CoverageEvidence::count
    // PURPOSE: Count true evidence types for a module
    // OUTPUTS: { usize }
    // LINKS:
    //   -> NFR-003 (traces_to) - compact coverage scoring
    // START_coverage_evidence_count
    pub fn count(&self) -> usize {
        [
            self.contract,
            self.verification,
            self.verify_status,
            self.mental_test,
            self.test_guide,
            self.log_marker,
        ]
        .into_iter()
        .filter(|present| *present)
        .count()
    }
    // END_coverage_evidence_count
}

impl CoverageStatus {
    // START_CONTRACT_CoverageStatus::from_count
    // PURPOSE: Classify a module coverage status from its evidence count
    // INPUTS: { count: usize }
    // OUTPUTS: { CoverageStatus }
    // LINKS:
    //   -> V-M-TEST-COVERAGE-MATRIX (verified_by) - threshold classification tests
    //   -> NFR-003 (traces_to) - evidence threshold keeps output compact
    // START_coverage_status_from_count
    pub fn from_count(count: usize) -> Self {
        if count >= COVERED_EVIDENCE_THRESHOLD {
            Self::Covered
        } else if count > 0 {
            Self::Partial
        } else {
            Self::Uncovered
        }
    }
    // END_coverage_status_from_count

    // START_CONTRACT_CoverageStatus::as_str
    // PURPOSE: Return stable text for coverage table output
    // OUTPUTS: { &'static str }
    // LINKS:
    //   -> NFR-003 (traces_to) - stable status text supports compact automation
    // START_coverage_status_as_str
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Covered => "covered",
            Self::Partial => "partial",
            Self::Uncovered => "uncovered",
        }
    }
    // END_coverage_status_as_str
}

// START_CONTRACT_build_coverage_matrix
// PURPOSE: Build module coverage rows by reading graph-index before any per-module shard
// INPUTS: { root: &Path }
// OUTPUTS: { anyhow::Result<CoverageMatrix> }
// LINKS:
//   -> M-GRACE-LAYOUT (depends) - sharded artifact paths
//   -> M-GRACE-VERIFY (depends) - verification shard/status evidence
//   -> M-GRACE-MENTAL-TEST (depends) - mental test artifact evidence
//   -> M-GRACE-LOG (depends) - structured LOG marker evidence
//   -> NFR-003 (traces_to) - index-first scan avoids monolithic artifact loading
// <LOG id="coverage_matrix_built" level="INFO" ref="coverage-build" module="M-TEST-COVERAGE-MATRIX" contract="build_coverage_matrix">
//   EVENT: coverage_matrix_built
//   EXPECTATION: coverage scan starts from graph-index and reads only referenced module shards
//   DECISION: orphan shards are ignored until the index references them
//   RESULT: success
//   TRACEABILITY: NFR-003
// </LOG>
// START_build_coverage_matrix
pub fn build_coverage_matrix(root: &Path) -> anyhow::Result<CoverageMatrix> {
    let layout = DocsLayout::new(root);
    let graph_entries = parse_graph_index(&layout.graph_index_path());
    let verification_by_module = verification_index_by_module(&layout.verification_index_path());
    let mental_test_text = collect_text_under(&layout.mental_tests_dir())?
        + &std::fs::read_to_string(root.join("docs").join("development-plan.xml"))
            .unwrap_or_default();
    let guide_text = collect_text_under(&layout.tests_guides_dir())?;

    let mut modules = Vec::new();
    for entry in graph_entries {
        modules.push(build_module_coverage(
            root,
            &entry,
            verification_by_module.get(&entry.id),
            &mental_test_text,
            &guide_text,
        )?);
    }
    let summary = summarize_modules(&modules);
    Ok(CoverageMatrix {
        root: root.display().to_string(),
        summary,
        modules,
    })
}
// END_build_coverage_matrix

// START_CONTRACT_render_coverage_table
// PURPOSE: Render a compact coverage table for humans
// INPUTS: { matrix: &CoverageMatrix }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-003 (traces_to) - output remains bounded and scan-friendly
// START_render_coverage_table
pub fn render_coverage_table(matrix: &CoverageMatrix) -> String {
    let mut lines = vec![
        format!(
            "Coverage matrix: {}/{} covered, {} partial, {} uncovered",
            matrix.summary.covered_modules,
            matrix.summary.total_modules,
            matrix.summary.partial_modules,
            matrix.summary.uncovered_modules
        ),
        "module | status | evidence | C V S M G L".to_string(),
    ];
    for module in &matrix.modules {
        lines.push(format!(
            "{} | {} | {} | {} {} {} {} {} {}",
            module.module_id,
            module.status.as_str(),
            module.evidence_count,
            marker(module.evidence.contract),
            marker(module.evidence.verification),
            marker(module.evidence.verify_status),
            marker(module.evidence.mental_test),
            marker(module.evidence.test_guide),
            marker(module.evidence.log_marker)
        ));
    }
    lines.join("\n")
}
// END_render_coverage_table

// START_CONTRACT_render_coverage_json
// PURPOSE: Render coverage matrix as stable pretty JSON
// INPUTS: { matrix: &CoverageMatrix }
// OUTPUTS: { anyhow::Result<String> }
// LINKS:
//   -> V-M-TEST-COVERAGE-MATRIX (verified_by) - JSON output contract
//   -> NFR-003 (traces_to) - JSON output supports token-efficient automation
// START_render_coverage_json
pub fn render_coverage_json(matrix: &CoverageMatrix) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(matrix)?)
}
// END_render_coverage_json

// START_CONTRACT_measure_code_coverage
// PURPOSE: Run cargo-tarpaulin and parse its JSON report into module-level code coverage.
// INPUTS: { root: &Path }
// OUTPUTS: { anyhow::Result<CodeCoverageReport> }
// SIDE_EFFECTS: executes cargo tarpaulin and writes coverage/tarpaulin-report.json
// LINKS:
//   -> Phase-94 (implements) - code coverage gate
//   -> NFR-002 (traces_to) - release verification command must fail clearly
// START_measure_code_coverage
pub fn measure_code_coverage(root: &Path) -> anyhow::Result<CodeCoverageReport> {
    let output = Command::new("cargo")
        .args([
            "tarpaulin",
            "--out",
            "Json",
            "--output-dir",
            "coverage",
            "--exclude-files",
            "tests/*",
            "--exclude-files",
            "benches/*",
        ])
        .current_dir(root)
        .output()
        .map_err(|error| anyhow::anyhow!("failed to run cargo tarpaulin: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("cargo-tarpaulin failed: {}", stderr.trim());
    }
    let report_path = root.join("coverage").join("tarpaulin-report.json");
    parse_code_coverage_report(
        root,
        &std::fs::read_to_string(&report_path)?,
        DEFAULT_CODE_COVERAGE_THRESHOLD,
    )
}
// END_measure_code_coverage

// START_CONTRACT_render_coverage_output
// PURPOSE: Render evidence coverage alone or combined with optional cargo-tarpaulin code coverage.
// INPUTS: { root: &Path }, { json_output: bool }, { include_code: bool }
// OUTPUTS: { anyhow::Result<String> }
// LINKS:
//   -> M-CLI-TEST-COMMANDS (depends) - syn test coverage rendering
//   -> Phase-94 (implements) - syn test coverage --code
// START_render_coverage_output
pub fn render_coverage_output(
    root: &Path,
    json_output: bool,
    include_code: bool,
) -> anyhow::Result<String> {
    let matrix = build_coverage_matrix(root)?;
    if !include_code {
        return if json_output {
            render_coverage_json(&matrix)
        } else {
            Ok(render_coverage_table(&matrix))
        };
    }
    match measure_code_coverage(root) {
        Ok(report) => render_combined_coverage(&matrix, Some(report), None, json_output),
        Err(error) => render_combined_coverage(
            &matrix,
            None,
            Some(format!(
                "{}. Install cargo-tarpaulin with: cargo install cargo-tarpaulin",
                error
            )),
            json_output,
        ),
    }
}
// END_render_coverage_output

// END_public_api

// START_CONTRACT_build_module_coverage
// PURPOSE: Build one module coverage row from an indexed graph entry
// INPUTS: { root: &Path }, { entry: &GraphEntry }, { verification: Option<&VerificationEntry> }, { mental_test_text: &str }, { guide_text: &str }
// OUTPUTS: { anyhow::Result<ModuleCoverage> }
// LINKS:
//   -> M-GRACE-LAYOUT (depends) - module shard path resolution
//   -> NFR-003 (traces_to) - module rows are built from indexed references only
// START_build_module_coverage
fn build_module_coverage(
    root: &Path,
    entry: &GraphEntry,
    verification: Option<&VerificationEntry>,
    mental_test_text: &str,
    guide_text: &str,
) -> anyhow::Result<ModuleCoverage> {
    let module_shard_path = root.join(&entry.path);
    let source_files = module_source_files(&module_shard_path)?;
    let evidence = CoverageEvidence {
        contract: source_files
            .iter()
            .any(|source_file| source_declares_module(root, source_file, &entry.id)),
        verification: verification
            .map(|entry| root.join(&entry.path).exists())
            .unwrap_or(false),
        verify_status: verification
            .map(|entry| matches!(entry.status.as_str(), "active" | "done" | "passed"))
            .unwrap_or(false),
        mental_test: contains_module_ref(mental_test_text, &entry.id),
        test_guide: contains_module_ref(guide_text, &entry.id),
        log_marker: source_files
            .iter()
            .any(|source_file| source_has_log_marker(root, source_file, &entry.id)),
    };
    let evidence_count = evidence.count();
    let status = CoverageStatus::from_count(evidence_count);
    Ok(ModuleCoverage {
        module_id: entry.id.clone(),
        graph_status: entry.status.clone(),
        module_shard: entry.path.clone(),
        source_files,
        evidence,
        evidence_count,
        status,
    })
}
// END_build_module_coverage

// START_CONTRACT_verification_index_by_module
// PURPOSE: Parse verification-index entries keyed by module id
// INPUTS: { path: &Path }
// OUTPUTS: { BTreeMap<String, VerificationEntry> }
// LINKS:
//   -> M-GRACE-VERIFY (depends) - verification index evidence
//   -> NFR-003 (traces_to) - verification evidence uses index metadata
// START_verification_index_by_module
fn verification_index_by_module(path: &Path) -> BTreeMap<String, VerificationEntry> {
    parse_verification_index(path)
        .into_iter()
        .map(|entry| (entry.module.clone(), entry))
        .collect()
}
// END_verification_index_by_module

// START_CONTRACT_module_source_files
// PURPOSE: Read source file refs from one indexed module shard
// INPUTS: { module_shard_path: &Path }
// OUTPUTS: { anyhow::Result<Vec<String>> }
// LINKS:
//   -> NFR-003 (traces_to) - only referenced shards are read
// START_module_source_files
fn module_source_files(module_shard_path: &Path) -> anyhow::Result<Vec<String>> {
    if !module_shard_path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(module_shard_path)?;
    let mut files = extract_tag_values(&content, "FILE");
    files.sort();
    files.dedup();
    Ok(files)
}
// END_module_source_files

// START_CONTRACT_source_declares_module
// PURPOSE: Return true when a source file declares the expected MODULE_ID
// INPUTS: { root: &Path }, { source_file: &str }, { module_id: &str }
// OUTPUTS: { bool }
// LINKS:
//   -> M-GRACE-VERIFY (depends) - contract evidence aligns with verifier expectations
//   -> NFR-003 (traces_to) - contract evidence is read only from shard-listed files
// START_source_declares_module
fn source_declares_module(root: &Path, source_file: &str, module_id: &str) -> bool {
    let path = root.join(source_file);
    std::fs::read_to_string(path).is_ok_and(|content| {
        content.contains("MODULE_CONTRACT") && content.contains(&format!("MODULE_ID: {module_id}"))
    })
}
// END_source_declares_module

// START_CONTRACT_source_has_log_marker
// PURPOSE: Return true when a source file contains a structured LOG marker for the module
// INPUTS: { root: &Path }, { source_file: &str }, { module_id: &str }
// OUTPUTS: { bool }
// LINKS:
//   -> M-GRACE-LOG (depends) - LOG marker evidence
//   -> NFR-003 (traces_to) - LOG evidence stays scoped to shard-listed files
// START_source_has_log_marker
fn source_has_log_marker(root: &Path, source_file: &str, module_id: &str) -> bool {
    let path = root.join(source_file);
    std::fs::read_to_string(path).is_ok_and(|content| {
        content.contains("<LOG") && content.contains(&format!("module=\"{module_id}\""))
    })
}
// END_source_has_log_marker

// START_CONTRACT_collect_text_under
// PURPOSE: Collect text from XML/Markdown files directly under a bounded evidence directory tree
// INPUTS: { dir: &Path }
// OUTPUTS: { anyhow::Result<String> }
// LINKS:
//   -> M-GRACE-MENTAL-TEST (depends) - mental test evidence directory
//   -> M-GRACE-LAYOUT (depends) - test guide evidence directory
//   -> NFR-003 (traces_to) - evidence text scan is limited to known evidence directories
// START_collect_text_under
fn collect_text_under(dir: &Path) -> anyhow::Result<String> {
    let mut files = Vec::new();
    collect_evidence_files(dir, &mut files)?;
    files.sort();
    let mut text = String::new();
    for file in files {
        text.push_str(&std::fs::read_to_string(file).unwrap_or_default());
        text.push('\n');
    }
    Ok(text)
}
// END_collect_text_under

// START_CONTRACT_collect_evidence_files
// PURPOSE: Recursively collect bounded evidence files from docs/mental-tests or docs/tests/guides
// INPUTS: { dir: &Path }, { files: &mut Vec<PathBuf> }
// OUTPUTS: { anyhow::Result<()> }
// LINKS:
//   -> NFR-003 (traces_to) - evidence scan ignores unrelated directories
// START_collect_evidence_files
fn collect_evidence_files(dir: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_evidence_files(&path, files)?;
        } else if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("xml" | "md")
        ) {
            files.push(path);
        }
    }
    Ok(())
}
// END_collect_evidence_files

// START_CONTRACT_extract_tag_values
// PURPOSE: Extract simple XML tag values from compact MyGRACE shards
// INPUTS: { content: &str }, { tag: &str }
// OUTPUTS: { Vec<String> }
// LINKS:
//   -> M-GRACE-LAYOUT (depends) - shard file references
//   -> NFR-003 (traces_to) - simple tag extraction avoids monolithic XML loading
// START_extract_tag_values
fn extract_tag_values(content: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut values = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find(&open) {
        let after_open = &rest[start + open.len()..];
        let Some(end) = after_open.find(&close) else {
            break;
        };
        values.push(after_open[..end].trim().to_string());
        rest = &after_open[end + close.len()..];
    }
    values
}
// END_extract_tag_values

// START_CONTRACT_contains_module_ref
// PURPOSE: Match module ids in collected evidence text without per-module full scans
// INPUTS: { text: &str }, { module_id: &str }
// OUTPUTS: { bool }
// LINKS:
//   -> NFR-003 (traces_to) - evidence attribution is bounded to known module ids
// START_contains_module_ref
fn contains_module_ref(text: &str, module_id: &str) -> bool {
    text.contains(module_id)
}
// END_contains_module_ref

// START_CONTRACT_parse_code_coverage_report
// PURPOSE: Parse tarpaulin JSON and map covered source files to graph-index modules.
// INPUTS: { root: &Path }, { content: &str }, { threshold_pct: f64 }
// OUTPUTS: { anyhow::Result<CodeCoverageReport> }
// LINKS:
//   -> M-GRACE-LAYOUT (depends) - graph-index-first source mapping
//   -> Phase-94 (implements) - tarpaulin JSON parsing
// START_parse_code_coverage_report
fn parse_code_coverage_report(
    root: &Path,
    content: &str,
    threshold_pct: f64,
) -> anyhow::Result<CodeCoverageReport> {
    let report: serde_json::Value = serde_json::from_str(content)?;
    let source_map = module_source_map(root)?;
    let mut by_module = Vec::new();
    for (path, data) in tarpaulin_file_entries(&report) {
        let normalized = normalize_report_path(root, &path);
        if !normalized.starts_with("src/") {
            continue;
        }
        let Some(module_id) = source_map
            .get(&normalized)
            .cloned()
            .or_else(|| fallback_module_id(&normalized))
        else {
            continue;
        };
        let line_pct = coverage_pct(&data, &["line_coverage", "coverage"]);
        by_module.push(ModuleCodeCoverage {
            module_id,
            source_file: normalized,
            line_pct,
            function_pct: coverage_pct(&data, &["function_coverage", "functions_coverage"]),
            branch_pct: coverage_pct(&data, &["branch_coverage", "branches_coverage"]),
            uncovered_functions: extract_string_array(&data, "uncovered_functions")
                .or_else(|| extract_string_array(&data, "missed_functions"))
                .unwrap_or_default(),
            below_threshold: line_pct < threshold_pct,
        });
    }
    by_module.sort_by(|left, right| {
        left.module_id
            .cmp(&right.module_id)
            .then_with(|| left.source_file.cmp(&right.source_file))
    });
    let overall_pct = coverage_pct(&report, &["coverage", "line_coverage"]);
    Ok(CodeCoverageReport {
        overall_pct,
        threshold_pct,
        passed: overall_pct >= threshold_pct,
        by_module,
    })
}
// END_parse_code_coverage_report

// START_CONTRACT_render_combined_coverage
// PURPOSE: Render evidence coverage plus optional code coverage in text or JSON.
// INPUTS: { matrix: &CoverageMatrix }, { code: Option<CodeCoverageReport> }, { error: Option<String> }, { json_output: bool }
// OUTPUTS: { anyhow::Result<String> }
// START_render_combined_coverage
fn render_combined_coverage(
    matrix: &CoverageMatrix,
    code: Option<CodeCoverageReport>,
    error: Option<String>,
    json_output: bool,
) -> anyhow::Result<String> {
    if json_output {
        let code_json = match (code, error) {
            (Some(report), _) => json!({"available": true, "report": report}),
            (None, Some(error)) => json!({
                "available": false,
                "error": error,
                "install": "cargo install cargo-tarpaulin"
            }),
            (None, None) => json!({"available": false}),
        };
        return Ok(serde_json::to_string_pretty(&json!({
            "evidence": matrix,
            "code": code_json
        }))?);
    }
    let mut text = render_coverage_table(matrix);
    text.push_str("\n\n--- Code Coverage (cargo-tarpaulin) ---\n");
    match (code, error) {
        (Some(report), _) => text.push_str(&render_code_coverage_table(&report)),
        (None, Some(error)) => {
            text.push_str("Code coverage unavailable: ");
            text.push_str(&error);
            text.push('\n');
        }
        (None, None) => text.push_str("Code coverage unavailable\n"),
    }
    Ok(text)
}
// END_render_combined_coverage

// START_CONTRACT_render_code_coverage_table
// PURPOSE: Render module-level code coverage rows with threshold status.
// INPUTS: { report: &CodeCoverageReport }
// OUTPUTS: { String }
// START_render_code_coverage_table
fn render_code_coverage_table(report: &CodeCoverageReport) -> String {
    let mut lines = vec![
        format!(
            "Overall: {:.1}% ({})",
            report.overall_pct,
            if report.passed {
                "above threshold"
            } else {
                "below threshold"
            }
        ),
        "module | lines% | funcs% | branch% | status | source".to_string(),
    ];
    for module in &report.by_module {
        lines.push(format!(
            "{} | {:.1} | {:.1} | {:.1} | {} | {}",
            module.module_id,
            module.line_pct,
            module.function_pct,
            module.branch_pct,
            if module.below_threshold {
                "below-threshold"
            } else {
                "ok"
            },
            module.source_file
        ));
    }
    lines.join("\n")
}
// END_render_code_coverage_table

// START_CONTRACT_module_source_map
// PURPOSE: Map graph-index module source files to module ids using only indexed shards.
// INPUTS: { root: &Path }
// OUTPUTS: { anyhow::Result<BTreeMap<String, String>> }
// START_module_source_map
fn module_source_map(root: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    let layout = DocsLayout::new(root);
    let mut map = BTreeMap::new();
    for entry in parse_graph_index(&layout.graph_index_path()) {
        for source_file in module_source_files(&root.join(&entry.path))? {
            map.insert(normalize_report_path(root, &source_file), entry.id.clone());
        }
    }
    Ok(map)
}
// END_module_source_map

// START_CONTRACT_tarpaulin_file_entries
// PURPOSE: Extract path/data pairs from supported tarpaulin JSON shapes.
// INPUTS: { report: &serde_json::Value }
// OUTPUTS: { Vec<(String, serde_json::Value)> }
// START_tarpaulin_file_entries
fn tarpaulin_file_entries(report: &serde_json::Value) -> Vec<(String, serde_json::Value)> {
    match report.get("files") {
        Some(serde_json::Value::Object(files)) => files
            .iter()
            .map(|(path, data)| (path.clone(), data.clone()))
            .collect(),
        Some(serde_json::Value::Array(files)) => files
            .iter()
            .filter_map(|data| {
                let path = data
                    .get("path")
                    .or_else(|| data.get("name"))
                    .or_else(|| data.get("file"))
                    .and_then(tarpaulin_path_value)?;
                Some((path, data.clone()))
            })
            .collect(),
        _ => Vec::new(),
    }
}
// END_tarpaulin_file_entries

// START_CONTRACT_tarpaulin_path_value
// PURPOSE: Convert tarpaulin path fields from string or component-array shapes into a file path
// INPUTS: { value: &serde_json::Value }
// OUTPUTS: { Option<String> }
// LINKS:
//   -> Phase-94 (implements) - real cargo-tarpaulin JSON compatibility
//   -> NFR-003 (traces_to) - parser avoids verbose post-processing
// START_tarpaulin_path_value
fn tarpaulin_path_value(value: &serde_json::Value) -> Option<String> {
    if let Some(path) = value.as_str() {
        return Some(path.to_string());
    }
    let parts = value.as_array()?;
    let mut path = String::new();
    for part in parts {
        let part = part.as_str()?;
        if part == "/" {
            path.push('/');
            continue;
        }
        if !path.is_empty() && !path.ends_with('/') {
            path.push('/');
        }
        path.push_str(part.trim_matches('/'));
    }
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}
// END_tarpaulin_path_value

// START_CONTRACT_coverage_pct
// PURPOSE: Read a percentage value from one of several tarpaulin field names.
// INPUTS: { data: &serde_json::Value }, { keys: &[&str] }
// OUTPUTS: { f64 }
// START_coverage_pct
fn coverage_pct(data: &serde_json::Value, keys: &[&str]) -> f64 {
    keys.iter()
        .filter_map(|key| data.get(*key).and_then(serde_json::Value::as_f64))
        .map(|value| if value <= 1.0 { value * 100.0 } else { value })
        .next()
        .or_else(|| {
            if !keys
                .iter()
                .any(|key| matches!(*key, "coverage" | "line_coverage"))
            {
                return None;
            }
            let covered = data.get("covered")?.as_f64()?;
            let coverable = data.get("coverable")?.as_f64()?;
            (coverable > 0.0).then_some((covered / coverable) * 100.0)
        })
        .unwrap_or(0.0)
}
// END_coverage_pct

// START_CONTRACT_extract_string_array
// PURPOSE: Extract optional string arrays from tarpaulin file metadata.
// INPUTS: { data: &serde_json::Value }, { key: &str }
// OUTPUTS: { Option<Vec<String>> }
// START_extract_string_array
fn extract_string_array(data: &serde_json::Value, key: &str) -> Option<Vec<String>> {
    Some(
        data.get(key)?
            .as_array()?
            .iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect(),
    )
}
// END_extract_string_array

// START_CONTRACT_normalize_report_path
// PURPOSE: Normalize tarpaulin paths to project-relative slash-separated paths.
// INPUTS: { root: &Path }, { path: &str }
// OUTPUTS: { String }
// START_normalize_report_path
fn normalize_report_path(root: &Path, path: &str) -> String {
    let value = Path::new(path);
    let canonical_root = std::fs::canonicalize(root).unwrap_or_else(|_| {
        if root.is_absolute() {
            root.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(root)
        }
    });
    let relative = value
        .strip_prefix(&canonical_root)
        .or_else(|_| value.strip_prefix(root))
        .unwrap_or(value);
    relative.to_string_lossy().replace('\\', "/")
}
// END_normalize_report_path

// START_CONTRACT_fallback_module_id
// PURPOSE: Derive a best-effort module id when graph-index lacks a source mapping.
// INPUTS: { path: &str }
// OUTPUTS: { Option<String> }
// START_fallback_module_id
fn fallback_module_id(path: &str) -> Option<String> {
    let path = path.strip_prefix("src/")?.strip_suffix(".rs")?;
    let parts = path.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        ["main"] => Some("M-MAIN".to_string()),
        ["lib"] => Some("M-LIB".to_string()),
        [dir, "mod"] => Some(format!("M-{}", dir.replace('_', "-").to_ascii_uppercase())),
        [dir, file] => Some(format!(
            "M-{}-{}",
            dir.replace('_', "-").to_ascii_uppercase(),
            file.replace('_', "-").to_ascii_uppercase()
        )),
        _ => None,
    }
}
// END_fallback_module_id

// START_CONTRACT_summarize_modules
// PURPOSE: Build project-level coverage summary from module rows
// INPUTS: { modules: &[ModuleCoverage] }
// OUTPUTS: { CoverageSummary }
// LINKS:
//   -> NFR-003 (traces_to) - summary compresses coverage state for automation
// START_summarize_modules
fn summarize_modules(modules: &[ModuleCoverage]) -> CoverageSummary {
    let mut counts = BTreeMap::<&str, usize>::new();
    for module in modules {
        *counts.entry(module.status.as_str()).or_default() += 1;
    }
    CoverageSummary {
        total_modules: modules.len(),
        covered_modules: *counts.get("covered").unwrap_or(&0),
        partial_modules: *counts.get("partial").unwrap_or(&0),
        uncovered_modules: *counts.get("uncovered").unwrap_or(&0),
        evidence_threshold: COVERED_EVIDENCE_THRESHOLD,
        evidence_types: vec![
            "contract",
            "verification",
            "verify_status",
            "mental_test",
            "test_guide",
            "log_marker",
        ],
    }
}
// END_summarize_modules

// START_CONTRACT_marker
// PURPOSE: Render compact evidence markers for table output
// INPUTS: { present: bool }
// OUTPUTS: { &'static str }
// LINKS:
//   -> NFR-003 (traces_to) - one-character markers bound table width
// START_marker
fn marker(present: bool) -> &'static str {
    if present {
        "Y"
    } else {
        "-"
    }
}
// END_marker

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::fixture::{FixtureTemplate, TestFixture};

    #[test]
    fn coverage_matrix_discovers_modules_from_graph_index_only() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .with_file(
                "docs/modules/M-ORPHAN.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<MODULE id="M-ORPHAN" type="UTILITY" status="active">
  <FILES><FILE>src/orphan.rs</FILE></FILES>
</MODULE>
"#,
            )
            .with_file(
                "src/orphan.rs",
                "// MODULE_CONTRACT\n// MODULE_ID: M-ORPHAN\n// PURPOSE: Orphan\n// SCOPE: Orphan\n",
            )
            .build()
            .expect("fixture");

        let matrix = build_coverage_matrix(fixture.root()).expect("coverage matrix");

        assert_eq!(matrix.modules.len(), 1);
        assert_eq!(matrix.modules[0].module_id, "M-CORE");
    }

    #[test]
    fn coverage_status_is_covered_with_three_evidence_types() {
        let evidence = CoverageEvidence {
            contract: true,
            verification: true,
            verify_status: true,
            ..CoverageEvidence::default()
        };

        assert_eq!(evidence.count(), 3);
        assert_eq!(
            CoverageStatus::from_count(evidence.count()),
            CoverageStatus::Covered
        );
    }

    #[test]
    fn coverage_matrix_marks_multimodule_fixture_covered() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::MultiModule)
            .build()
            .expect("fixture");

        let matrix = build_coverage_matrix(fixture.root()).expect("coverage matrix");

        assert_eq!(matrix.summary.total_modules, 3);
        assert_eq!(matrix.summary.covered_modules, 3);
        assert!(matrix
            .modules
            .iter()
            .all(|module| module.status == CoverageStatus::Covered));
    }

    #[test]
    fn coverage_json_output_contains_summary_and_module_rows() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let matrix = build_coverage_matrix(fixture.root()).expect("coverage matrix");

        let json = render_coverage_json(&matrix).expect("coverage JSON");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(value["summary"]["total_modules"], 1);
        assert_eq!(value["modules"][0]["module_id"], "M-CORE");
    }

    #[test]
    fn coverage_table_renders_compact_evidence_columns() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let matrix = build_coverage_matrix(fixture.root()).expect("coverage matrix");

        let table = render_coverage_table(&matrix);

        assert!(table.contains("Coverage matrix: 1/1 covered"));
        assert!(table.contains("M-CORE | covered"));
    }

    #[test]
    fn coverage_matrix_detects_log_marker_evidence() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .with_file(
                "src/main.rs",
                r#"// MODULE_CONTRACT
// MODULE_ID: M-CORE
// PURPOSE: Core module for fixture projects.
// SCOPE: Deterministic test behavior.
// DEPENDS:
// LINKS:
//   <- V-M-CORE (verified_by) - core fixture verification

// <LOG id="fixture-coverage" level="INFO" ref="coverage" module="M-CORE" contract="main">
//   EVENT: coverage
//   CONTEXT: fixture=true
//   STATE: ready=true
//   DECISION: expose log marker evidence
//   EXPECTATION: coverage matrix sees this module
//   RESULT: success
//   TRACEABILITY: NFR-003
// </LOG>
fn main() {}
"#,
            )
            .build()
            .expect("fixture");

        let matrix = build_coverage_matrix(fixture.root()).expect("coverage matrix");

        assert!(matrix.modules[0].evidence.log_marker);
    }

    #[test]
    fn code_coverage_report_maps_tarpaulin_json_to_modules() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let json = r#"{
          "coverage": 72.5,
          "files": {
            "src/main.rs": {
              "line_coverage": 65.0,
              "function_coverage": 80.0,
              "branch_coverage": 55.0,
              "uncovered_functions": ["main"]
            },
            "tests/integration.rs": {
              "line_coverage": 100.0
            }
          }
        }"#;

        let report = parse_code_coverage_report(fixture.root(), json, 70.0).expect("code coverage");

        assert_eq!(report.overall_pct, 72.5);
        assert!(report.passed);
        assert_eq!(report.by_module.len(), 1);
        assert_eq!(report.by_module[0].module_id, "M-CORE");
        assert!(report.by_module[0].below_threshold);
        assert_eq!(report.by_module[0].uncovered_functions, ["main"]);
    }

    #[test]
    fn code_coverage_report_parses_real_tarpaulin_path_arrays() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let source_path = fixture.root().join("src").join("main.rs");
        let path_parts = source_path
            .components()
            .map(|part| part.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let json = serde_json::json!({
            "coverage": 0.673,
            "files": [{
                "path": path_parts,
                "covered": 2,
                "coverable": 4
            }]
        });

        let report = parse_code_coverage_report(fixture.root(), &json.to_string(), 65.0)
            .expect("code coverage");

        assert!(report.passed);
        assert_eq!(report.by_module.len(), 1);
        assert_eq!(report.by_module[0].source_file, "src/main.rs");
        assert_eq!(report.by_module[0].line_pct, 50.0);
        assert_eq!(report.by_module[0].function_pct, 0.0);
        assert_eq!(report.by_module[0].branch_pct, 0.0);
        assert!(report.by_module[0].below_threshold);
    }

    #[test]
    fn code_coverage_normalizes_absolute_paths_for_relative_project_root() {
        let absolute_source = std::env::current_dir()
            .expect("current dir")
            .join("src")
            .join("test")
            .join("coverage.rs");

        let normalized = normalize_report_path(Path::new("."), &absolute_source.to_string_lossy());

        assert_eq!(normalized, "src/test/coverage.rs");
    }

    #[test]
    fn combined_coverage_json_reports_unavailable_code_coverage() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let matrix = build_coverage_matrix(fixture.root()).expect("coverage matrix");

        let rendered = render_combined_coverage(
            &matrix,
            None,
            Some("cargo-tarpaulin failed".to_string()),
            true,
        )
        .expect("combined JSON");
        let value: serde_json::Value = serde_json::from_str(&rendered).expect("valid JSON");

        assert_eq!(value["evidence"]["summary"]["total_modules"], 1);
        assert_eq!(value["code"]["available"], false);
        assert!(value["code"]["install"]
            .as_str()
            .expect("install hint")
            .contains("cargo install cargo-tarpaulin"));
    }
}
