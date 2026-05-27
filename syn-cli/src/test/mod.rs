// MODULE_CONTRACT
// MODULE_ID: M-TEST-HARNESS
// PURPOSE: Planned facade for Synapse E2E and regression test infrastructure.
// SCOPE: Contract stub for fixture, e2e, snapshot, coverage, perf, contract differential, and resilience harness modules.
// DEPENDS: M-TEST-FIXTURE, M-TEST-E2E-RUNNER, M-TEST-SNAPSHOT, M-TEST-COVERAGE-MATRIX, M-TEST-PERF-REGRESSION, M-TEST-CONTRACT-DIFFERENTIAL, M-TEST-RESILIENCE-CHAOS
// LINKS:
//   -> Phase-76 (implements) - UPGRADE_3 test harness foundation
//   <- V-M-TEST-HARNESS (verified_by) - harness facade verification

// START_MODULE_MAP
// fixture - Reusable project fixture factory
// e2e - TOML scenario runner
// snapshot - Golden snapshot comparison engine
// coverage - Module evidence coverage matrix
// perf - Performance regression baseline checker
// contract_test - Cascade differential assertions
// resilience - Corrupted-state resilience harness
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v0.2.0 - Exposed UPGRADE_3 test harness module facade]
// END_CHANGE_SUMMARY

// START_CONTRACT_planned_scope
// PURPOSE: Declare the planned test harness facade until Phase-76 implementation begins
// OUTPUTS: { contract marker for M-TEST-HARNESS }
// LINKS:
//   -> NFR-002 (traces_to) - reliable release verification
//   -> NFR-003 (traces_to) - token-efficient test evidence
// START_planned_scope
// END_planned_scope

// START_public_api
pub mod contract_test;
pub mod coverage;
pub mod e2e;
pub mod fixture;
pub mod perf;
pub mod resilience;
pub mod snapshot;
// END_public_api
