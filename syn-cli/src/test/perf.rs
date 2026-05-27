// MODULE_CONTRACT
// MODULE_ID: M-TEST-PERF-REGRESSION
// PURPOSE: Baseline-based performance regression checker for Synapse core workflows.
// SCOPE: Benchmark execution, statistics, warmup, Synapse binary invocation, baseline persistence, threshold comparison, and platform metadata.
// DEPENDS: M-INDEXER, M-GRAPHRAG, M-GRACE-VERIFY, M-TEST-FIXTURE
// LINKS:
//   -> Phase-80 (implements) - performance regression gate
//   -> M-TEST-FIXTURE (depends) - isolated MultiModule benchmark project
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - baseline comparison verification

// START_MODULE_MAP
// PerfMode - Baseline, check, or report-only run mode
// PerfTestOptions - Runtime options for the perf regression suite
// PerfRunReport - Full benchmark report with optional comparison
// BenchmarkResult - Current benchmark samples and statistics
// PerfBaseline - Persisted platform-aware JSON baseline
// calculate_stats - Computes deterministic min, max, average, and median
// run_perf_suite - Runs the benchmark set on a MultiModule fixture
// compare_perf_baseline - Compares current medians with a stored baseline
// render_perf_report - Renders compact text output for CLI users
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Implemented Phase-80 performance regression baseline checker]
// END_CHANGE_SUMMARY

use crate::test::fixture::{FixtureTemplate, TestFixture};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::Instant;

const DEFAULT_ITERATIONS: usize = 1;
const DEFAULT_WARMUP: bool = true;
const COMMAND_OUTPUT_TAIL_LINES: usize = 120;
const COMMAND_OUTPUT_CHAR_LIMIT: usize = 12000;
const MIN_DELTA_BASELINE_MS: f64 = 100.0;

// START_public_api

// START_PerfMode
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PerfMode {
    Report,
    Baseline,
    Check,
}
// END_PerfMode

// START_PerfTestOptions
#[derive(Debug, Clone)]
pub struct PerfTestOptions {
    pub mode: PerfMode,
    pub baseline_path: PathBuf,
    pub threshold_pct: f64,
    pub filter: Option<String>,
    pub iterations: usize,
    pub warmup: bool,
    pub binary_path: Option<PathBuf>,
}
// END_PerfTestOptions

impl Default for PerfTestOptions {
    // START_CONTRACT_PerfTestOptions::default
    // PURPOSE: Provide conservative CI-friendly perf options
    // OUTPUTS: { PerfTestOptions }
    // LINKS:
    //   -> NFR-002 (traces_to) - perf gate should be runnable in CI
    // START_perf_test_options_default
    fn default() -> Self {
        Self {
            mode: PerfMode::Report,
            baseline_path: PathBuf::from("tests/baselines/perf.json"),
            threshold_pct: 20.0,
            filter: None,
            iterations: DEFAULT_ITERATIONS,
            warmup: DEFAULT_WARMUP,
            binary_path: None,
        }
    }
    // END_perf_test_options_default
}

// START_PlatformMetadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformMetadata {
    pub os: String,
    pub arch: String,
    pub profile: String,
    pub synapse_version: String,
}
// END_PlatformMetadata

// START_BenchmarkStats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkStats {
    pub samples: usize,
    pub min_ms: f64,
    pub max_ms: f64,
    pub avg_ms: f64,
    pub median_ms: f64,
}
// END_BenchmarkStats

// START_BenchmarkResult
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkResult {
    pub name: String,
    pub command: Vec<String>,
    pub success: bool,
    pub status_code: Option<i32>,
    pub samples_ms: Vec<f64>,
    pub stats: BenchmarkStats,
}
// END_BenchmarkResult

// START_BenchmarkBaseline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkBaseline {
    pub name: String,
    pub command: Vec<String>,
    pub success: bool,
    pub status_code: Option<i32>,
    pub median_ms: f64,
    pub stats: BenchmarkStats,
}
// END_BenchmarkBaseline

// START_PerfBaseline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerfBaseline {
    pub schema_version: u32,
    pub created_at: String,
    pub metadata: PlatformMetadata,
    pub benchmarks: Vec<BenchmarkBaseline>,
}
// END_PerfBaseline

