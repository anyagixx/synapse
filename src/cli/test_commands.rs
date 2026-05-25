// MODULE_CONTRACT
// MODULE_ID: M-CLI-TEST-COMMANDS
// PURPOSE: Structured Synapse test command dispatcher with E2E/MCP/coverage/perf/contract/snapshot handlers and RTK legacy fallback.
// SCOPE: syn test e2e, mcp, snapshot, coverage, perf, contract, resilience, E2E scenario execution, MCP cargo target execution, coverage matrix rendering, perf baseline/check execution, contract differential spec execution, snapshot listing, and legacy RTK fallback dispatch.
// DEPENDS: M-CLI, M-CLI-RTK-COMMANDS, M-TEST-HARNESS, M-TEST-E2E-RUNNER, M-TEST-MCP-REGRESSION, M-TEST-COVERAGE-MATRIX, M-TEST-PERF-REGRESSION, M-TEST-CONTRACT-DIFFERENTIAL, M-TEST-SNAPSHOT
// LINKS:
//   -> Phase-76 (implements) - UPGRADE_3 test command foundation
//   -> Phase-78 (implements) - MCP regression command wiring
//   -> Phase-79 (implements) - coverage and contract command wiring
//   -> Phase-80 (implements) - perf regression command wiring
//   -> M-CLI-RTK-COMMANDS (depends) - legacy compact test adapter
//   -> M-TEST-MCP-REGRESSION (depends) - real MCP stdio regression suite
//   -> M-TEST-COVERAGE-MATRIX (depends) - coverage matrix command surface
//   -> M-TEST-PERF-REGRESSION (depends) - performance regression command surface
//   -> M-TEST-CONTRACT-DIFFERENTIAL (depends) - contract differential command surface
//   <- V-M-CLI-TEST-COMMANDS (verified_by) - command parsing and compatibility verification

// START_MODULE_MAP
// TestAction - Structured syn test action schema and external legacy command capture
// StructuredTestArgs - Shared structured test action options
// TestCmd::run - Dispatches structured actions or delegates legacy commands to RTK fallback
// run_e2e_action - Runs one or more E2E scenario TOML files
// run_mcp_action - Runs the real MCP regression cargo target
// run_coverage_action - Builds and renders the module evidence coverage matrix
// run_perf_action - Runs performance baselines or threshold checks
// run_contract_action - Runs one or more contract differential TOML specs
// run_snapshot_action - Lists or acknowledges snapshot check/update mode
// render_structured_action - Emits compact text or machine-readable JSON planning output
// normalize_legacy_command - Converts clap external subcommand args into executable command argv
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.4.0 - Wired syn test perf baseline and check handlers]
// END_CHANGE_SUMMARY

use super::TestCmd;
use crate::config::Config;
use crate::test::contract_test::{
    render_contract_test_report, run_contract_test_file, ContractTestOptions, ContractTestReport,
};
use crate::test::coverage::{build_coverage_matrix, render_coverage_json, render_coverage_table};
use crate::test::e2e::{run_e2e_scenario, E2EResult};
use crate::test::perf::{render_perf_report, run_perf_suite, PerfMode, PerfTestOptions};
use serde::Serialize;
use serde_json::json;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

const MCP_OUTPUT_TAIL_LINES: usize = 40;
const MCP_OUTPUT_CHAR_LIMIT: usize = 6000;

// START_public_api

// START_TestAction
#[derive(clap::Subcommand, Debug, Clone)]
pub enum TestAction {
    #[command(name = "e2e", about = "Plan Synapse end-to-end harness runs")]
    E2e(StructuredTestArgs),
    #[command(name = "mcp", about = "Plan MCP tool regression runs")]
    Mcp(StructuredTestArgs),
    #[command(name = "snapshot", about = "Plan snapshot regression checks")]
    Snapshot(StructuredTestArgs),
    #[command(name = "coverage", about = "Plan test coverage matrix checks")]
    Coverage(StructuredTestArgs),
    #[command(name = "perf", about = "Plan performance regression checks")]
    Perf(StructuredTestArgs),
    #[command(name = "contract", about = "Plan contract differential checks")]
    Contract(StructuredTestArgs),
    #[command(name = "resilience", about = "Plan resilience and recovery checks")]
    Resilience(StructuredTestArgs),
    #[command(external_subcommand)]
    Legacy(Vec<OsString>),
}
// END_TestAction

// START_StructuredTestArgs
#[derive(clap::Args, Debug, Clone)]
pub struct StructuredTestArgs {
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
    #[arg(long)]
    pub fixture: Option<String>,
    #[arg(long = "scenario")]
    pub scenarios: Vec<String>,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub list: bool,
    #[arg(long)]
    pub update: bool,
    #[arg(long)]
    pub filter: Option<String>,
    #[arg(long)]
    pub baseline: bool,
    #[arg(long)]
    pub check: bool,
    #[arg(long, default_value_t = 20.0, value_name = "PCT")]
    pub threshold: f64,
    #[arg(long, default_value = "tests/baselines/perf.json")]
    pub baseline_path: PathBuf,
    #[arg(long, default_value = "tests/snapshots")]
    pub snapshot_dir: PathBuf,
    #[arg(value_name = "SPEC")]
    pub specs: Vec<PathBuf>,
    #[arg(last = true)]
    pub passthrough: Vec<String>,
}
// END_StructuredTestArgs

