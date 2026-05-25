// MODULE_CONTRACT
// MODULE_ID: M-TEST-COVERAGE-MATRIX
// PURPOSE: Build an index-first module evidence coverage matrix for Synapse test visibility.
// SCOPE: Index-based module discovery, evidence counting, table output, JSON output, and uncovered module reporting.
// DEPENDS: M-GRACE-LOG, M-GRACE-MENTAL-TEST, M-GRACE-VERIFY, M-GRACE-LAYOUT
// LINKS:
//   -> Phase-79 (implements) - coverage matrix
//   <- V-M-TEST-COVERAGE-MATRIX (verified_by) - coverage evidence verification

// START_MODULE_MAP
// CoverageMatrix - Project coverage report with summary and per-module rows
// ModuleCoverage - Per-module evidence and coverage status
// CoverageEvidence - Evidence booleans for contract, verification, mental tests, guides, LOGs, and verify status
// build_coverage_matrix - Builds the coverage matrix using graph-index first
// render_coverage_table - Renders a bounded human-readable coverage table
// render_coverage_json - Renders stable JSON for automation
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Implemented index-first coverage matrix]
// END_CHANGE_SUMMARY

use crate::grace::inventory_artifacts::{parse_graph_index, parse_verification_index};
use crate::grace::inventory_types::{GraphEntry, VerificationEntry};
use crate::grace::layout::DocsLayout;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const COVERED_EVIDENCE_THRESHOLD: usize = 3;

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
}
