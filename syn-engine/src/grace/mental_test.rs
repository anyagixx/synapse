// MODULE_CONTRACT
// MODULE_ID: M-GRACE-MENTAL-TEST
// PURPOSE: Mental test parser, validator, and runner for pre-code GRACE algorithm simulation
// SCOPE: MentalTestSpec, MentalTestReport, parse_mental_tests, scan_project_mental_tests, run_mental_test, XML trace persistence
// DEPENDS: M-GRACE-CONTRACT, M-GRACE-DEVELOPMENT-PLAN
// LINKS:
//   → V-M-GRACE-MENTAL-TEST (verified_by) — Mental test parser, runner, and verification tests

// START_MODULE_MAP
// MentalTestStatus — Status enum for pre-code mental tests
// MentalTestStep — One expected/actual simulation step
// EdgeCase — One documented edge case expectation
// MentalTestSpec — Parsed mental test definition
// MentalTestReport — Project-level mental test coverage report
// MentalTestRunReport — Result of running one mental test
// parse_mental_tests — Parse MentalTests from DevelopmentPlan content
// scan_project_mental_tests — Validate project mental tests
// run_mental_test — Execute one deterministic mental test and persist trace
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Added MentalTests parser, report, and runner]
// END_CHANGE_SUMMARY

use crate::grace::contract::{ContractValidator, GraceProfile};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

// START_public_api

// START_MentalTestStatus
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum MentalTestStatus {
    NotRun,
    Pass,
    Fail,
    NeedsClarification,
}
// END_MentalTestStatus

impl MentalTestStatus {
    // START_CONTRACT_MentalTestStatus::from_value
    // PURPOSE: Parse XML status/result text into a stable MentalTestStatus value
    // INPUTS: { value: &str }
    // OUTPUTS: { MentalTestStatus }
    // START_mental_test_status_from_value
    pub fn from_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "pass" | "passed" => Self::Pass,
            "fail" | "failed" => Self::Fail,
            "needs_clarification" | "needs-clarification" | "clarify" => Self::NeedsClarification,
            _ => Self::NotRun,
        }
    }
    // END_mental_test_status_from_value

    // START_CONTRACT_MentalTestStatus::as_str
    // PURPOSE: Return canonical lowercase XML status label
    // OUTPUTS: { &'static str }
    // START_mental_test_status_as_str
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotRun => "not_run",
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::NeedsClarification => "needs_clarification",
        }
    }
    // END_mental_test_status_as_str
}

// START_MentalTestStep
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct MentalTestStep {
    pub n: usize,
    pub action: String,
    pub expected: String,
    pub actual: String,
    pub passed: bool,
    pub description: String,
}
// END_MentalTestStep

// START_EdgeCase
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EdgeCase {
    pub id: String,
    pub description: String,
    pub expectation: String,
}
// END_EdgeCase

// START_MentalTestSpec
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct MentalTestSpec {
    pub id: String,
    pub target: String,
    pub status: MentalTestStatus,
    pub description: String,
    pub scenario: String,
    pub steps: Vec<MentalTestStep>,
    pub edge_cases: Vec<EdgeCase>,
    pub result: String,
    pub rationale: String,
}
// END_MentalTestSpec

// START_MentalTestReport
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct MentalTestReport {
    pub tests: Vec<MentalTestSpec>,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub not_run: usize,
    pub needs_clarification: usize,
    pub required_targets: Vec<String>,
    pub missing_required_targets: Vec<String>,
    pub drift_issues: Vec<String>,
    pub errors: Vec<String>,
}
// END_MentalTestReport

impl MentalTestReport {
    // START_CONTRACT_MentalTestReport::mental_tests_defined
    // PURPOSE: Return true when all critical DevelopmentPlan targets have mental tests
    // OUTPUTS: { bool }
    // START_mental_test_report_defined
    pub fn mental_tests_defined(&self) -> bool {
        self.missing_required_targets.is_empty()
            && (self.required_targets.is_empty() || self.total > 0)
    }
    // END_mental_test_report_defined

    // START_CONTRACT_MentalTestReport::mental_tests_passed
    // PURPOSE: Return true when every defined mental test has PASS status and valid steps
    // OUTPUTS: { bool }
    // START_mental_test_report_passed
    pub fn mental_tests_passed(&self) -> bool {
        if self.total == 0 {
            return self.required_targets.is_empty();
        }
        self.failed == 0
            && self.not_run == 0
            && self.needs_clarification == 0
            && self.errors.is_empty()
    }
    // END_mental_test_report_passed