impl TestCmd {
    // START_CONTRACT_TestCmd::run
    // PURPOSE: Dispatch syn test actions to real handlers, structured planning output, or RTK legacy compact test execution
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes stdout, may execute a child command through RTK fallback, records RTK savings for legacy commands
    // LINKS:
    //   -> M-CLI-TEST-COMMANDS (depends) - structured test command schema
    //   -> M-CLI-RTK-COMMANDS (depends) - legacy test command fallback
    //   -> M-TEST-E2E-RUNNER (depends) - E2E scenario execution
    //   -> M-TEST-MCP-REGRESSION (depends) - MCP regression command surface
    //   -> M-TEST-COVERAGE-MATRIX (depends) - coverage matrix command surface
    //   -> M-TEST-PERF-REGRESSION (depends) - performance regression command surface
    //   -> M-TEST-CONTRACT-DIFFERENTIAL (depends) - contract differential command surface
    //   -> M-TEST-SNAPSHOT (depends) - snapshot command surface
    //   -> Phase-77 (implements) - E2E and snapshot handler wiring
    //   -> Phase-78 (implements) - MCP regression handler wiring
    //   -> Phase-79 (implements) - coverage and contract handler wiring
    //   -> Phase-80 (implements) - perf regression handler wiring
    //   -> NFR-002 (traces_to) - release verification commands must route deterministically
    //   <- V-M-CLI-TEST-COMMANDS (verified_by) - parse and dispatch tests
    // <LOG id="test_command_dispatch" level="INFO" ref="test-command-dispatch" module="M-CLI-TEST-COMMANDS" contract="TestCmd::run">
    //   EVENT: test_command_dispatch
    //   EXPECTATION: known syn test actions stay structured while unknown actions use legacy RTK fallback
    //   DECISION: dispatch is selected from the clap action variant
    //   RESULT: success
    //   TRACEABILITY: NFR-002
    // </LOG>
    // START_test_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let Some(action) = &self.action else {
            anyhow::bail!(
                "Usage: syn test <e2e|mcp|snapshot|coverage|perf|contract|resilience> [options]\n       syn test -- <test-command> [args...]"
            );
        };

        if let TestAction::Legacy(args) = action {
            let command = normalize_legacy_command(args);
            return super::rtk_core_adapters::run_legacy_test_command(&command, config).await;
        }

        match action {
            TestAction::E2e(args) => run_e2e_action(args),
            TestAction::Mcp(args) => run_mcp_action(args),
            TestAction::Snapshot(args) => run_snapshot_action(args),
            TestAction::Coverage(args) => run_coverage_action(args),
            TestAction::Perf(args) => run_perf_action(args),
            TestAction::Contract(args) => run_contract_action(args),
            _ => {
                println!("{}", render_structured_action(action)?);
                Ok(())
            }
        }
    }
    // END_test_cmd_run
}

// END_public_api

// START_TestActionKind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestActionKind {
    E2e,
    Mcp,
    Snapshot,
    Coverage,
    Perf,
    Contract,
    Resilience,
}
// END_TestActionKind

impl TestActionKind {
    // START_CONTRACT_TestActionKind::cli_name
    // PURPOSE: Return the stable CLI token for a structured test action
    // OUTPUTS: { &'static str }
    // LINKS:
    //   -> M-CLI-TEST-COMMANDS (depends) - structured action metadata
    //   -> NFR-002 (traces_to) - stable action names support release verification
    // START_test_action_kind_cli_name
    fn cli_name(self) -> &'static str {
        match self {
            Self::E2e => "e2e",
            Self::Mcp => "mcp",
            Self::Snapshot => "snapshot",
            Self::Coverage => "coverage",
            Self::Perf => "perf",
            Self::Contract => "contract",
            Self::Resilience => "resilience",
        }
    }
    // END_test_action_kind_cli_name

    // START_CONTRACT_TestActionKind::module_id
    // PURPOSE: Return the planned test module that owns a structured action runner
    // OUTPUTS: { &'static str }
    // LINKS:
    //   -> M-TEST-HARNESS (depends) - structured test module facade
    //   -> NFR-002 (traces_to) - action ownership must be explicit for verification
    // START_test_action_kind_module_id
    fn module_id(self) -> &'static str {
        match self {
            Self::E2e => "M-TEST-E2E-RUNNER",
            Self::Mcp => "M-TEST-MCP-REGRESSION",
            Self::Snapshot => "M-TEST-SNAPSHOT",
            Self::Coverage => "M-TEST-COVERAGE-MATRIX",
            Self::Perf => "M-TEST-PERF-REGRESSION",
            Self::Contract => "M-TEST-CONTRACT-DIFFERENTIAL",
            Self::Resilience => "M-TEST-RESILIENCE-CHAOS",
        }
    }
    // END_test_action_kind_module_id

    // START_CONTRACT_TestActionKind::phase_id
    // PURPOSE: Return the UPGRADE_3 phase that will complete a structured action runner
    // OUTPUTS: { &'static str }
    // LINKS:
    //   -> Phase-76 (implements) - schema foundation
    //   -> NFR-002 (traces_to) - phased runners must be visible before execution
    // START_test_action_kind_phase_id
    fn phase_id(self) -> &'static str {
        match self {
            Self::E2e | Self::Snapshot => "Phase-77",
            Self::Mcp => "Phase-78",
            Self::Coverage | Self::Contract => "Phase-79",
            Self::Perf => "Phase-80",
            Self::Resilience => "Phase-82",
        }
    }
    // END_test_action_kind_phase_id
}

