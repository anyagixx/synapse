// MODULE_CONTRACT
// MODULE_ID: M-TEST-RESILIENCE-CHAOS
// PURPOSE: Corrupted-state resilience harness for Synapse recovery behavior.
// SCOPE: Fixture-local corruption actions, command execution, output checks, recovery state checks, spec discovery support, and bounded reports.
// DEPENDS: M-CONFIG, M-INDEXER-STORAGE, M-GRACE-STATUS, M-TEST-FIXTURE
// LINKS:
//   -> Phase-81 (implements) - resilience chaos testing
//   -> M-TEST-FIXTURE (depends) - isolated project roots and XDG homes
//   -> NFR-002 (traces_to) - graceful failure and recovery evidence
//   -> NFR-003 (traces_to) - bounded chaos test output
//   <- V-M-TEST-RESILIENCE-CHAOS (verified_by) - graceful recovery verification

// START_MODULE_MAP
// ResilienceSpec - TOML resilience suite with one or more cases
// ResilienceCaseSpec - Fresh-fixture corruption and command assertion case
// CorruptionAction - Delete, truncate, write, remove_dir, rename, lock, and delete_index_storage actions
// ResilienceCommand - Command, exit, output, and path-state expectations
// run_resilience_spec_file - Parses and runs one TOML spec
// run_resilience_specs - Runs multiple specs with fail_fast handling
// render_resilience_report - Renders compact CLI text output
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Implemented Phase-81 resilience chaos harness]
// END_CHANGE_SUMMARY

use crate::test::fixture::{FixtureTemplate, TestFixture};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use std::process::Command as ProcessCommand;

const OUTPUT_CHAR_LIMIT: usize = 4000;

// START_public_api

// START_ResilienceOptions
#[derive(Debug, Clone, Default)]
pub struct ResilienceOptions {
    pub binary_path: Option<PathBuf>,
    pub fail_fast: bool,
}
// END_ResilienceOptions

// START_ResilienceSpec
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ResilienceSpec {
    pub name: Option<String>,
    #[serde(default)]
    pub fixture: FixtureTemplateName,
    #[serde(default)]
    pub fail_fast: Option<bool>,
    #[serde(default)]
    pub cases: Vec<ResilienceCaseSpec>,
}
// END_ResilienceSpec

// START_FixtureTemplateName
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FixtureTemplateName {
    Empty,
    #[default]
    Minimal,
    MultiModule,
    Broken,
}
// END_FixtureTemplateName

impl FixtureTemplateName {
    // START_CONTRACT_FixtureTemplateName::to_fixture_template
    // PURPOSE: Convert TOML fixture names to reusable test fixture templates
    // OUTPUTS: { FixtureTemplate }
    // LINKS:
    //   -> M-TEST-FIXTURE (depends) - fixture materialization
    // START_fixture_template_name_to_fixture_template
    fn to_fixture_template(&self) -> FixtureTemplate {
        match self {
            Self::Empty => FixtureTemplate::Empty,
            Self::Minimal => FixtureTemplate::Minimal,
            Self::MultiModule => FixtureTemplate::MultiModule,
            Self::Broken => FixtureTemplate::Broken,
        }
    }
    // END_fixture_template_name_to_fixture_template
}

// START_ResilienceCaseSpec
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ResilienceCaseSpec {
    pub name: String,
    #[serde(default)]
    pub fixture: Option<FixtureTemplateName>,
    #[serde(default)]
    pub actions: Vec<CorruptionAction>,
    pub command: ResilienceCommand,
}
// END_ResilienceCaseSpec

// START_CorruptionAction
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CorruptionAction {
    Delete { path: PathBuf },
    Truncate { path: PathBuf },
    Write { path: PathBuf, content: String },
    RemoveDir { path: PathBuf },
    Rename { from: PathBuf, to: PathBuf },
    Lock { path: PathBuf },
    WriteUserConfig { content: String },
    DeleteIndexStorage,
}
// END_CorruptionAction

