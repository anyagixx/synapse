// MODULE_CONTRACT
// MODULE_ID: M-TEST-E2E-RUNNER
// PURPOSE: TOML scenario runner for full-cycle Synapse E2E tests against isolated fixtures.
// SCOPE: Scenario parsing, command steps, write_file steps, snapshot steps, fail_fast, and result reporting.
// DEPENDS: M-TEST-FIXTURE, M-TEST-SNAPSHOT, M-CLI
// LINKS:
//   -> Phase-77 (implements) - E2E runner and snapshots
//   -> NFR-002 (traces_to) - reliable end-to-end release evidence
//   -> NFR-003 (traces_to) - bounded scenario output
//   <- V-M-TEST-E2E-RUNNER (verified_by) - scenario execution verification

// START_MODULE_MAP
// E2EScenario - TOML scenario root with metadata and ordered steps
// ScenarioMeta - Scenario name, fixture selection, description, and fail-fast mode
// E2EStep - Command, write_file, and snapshot command step variants
// StepExpectation - Exit/stdout/stderr assertions for command steps
// E2EResult - Serializable scenario run result
// StepFailure - Bounded actionable failure detail
// run_e2e_scenario - Load a scenario file, create fixture, and run steps
// resolve_scenario_path - Resolve repository scenario paths for cargo and tarpaulin runners
// run_step - Execute one scenario step against a fixture
// run_command_in_fixture - Execute shell command inside isolated fixture root
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.3 - Made repository scenario paths tarpaulin-safe]
// END_CHANGE_SUMMARY

use crate::test::fixture::{FixtureBuilder, FixtureTemplate, TestFixture};
use crate::test::snapshot::assert_snapshot;
use crate::utils::truncate_chars;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const FAILURE_OUTPUT_LIMIT: usize = 500;
const SHELL_NOT_FOUND_EXIT: i32 = 127;
const DEFAULT_FAIL_FAST: bool = true;
const SNAPSHOT_DIR: &str = "tests/snapshots";

// START_public_api

// START_E2EScenario
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct E2EScenario {
    pub scenario: ScenarioMeta,
    #[serde(default)]
    pub steps: Vec<E2EStep>,
}
// END_E2EScenario

// START_ScenarioMeta
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ScenarioMeta {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub fixture: String,
    #[serde(default = "default_fail_fast")]
    pub fail_fast: bool,
}
// END_ScenarioMeta

// START_E2EStep
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum E2EStep {
    SnapshotCommand(SnapshotCommandStep),
    Command(CommandStep),
    WriteFile(WriteFileStep),
}
// END_E2EStep

// START_CommandStep
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CommandStep {
    pub command: String,
    #[serde(default)]
    pub expected: StepExpectation,
}
// END_CommandStep

// START_SnapshotCommandStep
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SnapshotCommandStep {
    pub command: String,
    pub snapshot: String,
    #[serde(default)]
    pub expected: StepExpectation,
    #[serde(default)]
    pub update_snapshot: bool,
}
// END_SnapshotCommandStep

// START_WriteFileStep
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WriteFileStep {
    pub write_file: FileWrite,
}
// END_WriteFileStep

// START_FileWrite
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileWrite {
    pub path: PathBuf,
    pub content: String,
}
// END_FileWrite

// START_StepExpectation
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct StepExpectation {
    #[serde(default)]
    pub exit_code: i32,
    #[serde(default)]
    pub stdout_contains: Option<String>,
    #[serde(default)]
    pub stdout_not_contains: Option<String>,
    #[serde(default)]
    pub stderr_contains: Option<String>,
}
// END_StepExpectation

// START_E2EResult
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct E2EResult {
    pub scenario: String,
    pub passed: bool,
    pub steps_total: usize,
    pub steps_passed: usize,
    pub steps_failed: usize,
    pub failures: Vec<StepFailure>,
    pub duration_ms: u64,
}
// END_E2EResult

// START_StepFailure
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StepFailure {
    pub step_index: usize,
    pub command: String,
    pub expected: String,
    pub actual: String,
}
// END_StepFailure