impl TestAction {
    // START_CONTRACT_TestAction::kind
    // PURPOSE: Return the metadata kind for a structured test action
    // OUTPUTS: { TestActionKind }
    // LINKS:
    //   -> M-CLI-TEST-COMMANDS (depends) - dispatch metadata
    //   -> NFR-002 (traces_to) - structured dispatch must be deterministic
    // START_test_action_kind
    fn kind(&self) -> TestActionKind {
        match self {
            Self::E2e(_) => TestActionKind::E2e,
            Self::Mcp(_) => TestActionKind::Mcp,
            Self::Snapshot(_) => TestActionKind::Snapshot,
            Self::Coverage(_) => TestActionKind::Coverage,
            Self::Perf(_) => TestActionKind::Perf,
            Self::Contract(_) => TestActionKind::Contract,
            Self::Resilience(_) => TestActionKind::Resilience,
            Self::Legacy(_) => unreachable!("legacy actions do not have structured metadata"),
        }
    }
    // END_test_action_kind

    // START_CONTRACT_TestAction::structured_args
    // PURPOSE: Return shared options for a structured test action
    // OUTPUTS: { Option<&StructuredTestArgs> }
    // LINKS:
    //   -> M-CLI-TEST-COMMANDS (depends) - structured action options
    //   -> NFR-002 (traces_to) - structured options must be inspectable by tests
    // START_test_action_structured_args
    fn structured_args(&self) -> Option<&StructuredTestArgs> {
        match self {
            Self::E2e(args)
            | Self::Mcp(args)
            | Self::Snapshot(args)
            | Self::Coverage(args)
            | Self::Perf(args)
            | Self::Contract(args)
            | Self::Resilience(args) => Some(args),
            Self::Legacy(_) => None,
        }
    }
    // END_test_action_structured_args
}

// START_E2ECliReport
#[derive(Debug, Serialize)]
struct E2ECliReport {
    #[serde(rename = "type")]
    kind: &'static str,
    passed: bool,
    scenarios: Vec<E2EResult>,
}
// END_E2ECliReport

// START_McpCliReport
#[derive(Debug, Serialize)]
struct McpCliReport {
    #[serde(rename = "type")]
    kind: &'static str,
    passed: bool,
    project: String,
    command: Vec<String>,
    status_code: Option<i32>,
    stdout_tail: String,
    stderr_tail: String,
}
// END_McpCliReport

// START_ContractCliReport
#[derive(Debug, Serialize)]
struct ContractCliReport {
    #[serde(rename = "type")]
    kind: &'static str,
    passed: bool,
    reports: Vec<ContractTestReport>,
}
// END_ContractCliReport

// START_SnapshotCliReport
#[derive(Debug, Serialize)]
struct SnapshotCliReport {
    #[serde(rename = "type")]
    kind: &'static str,
    mode: &'static str,
    snapshot_dir: String,
    total: usize,
    snapshots: Vec<SnapshotFileReport>,
}
// END_SnapshotCliReport

// START_SnapshotFileReport
#[derive(Debug, Serialize)]
struct SnapshotFileReport {
    name: String,
    path: String,
}
// END_SnapshotFileReport

// START_CONTRACT_run_e2e_action
// PURPOSE: Run structured syn test e2e scenarios when --scenario is provided, otherwise render planning output
// INPUTS: { args: &StructuredTestArgs }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: executes E2E scenario commands inside isolated fixtures, writes stdout
// LINKS:
//   -> M-TEST-E2E-RUNNER (depends) - scenario execution
//   -> M-CLI-TEST-COMMANDS (depends) - CLI handler wiring
//   -> Phase-77 (implements) - syn test e2e handler
//   -> NFR-002 (traces_to) - E2E scenarios must be invokable from CLI
// START_run_e2e_action
fn run_e2e_action(args: &StructuredTestArgs) -> anyhow::Result<()> {
    if args.scenarios.is_empty() {
        println!(
            "{}",
            render_structured_action(&TestAction::E2e(args.clone()))?
        );
        return Ok(());
    }

    let mut scenarios = Vec::new();
    for scenario in &args.scenarios {
        let path = resolve_project_path(&args.project, Path::new(scenario));
        scenarios.push(run_e2e_scenario(&path)?);
    }
    let report = E2ECliReport {
        kind: "synapse.test.e2e",
        passed: scenarios.iter().all(|scenario| scenario.passed),
        scenarios,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render_e2e_report(&report));
    }
    if !report.passed {
        anyhow::bail!("one or more E2E scenarios failed");
    }
    Ok(())
}
// END_run_e2e_action