// START_ResilienceCommand
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ResilienceCommand {
    pub args: Vec<String>,
    #[serde(default)]
    pub expect_exit: Option<i32>,
    #[serde(default)]
    pub stdout_contains: Vec<String>,
    #[serde(default)]
    pub stderr_contains: Vec<String>,
    #[serde(default)]
    pub paths_exist: Vec<PathBuf>,
    #[serde(default)]
    pub paths_missing: Vec<PathBuf>,
}
// END_ResilienceCommand

// START_ResilienceSuiteReport
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResilienceSuiteReport {
    pub spec: String,
    pub passed: bool,
    pub cases: Vec<ResilienceCaseResult>,
}
// END_ResilienceSuiteReport

// START_ResilienceCaseResult
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResilienceCaseResult {
    pub name: String,
    pub passed: bool,
    pub exit_code: Option<i32>,
    pub stdout_tail: String,
    pub stderr_tail: String,
    pub failures: Vec<String>,
}
// END_ResilienceCaseResult

// START_ResilienceRunReport
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResilienceRunReport {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub passed: bool,
    pub suites: Vec<ResilienceSuiteReport>,
}
// END_ResilienceRunReport

// START_CONTRACT_run_resilience_spec_file
// PURPOSE: Parse and execute one resilience TOML spec
// INPUTS: { spec_path: &Path }, { options: &ResilienceOptions }
// OUTPUTS: { anyhow::Result<ResilienceSuiteReport> }
// SIDE_EFFECTS: creates fresh fixtures, mutates only fixture-local files, invokes Synapse binary
// LINKS:
//   -> M-TEST-FIXTURE (depends) - fresh fixture per case
//   -> V-M-TEST-RESILIENCE-CHAOS (verified_by) - spec execution tests
// START_run_resilience_spec_file
pub fn run_resilience_spec_file(
    spec_path: &Path,
    options: &ResilienceOptions,
) -> anyhow::Result<ResilienceSuiteReport> {
    let content = std::fs::read_to_string(spec_path)
        .with_context(|| format!("read resilience spec {}", spec_path.display()))?;
    let spec: ResilienceSpec = toml::from_str(&content)
        .with_context(|| format!("parse resilience spec {}", spec_path.display()))?;
    run_resilience_spec(spec_path.display().to_string(), &spec, options)
}
// END_run_resilience_spec_file

// START_CONTRACT_run_resilience_specs
// PURPOSE: Run multiple resilience specs and aggregate pass/fail state
// INPUTS: { spec_paths: &[PathBuf] }, { options: &ResilienceOptions }
// OUTPUTS: { anyhow::Result<ResilienceRunReport> }
// LINKS:
//   -> M-CLI-TEST-COMMANDS (depends) - CLI invokes spec batches
//   -> NFR-003 (traces_to) - bounded aggregate chaos report
// START_run_resilience_specs
pub fn run_resilience_specs(
    spec_paths: &[PathBuf],
    options: &ResilienceOptions,
) -> anyhow::Result<ResilienceRunReport> {
    if spec_paths.is_empty() {
        anyhow::bail!("no resilience specs provided");
    }
    let mut suites = Vec::new();
    for spec_path in spec_paths {
        let suite = run_resilience_spec_file(spec_path, options)?;
        let suite_passed = suite.passed;
        suites.push(suite);
        if options.fail_fast && !suite_passed {
            break;
        }
    }
    Ok(ResilienceRunReport {
        kind: "synapse.test.resilience",
        passed: suites.iter().all(|suite| suite.passed),
        suites,
    })
}
// END_run_resilience_specs

// START_CONTRACT_render_resilience_report
// PURPOSE: Render compact human-readable resilience results
// INPUTS: { report: &ResilienceRunReport }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-003 (traces_to) - chaos output should stay bounded
// START_render_resilience_report
pub fn render_resilience_report(report: &ResilienceRunReport) -> String {
    let total_cases = report
        .suites
        .iter()
        .map(|suite| suite.cases.len())
        .sum::<usize>();
    let passed_cases = report
        .suites
        .iter()
        .flat_map(|suite| suite.cases.iter())
        .filter(|case| case.passed)
        .count();
    let mut lines = vec![format!(
        "Resilience chaos: {passed_cases}/{total_cases} cases passed"
    )];
    for suite in &report.suites {
        lines.push(format!(
            "- suite {}: {}",
            suite.spec,
            if suite.passed { "passed" } else { "failed" }
        ));
        for case in &suite.cases {
            lines.push(format!(
                "  - {}: {} exit={:?}",
                case.name,
                if case.passed { "passed" } else { "failed" },
                case.exit_code
            ));
            for failure in &case.failures {
                lines.push(format!("    failure: {failure}"));
            }
        }
    }
    lines.join("\n")
}
// END_render_resilience_report

