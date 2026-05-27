// MODULE_CONTRACT
// MODULE_ID: M-TEST-PERF-REGRESSION
// PURPOSE: Shared Criterion benchmark helpers for isolated Synapse performance fixtures.
// SCOPE: MultiModule fixture construction, Synapse binary command execution, index priming, assertion helpers, and compact Criterion configuration.
// DEPENDS: M-TEST-FIXTURE, M-TEST-PERF-REGRESSION
// LINKS:
//   -> Phase-89 (implements) - Criterion benchmark harness
//   -> M-TEST-FIXTURE (depends) - isolated benchmark project
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - benchmark command coverage

// START_MODULE_MAP
// bench_config - Returns bounded Criterion runtime settings
// multimodule_fixture - Builds an isolated MultiModule Synapse fixture
// run_syn - Runs the Synapse binary or cargo fallback inside the fixture
// assert_success - Fails a benchmark sample when a command unexpectedly fails
// prime_index - Builds the fixture index before search and graph measurements
// require_ok - Converts setup failures into explicit benchmark process failures
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.3 - Replaced manual binary candidate search with Iterator::find for clippy gate]
// END_CHANGE_SUMMARY

#![allow(dead_code)]

use criterion::Criterion;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::Duration;
use syn_cli::test::fixture::{FixtureTemplate, TestFixture};

const BENCH_SAMPLE_SIZE: usize = 10;
const BENCH_WARM_UP_MS: u64 = 50;
const BENCH_MEASUREMENT_MS: u64 = 250;

// START_public_api

// START_CONTRACT_bench_config
// PURPOSE: Return compact Criterion settings that keep Phase-89 benchmark smoke runs bounded.
// OUTPUTS: { Criterion }
// LINKS:
//   -> Phase-89 (implements) - bounded benchmark execution
//   -> NFR-003 (traces_to) - bounded benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - Criterion smoke coverage
// START_bench_config
pub fn bench_config() -> Criterion {
    Criterion::default()
        .sample_size(BENCH_SAMPLE_SIZE)
        .warm_up_time(Duration::from_millis(BENCH_WARM_UP_MS))
        .measurement_time(Duration::from_millis(BENCH_MEASUREMENT_MS))
}
// END_bench_config

// START_CONTRACT_multimodule_fixture
// PURPOSE: Build a reusable isolated MultiModule fixture for benchmark samples.
// OUTPUTS: { TestFixture }
// SIDE_EFFECTS: creates temporary project, config, and data directories
// LINKS:
//   -> M-TEST-FIXTURE (depends) - MultiModule fixture provider
//   -> NFR-003 (traces_to) - isolated benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - Criterion fixture coverage
// START_multimodule_fixture
pub fn multimodule_fixture() -> TestFixture {
    require_ok(
        "create MultiModule benchmark fixture",
        TestFixture::builder()
            .with_template(FixtureTemplate::MultiModule)
            .build(),
    )
}
// END_multimodule_fixture

// START_CONTRACT_run_syn
// PURPOSE: Run Synapse with isolated fixture state, preferring a built binary and falling back to cargo run.
// INPUTS: { fixture: &TestFixture }, { args: &[&str] }
// OUTPUTS: { Output }
// SIDE_EFFECTS: spawns a child process inside the fixture root
// LINKS:
//   -> M-TEST-PERF-REGRESSION (depends) - command benchmark execution
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - Criterion command coverage
// <LOG id="criterion_benchmark_started" level="INFO" ref="criterion-command" module="M-TEST-PERF-REGRESSION" contract="run_syn">
//   EVENT: criterion_benchmark_started
//   EXPECTATION: each benchmark command runs with isolated config and data homes
//   DECISION: prefer an already built binary and fall back to cargo run only when needed
//   RESULT: success
//   TRACEABILITY: NFR-002,NFR-003
// </LOG>
// START_run_syn
pub fn run_syn(fixture: &TestFixture, args: &[&str]) -> Output {
    let mut command = if let Some(binary) = syn_binary_path() {
        let mut command = Command::new(binary);
        command.args(args);
        command
    } else {
        let mut command = Command::new("cargo");
        command
            .arg("run")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .arg("--")
            .args(args);
        command
    };
    let output = command
        .current_dir(fixture.root())
        .env("RUST_LOG", "error")
        .envs(fixture.command_env())
        .output();
    require_ok("run syn benchmark command", output)
}
// END_run_syn