// START_CONTRACT_run_e2e_scenario
// PURPOSE: Run a TOML-defined E2E scenario inside an isolated fixture and return bounded results
// INPUTS: { scenario_path: &Path }
// OUTPUTS: { anyhow::Result<E2EResult> }
// SIDE_EFFECTS: reads scenario TOML, creates temp fixture, writes requested fixture files, executes child shell commands
// LINKS:
//   -> M-TEST-FIXTURE (depends) - isolated project setup
//   -> M-TEST-SNAPSHOT (depends) - snapshot step comparison
//   -> NFR-002 (traces_to) - full-cycle release verification evidence
//   -> NFR-003 (traces_to) - bounded failure output
// <LOG id="e2e_scenario_started" level="INFO" ref="e2e-scenario-start" module="M-TEST-E2E-RUNNER" contract="run_e2e_scenario">
//   EVENT: e2e_scenario_started
//   EXPECTATION: scenario execution creates a fixture before any command runs
//   DECISION: fixture selection is read from scenario metadata
//   RESULT: success
//   TRACEABILITY: NFR-002
// </LOG>
// <LOG id="e2e_scenario_completed" level="INFO" ref="e2e-scenario-complete" module="M-TEST-E2E-RUNNER" contract="run_e2e_scenario">
//   EVENT: e2e_scenario_completed
//   EXPECTATION: result reports total, passed, failed, failures, and duration
//   DECISION: scenario success is derived from zero failed steps
//   RESULT: success
//   TRACEABILITY: NFR-002
// </LOG>
// START_run_e2e_scenario
pub fn run_e2e_scenario(scenario_path: &Path) -> anyhow::Result<E2EResult> {
    let started = Instant::now();
    let scenario_path = resolve_scenario_path(scenario_path)?;
    let content = std::fs::read_to_string(scenario_path)?;
    let scenario: E2EScenario = toml::from_str(&content)?;
    let template = fixture_template_from_name(&scenario.scenario.fixture)?;
    let fixture = FixtureBuilder::new().with_template(template).build()?;

    let mut failures = Vec::new();
    let mut steps_passed = 0_usize;
    let mut steps_failed = 0_usize;

    for (index, step) in scenario.steps.iter().enumerate() {
        match run_step(&fixture, index, step)? {
            StepOutcome::Passed => steps_passed += 1,
            StepOutcome::Failed(failure) => {
                steps_failed += 1;
                failures.push(failure);
                if scenario.scenario.fail_fast {
                    break;
                }
            }
        }
    }

    Ok(E2EResult {
        scenario: scenario.scenario.name,
        passed: steps_failed == 0,
        steps_total: scenario.steps.len(),
        steps_passed,
        steps_failed,
        failures,
        duration_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
    })
}
// END_run_e2e_scenario

// END_public_api

// START_CONTRACT_resolve_scenario_path
// PURPOSE: Resolve scenario files from either the current working directory or the crate manifest root
// INPUTS: { scenario_path: &Path }
// OUTPUTS: { anyhow::Result<PathBuf> }
// LINKS:
//   -> Phase-94 (implements) - tarpaulin coverage runner compatibility
//   -> NFR-002 (traces_to) - E2E release evidence must not depend on runner cwd
// START_resolve_scenario_path
fn resolve_scenario_path(scenario_path: &Path) -> anyhow::Result<PathBuf> {
    if scenario_path.exists() {
        return Ok(scenario_path.to_path_buf());
    }
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(scenario_path);
    if manifest_path.exists() {
        return Ok(manifest_path);
    }
    Ok(scenario_path.to_path_buf())
}
// END_resolve_scenario_path

// START_StepOutcome
#[derive(Debug, Clone, PartialEq, Eq)]
enum StepOutcome {
    Passed,
    Failed(StepFailure),
}
// END_StepOutcome

// START_CommandResult
#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandResult {
    exit_code: i32,
    stdout: String,
    stderr: String,
}
// END_CommandResult

// START_CONTRACT_default_fail_fast
// PURPOSE: Return the default scenario fail-fast setting
// OUTPUTS: { bool }
// LINKS:
//   -> NFR-002 (traces_to) - failing E2E scenarios stop deterministically by default
// START_default_fail_fast
fn default_fail_fast() -> bool {
    DEFAULT_FAIL_FAST
}
// END_default_fail_fast