// END_public_api

// START_CONTRACT_run_resilience_spec
// PURPOSE: Execute parsed spec cases using fresh fixtures
// INPUTS: { spec_name: String }, { spec: &ResilienceSpec }, { options: &ResilienceOptions }
// OUTPUTS: { anyhow::Result<ResilienceSuiteReport> }
// LINKS:
//   -> NFR-002 (traces_to) - each case must be isolated
// <LOG id="resilience_corruption_applied" level="INFO" ref="resilience-corruption" module="M-TEST-RESILIENCE-CHAOS" contract="run_resilience_spec">
//   EVENT: resilience_corruption_applied
//   EXPECTATION: corruption actions only touch fixture-local paths
//   DECISION: build a fresh fixture for each case before applying actions
//   RESULT: success
//   TRACEABILITY: NFR-002
// </LOG>
// START_run_resilience_spec
fn run_resilience_spec(
    spec_name: String,
    spec: &ResilienceSpec,
    options: &ResilienceOptions,
) -> anyhow::Result<ResilienceSuiteReport> {
    let fail_fast = options.fail_fast || spec.fail_fast.unwrap_or(false);
    let binary_path = options
        .binary_path
        .clone()
        .unwrap_or_else(resolve_syn_binary_path);
    let mut cases = Vec::new();
    for case in &spec.cases {
        let fixture_template = case.fixture.as_ref().unwrap_or(&spec.fixture);
        let fixture = TestFixture::builder()
            .with_template(fixture_template.to_fixture_template())
            .build()?;
        let result = run_resilience_case(&binary_path, &fixture, case)?;
        let passed = result.passed;
        cases.push(result);
        if fail_fast && !passed {
            break;
        }
    }
    Ok(ResilienceSuiteReport {
        spec: spec_name,
        passed: cases.iter().all(|case| case.passed),
        cases,
    })
}
// END_run_resilience_spec

// START_CONTRACT_run_resilience_case
// PURPOSE: Apply corruption actions, run command, and evaluate expectations for one case
// INPUTS: { binary_path: &Path }, { fixture: &TestFixture }, { case: &ResilienceCaseSpec }
// OUTPUTS: { anyhow::Result<ResilienceCaseResult> }
// SIDE_EFFECTS: mutates fixture-local files and invokes child process
// LINKS:
//   -> V-M-TEST-RESILIENCE-CHAOS (verified_by) - corruption and recovery checks
// <LOG id="resilience_recovery_checked" level="INFO" ref="resilience-recovery" module="M-TEST-RESILIENCE-CHAOS" contract="run_resilience_case">
//   EVENT: resilience_recovery_checked
//   EXPECTATION: command output and path-state checks produce actionable failures
//   DECISION: collect all expectation failures unless fail_fast stops the suite
//   RESULT: success
//   TRACEABILITY: NFR-002,NFR-003
// </LOG>
// START_run_resilience_case
fn run_resilience_case(
    binary_path: &Path,
    fixture: &TestFixture,
    case: &ResilienceCaseSpec,
) -> anyhow::Result<ResilienceCaseResult> {
    for action in &case.actions {
        apply_corruption_action(fixture, action)?;
    }
    let output = ProcessCommand::new(binary_path)
        .args(&case.command.args)
        .current_dir(fixture.root())
        .env("RUST_LOG", "error")
        .envs(fixture.command_env())
        .output()
        .with_context(|| format!("run resilience command {}", case.command.args.join(" ")))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let mut failures = Vec::new();
    let exit_code = output.status.code();
    if let Some(expected) = case.command.expect_exit {
        if exit_code != Some(expected) {
            failures.push(format!("expected exit {expected}, got {exit_code:?}"));
        }
    }
    for needle in &case.command.stdout_contains {
        if !stdout.contains(needle) {
            failures.push(format!("stdout missing {:?}", needle));
        }
    }
    for needle in &case.command.stderr_contains {
        if !stderr.contains(needle) {
            failures.push(format!("stderr missing {:?}", needle));
        }
    }
    for path in &case.command.paths_exist {
        if !safe_fixture_path(fixture.root(), path)?.exists() {
            failures.push(format!("path should exist: {}", path.display()));
        }
    }
    for path in &case.command.paths_missing {
        if safe_fixture_path(fixture.root(), path)?.exists() {
            failures.push(format!("path should be missing: {}", path.display()));
        }
    }
    Ok(ResilienceCaseResult {
        name: case.name.clone(),
        passed: failures.is_empty(),
        exit_code,
        stdout_tail: compact_text(&stdout),
        stderr_tail: compact_text(&stderr),
        failures,
    })
}
// END_run_resilience_case