// START_CONTRACT_run_mcp_action
// PURPOSE: Run the structured syn test mcp regression target through cargo
// INPUTS: { args: &StructuredTestArgs }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: executes cargo test --test e2e_mcp in the requested project, writes bounded stdout
// LINKS:
//   -> M-TEST-MCP-REGRESSION (depends) - real stdio MCP regression suite
//   -> M-CLI-TEST-COMMANDS (depends) - CLI handler wiring
//   -> Phase-78 (implements) - syn test mcp handler
//   -> NFR-002 (traces_to) - MCP regression suite must be invokable from CLI
// START_run_mcp_action
fn run_mcp_action(args: &StructuredTestArgs) -> anyhow::Result<()> {
    let project = args.project.clone();
    let cargo_args = build_mcp_cargo_args(args);
    let output = ProcessCommand::new("cargo")
        .args(&cargo_args)
        .current_dir(&project)
        .output()?;
    let report = McpCliReport {
        kind: "synapse.test.mcp",
        passed: output.status.success(),
        project: project.display().to_string(),
        command: std::iter::once("cargo".to_string())
            .chain(cargo_args.iter().cloned())
            .collect(),
        status_code: output.status.code(),
        stdout_tail: compact_command_output(&output.stdout),
        stderr_tail: compact_command_output(&output.stderr),
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render_mcp_report(&report));
    }
    if !report.passed {
        anyhow::bail!("MCP regression suite failed");
    }
    Ok(())
}
// END_run_mcp_action

// START_CONTRACT_run_coverage_action
// PURPOSE: Build and render the module evidence coverage matrix for a project
// INPUTS: { args: &StructuredTestArgs }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: reads MyGRACE indexes and evidence files, writes stdout
// LINKS:
//   -> M-TEST-COVERAGE-MATRIX (depends) - coverage matrix builder
//   -> M-CLI-TEST-COMMANDS (depends) - CLI handler wiring
//   -> Phase-79 (implements) - syn test coverage handler
//   -> NFR-003 (traces_to) - coverage output is compact and JSON-capable
// START_run_coverage_action
fn run_coverage_action(args: &StructuredTestArgs) -> anyhow::Result<()> {
    let matrix = build_coverage_matrix(&args.project)?;
    if args.json {
        println!("{}", render_coverage_json(&matrix)?);
    } else {
        println!("{}", render_coverage_table(&matrix));
    }
    Ok(())
}
// END_run_coverage_action

// START_CONTRACT_run_perf_action
// PURPOSE: Run structured syn test perf baseline save or threshold check modes
// INPUTS: { args: &StructuredTestArgs }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: executes Synapse benchmark commands in an isolated fixture, reads or writes baseline JSON, writes stdout
// LINKS:
//   -> M-TEST-PERF-REGRESSION (depends) - performance regression runner
//   -> M-CLI-TEST-COMMANDS (depends) - CLI handler wiring
//   -> Phase-80 (implements) - syn test perf handler
//   -> NFR-002 (traces_to) - perf gates must be invokable from CLI
//   -> NFR-003 (traces_to) - perf reports are bounded and JSON-capable
// START_run_perf_action
fn run_perf_action(args: &StructuredTestArgs) -> anyhow::Result<()> {
    if args.baseline && args.check {
        anyhow::bail!("syn test perf accepts only one of --baseline or --check");
    }
    if !args.baseline && !args.check {
        println!(
            "{}",
            render_structured_action(&TestAction::Perf(args.clone()))?
        );
        return Ok(());
    }

    let baseline_path = resolve_project_path(&args.project, &args.baseline_path);
    let options = PerfTestOptions {
        mode: if args.baseline {
            PerfMode::Baseline
        } else {
            PerfMode::Check
        },
        baseline_path,
        threshold_pct: args.threshold,
        filter: args.filter.clone(),
        ..PerfTestOptions::default()
    };
    let report = run_perf_suite(&options)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render_perf_report(&report));
    }
    if report
        .comparison
        .as_ref()
        .is_some_and(|comparison| !comparison.passed)
    {
        anyhow::bail!("performance regression check failed");
    }
    Ok(())
}
// END_run_perf_action

// START_CONTRACT_run_contract_action
// PURPOSE: Run one or more TOML contract differential specs
// INPUTS: { args: &StructuredTestArgs }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: runs cascade impact previews and writes stdout
// LINKS:
//   -> M-TEST-CONTRACT-DIFFERENTIAL (depends) - contract differential runner
//   -> M-CLI-TEST-COMMANDS (depends) - CLI handler wiring
//   -> Phase-79 (implements) - syn test contract handler
//   -> NFR-002 (traces_to) - contract specs are deterministic release gates
//   -> NFR-003 (traces_to) - contract reports are bounded and JSON-capable
// START_run_contract_action
fn run_contract_action(args: &StructuredTestArgs) -> anyhow::Result<()> {
    if args.specs.is_empty() {
        println!(
            "{}",
            render_structured_action(&TestAction::Contract(args.clone()))?
        );
        return Ok(());
    }

    let options = ContractTestOptions {
        project_root: args.project.clone(),
        trigger_filter: args.filter.clone(),
        cleanup_previews: true,
    };
    let mut reports = Vec::new();
    for spec in &args.specs {
        let path = resolve_project_path(&args.project, spec);
        reports.push(run_contract_test_file(&path, &options)?);
    }
    let cli_report = ContractCliReport {
        kind: "synapse.test.contract",
        passed: reports.iter().all(|report| report.passed),
        reports,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&cli_report)?);
    } else {
        println!("{}", render_contract_cli_report(&cli_report));
    }
    if !cli_report.passed {
        anyhow::bail!("one or more contract differential specs failed");
    }
    Ok(())
}
// END_run_contract_action