// START_ComparisonStatus
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ComparisonStatus {
    Passed,
    Regression,
    MissingBaseline,
}
// END_ComparisonStatus

impl ComparisonStatus {
    // START_CONTRACT_ComparisonStatus::as_str
    // PURPOSE: Return stable status text for compact perf reports
    // OUTPUTS: { &'static str }
    // LINKS:
    //   -> NFR-003 (traces_to) - report output must be compact and stable
    // START_comparison_status_as_str
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Regression => "regression",
            Self::MissingBaseline => "missing-baseline",
        }
    }
    // END_comparison_status_as_str
}

// START_BenchmarkComparison
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkComparison {
    pub name: String,
    pub baseline_median_ms: Option<f64>,
    pub current_median_ms: f64,
    pub delta_pct: Option<f64>,
    pub status: ComparisonStatus,
}
// END_BenchmarkComparison

// START_PerfComparisonReport
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerfComparisonReport {
    pub passed: bool,
    pub threshold_pct: f64,
    pub metadata_match: bool,
    pub metadata_notes: Vec<String>,
    pub benchmarks: Vec<BenchmarkComparison>,
}
// END_PerfComparisonReport

// START_PerfRunReport
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PerfRunReport {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub mode: PerfMode,
    pub metadata: PlatformMetadata,
    pub baseline_path: String,
    pub baseline_written: bool,
    pub benchmarks: Vec<BenchmarkResult>,
    pub comparison: Option<PerfComparisonReport>,
}
// END_PerfRunReport

// START_CONTRACT_calculate_stats
// PURPOSE: Compute min, max, average, and median from benchmark samples deterministically
// INPUTS: { samples_ms: &[f64] }
// OUTPUTS: { anyhow::Result<BenchmarkStats> }
// LINKS:
//   -> V-M-TEST-PERF-REGRESSION (verified_by) - statistic determinism tests
//   -> NFR-002 (traces_to) - perf gate must be deterministic
// START_calculate_stats
pub fn calculate_stats(samples_ms: &[f64]) -> anyhow::Result<BenchmarkStats> {
    if samples_ms.is_empty() {
        anyhow::bail!("cannot calculate benchmark statistics from zero samples");
    }
    if samples_ms.iter().any(|sample| !sample.is_finite()) {
        anyhow::bail!("benchmark samples must be finite");
    }

    let mut sorted = samples_ms.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let samples = sorted.len();
    let min_ms = sorted[0];
    let max_ms = sorted[samples - 1];
    let avg_ms = sorted.iter().sum::<f64>() / samples as f64;
    let middle = samples / 2;
    let median_ms = if samples.is_multiple_of(2) {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    };

    Ok(BenchmarkStats {
        samples,
        min_ms,
        max_ms,
        avg_ms,
        median_ms,
    })
}
// END_calculate_stats