// START_CONTRACT_apply_corruption_action
// PURPOSE: Apply one fixture-local corruption action
// INPUTS: { fixture: &TestFixture }, { action: &CorruptionAction }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: mutates files under fixture root or isolated fixture data home
// LINKS:
//   -> NFR-002 (traces_to) - chaos actions must not escape fixture isolation
// START_apply_corruption_action
fn apply_corruption_action(fixture: &TestFixture, action: &CorruptionAction) -> anyhow::Result<()> {
    match action {
        CorruptionAction::Delete { path } => {
            let target = safe_fixture_path(fixture.root(), path)?;
            if target.is_dir() {
                std::fs::remove_dir_all(target)?;
            } else if target.exists() {
                std::fs::remove_file(target)?;
            }
        }
        CorruptionAction::Truncate { path } => {
            std::fs::write(safe_fixture_path(fixture.root(), path)?, "")?;
        }
        CorruptionAction::Write { path, content } => {
            let target = safe_fixture_path(fixture.root(), path)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(target, content)?;
        }
        CorruptionAction::RemoveDir { path } => {
            let target = safe_fixture_path(fixture.root(), path)?;
            if target.exists() {
                std::fs::remove_dir_all(target)?;
            }
        }
        CorruptionAction::Rename { from, to } => {
            let source = safe_fixture_path(fixture.root(), from)?;
            let target = safe_fixture_path(fixture.root(), to)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::rename(source, target)?;
        }
        CorruptionAction::Lock { path } => {
            let target = safe_fixture_path(fixture.root(), path)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(target.with_extension("lock"), "locked")?;
        }
        CorruptionAction::WriteUserConfig { content } => {
            let config_path = fixture.config_home().join("synapse").join("synapsec.toml");
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(config_path, content)?;
        }
        CorruptionAction::DeleteIndexStorage => {
            let index_path = fixture_index_storage_path(fixture);
            if let Some(parent) = index_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if index_path.exists() {
                std::fs::remove_file(index_path)?;
            }
        }
    }
    Ok(())
}
// END_apply_corruption_action

// START_CONTRACT_safe_fixture_path
// PURPOSE: Resolve a relative fixture path and reject absolute paths or parent traversal
// INPUTS: { root: &Path }, { relative: &Path }
// OUTPUTS: { anyhow::Result<PathBuf> }
// LINKS:
//   -> V-M-TEST-RESILIENCE-CHAOS (verified_by) - fixture-local safety checks
// START_safe_fixture_path
fn safe_fixture_path(root: &Path, relative: &Path) -> anyhow::Result<PathBuf> {
    if relative.is_absolute() {
        anyhow::bail!("resilience path must be relative: {}", relative.display());
    }
    for component in relative.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            anyhow::bail!(
                "resilience path must stay inside fixture: {}",
                relative.display()
            );
        }
    }
    Ok(root.join(relative))
}
// END_safe_fixture_path