// START_CONTRACT_run_snapshot_action
// PURPOSE: Run structured syn test snapshot listing/check/update command surface
// INPUTS: { args: &StructuredTestArgs }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: reads snapshot directory and writes stdout
// LINKS:
//   -> M-TEST-SNAPSHOT (depends) - snapshot artifact convention
//   -> M-CLI-TEST-COMMANDS (depends) - CLI handler wiring
//   -> Phase-77 (implements) - syn test snapshot handler
//   -> NFR-002 (traces_to) - snapshot command must be invokable from CLI
// START_run_snapshot_action
fn run_snapshot_action(args: &StructuredTestArgs) -> anyhow::Result<()> {
    let snapshot_dir = resolve_project_path(&args.project, &args.snapshot_dir);
    let snapshots = collect_snapshot_files(&snapshot_dir, args.filter.as_deref())?;
    let report = SnapshotCliReport {
        kind: "synapse.test.snapshot",
        mode: if args.update { "update" } else { "check" },
        snapshot_dir: snapshot_dir.display().to_string(),
        total: snapshots.len(),
        snapshots,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render_snapshot_report(&report));
    }
    Ok(())
}
// END_run_snapshot_action

// START_CONTRACT_render_e2e_report
// PURPOSE: Render compact text output for E2E CLI scenario results
// INPUTS: { report: &E2ECliReport }
// OUTPUTS: { String }
// LINKS:
//   -> M-TEST-E2E-RUNNER (depends) - result rendering
//   -> NFR-003 (traces_to) - bounded E2E CLI output
// START_render_e2e_report
fn render_e2e_report(report: &E2ECliReport) -> String {
    let total = report.scenarios.len();
    let passed = report
        .scenarios
        .iter()
        .filter(|scenario| scenario.passed)
        .count();
    let mut lines = vec![format!("E2E scenarios: {passed}/{total} passed")];
    for scenario in &report.scenarios {
        lines.push(format!(
            "- {}: {} (steps {}/{}, failed {})",
            scenario.scenario,
            if scenario.passed { "passed" } else { "failed" },
            scenario.steps_passed,
            scenario.steps_total,
            scenario.steps_failed
        ));
        for failure in &scenario.failures {
            lines.push(format!(
                "  step {} {}: {}",
                failure.step_index + 1,
                failure.command,
                failure.actual
            ));
        }
    }
    lines.join("\n")
}
// END_render_e2e_report

// START_CONTRACT_build_mcp_cargo_args
// PURPOSE: Build the cargo argv for syn test mcp from shared structured options
// INPUTS: { args: &StructuredTestArgs }
// OUTPUTS: { Vec<String> }
// LINKS:
//   -> M-TEST-MCP-REGRESSION (depends) - e2e_mcp cargo target
//   -> NFR-002 (traces_to) - command construction must be deterministic and testable
// START_build_mcp_cargo_args
fn build_mcp_cargo_args(args: &StructuredTestArgs) -> Vec<String> {
    let mut cargo_args = vec![
        "test".to_string(),
        "--quiet".to_string(),
        "--test".to_string(),
        "e2e_mcp".to_string(),
    ];
    if let Some(filter) = args.filter.as_ref().filter(|value| !value.is_empty()) {
        cargo_args.push(filter.clone());
    }

    let mut harness_args = Vec::new();
    if args.list {
        harness_args.push("--list".to_string());
    }
    harness_args.extend(args.passthrough.iter().cloned());
    if !harness_args.is_empty() {
        cargo_args.push("--".to_string());
        cargo_args.extend(harness_args);
    }
    cargo_args
}
// END_build_mcp_cargo_args

// START_CONTRACT_render_mcp_report
// PURPOSE: Render compact text output for the MCP regression CLI runner
// INPUTS: { report: &McpCliReport }
// OUTPUTS: { String }
// LINKS:
//   -> M-TEST-MCP-REGRESSION (depends) - regression result summary
//   -> NFR-003 (traces_to) - bounded MCP CLI output
// START_render_mcp_report
fn render_mcp_report(report: &McpCliReport) -> String {
    let mut lines = vec![
        format!(
            "MCP regression: {}",
            if report.passed { "passed" } else { "failed" }
        ),
        format!("project: {}", report.project),
        format!("command: {}", report.command.join(" ")),
    ];
    if !report.stdout_tail.is_empty() {
        lines.push("stdout:".to_string());
        lines.push(report.stdout_tail.clone());
    }
    if !report.stderr_tail.is_empty() {
        lines.push("stderr:".to_string());
        lines.push(report.stderr_tail.clone());
    }
    lines.join("\n")
}
// END_render_mcp_report