// START_CONTRACT_fixture_template_from_name
// PURPOSE: Map scenario fixture names to fixture templates
// INPUTS: { name: &str }
// OUTPUTS: { anyhow::Result<FixtureTemplate> }
// LINKS:
//   -> M-TEST-FIXTURE (depends) - scenario fixture selection
//   -> NFR-002 (traces_to) - fixture selection must be explicit
// START_fixture_template_from_name
fn fixture_template_from_name(name: &str) -> anyhow::Result<FixtureTemplate> {
    match name {
        "empty" => Ok(FixtureTemplate::Empty),
        "minimal" => Ok(FixtureTemplate::Minimal),
        "multi-module" | "multimodule" => Ok(FixtureTemplate::MultiModule),
        "broken" => Ok(FixtureTemplate::Broken),
        other => anyhow::bail!("Unknown fixture template: {other}"),
    }
}
// END_fixture_template_from_name

// START_CONTRACT_run_step
// PURPOSE: Execute one scenario step and return pass/fail outcome
// INPUTS: { fixture: &TestFixture }, { index: usize }, { step: &E2EStep }
// OUTPUTS: { anyhow::Result<StepOutcome> }
// SIDE_EFFECTS: may write fixture files, run a child command, or write snapshot files
// LINKS:
//   -> M-TEST-FIXTURE (depends) - step execution root
//   -> M-TEST-SNAPSHOT (depends) - snapshot command steps
//   -> NFR-002 (traces_to) - E2E step verification
// <LOG id="e2e_step_completed" level="INFO" ref="e2e-step-complete" module="M-TEST-E2E-RUNNER" contract="run_step">
//   EVENT: e2e_step_completed
//   EXPECTATION: every executed step returns either passed or a bounded failure
//   DECISION: step variant selects command, snapshot, or write_file execution path
//   RESULT: success
//   TRACEABILITY: NFR-002
// </LOG>
// START_run_step
fn run_step(fixture: &TestFixture, index: usize, step: &E2EStep) -> anyhow::Result<StepOutcome> {
    match step {
        E2EStep::Command(command_step) => {
            let output = run_command_in_fixture(fixture, &command_step.command);
            Ok(evaluate_command_step(
                index,
                &command_step.command,
                &command_step.expected,
                &output,
            ))
        }
        E2EStep::SnapshotCommand(snapshot_step) => {
            let output = run_command_in_fixture(fixture, &snapshot_step.command);
            let expectation = evaluate_command_step(
                index,
                &snapshot_step.command,
                &snapshot_step.expected,
                &output,
            );
            if let StepOutcome::Failed(_) = expectation {
                return Ok(expectation);
            }
            let snapshot = assert_snapshot(
                &fixture.root().join(SNAPSHOT_DIR),
                &snapshot_step.snapshot,
                &output.stdout,
                snapshot_step.update_snapshot,
            )?;
            if snapshot.matched {
                Ok(StepOutcome::Passed)
            } else {
                Ok(StepOutcome::Failed(StepFailure {
                    step_index: index,
                    command: snapshot_step.command.clone(),
                    expected: format!(
                        "stdout matches snapshot {}",
                        snapshot.snapshot_path.display()
                    ),
                    actual: snapshot
                        .diff
                        .unwrap_or_else(|| "snapshot output differs".to_string()),
                }))
            }
        }
        E2EStep::WriteFile(write_step) => {
            write_fixture_file(fixture.root(), &write_step.write_file)?;
            Ok(StepOutcome::Passed)
        }
    }
}
// END_run_step

// START_CONTRACT_evaluate_command_step
// PURPOSE: Evaluate command output against step expectations
// INPUTS: { index: usize }, { command: &str }, { expected: &StepExpectation }, { output: &CommandResult }
// OUTPUTS: { StepOutcome }
// LINKS:
//   -> M-TEST-E2E-RUNNER (depends) - command expectation checks
//   -> NFR-002 (traces_to) - actionable E2E failure reasons
// START_evaluate_command_step
fn evaluate_command_step(
    index: usize,
    command: &str,
    expected: &StepExpectation,
    output: &CommandResult,
) -> StepOutcome {
    let mut failures = Vec::new();
    if output.exit_code != expected.exit_code {
        failures.push(format!(
            "exit_code expected {}, got {}",
            expected.exit_code, output.exit_code
        ));
    }
    if let Some(needle) = &expected.stdout_contains {
        if !output.stdout.contains(needle) {
            failures.push(format!(
                "stdout missing {:?}; stdout={}",
                needle,
                truncate_chars(&output.stdout, FAILURE_OUTPUT_LIMIT)
            ));
        }
    }
    if let Some(needle) = &expected.stdout_not_contains {
        if output.stdout.contains(needle) {
            failures.push(format!(
                "stdout unexpectedly contains {:?}; stdout={}",
                needle,
                truncate_chars(&output.stdout, FAILURE_OUTPUT_LIMIT)
            ));
        }
    }
    if let Some(needle) = &expected.stderr_contains {
        if !output.stderr.contains(needle) {
            failures.push(format!(
                "stderr missing {:?}; stderr={}",
                needle,
                truncate_chars(&output.stderr, FAILURE_OUTPUT_LIMIT)
            ));
        }
    }
    if failures.is_empty() {
        StepOutcome::Passed
    } else {
        StepOutcome::Failed(StepFailure {
            step_index: index,
            command: command.to_string(),
            expected: format!("{expected:?}"),
            actual: failures.join("; "),
        })
    }
}
// END_evaluate_command_step