// START_CONTRACT_fixture_index_storage_path
// PURPOSE: Return the index storage file path inside the fixture XDG data home
// INPUTS: { fixture: &TestFixture }
// OUTPUTS: { PathBuf }
// LINKS:
//   -> M-INDEXER-STORAGE (depends) - mirrors Storage root hashing while preserving fixture isolation
// START_fixture_index_storage_path
fn fixture_index_storage_path(fixture: &TestFixture) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(fixture.root().to_string_lossy().as_bytes());
    let hash = hasher
        .finalize()
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    fixture
        .data_home()
        .join("synapse")
        .join("index")
        .join(hash)
        .join("blocks.json")
}
// END_fixture_index_storage_path

// START_CONTRACT_resolve_syn_binary_path
// PURPOSE: Resolve the Synapse binary for resilience CLI runs and tests
// OUTPUTS: { PathBuf }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic child command execution
// START_resolve_syn_binary_path
fn resolve_syn_binary_path() -> PathBuf {
    if let Ok(path) = std::env::var("SYN_TEST_BINARY") {
        return PathBuf::from(path);
    }
    if let Some(path) = option_env!("CARGO_BIN_EXE_syn") {
        return PathBuf::from(path);
    }
    if let Ok(current) = std::env::current_exe() {
        if current
            .file_stem()
            .and_then(|value| value.to_str())
            .is_some_and(|stem| stem == "syn")
        {
            return current;
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("target/debug/syn")
}
// END_resolve_syn_binary_path

// START_CONTRACT_compact_text
// PURPOSE: Return bounded command output text
// INPUTS: { text: &str }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-003 (traces_to) - chaos failure output must stay bounded
// START_compact_text
fn compact_text(text: &str) -> String {
    syn_core::utils::truncate_chars(text.trim(), OUTPUT_CHAR_LIMIT)
}
// END_compact_text

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resilience_spec_parses_tagged_actions() {
        let spec: ResilienceSpec = toml::from_str(
            r#"
name = "fixture"
fixture = "minimal"

[[cases]]
name = "bad-config"
command = { args = ["config", "list"], expect_exit = 0 }

[[cases.actions]]
kind = "write"
path = "synapsec.toml"
content = "bad = ["
"#,
        )
        .expect("spec");

        assert_eq!(spec.cases.len(), 1);
        assert!(matches!(
            spec.cases[0].actions[0],
            CorruptionAction::Write { .. }
        ));
    }

    #[test]
    fn resilience_safe_path_rejects_absolute_and_parent_paths() {
        let dir = TempDir::new().expect("temp dir");

        assert!(safe_fixture_path(dir.path(), Path::new("docs/graph-index.xml")).is_ok());
        assert!(safe_fixture_path(dir.path(), Path::new("../outside")).is_err());
        assert!(safe_fixture_path(dir.path(), Path::new("/tmp/outside")).is_err());
    }

    #[test]
    fn resilience_delete_index_storage_stays_in_isolated_data_home() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let index_path = fixture_index_storage_path(&fixture);
        std::fs::create_dir_all(index_path.parent().unwrap()).expect("index dir");
        std::fs::write(&index_path, "[]").expect("index file");

        apply_corruption_action(&fixture, &CorruptionAction::DeleteIndexStorage)
            .expect("delete index storage");

        assert!(!index_path.exists());
    }

    #[test]
    fn resilience_output_checks_report_actionable_failures() {
        let case = ResilienceCaseSpec {
            name: "missing-output".into(),
            fixture: None,
            actions: Vec::new(),
            command: ResilienceCommand {
                args: vec!["config".into(), "get".into(), "project.name".into()],
                expect_exit: Some(0),
                stdout_contains: vec!["definitely-missing".into()],
                stderr_contains: Vec::new(),
                paths_exist: Vec::new(),
                paths_missing: Vec::new(),
            },
        };
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let binary = PathBuf::from("/bin/echo");

        let result = run_resilience_case(&binary, &fixture, &case).expect("echo case");

        assert!(!result.passed);
        assert!(result
            .failures
            .iter()
            .any(|failure| failure.contains("stdout missing")));
    }
}
