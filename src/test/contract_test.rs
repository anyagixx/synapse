// MODULE_CONTRACT
// MODULE_ID: M-TEST-CONTRACT-DIFFERENTIAL
// PURPOSE: Run TOML-defined cascade differential tests against expected affected modules.
// SCOPE: TOML specs, fixture selection, trigger filtering, missing and unexpected affected modules, allow_extra, bounded reporting, and cascade preview cleanup.
// DEPENDS: M-GRACE-CASCADE, M-TEST-FIXTURE
// LINKS:
//   -> Phase-79 (implements) - contract differential testing
//   <- V-M-TEST-CONTRACT-DIFFERENTIAL (verified_by) - cascade assertion verification

// START_MODULE_MAP
// ContractTestSuite - TOML wrapper for a contract differential suite
// ContractTestSpec - One cascade trigger expectation
// ContractTestOptions - Runtime options for project root and trigger filtering
// ContractTestReport - Suite-level contract differential result
// run_contract_test_file - Parse and run one TOML contract test file
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Implemented cascade differential contract test runner]
// END_CHANGE_SUMMARY

use crate::grace::cascade::ImpactAnalysis;
use crate::grace::layout::DocsLayout;
use crate::test::fixture::{FixtureTemplate, TestFixture};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

// START_public_api

// START_ContractTestSuite
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ContractTestSuite {
    pub fixture: Option<String>,
    #[serde(default, rename = "test")]
    pub tests: Vec<ContractTestSpec>,
}
// END_ContractTestSuite

// START_ContractTestSpec
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ContractTestSpec {
    pub name: String,
    pub changed_artifact: String,
    #[serde(default)]
    pub change_description: String,
    #[serde(default, alias = "expected_affected_modules")]
    pub expected_modules: Vec<String>,
    #[serde(default)]
    pub allow_extra: bool,
    pub trigger: Option<String>,
}
// END_ContractTestSpec

// START_ContractTestOptions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractTestOptions {
    pub project_root: PathBuf,
    pub trigger_filter: Option<String>,
    pub cleanup_previews: bool,
}
// END_ContractTestOptions

impl Default for ContractTestOptions {
    // START_CONTRACT_ContractTestOptions::default
    // PURPOSE: Return default options for running contract tests in the current project
    // OUTPUTS: { ContractTestOptions }
    // LINKS:
    //   -> NFR-002 (traces_to) - deterministic default execution context
    // START_contract_test_options_default
    fn default() -> Self {
        Self {
            project_root: PathBuf::from("."),
            trigger_filter: None,
            cleanup_previews: true,
        }
    }
    // END_contract_test_options_default
}

// START_ContractTestReport
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ContractTestReport {
    pub spec_path: String,
    pub fixture: Option<String>,
    pub passed: bool,
    pub total: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub results: Vec<ContractTestResult>,
}
// END_ContractTestReport

// START_ContractTestResult
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ContractTestResult {
    pub name: String,
    pub trigger: Option<String>,
    pub changed_artifact: String,
    pub cascade_id: String,
    pub expected_modules: Vec<String>,
    pub actual_modules: Vec<String>,
    pub missing_modules: Vec<String>,
    pub unexpected_modules: Vec<String>,
    pub allow_extra: bool,
    pub passed: bool,
}
// END_ContractTestResult