// START_CONTRACT_write_fixture_file
// PURPOSE: Write one scenario-defined file inside the fixture root after rejecting path traversal
// INPUTS: { root: &Path }, { write: &FileWrite }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: creates parent directories and writes a file under fixture root
// LINKS:
//   -> M-TEST-FIXTURE (depends) - scenario file mutation
//   -> NFR-002 (traces_to) - scenarios must not write outside fixtures
// START_write_fixture_file
fn write_fixture_file(root: &Path, write: &FileWrite) -> anyhow::Result<()> {
    let path = safe_fixture_path(root, &write.path)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, &write.content)?;
    Ok(())
}
// END_write_fixture_file

// START_CONTRACT_safe_fixture_path
// PURPOSE: Join a fixture-relative path while rejecting absolute paths and parent traversal
// INPUTS: { root: &Path }, { relative: &Path }
// OUTPUTS: { anyhow::Result<PathBuf> }
// LINKS:
//   -> M-TEST-FIXTURE (depends) - fixture isolation
//   -> NFR-002 (traces_to) - E2E scenarios cannot mutate developer checkout
// START_safe_fixture_path
fn safe_fixture_path(root: &Path, relative: &Path) -> anyhow::Result<PathBuf> {
    if relative.is_absolute() {
        anyhow::bail!(
            "E2E write_file path must be relative: {}",
            relative.display()
        );
    }
    for component in relative.components() {
        match component {
            Component::Normal(_) => {}
            _ => anyhow::bail!(
                "E2E write_file path cannot escape fixture: {}",
                relative.display()
            ),
        }
    }
    Ok(root.join(relative))
}
// END_safe_fixture_path