// START_CONTRACT_platform_metadata
// PURPOSE: Capture platform fields that make perf baselines comparable
// OUTPUTS: { PlatformMetadata }
// LINKS:
//   -> V-M-TEST-PERF-REGRESSION (verified_by) - metadata trace assertion
// START_platform_metadata
pub fn platform_metadata() -> PlatformMetadata {
    PlatformMetadata {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        profile: if cfg!(debug_assertions) {
            "debug".to_string()
        } else {
            "release".to_string()
        },
        synapse_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}
// END_platform_metadata

// START_CONTRACT_run_perf_suite
// PURPOSE: Run the full performance benchmark suite and optionally save or compare a baseline
// INPUTS: { options: &PerfTestOptions }
// OUTPUTS: { anyhow::Result<PerfRunReport> }
// SIDE_EFFECTS: creates an isolated fixture, invokes the Synapse binary, may read or write baseline JSON
// LINKS:
//   -> M-TEST-FIXTURE (depends) - MultiModule benchmark project
//   -> M-INDEXER (depends) - index benchmark
//   -> M-GRAPHRAG (depends) - graph query benchmark
//   -> M-GRACE-VERIFY (depends) - verify benchmark
//   -> V-M-TEST-PERF-REGRESSION (verified_by) - CLI baseline/check integration
// <LOG id="perf_benchmark_completed" level="INFO" ref="perf-suite" module="M-TEST-PERF-REGRESSION" contract="run_perf_suite">
//   EVENT: perf_benchmark_completed
//   EXPECTATION: benchmark medians are captured before baseline save or comparison
//   DECISION: run a fixed benchmark set inside an isolated MultiModule fixture
//   RESULT: success
//   TRACEABILITY: NFR-002,NFR-003
// </LOG>
// START_run_perf_suite
pub fn run_perf_suite(options: &PerfTestOptions) -> anyhow::Result<PerfRunReport> {
    let fixture = TestFixture::builder()
        .with_template(FixtureTemplate::MultiModule)
        .build()
        .context("create MultiModule performance fixture")?;
    let binary_path = options
        .binary_path
        .clone()
        .unwrap_or_else(resolve_syn_binary_path);
    let definitions = selected_benchmarks(options.filter.as_deref())?;

    if definitions
        .iter()
        .any(|definition| definition.requires_index)
    {
        run_syn_command(&binary_path, &fixture, &["index", "--force", "--no-git"])
            .context("prime fixture index before performance benchmarks")?;
    }

    let mut benchmarks = Vec::new();
    for definition in definitions {
        benchmarks.push(run_benchmark_definition(
            &binary_path,
            &fixture,
            &definition,
            options,
        )?);
    }

    let metadata = platform_metadata();
    let mut report = PerfRunReport {
        kind: "synapse.test.perf",
        mode: options.mode,
        metadata: metadata.clone(),
        baseline_path: options.baseline_path.display().to_string(),
        baseline_written: false,
        benchmarks,
        comparison: None,
    };

    match options.mode {
        PerfMode::Report => {}
        PerfMode::Baseline => {
            let baseline = baseline_from_report(&report);
            write_perf_baseline(&options.baseline_path, &baseline)?;
            report.baseline_written = true;
        }
        PerfMode::Check => {
            let baseline = read_perf_baseline(&options.baseline_path)?;
            let comparison = compare_perf_baseline(&baseline, &report, options.threshold_pct)?;
            report.comparison = Some(comparison);
        }
    }

    Ok(report)
}
// END_run_perf_suite

// START_CONTRACT_baseline_from_report
// PURPOSE: Convert a current run report into the persisted JSON baseline schema
// INPUTS: { report: &PerfRunReport }
// OUTPUTS: { PerfBaseline }
// LINKS:
//   -> V-M-TEST-PERF-REGRESSION (verified_by) - baseline JSON persistence tests
// START_baseline_from_report
pub fn baseline_from_report(report: &PerfRunReport) -> PerfBaseline {
    PerfBaseline {
        schema_version: 1,
        created_at: chrono::Utc::now().to_rfc3339(),
        metadata: report.metadata.clone(),
        benchmarks: report
            .benchmarks
            .iter()
            .map(|benchmark| BenchmarkBaseline {
                name: benchmark.name.clone(),
                command: benchmark.command.clone(),
                success: benchmark.success,
                status_code: benchmark.status_code,
                median_ms: benchmark.stats.median_ms,
                stats: benchmark.stats.clone(),
            })
            .collect(),
    }
}
// END_baseline_from_report

// START_CONTRACT_write_perf_baseline
// PURPOSE: Persist a performance baseline as pretty JSON
// INPUTS: { path: &Path }, { baseline: &PerfBaseline }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: creates parent directories and writes JSON
// LINKS:
//   -> V-M-TEST-PERF-REGRESSION (verified_by) - baseline save verification
// START_write_perf_baseline
pub fn write_perf_baseline(path: &Path, baseline: &PerfBaseline) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(baseline)?)?;
    Ok(())
}
// END_write_perf_baseline