// START_CONTRACT_render_contract_cli_report
// PURPOSE: Render compact text output for one or more contract differential reports
// INPUTS: { report: &ContractCliReport }
// OUTPUTS: { String }
// LINKS:
//   -> M-TEST-CONTRACT-DIFFERENTIAL (depends) - contract report rendering
//   -> NFR-003 (traces_to) - bounded multi-spec output
// START_render_contract_cli_report
fn render_contract_cli_report(report: &ContractCliReport) -> String {
    let mut lines = vec![format!(
        "Contract differential suites: {}/{} passed",
        report.reports.iter().filter(|suite| suite.passed).count(),
        report.reports.len()
    )];
    for suite in &report.reports {
        lines.push(render_contract_test_report(suite));
    }
    lines.join("\n\n")
}
// END_render_contract_cli_report

// START_CONTRACT_render_snapshot_report
// PURPOSE: Render compact text output for snapshot CLI mode
// INPUTS: { report: &SnapshotCliReport }
// OUTPUTS: { String }
// LINKS:
//   -> M-TEST-SNAPSHOT (depends) - snapshot artifact summary
//   -> NFR-003 (traces_to) - bounded snapshot CLI output
// START_render_snapshot_report
fn render_snapshot_report(report: &SnapshotCliReport) -> String {
    let mut lines = vec![
        format!("Snapshot testing: {} mode", report.mode),
        format!("snapshot_dir: {}", report.snapshot_dir),
        format!("snapshots: {}", report.total),
    ];
    for snapshot in &report.snapshots {
        lines.push(format!("- {} ({})", snapshot.name, snapshot.path));
    }
    lines.join("\n")
}
// END_render_snapshot_report

// START_CONTRACT_collect_snapshot_files
// PURPOSE: Collect .snap files from a snapshot directory with optional name filtering
// INPUTS: { snapshot_dir: &Path }, { filter: Option<&str> }
// OUTPUTS: { anyhow::Result<Vec<SnapshotFileReport>> }
// LINKS:
//   -> M-TEST-SNAPSHOT (depends) - snapshot artifact discovery
//   -> NFR-002 (traces_to) - deterministic sorted snapshot inventory
// START_collect_snapshot_files
fn collect_snapshot_files(
    snapshot_dir: &Path,
    filter: Option<&str>,
) -> anyhow::Result<Vec<SnapshotFileReport>> {
    if !snapshot_dir.exists() {
        return Ok(Vec::new());
    }
    let mut snapshots = Vec::new();
    for entry in std::fs::read_dir(snapshot_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("snap") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if filter.is_some_and(|needle| !name.contains(needle)) {
            continue;
        }
        snapshots.push(SnapshotFileReport {
            name: name.to_string(),
            path: path.display().to_string(),
        });
    }
    snapshots.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(snapshots)
}
// END_collect_snapshot_files

// START_CONTRACT_compact_command_output
// PURPOSE: Return a bounded UTF-8 tail from command output bytes
// INPUTS: { bytes: &[u8] }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-003 (traces_to) - CLI test runner output must stay token-bounded
// START_compact_command_output
fn compact_command_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text
        .lines()
        .rev()
        .take(MCP_OUTPUT_TAIL_LINES)
        .collect::<Vec<_>>();
    lines.reverse();
    let tail = lines.join("\n");
    crate::utils::truncate_chars(&tail, MCP_OUTPUT_CHAR_LIMIT)
}
// END_compact_command_output

// START_CONTRACT_resolve_project_path
// PURPOSE: Resolve a possibly relative CLI path against --project
// INPUTS: { project: &Path }, { path: &Path }
// OUTPUTS: { PathBuf }
// LINKS:
//   -> M-CLI-TEST-COMMANDS (depends) - stable CLI path resolution
//   -> NFR-002 (traces_to) - scenario and snapshot paths resolve predictably
// START_resolve_project_path
fn resolve_project_path(project: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project.join(path)
    }
}
// END_resolve_project_path