// START_CONTRACT_run_contract_test_file
// PURPOSE: Parse and execute one TOML contract differential suite
// INPUTS: { spec_path: &Path }, { options: &ContractTestOptions }
// OUTPUTS: { anyhow::Result<ContractTestReport> }
// SIDE_EFFECTS: may create a test fixture and writes then cleans cascade preview artifacts
// LINKS:
//   -> M-GRACE-CASCADE (depends) - source of actual affected module analysis
//   -> M-TEST-FIXTURE (depends) - optional fixture-backed contract tests
//   -> NFR-002 (traces_to) - cascade assertions must be deterministic
//   -> NFR-003 (traces_to) - reports are bounded and machine-readable
// <LOG id="contract_test_started" level="INFO" ref="contract-test-run" module="M-TEST-CONTRACT-DIFFERENTIAL" contract="run_contract_test_file">
//   EVENT: contract_test_started
//   EXPECTATION: TOML specs are parsed and filtered before cascade analysis
//   DECISION: optional fixture roots isolate generated cascade previews
//   RESULT: success
//   TRACEABILITY: NFR-002
// </LOG>
// <LOG id="contract_test_completed" level="INFO" ref="contract-test-run" module="M-TEST-CONTRACT-DIFFERENTIAL" contract="run_contract_test_file">
//   EVENT: contract_test_completed
//   EXPECTATION: missing and unexpected affected modules are reported separately
//   DECISION: cascade previews are removed or restored after the suite
//   RESULT: success
//   TRACEABILITY: NFR-002
// </LOG>
// START_run_contract_test_file
pub fn run_contract_test_file(
    spec_path: &Path,
    options: &ContractTestOptions,
) -> anyhow::Result<ContractTestReport> {
    let content = std::fs::read_to_string(spec_path)?;
    let suite: ContractTestSuite = toml::from_str(&content)?;
    run_contract_test_suite(spec_path, suite, options)
}
// END_run_contract_test_file

// START_CONTRACT_render_contract_test_report
// PURPOSE: Render compact text output for a contract differential report
// INPUTS: { report: &ContractTestReport }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-003 (traces_to) - text output remains bounded and scan-friendly
// START_render_contract_test_report
pub fn render_contract_test_report(report: &ContractTestReport) -> String {
    let mut lines = vec![format!(
        "Contract tests: {}/{} passed",
        report.passed_count, report.total
    )];
    for result in &report.results {
        lines.push(format!(
            "- {}: {} changed={} expected={} actual={}",
            result.name,
            if result.passed { "passed" } else { "failed" },
            result.changed_artifact,
            result.expected_modules.join(","),
            result.actual_modules.join(",")
        ));
        if !result.missing_modules.is_empty() {
            lines.push(format!("  missing: {}", result.missing_modules.join(",")));
        }
        if !result.unexpected_modules.is_empty() {
            lines.push(format!(
                "  unexpected: {}",
                result.unexpected_modules.join(",")
            ));
        }
    }
    lines.join("\n")
}
// END_render_contract_test_report

// START_CONTRACT_render_contract_test_json
// PURPOSE: Render contract differential report as stable pretty JSON
// INPUTS: { report: &ContractTestReport }
// OUTPUTS: { anyhow::Result<String> }
// LINKS:
//   -> NFR-003 (traces_to) - JSON output supports automation
// START_render_contract_test_json
pub fn render_contract_test_json(report: &ContractTestReport) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(report)?)
}
// END_render_contract_test_json

// END_public_api

// START_CONTRACT_run_contract_test_suite
// PURPOSE: Execute a parsed contract test suite with optional fixture override
// INPUTS: { spec_path: &Path }, { suite: ContractTestSuite }, { options: &ContractTestOptions }
// OUTPUTS: { anyhow::Result<ContractTestReport> }
// SIDE_EFFECTS: may create an isolated fixture root and cascade preview files
// LINKS:
//   -> M-TEST-FIXTURE (depends) - optional fixture root construction
//   -> NFR-002 (traces_to) - suite execution is deterministic
// START_run_contract_test_suite
fn run_contract_test_suite(
    spec_path: &Path,
    suite: ContractTestSuite,
    options: &ContractTestOptions,
) -> anyhow::Result<ContractTestReport> {
    let fixture = build_contract_fixture(suite.fixture.as_deref())?;
    let root = fixture
        .as_ref()
        .map(TestFixture::root)
        .unwrap_or_else(|| options.project_root.as_path());
    let mut cleanup = CascadePreviewGuard::capture(root, options.cleanup_previews)?;
    let mut results = Vec::new();

    for spec in suite
        .tests
        .iter()
        .filter(|spec| spec_matches_filter(spec, options))
    {
        let analysis = crate::grace::cascade::cascade_impact(
            root,
            &spec.changed_artifact,
            change_description(spec),
        )?;
        cleanup.remember(&analysis.cascade_id);
        results.push(evaluate_contract_spec(spec, &analysis));
    }

    let passed_count = results.iter().filter(|result| result.passed).count();
    let failed_count = results.len().saturating_sub(passed_count);
    Ok(ContractTestReport {
        spec_path: spec_path.display().to_string(),
        fixture: suite.fixture,
        passed: failed_count == 0,
        total: results.len(),
        passed_count,
        failed_count,
        results,
    })
}
// END_run_contract_test_suite