    // START_CONTRACT_MentalTestReport::mental_test_no_drift
    // PURPOSE: Return true when mental test targets still resolve to current source modules
    // OUTPUTS: { bool }
    // START_mental_test_report_no_drift
    pub fn mental_test_no_drift(&self) -> bool {
        self.drift_issues.is_empty()
    }
    // END_mental_test_report_no_drift
}

// START_MentalTestRunReport
#[derive(Debug, Clone, serde::Serialize)]
pub struct MentalTestRunReport {
    pub id: String,
    pub target: String,
    pub passed: bool,
    pub failed_steps: Vec<usize>,
    pub edge_cases_handled: bool,
    pub trace_path: String,
}
// END_MentalTestRunReport

impl MentalTestRunReport {
    // START_CONTRACT_MentalTestRunReport::to_xml
    // PURPOSE: Serialize a mental test run report as compact XML for trace persistence and MCP output
    // OUTPUTS: { String }
    // START_mental_test_run_report_to_xml
    pub fn to_xml(&self) -> String {
        let failed = self
            .failed_steps
            .iter()
            .map(|step| step.to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "<MentalTestRun id=\"{}\" target=\"{}\" passed=\"{}\">\n  <FailedSteps>{}</FailedSteps>\n  <EdgeCasesHandled>{}</EdgeCasesHandled>\n  <TracePath>{}</TracePath>\n</MentalTestRun>",
            xml_text(&self.id),
            xml_text(&self.target),
            self.passed,
            xml_text(&failed),
            self.edge_cases_handled,
            xml_text(&self.trace_path)
        )
    }
    // END_mental_test_run_report_to_xml
}

// START_CONTRACT_parse_mental_tests
// PURPOSE: Parse MentalTest definitions from DevelopmentPlan XML content
// INPUTS: { plan_content: &str }
// OUTPUTS: { anyhow::Result<Vec<MentalTestSpec>> }
// START_parse_mental_tests
pub fn parse_mental_tests(plan_content: &str) -> anyhow::Result<Vec<MentalTestSpec>> {
    let section = section_content(plan_content, "MentalTests");
    if section.trim().is_empty() {
        return Ok(Vec::new());
    }
    let test_re = regex::Regex::new(
        r#"(?s)<MentalTest\s+id="([^"]+)"\s+target="([^"]+)"\s+status="([^"]+)"[^>]*>(.*?)</MentalTest>"#,
    )?;
    let mut tests = Vec::new();
    for cap in test_re.captures_iter(&section) {
        let body = &cap[4];
        let result_text = tag_text(body, "Result");
        tests.push(MentalTestSpec {
            id: cap[1].to_string(),
            target: cap[2].to_string(),
            status: MentalTestStatus::from_value(&cap[3]),
            description: tag_text(body, "Description"),
            scenario: tag_text(body, "Scenario"),
            steps: parse_steps(&section_content(body, "Steps"))?,
            edge_cases: parse_edge_cases(&section_content(body, "EdgeCases"))?,
            result: result_text.clone(),
            rationale: tag_text(body, "Rationale"),
        });
    }
    Ok(tests)
}
// END_parse_mental_tests

// START_CONTRACT_scan_project_mental_tests
// PURPOSE: Validate DevelopmentPlan MentalTests against critical targets and current source modules
// INPUTS: { root: &Path }
// OUTPUTS: { anyhow::Result<MentalTestReport> }
// START_scan_project_mental_tests
pub fn scan_project_mental_tests(root: &Path) -> anyhow::Result<MentalTestReport> {
    let plan_path = root.join("docs").join("development-plan.xml");
    let content = std::fs::read_to_string(&plan_path).unwrap_or_default();
    let tests = parse_mental_tests(&content)?;
    let required_targets = extract_required_targets(&content);
    let source_modules = source_module_ids(root)?;
    let mut report = MentalTestReport {
        total: tests.len(),
        tests,
        required_targets,
        ..MentalTestReport::default()
    };

    for test in &report.tests {
        match test.status {
            MentalTestStatus::Pass => report.passed += 1,
            MentalTestStatus::Fail => report.failed += 1,
            MentalTestStatus::NotRun => report.not_run += 1,
            MentalTestStatus::NeedsClarification => report.needs_clarification += 1,
        }
        validate_test_shape(test, &mut report.errors);
        let module = target_module(&test.target);
        if module.starts_with("M-") && !source_modules.contains(&module) {
            report.drift_issues.push(format!(
                "{} targets missing source module {}",
                test.id, module
            ));
        }
    }

    for target in &report.required_targets {
        let covered = report
            .tests
            .iter()
            .any(|test| target_module(&test.target) == *target);
        if !covered {
            report.missing_required_targets.push(target.clone());
        }
    }
    Ok(report)
}
// END_scan_project_mental_tests