// START_CONTRACT_render_structured_action
// PURPOSE: Render structured syn test planning output as compact text or JSON
// INPUTS: { action: &TestAction }
// OUTPUTS: { anyhow::Result<String> }
// LINKS:
//   -> M-CLI-TEST-COMMANDS (depends) - structured action output
//   -> M-TEST-HARNESS (depends) - future runner ownership
//   -> NFR-002 (traces_to) - structured test output must be verifiable
// START_render_structured_action
fn render_structured_action(action: &TestAction) -> anyhow::Result<String> {
    let kind = action.kind();
    let args = action
        .structured_args()
        .ok_or_else(|| anyhow::anyhow!("legacy test commands do not render structured output"))?;
    let project = args.project.display().to_string();
    let status = if args.list { "listed" } else { "planned" };

    if args.json {
        return Ok(serde_json::to_string_pretty(&json!({
            "type": "synapse.test.action",
            "status": status,
            "matrix": [{
                "action": kind.cli_name(),
                "module": kind.module_id(),
                "phase": kind.phase_id(),
                "project": project,
                "fixture": args.fixture,
                "scenarios": args.scenarios,
                "update": args.update,
                "filter": args.filter,
                "baseline": args.baseline,
                "check": args.check,
                "threshold": args.threshold,
                "baseline_path": args.baseline_path.display().to_string(),
                "snapshot_dir": args.snapshot_dir.display().to_string(),
                "specs": args.specs.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
                "passthrough": args.passthrough,
                "runner": "pending"
            }]
        }))?);
    }

    let mut lines = vec![
        format!("syn test {}", kind.cli_name()),
        format!("status: {status}"),
        format!("module: {}", kind.module_id()),
        format!("phase: {}", kind.phase_id()),
        format!("project: {project}"),
    ];
    if let Some(fixture) = &args.fixture {
        lines.push(format!("fixture: {fixture}"));
    }
    if !args.scenarios.is_empty() {
        lines.push(format!("scenarios: {}", args.scenarios.join(",")));
    }
    if args.update {
        lines.push("update: true".to_string());
    }
    if let Some(filter) = &args.filter {
        lines.push(format!("filter: {filter}"));
    }
    if args.baseline {
        lines.push("baseline: true".to_string());
    }
    if args.check {
        lines.push("check: true".to_string());
    }
    lines.push(format!("threshold: {:.1}", args.threshold));
    lines.push(format!("baseline_path: {}", args.baseline_path.display()));
    if !args.specs.is_empty() {
        let specs = args
            .specs
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(",");
        lines.push(format!("specs: {specs}"));
    }
    lines.push(format!("snapshot_dir: {}", args.snapshot_dir.display()));
    if !args.passthrough.is_empty() {
        lines.push(format!("passthrough: {}", args.passthrough.join(" ")));
    }
    Ok(lines.join("\n"))
}
// END_render_structured_action

