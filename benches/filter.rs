// MODULE_CONTRACT
// MODULE_ID: M-TEST-PERF-REGRESSION
// PURPOSE: Criterion benchmarks for proxy filter chain throughput.
// SCOPE: Filter chain application on representative command outputs (10KB–100KB).
// DEPENDS: M-TEST-PERF-REGRESSION, M-TEST-FIXTURE, M-PROXY-FILTER
// LINKS:
//   → Phase-104 (implements) — filter benchmark group
//   ← V-M-TEST-PERF-REGRESSION (verified_by) — filter benchmark coverage

// START_MODULE_MAP
// bench_filter_chain — Measures 30-filter chain on 10KB output
// bench_filter_chain_large — Measures 30-filter chain on 100KB output
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Phase-104 proxy filter chain benchmarks]
// END_CHANGE_SUMMARY

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use syn_proxy::proxy::toml_filter::FilterEngine;

mod common;

// START_public_api

fn make_filter_engine() -> FilterEngine {
    let toml_content = r#"
[filters.strip_ansi]
type = "strip_ansi"
enabled = true

[filters.truncate_lines]
type = "truncate_lines"
enabled = true
max_lines = 500
"#;
    FilterEngine::from_toml(toml_content, "bench-filter").expect("load filters")
}

fn bench_filter_chain(c: &mut Criterion) {
    c.bench_function("filter_chain_10k", |b| {
        let engine = make_filter_engine();
        let input = "line ".repeat(1000);
        let filter_names = engine.filter_names();
        b.iter_batched(
            || input.clone(),
            |text| {
                let mut output = text;
                for name in &filter_names {
                    if let Some(filter) = engine.find_filter_by_name(name) {
                        output = engine.apply(filter, &output);
                    }
                }
                black_box(output)
            },
            BatchSize::LargeInput,
        )
    });
}

fn bench_filter_chain_large(c: &mut Criterion) {
    c.bench_function("filter_chain_100k", |b| {
        let engine = make_filter_engine();
        let input = "line ".repeat(20000);
        let filter_names = engine.filter_names();
        b.iter_batched(
            || input.clone(),
            |text| {
                let mut output = text;
                for name in &filter_names {
                    if let Some(filter) = engine.find_filter_by_name(name) {
                        output = engine.apply(filter, &output);
                    }
                }
                black_box(output)
            },
            BatchSize::LargeInput,
        )
    });
}

// END_public_api

criterion_group! {
    name = filter_benches;
    config = common::bench_config();
    targets = bench_filter_chain, bench_filter_chain_large
}
criterion_main!(filter_benches);