// START_CONTRACT_evaluate_contract_spec
// PURPOSE: Compare expected modules against actual cascade affected modules
// INPUTS: { spec: &ContractTestSpec }, { analysis: &ImpactAnalysis }
// OUTPUTS: { ContractTestResult }
// LINKS:
//   -> M-GRACE-CASCADE (depends) - actual affected modules
//   -> NFR-002 (traces_to) - missing and unexpected modules are explicit
// START_evaluate_contract_spec
fn evaluate_contract_spec(
    spec: &ContractTestSpec,
    analysis: &ImpactAnalysis,
) -> ContractTestResult {
    let expected = sorted_unique(&spec.expected_modules);
    let actual = actual_module_ids(analysis);
    let expected_set: BTreeSet<_> = expected.iter().cloned().collect();
    let actual_set: BTreeSet<_> = actual.iter().cloned().collect();
    let missing_modules = expected_set
        .difference(&actual_set)
        .cloned()
        .collect::<Vec<_>>();
    let unexpected_modules = if spec.allow_extra {
        Vec::new()
    } else {
        actual_set
            .difference(&expected_set)
            .cloned()
            .collect::<Vec<_>>()
    };
    let passed = missing_modules.is_empty() && unexpected_modules.is_empty();

    ContractTestResult {
        name: spec.name.clone(),
        trigger: spec.trigger.clone(),
        changed_artifact: spec.changed_artifact.clone(),
        cascade_id: analysis.cascade_id.clone(),
        expected_modules: expected,
        actual_modules: actual,
        missing_modules,
        unexpected_modules,
        allow_extra: spec.allow_extra,
        passed,
    }
}
// END_evaluate_contract_spec

// START_CONTRACT_build_contract_fixture
// PURPOSE: Build an optional fixture root declared by a contract test suite
// INPUTS: { fixture: Option<&str> }
// OUTPUTS: { anyhow::Result<Option<TestFixture>> }
// SIDE_EFFECTS: creates temporary fixture directories when requested
// LINKS:
//   -> M-TEST-FIXTURE (depends) - fixture templates
//   -> NFR-002 (traces_to) - fixture-backed contract tests are deterministic
// START_build_contract_fixture
fn build_contract_fixture(fixture: Option<&str>) -> anyhow::Result<Option<TestFixture>> {
    let Some(fixture) = fixture else {
        return Ok(None);
    };
    let template = match fixture {
        "empty" => FixtureTemplate::Empty,
        "minimal" => FixtureTemplate::Minimal,
        "multi-module" | "multimodule" => FixtureTemplate::MultiModule,
        "broken" => FixtureTemplate::Broken,
        other => anyhow::bail!("unknown contract test fixture: {other}"),
    };
    Ok(Some(
        TestFixture::builder()
            .with_template(template)
            .build()
            .map_err(|error| anyhow::anyhow!("build fixture {fixture}: {error}"))?,
    ))
}
// END_build_contract_fixture