// START_CONTRACT_read_perf_baseline
// PURPOSE: Read a persisted performance baseline with an actionable missing-baseline error
// INPUTS: { path: &Path }
// OUTPUTS: { anyhow::Result<PerfBaseline> }
// LINKS:
//   -> V-M-TEST-PERF-REGRESSION (verified_by) - missing baseline test
// START_read_perf_baseline
pub fn read_perf_baseline(path: &Path) -> anyhow::Result<PerfBaseline> {
    if !path.exists() {
        anyhow::bail!(
            "performance baseline missing at {}; run `syn test perf --baseline --baseline-path {}` first",
            path.display(),
            path.display()
        );
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("read performance baseline {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("parse performance baseline {}", path.display()))
}
// END_read_perf_baseline

// START_CONTRACT_compare_perf_baseline
// PURPOSE: Compare current benchmark medians with a platform-aware baseline and threshold
// INPUTS: { baseline: &PerfBaseline }, { current: &PerfRunReport }, { threshold_pct: f64 }
// OUTPUTS: { anyhow::Result<PerfComparisonReport> }
// LINKS:
//   -> V-M-TEST-PERF-REGRESSION (verified_by) - threshold comparison verification
//   -> NFR-002 (traces_to) - CI regression gate behavior
// <LOG id="perf_baseline_compared" level="INFO" ref="perf-compare" module="M-TEST-PERF-REGRESSION" contract="compare_perf_baseline">
//   EVENT: perf_baseline_compared
//   EXPECTATION: only deltas above threshold fail when platform metadata matches
//   DECISION: compare median deltas against a caller-provided non-negative threshold
//   RESULT: success
//   TRACEABILITY: NFR-002
// </LOG>
// START_compare_perf_baseline
pub fn compare_perf_baseline(
    baseline: &PerfBaseline,
    current: &PerfRunReport,
    threshold_pct: f64,
) -> anyhow::Result<PerfComparisonReport> {
    if !threshold_pct.is_finite() || threshold_pct < 0.0 {
        anyhow::bail!("performance threshold must be a finite non-negative percentage");
    }

    let mut metadata_notes = Vec::new();
    compare_metadata_field(
        "os",
        &baseline.metadata.os,
        &current.metadata.os,
        &mut metadata_notes,
    );
    compare_metadata_field(
        "arch",
        &baseline.metadata.arch,
        &current.metadata.arch,
        &mut metadata_notes,
    );
    compare_metadata_field(
        "profile",
        &baseline.metadata.profile,
        &current.metadata.profile,
        &mut metadata_notes,
    );
    if baseline.metadata.synapse_version != current.metadata.synapse_version {
        metadata_notes.push(format!(
            "synapse_version differs: baseline={} current={}",
            baseline.metadata.synapse_version, current.metadata.synapse_version
        ));
    }
    let metadata_match = metadata_notes
        .iter()
        .all(|note| note.starts_with("synapse_version differs"));

    let baseline_by_name = baseline
        .benchmarks
        .iter()
        .map(|benchmark| (benchmark.name.as_str(), benchmark))
        .collect::<BTreeMap<_, _>>();
    let mut comparisons = Vec::new();
    for benchmark in &current.benchmarks {
        let Some(expected) = baseline_by_name.get(benchmark.name.as_str()) else {
            comparisons.push(BenchmarkComparison {
                name: benchmark.name.clone(),
                baseline_median_ms: None,
                current_median_ms: benchmark.stats.median_ms,
                delta_pct: None,
                status: ComparisonStatus::MissingBaseline,
            });
            continue;
        };
        let delta_pct = percentage_delta(benchmark.stats.median_ms, expected.median_ms);
        let status = if delta_pct > threshold_pct {
            ComparisonStatus::Regression
        } else {
            ComparisonStatus::Passed
        };
        comparisons.push(BenchmarkComparison {
            name: benchmark.name.clone(),
            baseline_median_ms: Some(expected.median_ms),
            current_median_ms: benchmark.stats.median_ms,
            delta_pct: Some(delta_pct),
            status,
        });
    }
    let passed = metadata_match
        && comparisons
            .iter()
            .all(|comparison| comparison.status == ComparisonStatus::Passed);

    Ok(PerfComparisonReport {
        passed,
        threshold_pct,
        metadata_match,
        metadata_notes,
        benchmarks: comparisons,
    })
}
// END_compare_perf_baseline

// START_CONTRACT_render_perf_report
// PURPOSE: Render compact human-readable performance benchmark output
// INPUTS: { report: &PerfRunReport }
// OUTPUTS: { String }
// LINKS:
//   -> M-CLI-TEST-COMMANDS (depends) - CLI text rendering
//   -> NFR-003 (traces_to) - perf output must remain token-bounded
// START_render_perf_report
pub fn render_perf_report(report: &PerfRunReport) -> String {
    let mut lines = vec![
        format!("Performance regression: {}", perf_mode_label(report.mode)),
        format!(
            "platform: {} {} {} synapse {}",
            report.metadata.os,
            report.metadata.arch,
            report.metadata.profile,
            report.metadata.synapse_version
        ),
        format!("baseline_path: {}", report.baseline_path),
    ];
    if report.baseline_written {
        lines.push("baseline_written: true".to_string());
    }
    lines.push("benchmarks:".to_string());
    for benchmark in &report.benchmarks {
        lines.push(format!(
            "- {} median={:.2}ms avg={:.2}ms samples={} status={}",
            benchmark.name,
            benchmark.stats.median_ms,
            benchmark.stats.avg_ms,
            benchmark.stats.samples,
            benchmark_status_label(benchmark.success, benchmark.status_code)
        ));
    }
    if let Some(comparison) = &report.comparison {
        lines.push(format!(
            "comparison: {} threshold={:.1}% metadata_match={}",
            if comparison.passed {
                "passed"
            } else {
                "failed"
            },
            comparison.threshold_pct,
            comparison.metadata_match
        ));
        for note in &comparison.metadata_notes {
            lines.push(format!("- metadata: {note}"));
        }
        for benchmark in &comparison.benchmarks {
            let delta = benchmark
                .delta_pct
                .map(|value| format!("{value:+.1}%"))
                .unwrap_or_else(|| "n/a".to_string());
            let baseline = benchmark
                .baseline_median_ms
                .map(|value| format!("{value:.2}ms"))
                .unwrap_or_else(|| "missing".to_string());
            lines.push(format!(
                "- {}: {} current={:.2}ms baseline={} delta={}",
                benchmark.name,
                benchmark.status.as_str(),
                benchmark.current_median_ms,
                baseline,
                delta
            ));
        }
    }
    lines.join("\n")
}
// END_render_perf_report

// END_public_api

// START_BenchmarkDefinition
#[derive(Debug, Clone, Copy)]
struct BenchmarkDefinition {
    name: &'static str,
    args: &'static [&'static str],
    requires_index: bool,
    allow_failure: bool,
}
// END_BenchmarkDefinition

const BENCHMARKS: &[BenchmarkDefinition] = &[
    BenchmarkDefinition {
        name: "index",
        args: &["index", "--force", "--no-git"],
        requires_index: false,
        allow_failure: false,
    },
    BenchmarkDefinition {
        name: "search_cold",
        args: &["search", "auth", "--max-results", "3"],
        requires_index: true,
        allow_failure: false,
    },
    BenchmarkDefinition {
        name: "search_warm",
        args: &["search", "auth", "--max-results", "3"],
        requires_index: true,
        allow_failure: false,
    },
    BenchmarkDefinition {
        name: "verify",
        args: &["verify", "--profile", "lite", "--ci", "--mod", "M-CORE"],
        requires_index: false,
        allow_failure: true,
    },
    BenchmarkDefinition {
        name: "graphrag",
        args: &["graphrag", "overview"],
        requires_index: true,
        allow_failure: false,
    },
];

// START_CommandInvocation
#[derive(Debug)]
struct CommandInvocation {
    duration_ms: f64,
    success: bool,
    status_code: Option<i32>,
    stdout_tail: String,
    stderr_tail: String,
}
// END_CommandInvocation

// START_CONTRACT_selected_benchmarks
// PURPOSE: Select benchmark definitions by optional substring filter
// INPUTS: { filter: Option<&str> }
// OUTPUTS: { anyhow::Result<Vec<BenchmarkDefinition>> }
// LINKS:
//   -> NFR-002 (traces_to) - scoped local perf runs should be deterministic
// START_selected_benchmarks
fn selected_benchmarks(filter: Option<&str>) -> anyhow::Result<Vec<BenchmarkDefinition>> {
    let selected = BENCHMARKS
        .iter()
        .copied()
        .filter(|definition| {
            filter.is_none_or(|needle| needle.is_empty() || definition.name.contains(needle))
        })
        .collect::<Vec<_>>();
    if selected.is_empty() {
        anyhow::bail!("no performance benchmarks matched the requested filter");
    }
    Ok(selected)
}
// END_selected_benchmarks

// START_CONTRACT_run_benchmark_definition
// PURPOSE: Execute warmup and measured samples for one benchmark definition
// INPUTS: { binary_path: &Path }, { fixture: &TestFixture }, { definition: &BenchmarkDefinition }, { options: &PerfTestOptions }
// OUTPUTS: { anyhow::Result<BenchmarkResult> }
// SIDE_EFFECTS: invokes the Synapse binary inside the benchmark fixture
// LINKS:
//   -> V-M-TEST-PERF-REGRESSION (verified_by) - benchmark model verification
// START_run_benchmark_definition
fn run_benchmark_definition(
    binary_path: &Path,
    fixture: &TestFixture,
    definition: &BenchmarkDefinition,
    options: &PerfTestOptions,
) -> anyhow::Result<BenchmarkResult> {
    if options.warmup {
        let invocation = run_syn_command(binary_path, fixture, definition.args)
            .with_context(|| format!("warmup benchmark {}", definition.name))?;
        ensure_allowed_status(definition, &invocation)?;
    }
    let iterations = options.iterations.max(1);
    let mut samples = Vec::with_capacity(iterations);
    let mut success = true;
    let mut status_code = Some(0);
    for _ in 0..iterations {
        let invocation = run_syn_command(binary_path, fixture, definition.args)
            .with_context(|| format!("run benchmark {}", definition.name))?;
        ensure_allowed_status(definition, &invocation)?;
        success &= invocation.success;
        status_code = invocation.status_code;
        samples.push(invocation.duration_ms);
    }
    let stats = calculate_stats(&samples)?;
    Ok(BenchmarkResult {
        name: definition.name.to_string(),
        command: definition
            .args
            .iter()
            .map(|arg| (*arg).to_string())
            .collect(),
        success,
        status_code,
        samples_ms: samples,
        stats,
    })
}
// END_run_benchmark_definition

// START_CONTRACT_run_syn_command
// PURPOSE: Invoke the Synapse binary in a fixture and return bounded timing evidence
// INPUTS: { binary_path: &Path }, { fixture: &TestFixture }, { args: &[&str] }
// OUTPUTS: { anyhow::Result<CommandInvocation> }
// SIDE_EFFECTS: spawns a child process
// LINKS:
//   -> M-TEST-FIXTURE (depends) - isolated process environment
// START_run_syn_command
fn run_syn_command(
    binary_path: &Path,
    fixture: &TestFixture,
    args: &[&str],
) -> anyhow::Result<CommandInvocation> {
    let started = Instant::now();
    let output = ProcessCommand::new(binary_path)
        .args(args)
        .current_dir(fixture.root())
        .env("RUST_LOG", "error")
        .envs(fixture.command_env())
        .output()
        .with_context(|| format!("spawn syn binary {}", binary_path.display()))?;
    let duration_ms = started.elapsed().as_secs_f64() * 1000.0;

    Ok(CommandInvocation {
        duration_ms,
        success: output.status.success(),
        status_code: output.status.code(),
        stdout_tail: compact_output(&output.stdout),
        stderr_tail: compact_output(&output.stderr),
    })
}
// END_run_syn_command

// START_CONTRACT_ensure_allowed_status
// PURPOSE: Fail fast for unexpected benchmark exits while allowing documented verify fixture exits
// INPUTS: { definition: &BenchmarkDefinition }, { invocation: &CommandInvocation }
// OUTPUTS: { anyhow::Result<()> }
// LINKS:
//   -> NFR-002 (traces_to) - perf runner distinguishes process failure from expected fixture verification failure
// START_ensure_allowed_status
fn ensure_allowed_status(
    definition: &BenchmarkDefinition,
    invocation: &CommandInvocation,
) -> anyhow::Result<()> {
    if invocation.success || definition.allow_failure {
        return Ok(());
    }
    anyhow::bail!(
        "benchmark command failed: {} status {:?}\nstdout:\n{}\nstderr:\n{}",
        definition.name,
        invocation.status_code,
        invocation.stdout_tail,
        invocation.stderr_tail
    );
}
// END_ensure_allowed_status

// START_CONTRACT_resolve_syn_binary_path
// PURPOSE: Resolve the Synapse binary for perf CLI runs and integration tests
// OUTPUTS: { PathBuf }
// LINKS:
//   -> V-M-TEST-PERF-REGRESSION (verified_by) - binary invocation helper coverage
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

// START_CONTRACT_compare_metadata_field
// PURPOSE: Append a metadata mismatch note for platform fields that must match
// INPUTS: { field: &str }, { baseline: &str }, { current: &str }, { notes: &mut Vec<String> }
// OUTPUTS: { () }
// LINKS:
//   -> V-M-TEST-PERF-REGRESSION (verified_by) - platform mismatch behavior
// START_compare_metadata_field
fn compare_metadata_field(field: &str, baseline: &str, current: &str, notes: &mut Vec<String>) {
    if baseline != current {
        notes.push(format!(
            "{field} differs: baseline={baseline} current={current}"
        ));
    }
}
// END_compare_metadata_field

// START_CONTRACT_percentage_delta
// PURPOSE: Calculate positive regression percentage with a minimum denominator for short-command noise
// INPUTS: { current: f64 }, { baseline: f64 }
// OUTPUTS: { f64 }
// LINKS:
//   -> V-M-TEST-PERF-REGRESSION (verified_by) - threshold comparison tests
// START_percentage_delta
fn percentage_delta(current: f64, baseline: f64) -> f64 {
    let denominator = baseline.abs().max(MIN_DELTA_BASELINE_MS);
    if denominator <= f64::EPSILON {
        if current.abs() <= f64::EPSILON {
            0.0
        } else {
            100.0
        }
    } else {
        ((current - baseline) / denominator) * 100.0
    }
}
// END_percentage_delta

// START_CONTRACT_perf_mode_label
// PURPOSE: Return a lowercase stable label for perf report modes
// INPUTS: { mode: PerfMode }
// OUTPUTS: { &'static str }
// LINKS:
//   -> NFR-003 (traces_to) - compact report mode labels
// START_perf_mode_label
fn perf_mode_label(mode: PerfMode) -> &'static str {
    match mode {
        PerfMode::Report => "report",
        PerfMode::Baseline => "baseline",
        PerfMode::Check => "check",
    }
}
// END_perf_mode_label