// START_CONTRACT_run_command_in_fixture
// PURPOSE: Run a shell command inside a fixture root with isolated config/data homes
// INPUTS: { fixture: &TestFixture }, { command: &str }
// OUTPUTS: { CommandResult }
// SIDE_EFFECTS: executes a child shell process
// LINKS:
//   -> M-TEST-FIXTURE (depends) - fixture root and XDG env
//   -> NFR-002 (traces_to) - commands never run in developer checkout
// START_run_command_in_fixture
fn run_command_in_fixture(fixture: &TestFixture, command: &str) -> CommandResult {
    if command.trim().is_empty() {
        return CommandResult {
            exit_code: SHELL_NOT_FOUND_EXIT,
            stdout: String::new(),
            stderr: "empty command".to_string(),
        };
    }

    let mut process = shell_command(command);
    process.current_dir(fixture.root());
    for (key, value) in fixture.command_env() {
        process.env(key, value);
    }

    match process.output() {
        Ok(output) => CommandResult {
            exit_code: output.status.code().unwrap_or(SHELL_NOT_FOUND_EXIT),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Err(error) => CommandResult {
            exit_code: SHELL_NOT_FOUND_EXIT,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}
// END_run_command_in_fixture

// START_CONTRACT_shell_command
// PURPOSE: Build a platform shell command for scenario command execution
// INPUTS: { command: &str }
// OUTPUTS: { Command }
// LINKS:
//   -> M-TEST-E2E-RUNNER (depends) - command execution adapter
//   -> NFR-002 (traces_to) - shell execution is explicit and fixture-scoped
// START_shell_command
fn shell_command(command: &str) -> Command {
    if cfg!(windows) {
        let mut process = Command::new("cmd");
        process.arg("/C").arg(command);
        process
    } else {
        let mut process = Command::new("sh");
        process.arg("-c").arg(command);
        process
    }
}
// END_shell_command

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_scenario(dir: &TempDir, content: &str) -> PathBuf {
        let path = dir.path().join("scenario.toml");
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn e2e_test_parses_command_write_and_snapshot_steps() {
        let content = r#"
[scenario]
name = "Parse"
fixture = "minimal"

[[steps]]
command = "printf ok"
expected = { exit_code = 0, stdout_contains = "ok" }

[[steps]]
write_file = { path = "src/generated.rs", content = "pub fn generated() {}" }

[[steps]]
command = "printf snap"
snapshot = "snap-output"
"#;

        let scenario: E2EScenario = toml::from_str(content).unwrap();

        assert_eq!(scenario.scenario.name, "Parse");
        assert_eq!(scenario.steps.len(), 3);
        assert!(matches!(scenario.steps[0], E2EStep::Command(_)));
        assert!(matches!(scenario.steps[1], E2EStep::WriteFile(_)));
        assert!(matches!(scenario.steps[2], E2EStep::SnapshotCommand(_)));
    }

    #[test]
    fn e2e_test_command_expectation_failure_is_actionable() {
        let dir = TempDir::new().unwrap();
        let path = write_scenario(
            &dir,
            r#"
[scenario]
name = "Failure"
fixture = "minimal"

[[steps]]
command = "printf hello"
expected = { exit_code = 0, stdout_contains = "missing" }
"#,
        );

        let result = run_e2e_scenario(&path).unwrap();

        assert!(!result.passed);
        assert_eq!(result.steps_failed, 1);
        assert!(result.failures[0].actual.contains("stdout missing"));
    }

    #[test]
    fn e2e_test_fail_fast_stops_after_first_failed_step() {
        let dir = TempDir::new().unwrap();
        let path = write_scenario(
            &dir,
            r#"
[scenario]
name = "Fail fast"
fixture = "minimal"
fail_fast = true

[[steps]]
command = "false"
expected = { exit_code = 0 }

[[steps]]
command = "printf should-not-run"
expected = { exit_code = 0 }
"#,
        );

        let result = run_e2e_scenario(&path).unwrap();

        assert!(!result.passed);
        assert_eq!(result.steps_total, 2);
        assert_eq!(result.steps_passed, 0);
        assert_eq!(result.steps_failed, 1);
    }

    #[test]
    fn e2e_test_write_file_step_runs_inside_fixture() {
        let dir = TempDir::new().unwrap();
        let path = write_scenario(
            &dir,
            r#"
[scenario]
name = "Write file"
fixture = "minimal"

[[steps]]
write_file = { path = "src/generated.txt", content = "generated" }

[[steps]]
command = "test -f src/generated.txt"
expected = { exit_code = 0 }
"#,
        );

        let result = run_e2e_scenario(&path).unwrap();

        assert!(result.passed);
        assert_eq!(result.steps_passed, 2);
    }

    #[test]
    fn e2e_test_snapshot_step_creates_baseline_and_passes() {
        let dir = TempDir::new().unwrap();
        let path = write_scenario(
            &dir,
            r#"
[scenario]
name = "Snapshot"
fixture = "minimal"

[[steps]]
command = "printf snapshot-output"
snapshot = "command-output"
expected = { exit_code = 0 }
"#,
        );

        let result = run_e2e_scenario(&path).unwrap();

        assert!(result.passed);
        assert_eq!(result.steps_passed, 1);
    }

    #[test]
    fn e2e_test_write_file_rejects_path_escape() {
        let fixture = FixtureBuilder::new()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .unwrap();
        let write = FileWrite {
            path: PathBuf::from("../escape.txt"),
            content: "bad".to_string(),
        };

        let error = write_fixture_file(fixture.root(), &write).unwrap_err();

        assert!(error.to_string().contains("cannot escape fixture"));
    }

    #[test]
    fn e2e_test_repository_full_cycle_scenario_passes() {
        let result = run_e2e_scenario(Path::new("tests/e2e/scenarios/full-cycle.toml")).unwrap();

        assert!(result.passed);
        assert_eq!(result.steps_failed, 0);
        assert!(result.steps_passed >= 4);
    }

    #[test]
    fn e2e_test_repository_failure_scenario_reports_failure() {
        let result = run_e2e_scenario(Path::new("tests/e2e/scenarios/failure.toml")).unwrap();

        assert!(!result.passed);
        assert_eq!(result.steps_failed, 1);
        assert_eq!(result.steps_passed, 0);
        assert!(result.failures[0].actual.contains("exit_code expected"));
    }
}