// START_CONTRACT_run_mental_test
// PURPOSE: Execute one deterministic mental test and persist a run trace under docs/mental-tests
// INPUTS: { root: &Path }, { module_id: &str }, { mental_test_id: &str }, { step_by_step: bool }
// OUTPUTS: { anyhow::Result<MentalTestRunReport> }
// SIDE_EFFECTS: writes docs/mental-tests/<mental_test_id>.xml
// START_run_mental_test
pub fn run_mental_test(
    root: &Path,
    module_id: &str,
    mental_test_id: &str,
    _step_by_step: bool,
) -> anyhow::Result<MentalTestRunReport> {
    let content = std::fs::read_to_string(root.join("docs").join("development-plan.xml"))?;
    let tests = parse_mental_tests(&content)?;
    let test = tests
        .into_iter()
        .find(|test| test.id == mental_test_id && target_module(&test.target) == module_id)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Mental test {} for module {} not found",
                mental_test_id,
                module_id
            )
        })?;
    let failed_steps = test
        .steps
        .iter()
        .filter(|step| !step.passed)
        .map(|step| step.n)
        .collect::<Vec<_>>();
    let edge_cases_handled = test
        .edge_cases
        .iter()
        .all(|case| !case.expectation.trim().is_empty());
    let passed =
        failed_steps.is_empty() && edge_cases_handled && test.status == MentalTestStatus::Pass;
    let trace_path = write_trace(root, &test, &failed_steps, edge_cases_handled, passed)?;
    Ok(MentalTestRunReport {
        id: test.id,
        target: test.target,
        passed,
        failed_steps,
        edge_cases_handled,
        trace_path: trace_path.display().to_string(),
    })
}
// END_run_mental_test

// START_CONTRACT_mental_tests_template
// PURPOSE: Build a compact passing MentalTests XML section for generated DevelopmentPlan artifacts
// INPUTS: { target_module: &str }, { target_function: &str }
// OUTPUTS: { String }
// START_mental_tests_template
pub fn mental_tests_template(target_module: &str, target_function: &str) -> String {
    let target = if target_function.trim().is_empty() {
        target_module.to_string()
    } else {
        format!("{}::{}", target_module, target_function)
    };
    format!(
        "    <MentalTest id=\"MT-001\" target=\"{}\" status=\"pass\">\n      <Description>Pre-code simulation for the primary generation path</Description>\n      <Scenario>GIVEN: a bounded implementation request. WHEN: the module contract is executed mentally before code.</Scenario>\n      <Steps>\n        <Step n=\"1\" action=\"readContract\" expected=\"module purpose and constraints are known\" actual=\"PASS — module purpose and constraints are known\">The agent reads MODULE_CONTRACT and linked DevelopmentPlan context.</Step>\n        <Step n=\"2\" action=\"simulateChange\" expected=\"bounded change has no unplanned side effects\" actual=\"PASS — bounded change has no unplanned side effects\">The agent traces expected inputs, outputs, and affected artifacts before editing.</Step>\n      </Steps>\n      <EdgeCases>\n        <Case id=\"EC-001\" description=\"Verification fails after implementation\" expectation=\"stop, inspect failure, redesign before more code\" />\n      </EdgeCases>\n      <Result>pass</Result>\n      <Rationale>The simulation covers contract reading, bounded execution, and failure handling before code generation.</Rationale>\n    </MentalTest>",
        xml_text(&target)
    )
}
// END_mental_tests_template

// END_public_api

fn parse_steps(content: &str) -> anyhow::Result<Vec<MentalTestStep>> {
    let step_re = regex::Regex::new(
        r#"(?s)<Step\s+n="([^"]+)"\s+action="([^"]*)"\s+expected="([^"]*)"\s+actual="([^"]*)"[^>]*>(.*?)</Step>"#,
    )?;
    Ok(step_re
        .captures_iter(content)
        .map(|cap| {
            let actual = cap[4].to_string();
            MentalTestStep {
                n: cap[1].parse().unwrap_or(0),
                action: cap[2].to_string(),
                expected: cap[3].to_string(),
                passed: step_passed(&cap[3], &actual),
                actual,
                description: cap[5].trim().to_string(),
            }
        })
        .collect())
}