// START_CONTRACT_benchmark_status_label
// PURPOSE: Render a stable benchmark process status for text reports
// INPUTS: { success: bool }, { status_code: Option<i32> }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-003 (traces_to) - perf report status evidence must stay compact
// START_benchmark_status_label
fn benchmark_status_label(success: bool, status_code: Option<i32>) -> String {
    if success {
        "ok".to_string()
    } else {
        format!(
            "exit({})",
            status_code.map_or("signal".to_string(), |code| code.to_string())
        )
    }
}
// END_benchmark_status_label

// START_CONTRACT_compact_output
// PURPOSE: Return a bounded UTF-8 command output tail for failed benchmark invocations
// INPUTS: { bytes: &[u8] }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-003 (traces_to) - failure evidence must remain compact
// START_compact_output
fn compact_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text
        .lines()
        .rev()
        .take(COMMAND_OUTPUT_TAIL_LINES)
        .collect::<Vec<_>>();
    lines.reverse();
    syn_core::utils::truncate_chars(&lines.join("\n"), COMMAND_OUTPUT_CHAR_LIMIT)
}
// END_compact_output

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn report_with_medians(metadata: PlatformMetadata, medians: &[(&str, f64)]) -> PerfRunReport {
        PerfRunReport {
            kind: "synapse.test.perf",
            mode: PerfMode::Check,
            metadata,
            baseline_path: "baseline.json".to_string(),
            baseline_written: false,
            benchmarks: medians
                .iter()
                .map(|(name, median)| BenchmarkResult {
                    name: (*name).to_string(),
                    command: vec![(*name).to_string()],
                    success: true,
                    status_code: Some(0),
                    samples_ms: vec![*median],
                    stats: BenchmarkStats {
                        samples: 1,
                        min_ms: *median,
                        max_ms: *median,
                        avg_ms: *median,
                        median_ms: *median,
                    },
                })
                .collect(),
            comparison: None,
        }
    }

    #[test]
    fn perf_stats_compute_min_max_average_and_median() {
        let stats = calculate_stats(&[10.0, 2.0, 8.0, 4.0]).expect("stats");

        assert_eq!(stats.samples, 4);
        assert_eq!(stats.min_ms, 2.0);
        assert_eq!(stats.max_ms, 10.0);
        assert_eq!(stats.avg_ms, 6.0);
        assert_eq!(stats.median_ms, 6.0);
    }

    #[test]
    fn perf_comparison_fails_only_above_threshold() {
        let metadata = platform_metadata();
        let baseline_report = report_with_medians(metadata.clone(), &[("index", 100.0)]);
        let baseline = baseline_from_report(&baseline_report);
        let current_pass = report_with_medians(metadata.clone(), &[("index", 115.0)]);
        let current_fail = report_with_medians(metadata, &[("index", 121.0)]);

        let pass = compare_perf_baseline(&baseline, &current_pass, 20.0).expect("compare");
        let fail = compare_perf_baseline(&baseline, &current_fail, 20.0).expect("compare");

        assert!(pass.passed);
        assert!(!fail.passed);
        assert_eq!(fail.benchmarks[0].status, ComparisonStatus::Regression);
    }

    #[test]
    fn perf_comparison_reports_missing_baseline_benchmark() {
        let metadata = platform_metadata();
        let baseline_report = report_with_medians(metadata.clone(), &[("index", 100.0)]);
        let baseline = baseline_from_report(&baseline_report);
        let current = report_with_medians(metadata, &[("search_cold", 30.0)]);

        let comparison = compare_perf_baseline(&baseline, &current, 20.0).expect("compare");

        assert!(!comparison.passed);
        assert_eq!(
            comparison.benchmarks[0].status,
            ComparisonStatus::MissingBaseline
        );
    }

    #[test]
    fn perf_short_benchmark_delta_uses_noise_floor() {
        let metadata = platform_metadata();
        let baseline_report = report_with_medians(metadata.clone(), &[("graphrag", 36.0)]);
        let baseline = baseline_from_report(&baseline_report);
        let current = report_with_medians(metadata, &[("graphrag", 46.0)]);

        let comparison = compare_perf_baseline(&baseline, &current, 20.0).expect("compare");

        assert!(comparison.passed);
        assert_eq!(comparison.benchmarks[0].delta_pct, Some(10.0));
    }

    #[test]
    fn perf_missing_baseline_has_actionable_message() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("missing.json");

        let error = read_perf_baseline(&path).expect_err("missing baseline should fail");

        assert!(error.to_string().contains("syn test perf --baseline"));
    }

    #[test]
    fn perf_baseline_round_trips_platform_metadata() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("perf.json");
        let metadata = platform_metadata();
        let report = report_with_medians(metadata.clone(), &[("verify", 42.0)]);
        let baseline = baseline_from_report(&report);

        write_perf_baseline(&path, &baseline).expect("write baseline");
        let loaded = read_perf_baseline(&path).expect("read baseline");

        assert_eq!(loaded.metadata, metadata);
        assert_eq!(loaded.benchmarks[0].name, "verify");
        assert_eq!(loaded.benchmarks[0].median_ms, 42.0);
    }

    #[test]
    fn perf_platform_metadata_mismatch_blocks_check() {
        let mut baseline_metadata = platform_metadata();
        let current_metadata = platform_metadata();
        baseline_metadata.os = "other-os".to_string();
        let baseline_report = report_with_medians(baseline_metadata, &[("index", 100.0)]);
        let baseline = baseline_from_report(&baseline_report);
        let current = report_with_medians(current_metadata, &[("index", 101.0)]);

        let comparison = compare_perf_baseline(&baseline, &current, 20.0).expect("compare");

        assert!(!comparison.passed);
        assert!(!comparison.metadata_match);
        assert!(comparison.metadata_notes[0].contains("os differs"));
    }

    #[test]
    fn perf_text_report_includes_comparison_rows() {
        let metadata = platform_metadata();
        let mut report = report_with_medians(metadata.clone(), &[("index", 110.0)]);
        let baseline = baseline_from_report(&report_with_medians(metadata, &[("index", 100.0)]));
        report.comparison = Some(compare_perf_baseline(&baseline, &report, 20.0).expect("compare"));

        let rendered = render_perf_report(&report);

        assert!(rendered.contains("Performance regression: check"));
        assert!(rendered.contains("index"));
        assert!(rendered.contains("delta=+10.0%"));
    }
}