// START_CONTRACT_spec_matches_filter
// PURPOSE: Return true when a spec should run for the selected trigger filter
// INPUTS: { spec: &ContractTestSpec }, { options: &ContractTestOptions }
// OUTPUTS: { bool }
// LINKS:
//   -> NFR-002 (traces_to) - trigger filtering is deterministic
// START_spec_matches_filter
fn spec_matches_filter(spec: &ContractTestSpec, options: &ContractTestOptions) -> bool {
    let Some(filter) = options.trigger_filter.as_deref() else {
        return true;
    };
    spec.trigger.as_deref() == Some(filter)
        || spec.name == filter
        || spec.changed_artifact == filter
}
// END_spec_matches_filter

// START_CONTRACT_actual_module_ids
// PURPOSE: Extract sorted affected module ids from a cascade impact analysis
// INPUTS: { analysis: &ImpactAnalysis }
// OUTPUTS: { Vec<String> }
// LINKS:
//   -> M-GRACE-CASCADE (depends) - affected artifact source
//   -> NFR-003 (traces_to) - function artifacts are excluded from module-level differential output
// START_actual_module_ids
fn actual_module_ids(analysis: &ImpactAnalysis) -> Vec<String> {
    sorted_unique(
        &analysis
            .affected_modules
            .iter()
            .filter_map(|affected| {
                let id = affected.module_id.as_str();
                if id.starts_with("M-") && !id.contains("::") {
                    Some(affected.module_id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>(),
    )
}
// END_actual_module_ids

// START_CONTRACT_change_description
// PURPOSE: Return a non-empty cascade change description for one spec
// INPUTS: { spec: &ContractTestSpec }
// OUTPUTS: { &str }
// LINKS:
//   -> NFR-002 (traces_to) - cascade ids remain stable for empty descriptions
// START_change_description
fn change_description(spec: &ContractTestSpec) -> &str {
    if spec.change_description.trim().is_empty() {
        "Contract differential test"
    } else {
        spec.change_description.as_str()
    }
}
// END_change_description

// START_CONTRACT_sorted_unique
// PURPOSE: Return sorted unique string values
// INPUTS: { values: &[String] }
// OUTPUTS: { Vec<String> }
// LINKS:
//   -> NFR-003 (traces_to) - deterministic result ordering bounds diff noise
// START_sorted_unique
fn sorted_unique(values: &[String]) -> Vec<String> {
    values
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
// END_sorted_unique

// START_CascadePreviewGuard
struct CascadePreviewGuard {
    root: PathBuf,
    previous_last_impact: Option<Vec<u8>>,
    cascade_ids: Vec<String>,
    enabled: bool,
}
// END_CascadePreviewGuard

impl CascadePreviewGuard {
    // START_CONTRACT_CascadePreviewGuard::capture
    // PURPOSE: Capture existing cascade last-impact state before running tests
    // INPUTS: { root: &Path }, { enabled: bool }
    // OUTPUTS: { anyhow::Result<CascadePreviewGuard> }
    // SIDE_EFFECTS: reads existing cascade last-impact file when present
    // LINKS:
    //   -> M-GRACE-CASCADE (depends) - preview cleanup
    //   -> NFR-002 (traces_to) - contract tests restore prior cascade state
    // START_cascade_preview_guard_capture
    fn capture(root: &Path, enabled: bool) -> anyhow::Result<Self> {
        let layout = DocsLayout::new(root);
        let previous_last_impact = if enabled {
            std::fs::read(layout.cascade_last_impact_path()).ok()
        } else {
            None
        };
        Ok(Self {
            root: root.to_path_buf(),
            previous_last_impact,
            cascade_ids: Vec::new(),
            enabled,
        })
    }
    // END_cascade_preview_guard_capture

    // START_CONTRACT_CascadePreviewGuard::remember
    // PURPOSE: Track one generated cascade preview id for cleanup
    // INPUTS: { cascade_id: &str }
    // OUTPUTS: { () }
    // LINKS:
    //   -> NFR-002 (traces_to) - generated previews are cleaned after tests
    // START_cascade_preview_guard_remember
    fn remember(&mut self, cascade_id: &str) {
        if !self.cascade_ids.iter().any(|id| id == cascade_id) {
            self.cascade_ids.push(cascade_id.to_string());
        }
    }
    // END_cascade_preview_guard_remember
}

impl Drop for CascadePreviewGuard {
    // START_CONTRACT_CascadePreviewGuard::drop
    // PURPOSE: Remove generated preview files and restore previous last-impact state
    // SIDE_EFFECTS: deletes generated preview JSON and restores/removes docs/cascade/last-impact.xml
    // LINKS:
    //   -> M-GRACE-CASCADE (depends) - generated preview files
    //   -> NFR-002 (traces_to) - test cleanup prevents cascade drift
    // START_cascade_preview_guard_drop
    fn drop(&mut self) {
        if !self.enabled {
            return;
        }
        let layout = DocsLayout::new(&self.root);
        for cascade_id in &self.cascade_ids {
            let _ = std::fs::remove_file(
                layout
                    .cascade_previews_dir()
                    .join(format!("{cascade_id}.json")),
            );
        }
        if let Some(previous) = &self.previous_last_impact {
            let _ = std::fs::create_dir_all(layout.cascade_dir());
            let _ = std::fs::write(layout.cascade_last_impact_path(), previous);
        } else {
            let _ = std::fs::remove_file(layout.cascade_last_impact_path());
        }
    }
    // END_cascade_preview_guard_drop
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grace::cascade::{AffectedModule, ImpactLevel};
    use tempfile::TempDir;

    #[test]
    fn contract_test_parses_multiple_specs() {
        let suite: ContractTestSuite = toml::from_str(
            r#"
fixture = "multi-module"

[[test]]
name = "core"
trigger = "module-change"
changed_artifact = "M-CORE"
expected_modules = ["M-AUTH"]

[[test]]
name = "storage"
changed_artifact = "M-STORAGE"
expected_modules = ["M-AUTH"]
allow_extra = true
"#,
        )
        .expect("parse suite");

        assert_eq!(suite.fixture.as_deref(), Some("multi-module"));
        assert_eq!(suite.tests.len(), 2);
        assert!(suite.tests[1].allow_extra);
    }

    #[test]
    fn contract_test_reports_missing_and_unexpected_modules_separately() {
        let spec = ContractTestSpec {
            name: "diff".into(),
            changed_artifact: "M-CORE".into(),
            change_description: String::new(),
            expected_modules: vec!["M-AUTH".into(), "M-STORAGE".into()],
            allow_extra: false,
            trigger: None,
        };
        let analysis = fake_analysis(["M-AUTH", "M-EXTRA"]);

        let result = evaluate_contract_spec(&spec, &analysis);

        assert!(!result.passed);
        assert_eq!(result.missing_modules, ["M-STORAGE"]);
        assert_eq!(result.unexpected_modules, ["M-EXTRA"]);
    }

    #[test]
    fn contract_test_allow_extra_still_requires_expected_modules() {
        let spec = ContractTestSpec {
            name: "diff".into(),
            changed_artifact: "M-CORE".into(),
            change_description: String::new(),
            expected_modules: vec!["M-AUTH".into(), "M-STORAGE".into()],
            allow_extra: true,
            trigger: None,
        };
        let analysis = fake_analysis(["M-AUTH", "M-EXTRA"]);

        let result = evaluate_contract_spec(&spec, &analysis);

        assert!(!result.passed);
        assert_eq!(result.missing_modules, ["M-STORAGE"]);
        assert!(result.unexpected_modules.is_empty());
    }

    #[test]
    fn contract_test_trigger_filter_runs_only_matching_specs() {
        let (_dir, spec_path) = write_suite(
            r#"
fixture = "multi-module"

[[test]]
name = "core"
trigger = "module-change"
changed_artifact = "M-CORE"
expected_modules = ["M-AUTH", "M-STORAGE"]

[[test]]
name = "storage"
trigger = "storage-change"
changed_artifact = "M-STORAGE"
expected_modules = ["M-AUTH"]
"#,
        );
        let options = ContractTestOptions {
            trigger_filter: Some("storage-change".into()),
            ..ContractTestOptions::default()
        };

        let report = run_contract_test_file(&spec_path, &options).expect("contract report");

        assert_eq!(report.total, 1);
        assert_eq!(report.results[0].name, "storage");
        assert!(report.passed);
    }

    #[test]
    fn contract_test_multimodule_fixture_passes_module_change_spec() {
        let (_dir, spec_path) = write_suite(
            r#"
fixture = "multi-module"

[[test]]
name = "core change"
changed_artifact = "M-CORE"
change_description = "Core contract changed"
expected_modules = ["M-AUTH", "M-STORAGE"]
"#,
        );

        let report =
            run_contract_test_file(&spec_path, &ContractTestOptions::default()).expect("report");

        assert!(report.passed, "{}", render_contract_test_report(&report));
        assert_eq!(report.results[0].actual_modules, ["M-AUTH", "M-STORAGE"]);
    }

    #[test]
    fn contract_test_cleans_generated_cascade_preview() {
        let (_dir, spec_path) = write_suite(
            r#"
[[test]]
name = "storage"
changed_artifact = "M-STORAGE"
expected_modules = ["M-AUTH"]
"#,
        );
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::MultiModule)
            .build()
            .expect("fixture");
        let options = ContractTestOptions {
            project_root: fixture.root().to_path_buf(),
            ..ContractTestOptions::default()
        };

        let report = run_contract_test_file(&spec_path, &options).expect("report");

        assert!(report.passed);
        let layout = DocsLayout::new(fixture.root());
        assert!(!layout.cascade_last_impact_path().exists());
        let preview_count = std::fs::read_dir(layout.cascade_previews_dir())
            .ok()
            .map(|entries| entries.filter_map(Result::ok).count())
            .unwrap_or(0);
        assert_eq!(preview_count, 0);
    }

    #[test]
    fn contract_test_json_report_contains_results() {
        let spec = ContractTestSpec {
            name: "diff".into(),
            changed_artifact: "M-CORE".into(),
            change_description: String::new(),
            expected_modules: vec!["M-AUTH".into()],
            allow_extra: false,
            trigger: Some("module-change".into()),
        };
        let result = evaluate_contract_spec(&spec, &fake_analysis(["M-AUTH"]));
        let report = ContractTestReport {
            spec_path: "spec.toml".into(),
            fixture: None,
            passed: true,
            total: 1,
            passed_count: 1,
            failed_count: 0,
            results: vec![result],
        };

        let json = render_contract_test_json(&report).expect("json");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(value["results"][0]["name"], "diff");
        assert_eq!(value["passed"], true);
    }

    fn fake_analysis<const N: usize>(modules: [&str; N]) -> ImpactAnalysis {
        ImpactAnalysis {
            cascade_id: "CASCADE-TEST".into(),
            changed_artifact: "M-CORE".into(),
            change_type: "module".into(),
            change_description: "test".into(),
            affected_modules: modules
                .into_iter()
                .map(|module| AffectedModule {
                    module_id: module.into(),
                    impact_level: ImpactLevel::Direct,
                    change_required: "review".into(),
                    distance: 1,
                    path: vec!["M-CORE".into(), module.into()],
                })
                .collect(),
            total_affected: N,
            estimated_effort: "low".into(),
            preview: String::new(),
        }
    }

    fn write_suite(content: &str) -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("contract.toml");
        std::fs::write(&path, content).expect("write spec");
        (dir, path)
    }
}