fn parse_edge_cases(content: &str) -> anyhow::Result<Vec<EdgeCase>> {
    let case_re = regex::Regex::new(
        r#"<Case\s+id="([^"]+)"\s+description="([^"]*)"\s+expectation="([^"]*)"[^/]*/>"#,
    )?;
    Ok(case_re
        .captures_iter(content)
        .map(|cap| EdgeCase {
            id: cap[1].to_string(),
            description: cap[2].to_string(),
            expectation: cap[3].to_string(),
        })
        .collect())
}

fn validate_test_shape(test: &MentalTestSpec, errors: &mut Vec<String>) {
    if test.description.trim().is_empty() {
        errors.push(format!("{} missing Description", test.id));
    }
    if test.scenario.trim().is_empty() {
        errors.push(format!("{} missing Scenario", test.id));
    }
    if test.steps.is_empty() {
        errors.push(format!("{} must define at least one Step", test.id));
    }
    for step in &test.steps {
        if step.action.trim().is_empty() || step.expected.trim().is_empty() {
            errors.push(format!(
                "{} step {} missing action or expected",
                test.id, step.n
            ));
        }
    }
    if test.edge_cases.is_empty() {
        errors.push(format!("{} must define at least one EdgeCase", test.id));
    }
    if test.rationale.trim().is_empty() {
        errors.push(format!("{} missing Rationale", test.id));
    }
}

fn extract_required_targets(content: &str) -> Vec<String> {
    let section = section_content(content, "ArchitectureGraph");
    start_tags(&section, "Module")
        .into_iter()
        .filter(|tag| {
            matches!(
                attr_value(tag, "critical")
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .as_str(),
                "true" | "yes" | "1"
            ) || matches!(
                attr_value(tag, "complexity")
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .as_str(),
                "complex" | "critical"
            )
        })
        .filter_map(|tag| attr_value(&tag, "id"))
        .collect()
}

fn source_module_ids(root: &Path) -> anyhow::Result<BTreeSet<String>> {
    let contracts = ContractValidator::validate_project_with_profile(root, GraceProfile::Lite)?;
    Ok(contracts
        .contracts
        .into_iter()
        .filter_map(|contract| contract.module_id)
        .collect())
}

fn write_trace(
    root: &Path,
    test: &MentalTestSpec,
    failed_steps: &[usize],
    edge_cases_handled: bool,
    passed: bool,
) -> anyhow::Result<PathBuf> {
    let dir = root.join("docs").join("mental-tests");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.xml", test.id));
    let failed = failed_steps
        .iter()
        .map(|step| step.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let content = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<MentalTestTrace id=\"{}\" target=\"{}\" passed=\"{}\">\n  <FailedSteps>{}</FailedSteps>\n  <EdgeCasesHandled>{}</EdgeCasesHandled>\n  <Rationale>{}</Rationale>\n</MentalTestTrace>\n",
        xml_text(&test.id),
        xml_text(&test.target),
        passed,
        xml_text(&failed),
        edge_cases_handled,
        xml_text(&test.rationale)
    );
    std::fs::write(&path, content)?;
    Ok(path)
}

fn step_passed(expected: &str, actual: &str) -> bool {
    let expected = expected.trim().to_ascii_lowercase();
    let actual = actual.trim().to_ascii_lowercase();
    !expected.is_empty() && (actual.starts_with("pass") || actual.contains(&expected))
}

fn target_module(target: &str) -> String {
    target
        .split("::")
        .next()
        .unwrap_or(target)
        .split('.')
        .next()
        .unwrap_or(target)
        .trim()
        .to_string()
}

fn section_content(content: &str, tag: &str) -> String {
    let pattern = format!(
        r"(?s)<{}[^>]*>(.*?)</{}>",
        regex::escape(tag),
        regex::escape(tag)
    );
    regex::Regex::new(&pattern)
        .ok()
        .and_then(|re| re.captures(content))
        .and_then(|cap| cap.get(1))
        .map(|value| value.as_str().to_string())
        .unwrap_or_default()
}

fn tag_text(content: &str, tag: &str) -> String {
    let pattern = format!(
        r"(?s)<{}[^>]*>(.*?)</{}>",
        regex::escape(tag),
        regex::escape(tag)
    );
    regex::Regex::new(&pattern)
        .ok()
        .and_then(|re| re.captures(content))
        .and_then(|cap| cap.get(1))
        .map(|value| value.as_str().trim().to_string())
        .unwrap_or_default()
}

