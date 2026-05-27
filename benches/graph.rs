// MODULE_CONTRACT
// MODULE_ID: M-TEST-PERF-REGRESSION
// PURPOSE: Criterion benchmarks for graph construction, traceability scanning, and cascade reachability.
// SCOPE: GraphRAG build, MyGRACE traceability scan, and cascade impact traversal on isolated MultiModule fixtures.
// DEPENDS: M-TEST-PERF-REGRESSION, M-TEST-FIXTURE, M-GRAPHRAG, M-GRACE-CASCADE
// LINKS:
//   -> Phase-89 (implements) - Criterion graph benchmark group
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - graph benchmark execution

// START_MODULE_MAP
// bench_graph_build - Measures GraphRAG graph construction
// bench_traceability_scan - Measures project traceability scan
// bench_cascade_reachable - Measures downstream cascade reachability traversal
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.1 - Added Phase-89 Criterion graph benchmarks with explicit failure handling]
// END_CHANGE_SUMMARY

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

mod common;

// START_public_api

// START_CONTRACT_bench_graph_build
// PURPOSE: Measure GraphRAG build latency for a fresh MultiModule fixture.
// INPUTS: { c: &mut Criterion }
// OUTPUTS: { () }
// LINKS:
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - graph benchmark coverage
// START_bench_graph_build
fn bench_graph_build(c: &mut Criterion) {
    c.bench_function("graph_build", |b| {
        b.iter_batched(
            common::multimodule_fixture,
            |fixture| {
                let mut graphrag = syn_engine::graphrag::GraphRag::new();
                common::require_ok("build graph", graphrag.build(fixture.root()));
                black_box(graphrag.overview().map(|overview| overview.total_nodes))
            },
            BatchSize::LargeInput,
        );
    });
}
// END_bench_graph_build

// START_CONTRACT_bench_traceability_scan
// PURPOSE: Measure traceability scan latency on a MultiModule fixture.
// INPUTS: { c: &mut Criterion }
// OUTPUTS: { () }
// LINKS:
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - traceability benchmark coverage
// START_bench_traceability_scan
fn bench_traceability_scan(c: &mut Criterion) {
    c.bench_function("traceability_scan", |b| {
        b.iter_batched(
            common::multimodule_fixture,
            |fixture| {
                let report = common::require_ok(
                    "scan traceability",
                    syn_engine::grace::traceability::scan_project_traceability(fixture.root()),
                );
                black_box(report.traceability_score)
            },
            BatchSize::LargeInput,
        );
    });
}
// END_bench_traceability_scan

// START_CONTRACT_bench_cascade_reachable
// PURPOSE: Measure downstream cascade traversal from M-CORE.
// INPUTS: { c: &mut Criterion }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes cascade preview artifacts inside temporary fixtures
// LINKS:
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - cascade reachability benchmark coverage
// START_bench_cascade_reachable
fn bench_cascade_reachable(c: &mut Criterion) {
    c.bench_function("cascade_reachable", |b| {
        b.iter_batched(
            common::multimodule_fixture,
            |fixture| {
                let analysis = common::require_ok(
                    "cascade impact",
                    syn_engine::grace::cascade::cascade_impact(
                        fixture.root(),
                        "M-CORE",
                        "Benchmark dependency reachability",
                    ),
                );
                black_box(analysis.total_affected)
            },
            BatchSize::LargeInput,
        );
    });
}
// END_bench_cascade_reachable

// END_public_api

criterion_group! {
    name = graph_benches;
    config = common::bench_config();
    targets = bench_graph_build, bench_traceability_scan, bench_cascade_reachable
}
criterion_main!(graph_benches);
