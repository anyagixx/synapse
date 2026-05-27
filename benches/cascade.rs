// MODULE_CONTRACT
// MODULE_ID: M-TEST-PERF-REGRESSION
// PURPOSE: Criterion benchmarks for MyGRACE cascade impact and execute workflows.
// SCOPE: cascade_impact and cascade_execute benchmark targets using isolated MultiModule fixtures and cached previews.
// DEPENDS: M-TEST-PERF-REGRESSION, M-TEST-FIXTURE, M-GRACE-CASCADE
// LINKS:
//   -> Phase-89 (implements) - Criterion cascade benchmark group
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - cascade benchmark execution

// START_MODULE_MAP
// bench_cascade_impact - Measures cascade impact preview generation
// bench_cascade_execute - Measures execution of a cached cascade preview
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.1 - Added Phase-89 Criterion cascade benchmarks with explicit failure handling]
// END_CHANGE_SUMMARY

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use syn_engine::grace::cascade::{cascade_execute, cascade_impact, CascadeExecuteOptions};

mod common;

// START_public_api

// START_CONTRACT_bench_cascade_impact
// PURPOSE: Measure cascade impact preview generation for a representative module change.
// INPUTS: { c: &mut Criterion }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes cascade preview artifacts inside temporary fixtures
// LINKS:
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - cascade impact benchmark coverage
// START_bench_cascade_impact
fn bench_cascade_impact(c: &mut Criterion) {
    c.bench_function("cascade_impact_small", |b| {
        b.iter_batched(
            common::multimodule_fixture,
            |fixture| {
                let analysis = common::require_ok(
                    "cascade impact",
                    cascade_impact(fixture.root(), "M-CORE", "Benchmark cascade impact preview"),
                );
                black_box((analysis.cascade_id, analysis.total_affected))
            },
            BatchSize::LargeInput,
        );
    });
}
// END_bench_cascade_impact

// START_CONTRACT_bench_cascade_execute
// PURPOSE: Measure cascade execution from a cached preview without auto-applying source changes.
// INPUTS: { c: &mut Criterion }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes cascade preview, proposal, and changelog artifacts inside temporary fixtures
// LINKS:
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - cascade execute benchmark coverage
// START_bench_cascade_execute
fn bench_cascade_execute(c: &mut Criterion) {
    c.bench_function("cascade_execute", |b| {
        b.iter_batched(
            || {
                let fixture = common::multimodule_fixture();
                let analysis = common::require_ok(
                    "cascade impact before execute",
                    cascade_impact(
                        fixture.root(),
                        "M-CORE",
                        "Benchmark cascade execute preview",
                    ),
                );
                (fixture, analysis.cascade_id)
            },
            |(fixture, cascade_id)| {
                let report = common::require_ok(
                    "cascade execute",
                    cascade_execute(
                        fixture.root(),
                        CascadeExecuteOptions {
                            cascade_id,
                            auto_apply_contracts: false,
                            auto_apply_code: false,
                            apply_to_phases: Vec::new(),
                        },
                    ),
                );
                black_box((
                    report.applied_changes,
                    report.proposed_changes,
                    report.blocked_changes,
                ))
            },
            BatchSize::LargeInput,
        );
    });
}
// END_bench_cascade_execute

// END_public_api

criterion_group! {
    name = cascade_benches;
    config = common::bench_config();
    targets = bench_cascade_impact, bench_cascade_execute
}
criterion_main!(cascade_benches);