fn start_tags(content: &str, tag: &str) -> Vec<String> {
    let pattern = format!(r#"<{}\b[^>]*>"#, regex::escape(tag));
    regex::Regex::new(&pattern)
        .map(|re| {
            re.find_iter(content)
                .map(|m| m.as_str().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn attr_value(start_tag: &str, attr: &str) -> Option<String> {
    let pattern = format!(r#"{}\s*=\s*"([^"]*)""#, regex::escape(attr));
    regex::Regex::new(&pattern)
        .ok()
        .and_then(|re| re.captures(start_tag).map(|cap| cap[1].to_string()))
}

fn xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_parse_mental_tests_extracts_steps_and_edges
    // PURPOSE: Verify parser extracts MentalTest steps, statuses, and edge cases from DevelopmentPlan XML
    // START_test_parse_mental_tests_extracts_steps_and_edges
    #[test]
    fn test_parse_mental_tests_extracts_steps_and_edges() {
        let tests = parse_mental_tests(&sample_plan("pass", "PASS — ok")).expect("parse");
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].id, "MT-001");
        assert_eq!(tests[0].steps.len(), 1);
        assert!(tests[0].steps[0].passed);
        assert_eq!(tests[0].edge_cases.len(), 1);
    }
    // END_test_parse_mental_tests_extracts_steps_and_edges

    // START_CONTRACT_test_scan_reports_failed_mental_test
    // PURPOSE: Verify failed mental tests are reported as blocking
    // START_test_scan_reports_failed_mental_test
    #[test]
    fn test_scan_reports_failed_mental_test() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("docs")).expect("docs");
        std::fs::write(
            dir.path().join("docs/development-plan.xml"),
            sample_plan("fail", "FAIL — mismatch"),
        )
        .expect("plan");
        let report = scan_project_mental_tests(dir.path()).expect("scan");
        assert_eq!(report.failed, 1);
        assert!(!report.mental_tests_passed());
    }
    // END_test_scan_reports_failed_mental_test

    // START_CONTRACT_test_missing_required_target_is_reported
    // PURPOSE: Verify critical ArchitectureGraph modules require MentalTest coverage
    // START_test_missing_required_target_is_reported
    #[test]
    fn test_missing_required_target_is_reported() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("docs")).expect("docs");
        std::fs::write(
            dir.path().join("docs/development-plan.xml"),
            r#"<DevelopmentPlan><ArchitectureGraph><Module id="M-CRITICAL" critical="true" /></ArchitectureGraph><MentalTests></MentalTests></DevelopmentPlan>"#,
        )
        .expect("plan");
        let report = scan_project_mental_tests(dir.path()).expect("scan");
        assert_eq!(report.missing_required_targets, vec!["M-CRITICAL"]);
        assert!(!report.mental_tests_defined());
    }
    // END_test_missing_required_target_is_reported

    // START_CONTRACT_test_run_mental_test_persists_trace
    // PURPOSE: Verify run_mental_test writes a trace artifact and returns pass/fail status
    // START_test_run_mental_test_persists_trace
    #[test]
    fn test_run_mental_test_persists_trace() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("docs")).expect("docs");
        std::fs::write(
            dir.path().join("docs/development-plan.xml"),
            sample_plan("pass", "PASS — ok"),
        )
        .expect("plan");
        let report = run_mental_test(dir.path(), "M-TEST", "MT-001", true).expect("run");
        assert!(report.passed);
        assert!(dir.path().join("docs/mental-tests/MT-001.xml").exists());
    }
    // END_test_run_mental_test_persists_trace

    #[test]
    fn test_mental_test_failure_reports_failed_status() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("docs")).expect("docs");
        std::fs::write(
            dir.path().join("docs/development-plan.xml"),
            sample_plan("FAIL", "expected PASS but got FAIL"),
        )
        .expect("plan");
        let report = run_mental_test(dir.path(), "M-TEST", "MT-001", true).expect("run");
        assert!(!report.passed);
    }

    #[test]
    fn test_mental_test_missing_development_plan_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = run_mental_test(dir.path(), "M-TEST", "MT-001", true);
        assert!(result.is_err());
    }

    fn sample_plan(status: &str, actual: &str) -> String {
        format!(
            r#"<DevelopmentPlan>
  <ArchitectureGraph><Module id="M-TEST" critical="true" /></ArchitectureGraph>
  <MentalTests>
    <MentalTest id="MT-001" target="M-TEST::run" status="{status}">
      <Description>Simulation</Description>
      <Scenario>GIVEN x WHEN y</Scenario>
      <Steps>
        <Step n="1" action="run" expected="ok" actual="{actual}">Simulate one step.</Step>
      </Steps>
      <EdgeCases><Case id="EC-001" description="edge" expectation="handled" /></EdgeCases>
      <Result>{status}</Result>
      <Rationale>Trace is deterministic.</Rationale>
    </MentalTest>
  </MentalTests>
</DevelopmentPlan>"#
        )
    }
}
// END_public_api