// START_CONTRACT_normalize_legacy_command
// PURPOSE: Convert clap external subcommand values into a legacy executable argv vector
// INPUTS: { args: &[OsString] }
// OUTPUTS: { Vec<String> }
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - legacy RTK fallback invocation
//   -> NFR-003 (traces_to) - legacy test commands preserve token-saving compact output
// START_normalize_legacy_command
fn normalize_legacy_command(args: &[OsString]) -> Vec<String> {
    let mut command: Vec<String> = args
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    while command.first().is_some_and(|arg| arg == "--") {
        command.remove(0);
    }
    command
}
// END_normalize_legacy_command

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Command, SynCli};
    use clap::Parser;
    use tempfile::TempDir;

    fn parse_test_action(args: &[&str]) -> TestAction {
        match SynCli::try_parse_from(args)
            .expect("test command should parse")
            .command
        {
            Command::Test(cmd) => cmd.action.expect("test action should be present"),
            _ => panic!("expected test command"),
        }
    }

    #[test]
    fn cli_test_parses_all_structured_actions() {
        let cases = [
            ("e2e", TestActionKind::E2e),
            ("mcp", TestActionKind::Mcp),
            ("snapshot", TestActionKind::Snapshot),
            ("coverage", TestActionKind::Coverage),
            ("perf", TestActionKind::Perf),
            ("contract", TestActionKind::Contract),
            ("resilience", TestActionKind::Resilience),
        ];
        for (name, expected) in cases {
            let action = parse_test_action(&["syn", "test", name]);
            assert_eq!(action.kind(), expected);
        }
    }

    #[test]
    fn cli_test_legacy_external_command_is_preserved() {
        let action = parse_test_action(&["syn", "test", "cargo", "test", "--lib"]);
        match action {
            TestAction::Legacy(args) => {
                assert_eq!(normalize_legacy_command(&args), ["cargo", "test", "--lib"]);
            }
            _ => panic!("expected legacy external command"),
        }
    }

    #[test]
    fn cli_test_legacy_double_dash_command_is_preserved() {
        let action = parse_test_action(&["syn", "test", "--", "coverage", "--json"]);
        match action {
            TestAction::Legacy(args) => {
                assert_eq!(normalize_legacy_command(&args), ["coverage", "--json"]);
            }
            _ => panic!("expected legacy double-dash command"),
        }
    }

    #[test]
    fn cli_test_e2e_scenario_option_is_preserved() {
        let action =
            parse_test_action(&["syn", "test", "e2e", "--scenario", "tests/e2e/full.toml"]);
        match action {
            TestAction::E2e(args) => {
                assert_eq!(args.scenarios, ["tests/e2e/full.toml"]);
            }
            _ => panic!("expected e2e action"),
        }
    }

    #[test]
    fn cli_test_snapshot_update_and_filter_are_parsed() {
        let action =
            parse_test_action(&["syn", "test", "snapshot", "--update", "--filter", "verify"]);
        match action {
            TestAction::Snapshot(args) => {
                assert!(args.update);
                assert_eq!(args.filter.as_deref(), Some("verify"));
            }
            _ => panic!("expected snapshot action"),
        }
    }

    #[test]
    fn cli_test_coverage_json_renders_machine_readable_matrix() {
        let action = parse_test_action(&["syn", "test", "coverage", "--json"]);
        let rendered = render_structured_action(&action).expect("coverage JSON should render");
        let value: serde_json::Value =
            serde_json::from_str(&rendered).expect("coverage output should be JSON");
        assert_eq!(value["type"], "synapse.test.action");
        assert_eq!(value["matrix"][0]["action"], "coverage");
        assert_eq!(value["matrix"][0]["module"], "M-TEST-COVERAGE-MATRIX");
    }

    #[test]
    fn cli_test_contract_spec_path_is_parsed() {
        let action = parse_test_action(&[
            "syn",
            "test",
            "contract",
            "tests/e2e/contract-tests/module-change.toml",
        ]);
        let TestAction::Contract(args) = action else {
            panic!("expected contract action");
        };

        assert_eq!(
            args.specs,
            [PathBuf::from("tests/e2e/contract-tests/module-change.toml")]
        );
    }

    #[test]
    fn cli_test_contract_action_points_to_phase_79() {
        let action = parse_test_action(&["syn", "test", "contract", "--json"]);
        let rendered = render_structured_action(&action).expect("contract JSON should render");
        let value: serde_json::Value =
            serde_json::from_str(&rendered).expect("contract output should be JSON");

        assert_eq!(value["matrix"][0]["action"], "contract");
        assert_eq!(value["matrix"][0]["module"], "M-TEST-CONTRACT-DIFFERENTIAL");
        assert_eq!(value["matrix"][0]["phase"], "Phase-79");
    }

    #[test]
    fn cli_test_perf_baseline_options_are_parsed() {
        let action = parse_test_action(&[
            "syn",
            "test",
            "perf",
            "--baseline",
            "--threshold",
            "17.5",
            "--baseline-path",
            "tests/baselines/perf.json",
        ]);
        let TestAction::Perf(args) = action else {
            panic!("expected perf action");
        };

        assert!(args.baseline);
        assert!(!args.check);
        assert_eq!(args.threshold, 17.5);
        assert_eq!(
            args.baseline_path,
            PathBuf::from("tests/baselines/perf.json")
        );
    }

    #[test]
    fn cli_test_perf_action_points_to_phase_80() {
        let action = parse_test_action(&["syn", "test", "perf", "--json", "--check"]);
        let rendered = render_structured_action(&action).expect("perf JSON should render");
        let value: serde_json::Value =
            serde_json::from_str(&rendered).expect("perf output should be JSON");

        assert_eq!(value["matrix"][0]["action"], "perf");
        assert_eq!(value["matrix"][0]["module"], "M-TEST-PERF-REGRESSION");
        assert_eq!(value["matrix"][0]["phase"], "Phase-80");
        assert_eq!(value["matrix"][0]["check"], true);
        assert_eq!(
            value["matrix"][0]["baseline_path"],
            "tests/baselines/perf.json"
        );
    }

    #[test]
    fn cli_test_mcp_action_points_to_phase_78() {
        let action = parse_test_action(&["syn", "test", "mcp", "--json"]);
        let rendered = render_structured_action(&action).expect("mcp JSON should render");
        let value: serde_json::Value =
            serde_json::from_str(&rendered).expect("mcp output should be JSON");

        assert_eq!(value["matrix"][0]["action"], "mcp");
        assert_eq!(value["matrix"][0]["module"], "M-TEST-MCP-REGRESSION");
        assert_eq!(value["matrix"][0]["phase"], "Phase-78");
    }

    #[test]
    fn cli_test_mcp_cargo_args_include_filter_list_and_passthrough() {
        let action = parse_test_action(&[
            "syn",
            "test",
            "mcp",
            "--filter",
            "malformed",
            "--list",
            "--",
            "--nocapture",
        ]);
        let TestAction::Mcp(args) = action else {
            panic!("expected mcp action");
        };

        assert_eq!(
            build_mcp_cargo_args(&args),
            [
                "test",
                "--quiet",
                "--test",
                "e2e_mcp",
                "malformed",
                "--",
                "--list",
                "--nocapture"
            ]
        );
    }

    #[test]
    fn cli_test_mcp_report_renders_bounded_summary() {
        let report = McpCliReport {
            kind: "synapse.test.mcp",
            passed: true,
            project: ".".into(),
            command: vec![
                "cargo".into(),
                "test".into(),
                "--test".into(),
                "e2e_mcp".into(),
            ],
            status_code: Some(0),
            stdout_tail: "7 passed".into(),
            stderr_tail: String::new(),
        };

        let rendered = render_mcp_report(&report);

        assert!(rendered.contains("MCP regression: passed"));
        assert!(rendered.contains("cargo test --test e2e_mcp"));
        assert!(rendered.contains("7 passed"));
    }

    #[test]
    fn cli_test_snapshot_report_collects_sorted_snap_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("b.snap"), "b").unwrap();
        std::fs::write(dir.path().join("a.snap"), "a").unwrap();
        std::fs::write(dir.path().join("ignore.txt"), "x").unwrap();

        let files = collect_snapshot_files(dir.path(), None).unwrap();

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "a");
        assert_eq!(files[1].name, "b");
    }
}