// START_CONTRACT_assert_success
// PURPOSE: Fail a benchmark sample with bounded stdout/stderr when a command exits unsuccessfully.
// INPUTS: { label: &str }, { output: &Output }
// OUTPUTS: { () }
// LINKS:
//   -> V-M-TEST-PERF-REGRESSION (verified_by) - command status evidence
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark evidence
// <LOG id="criterion_benchmark_completed" level="INFO" ref="criterion-command" module="M-TEST-PERF-REGRESSION" contract="assert_success">
//   EVENT: criterion_benchmark_completed
//   EXPECTATION: benchmark commands return successful statuses
//   DECISION: include bounded stdout and stderr tails for actionable benchmark failures
//   RESULT: success
//   TRACEABILITY: NFR-002,NFR-003
// </LOG>
// START_assert_success
pub fn assert_success(label: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        label,
        output.status.code(),
        output_tail(&output.stdout),
        output_tail(&output.stderr)
    );
}
// END_assert_success

// START_CONTRACT_prime_index
// PURPOSE: Build the fixture index before benchmarks that require indexed storage.
// INPUTS: { fixture: &TestFixture }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes Synapse index storage under the fixture data home
// LINKS:
//   -> M-INDEXER (depends) - indexed benchmark setup
//   -> NFR-003 (traces_to) - stable search benchmark setup
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - indexed benchmark coverage
// START_prime_index
pub fn prime_index(fixture: &TestFixture) {
    let output = run_syn(fixture, &["index", "--force", "--no-git"]);
    assert_success("prime index", &output);
}
// END_prime_index

// START_CONTRACT_require_ok
// PURPOSE: Convert setup errors into explicit process failures without unwrap or expect flow control.
// INPUTS: { label: &str }, { result: Result<T, E> }
// OUTPUTS: { T }
// SIDE_EFFECTS: writes stderr and exits the benchmark process on failure
// LINKS:
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - explicit failure handling
// START_require_ok
pub fn require_ok<T, E: Display>(label: &str, result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{label} failed: {error}");
            std::process::exit(1);
        }
    }
}
// END_require_ok

// END_public_api

// START_CONTRACT_syn_binary_path
// PURPOSE: Resolve a locally built Synapse binary for benchmarks when Cargo exposes one.
// OUTPUTS: { Option<PathBuf> }
// LINKS:
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark execution
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - binary resolution coverage
// START_syn_binary_path
fn syn_binary_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("SYN_BENCH_BINARY") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_syn") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        manifest.join("target").join("release").join(binary_name()),
        manifest.join("target").join("debug").join(binary_name()),
    ]
    .into_iter()
    .find(|candidate| candidate.exists())
}
// END_syn_binary_path

// START_CONTRACT_binary_name
// PURPOSE: Return the platform-specific Synapse binary filename.
// OUTPUTS: { &'static str }
// LINKS:
//   -> NFR-002 (traces_to) - cross-platform benchmark command execution
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - binary resolution coverage
// START_binary_name
fn binary_name() -> &'static str {
    if cfg!(windows) {
        "syn.exe"
    } else {
        "syn"
    }
}
// END_binary_name

// START_CONTRACT_output_tail
// PURPOSE: Return bounded UTF-8 command output for assertion messages.
// INPUTS: { bytes: &[u8] }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-003 (traces_to) - compact benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - bounded failure output
// START_output_tail
fn output_tail(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut tail = text.lines().rev().take(40).collect::<Vec<_>>();
    tail.reverse();
    let tail = tail.join("\n");
    tail.chars()
        .rev()
        .take(4000)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}
// END_output_tail
