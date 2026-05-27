// MODULE_CONTRACT
// MODULE_ID: M-TEST-PERF-REGRESSION
// PURPOSE: Criterion benchmarks for Synapse indexing and semantic search workflows.
// SCOPE: index_multimodule, search_cold, and search_warm benchmark targets using isolated MultiModule fixtures.
// DEPENDS: M-TEST-PERF-REGRESSION, M-TEST-FIXTURE, M-INDEXER
// LINKS:
//   -> Phase-89 (implements) - Criterion search benchmark group
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - search benchmark execution

// START_MODULE_MAP
// bench_index_multimodule - Measures full MultiModule indexing
// bench_semantic_search_cold - Measures first search after fixture indexing
// bench_semantic_search_warm - Measures repeated search against a warmed fixture
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.1 - Added Phase-89 Criterion search benchmarks with NFR trace links]
// END_CHANGE_SUMMARY

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

mod common;

// START_public_api

// START_CONTRACT_bench_index_multimodule
// PURPOSE: Measure complete indexing on a fresh MultiModule fixture.
// INPUTS: { c: &mut Criterion }
// OUTPUTS: { () }
// SIDE_EFFECTS: creates temporary fixtures and Synapse index storage
// LINKS:
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - search benchmark coverage
// START_bench_index_multimodule
fn bench_index_multimodule(c: &mut Criterion) {
    c.bench_function("index_multimodule", |b| {
        b.iter_batched(
            common::multimodule_fixture,
            |fixture| {
                let output = common::run_syn(&fixture, &["index", "--force", "--no-git"]);
                common::assert_success("index_multimodule", &output);
                black_box(output.stdout.len() + output.stderr.len())
            },
            BatchSize::LargeInput,
        );
    });
}
// END_bench_index_multimodule

// START_CONTRACT_bench_semantic_search_cold
// PURPOSE: Measure the first search command after indexing a fresh fixture.
// INPUTS: { c: &mut Criterion }
// OUTPUTS: { () }
// SIDE_EFFECTS: creates temporary fixtures and Synapse index storage
// LINKS:
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - cold search benchmark coverage
// START_bench_semantic_search_cold
fn bench_semantic_search_cold(c: &mut Criterion) {
    c.bench_function("search_cold", |b| {
        b.iter_batched(
            || {
                let fixture = common::multimodule_fixture();
                common::prime_index(&fixture);
                fixture
            },
            |fixture| {
                let output = common::run_syn(&fixture, &["search", "auth", "--max-results", "3"]);
                common::assert_success("search_cold", &output);
                black_box(output.stdout.len() + output.stderr.len())
            },
            BatchSize::LargeInput,
        );
    });
}
// END_bench_semantic_search_cold

// START_CONTRACT_bench_semantic_search_warm
// PURPOSE: Measure repeated search against an indexed fixture after one warm-up query.
// INPUTS: { c: &mut Criterion }
// OUTPUTS: { () }
// SIDE_EFFECTS: creates one temporary fixture and reads Synapse index storage
// LINKS:
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - bounded benchmark evidence
//   <- V-M-TEST-PERF-REGRESSION (verified_by) - warm search benchmark coverage
// START_bench_semantic_search_warm
fn bench_semantic_search_warm(c: &mut Criterion) {
    let fixture = common::multimodule_fixture();
    common::prime_index(&fixture);
    let warmup = common::run_syn(&fixture, &["search", "auth", "--max-results", "3"]);
    common::assert_success("search_warm warmup", &warmup);

    c.bench_function("search_warm", |b| {
        b.iter(|| {
            let output = common::run_syn(&fixture, &["search", "auth", "--max-results", "3"]);
            common::assert_success("search_warm", &output);
            black_box(output.stdout.len() + output.stderr.len())
        });
    });
}
// END_bench_semantic_search_warm

// END_public_api

criterion_group! {
    name = search_benches;
    config = common::bench_config();
    targets = bench_index_multimodule, bench_semantic_search_cold, bench_semantic_search_warm
}
criterion_main!(search_benches);
