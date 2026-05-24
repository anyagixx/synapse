// MODULE_CONTRACT
// MODULE_ID: M-TESTS-INTEGRATION
// PURPOSE: End-to-end integration tests for Synapse CLI commands
// SCOPE: init, clean config bootstrap, tracking identity, index, search, verify, status, run scenario/action queue, proxy, RTK shortcuts, rewrite hook decisions, route preview, raw evidence, filter trust/verification, gain, doctor, hooks, compress
// DEPENDS: M-CLI, M-CLI-RTK-COMMANDS, M-INDEXER, M-GRACE, M-RUNNER, M-CONFIG, M-PROXY, M-PROXY-RUNNER
// LINKS:
//   ← V-M-CLI (verified_by) - CLI integration coverage
//   ← V-M-PROXY-ROUTER (verified_by) - route preview integration coverage

// START_MODULE_MAP
// test_clean_config_bootstrap_commands_do_not_require_config_file — Verifies clean config bootstrap behavior
// test_init_and_index — Verifies init, index, search, verify, and status
// test_proxy_and_gain — Verifies proxy execution and token savings output
// test_proxy_route_preview_for_rtk_like_command — Verifies route preview without execution
// test_proxy_project_filter_requires_trust — Verifies project-local proxy filters are trust gated
// test_filters_verify_cli_runs_inline_tests — Verifies filter inline tests run through CLI
// test_proxy_evidence_hint_writes_raw_output — Verifies explicit raw evidence artifact contains full unfiltered output
// test_rtk_read_shortcut_filters_and_preserves_evidence — Verifies first-class read shortcut delegates to proxy
// test_rewrite_cli_delegates_to_router — Verifies hook-facing command rewrites use proxy router decisions
// test_run_scenario_cli_reports_gate_and_replay_json — Verifies bounded run scenario CLI JSON output
// test_run_action_cli_plans_and_replays_run — Verifies run action queue CLI plan/replay output
// test_doctor_and_hooks — Verifies setup diagnostics and hook status
// test_scoped_and_json_cli — Verifies scoped JSON GRACE gates
// test_init_from_existing — Verifies existing repository bootstrap
// test_ci_commands — Verifies CI command aliases
// test_compress_roundtrip — Verifies compression restore behavior
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v4.3.0 — Added syn rewrite integration coverage]
// END_CHANGE_SUMMARY

use std::process::Command;

// START_CONTRACT_test_clean_config_bootstrap_commands_do_not_require_config_file
// PURPOSE: Verify clean-machine bootstrap commands work without an existing synapsec.toml
// SIDE_EFFECTS: creates isolated temp project and config directories
// START_test_clean_config_bootstrap_commands_do_not_require_config_file
#[test]
fn test_clean_config_bootstrap_commands_do_not_require_config_file() {
    let dir = tempfile::tempdir().unwrap();
    let config_home = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");
    let config_file = config_home.path().join("synapse").join("synapsec.toml");

    assert!(!config_file.exists());

    let out = Command::new(&syn)
        .args(["config", "path"])
        .env("XDG_CONFIG_HOME", config_home.path())
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "config path failed without config file: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("synapsec.toml"), "config path: {}", stdout);
    assert!(
        !config_file.exists(),
        "config path must not create config file"
    );

    let out = Command::new(&syn)
        .arg("init")
        .env("XDG_CONFIG_HOME", config_home.path())
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "init failed without config file: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dir.path().join("opencode.jsonc").exists());
    assert!(
        !config_file.exists(),
        "init must not create user config file"
    );
}
// END_test_clean_config_bootstrap_commands_do_not_require_config_file

#[test]
fn test_init_and_index() {
    let dir = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    // Init
    let out = Command::new(&syn)
        .arg("init")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Verify files created
    assert!(dir.path().join("opencode.jsonc").exists());
    assert!(dir.path().join(".opencode/plugins/synapse.ts").exists());

    // Create source + index
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/main.rs"),
        "// MODULE_CONTRACT\n// MODULE_ID: M-TEST\n// PURPOSE: Integration test\n// START_MODULE_MAP\n// main — entry\n// END_MODULE_MAP\n// START_CHANGE_SUMMARY\n// LAST_CHANGE: [v1.0 — test]\n// END_CHANGE_SUMMARY\n// START_CONTRACT_main\n// PURPOSE: Entry\n// START_main\nfn main() {}\n// END_main",
    ).unwrap();

    let out = Command::new(&syn)
        .arg("index")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "index failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Search
    let out = Command::new(&syn)
        .args(["search", "main"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("main"),
        "search should find main: {}",
        stdout
    );

    // Verify
    let out = Command::new(&syn)
        .arg("verify")
        .current_dir(&dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("contract-exists"),
        "verify output: {}",
        stdout
    );

    // Status
    let out = Command::new(&syn)
        .arg("status")
        .current_dir(&dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("CONTRACTS"), "status output: {}", stdout);
}

#[test]
fn test_proxy_and_gain() {
    let dir = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    // Run proxy
    let out = Command::new(&syn)
        .args(["proxy", "--", "echo", "hello"])
        .output()
        .unwrap();
    assert!(out.status.success());

    // Gain should show tracked commands
    let out = Command::new(&syn)
        .arg("gain")
        .current_dir(&dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Token") || stdout.contains("Commands"),
        "gain: {}",
        stdout
    );
}

// START_CONTRACT_test_proxy_preserves_nonzero_exit_status
// PURPOSE: Verify syn proxy exits with the wrapped command status instead of masking failures
// SIDE_EFFECTS: runs a failing shell command through syn proxy
// START_test_proxy_preserves_nonzero_exit_status
#[test]
fn test_proxy_preserves_nonzero_exit_status() {
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    let out = Command::new(&syn)
        .args(["proxy", "--", "sh", "-c", "printf failure >&2; exit 7"])
        .output()
        .unwrap();

    assert_eq!(
        out.status.code(),
        Some(7),
        "proxy must preserve wrapped command status"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(combined.contains("failure"), "proxy output: {combined}");
}
// END_test_proxy_preserves_nonzero_exit_status

// START_CONTRACT_test_proxy_unicode_passthrough_truncates_without_panic
// PURPOSE: Verify proxy passthrough truncation does not split UTF-8 characters
// SIDE_EFFECTS: runs printf through syn proxy
// START_test_proxy_unicode_passthrough_truncates_without_panic
#[test]
fn test_proxy_unicode_passthrough_truncates_without_panic() {
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");
    let payload = format!("{}😀x", "я".repeat(1999));

    let out = Command::new(&syn)
        .args(["proxy", "--", "printf", "%s"])
        .arg(&payload)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "proxy should not panic on UTF-8 truncation: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("[output truncated at 2000 chars]"),
        "proxy should report truncation: {stdout}"
    );
    assert!(
        stdout.contains('😀'),
        "proxy should keep valid UTF-8: {stdout}"
    );
}
// END_test_proxy_unicode_passthrough_truncates_without_panic

// START_CONTRACT_test_run_scenario_cli_reports_gate_and_replay_json
// PURPOSE: Verify syn run creates a bounded scenario record and emits gate/replay JSON
// SIDE_EFFECTS: initializes an isolated temp project and writes docs/runs/*.json there
// START_test_run_scenario_cli_reports_gate_and_replay_json
#[test]
fn test_run_scenario_cli_reports_gate_and_replay_json() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    let init = Command::new(&syn)
        .arg("init")
        .current_dir(&dir)
        .env("XDG_DATA_HOME", data_home.path())
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&init.stderr)
    );

    let out = Command::new(&syn)
        .args([
            "run",
            "--scenario",
            "happy",
            "--json",
            "--goal",
            "prove autonomous runtime",
            "--phase",
            "Phase-22",
            "--mod",
            "M-RUNNER",
            "--objective",
            "bounded objective to replay",
        ])
        .current_dir(&dir)
        .env("XDG_DATA_HOME", data_home.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "run scenario failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"run_id\""), "scenario json: {}", stdout);
    assert!(stdout.contains("\"decision\""), "scenario json: {}", stdout);
    assert!(stdout.contains("\"replay\""), "scenario json: {}", stdout);
    assert!(dir.path().join("docs/runs").exists());
}
// END_test_run_scenario_cli_reports_gate_and_replay_json

// START_CONTRACT_test_run_action_cli_plans_and_replays_run
// PURPOSE: Verify syn run --action can plan and replay a persisted bounded run
// SIDE_EFFECTS: initializes an isolated temp project and writes docs/runs/*.json there
// START_test_run_action_cli_plans_and_replays_run
#[test]
fn test_run_action_cli_plans_and_replays_run() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    let init = Command::new(&syn)
        .arg("init")
        .current_dir(&dir)
        .env("XDG_DATA_HOME", data_home.path())
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&init.stderr)
    );

    let scenario = Command::new(&syn)
        .args(["run", "--scenario", "happy", "--json"])
        .current_dir(&dir)
        .env("XDG_DATA_HOME", data_home.path())
        .output()
        .unwrap();
    assert!(
        scenario.status.success(),
        "scenario failed: {}",
        String::from_utf8_lossy(&scenario.stderr)
    );
    let scenario_json: serde_json::Value =
        serde_json::from_slice(&scenario.stdout).expect("scenario json");
    let run_id = scenario_json["run"]["run_id"]
        .as_str()
        .expect("run id in scenario output");

    let plan = Command::new(&syn)
        .args(["run", "--action", "plan", "--run-id", run_id, "--json"])
        .current_dir(&dir)
        .env("XDG_DATA_HOME", data_home.path())
        .output()
        .unwrap();
    assert!(
        plan.status.success(),
        "action plan failed: {}",
        String::from_utf8_lossy(&plan.stderr)
    );
    let plan_json: serde_json::Value = serde_json::from_slice(&plan.stdout).expect("plan json");
    assert_eq!(plan_json["run_id"].as_str(), Some(run_id));
    assert!(plan_json["actions"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));

    let replay = Command::new(&syn)
        .args(["run", "--action", "replay", "--run-id", run_id, "--json"])
        .current_dir(&dir)
        .env("XDG_DATA_HOME", data_home.path())
        .output()
        .unwrap();
    assert!(
        replay.status.success(),
        "action replay failed: {}",
        String::from_utf8_lossy(&replay.stderr)
    );
    let replay_json: serde_json::Value =
        serde_json::from_slice(&replay.stdout).expect("replay json");
    assert_eq!(replay_json["run_id"].as_str(), Some(run_id));
    assert!(replay_json["events"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
}
// END_test_run_action_cli_plans_and_replays_run

// START_CONTRACT_test_proxy_tracking_records_positive_savings_and_gain_graph
// PURPOSE: Verify project-local proxy filters produce tracked savings and gain --graph renders them
// SIDE_EFFECTS: creates isolated project filter and tracking DB, runs syn proxy/gain
// START_test_proxy_tracking_records_positive_savings_and_gain_graph
#[test]
fn test_proxy_tracking_records_positive_savings_and_gain_graph() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");
    let filter_dir = dir.path().join(".synapse");
    std::fs::create_dir(&filter_dir).unwrap();
    std::fs::write(
        filter_dir.join("filters.toml"),
        "[[filters]]\nmatch_command = \"printf\"\nmax_lines = 1\n",
    )
    .unwrap();
    let noisy = (1..=80).map(|i| format!("line {i}\n")).collect::<String>();

    let out = Command::new(&syn)
        .args(["proxy", "--", "printf", "%s"])
        .arg(&noisy)
        .env("XDG_DATA_HOME", data_home.path())
        .env("SYNAPSE_TRUST_PROJECT_FILTERS", "1")
        .env("SYNAPSE_SESSION_ID", "integration-route-session")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "proxy failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = Command::new(&syn)
        .args(["gain", "--graph", "--sessions", "--adapters"])
        .env("XDG_DATA_HOME", data_home.path())
        .env("SYNAPSE_SESSION_ID", "integration-route-session")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "gain --graph failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let saved = stdout
        .lines()
        .find(|line| line.contains("Tokens saved:"))
        .and_then(|line| line.split_whitespace().last())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    assert!(saved > 0, "expected positive token savings: {stdout}");
    assert!(
        stdout.contains("Savings graph:") && stdout.contains('#'),
        "gain --graph should render savings bars: {stdout}"
    );
    assert!(
        stdout.contains("Top adapters by token savings:"),
        "gain should render adapter stats: {stdout}"
    );
    assert!(
        stdout.contains("Session economics:"),
        "gain --sessions should render session stats: {stdout}"
    );
}
// END_test_proxy_tracking_records_positive_savings_and_gain_graph

// START_CONTRACT_test_proxy_project_filter_requires_trust
// PURPOSE: Verify project-local proxy filters are skipped until trusted and then applied
// SIDE_EFFECTS: creates isolated project filter and trust store, runs syn filters/proxy
// LINKS:
//   → M-PROXY-FILTER (depends) - project filter trust gate
//   ← V-M-PROXY-FILTER (verified_by) - trust-gated filter assertion
// START_test_proxy_project_filter_requires_trust
#[test]
fn test_proxy_project_filter_requires_trust() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");
    let filter_dir = dir.path().join(".synapse");
    std::fs::create_dir(&filter_dir).unwrap();
    std::fs::write(
        filter_dir.join("filters.toml"),
        "[[filters]]\nmatch_command = \"printf\"\nmax_lines = 1\n",
    )
    .unwrap();
    let noisy = (1..=5).map(|i| format!("line {i}\n")).collect::<String>();

    let untrusted = Command::new(&syn)
        .args(["proxy", "--", "printf", "%s"])
        .arg(&noisy)
        .env("XDG_DATA_HOME", data_home.path())
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        untrusted.status.success(),
        "untrusted proxy failed: {}",
        String::from_utf8_lossy(&untrusted.stderr)
    );
    let stdout = String::from_utf8_lossy(&untrusted.stdout);
    assert!(
        stdout.contains("line 5"),
        "untrusted project filter should not truncate output: {stdout}"
    );

    let trust = Command::new(&syn)
        .args(["filters", "trust"])
        .env("XDG_DATA_HOME", data_home.path())
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        trust.status.success(),
        "filters trust failed: {}",
        String::from_utf8_lossy(&trust.stderr)
    );

    let trusted = Command::new(&syn)
        .args(["proxy", "--", "printf", "%s"])
        .arg(&noisy)
        .env("XDG_DATA_HOME", data_home.path())
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        trusted.status.success(),
        "trusted proxy failed: {}",
        String::from_utf8_lossy(&trusted.stderr)
    );
    let stdout = String::from_utf8_lossy(&trusted.stdout);
    assert!(
        stdout.contains("line 1") && !stdout.contains("line 5"),
        "trusted project filter should truncate output: {stdout}"
    );
}
// END_test_proxy_project_filter_requires_trust

// START_CONTRACT_test_filters_verify_cli_runs_inline_tests
// PURPOSE: Verify syn filters verify runs RTK-style inline TOML filter tests
// SIDE_EFFECTS: creates isolated project filter and trust store, runs syn filters trust/verify
// LINKS:
//   → M-CLI-RUNTIME-COMMANDS (depends) - filter verification CLI handler
//   → M-PROXY-FILTER (depends) - inline test execution
//   ← V-M-CLI-RUNTIME-COMMANDS (verified_by) - CLI filter verification assertion
// START_test_filters_verify_cli_runs_inline_tests
#[test]
fn test_filters_verify_cli_runs_inline_tests() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");
    let filter_dir = dir.path().join(".synapse");
    std::fs::create_dir(&filter_dir).unwrap();
    std::fs::write(
        filter_dir.join("filters.toml"),
        r#"schema_version = 1

[filters.local-clean]
match_command = "^printf"
keep_lines_matching = ["^KEEP"]
max_lines = 1

[[tests.local-clean]]
name = "keeps-first-line"
input = "DROP\nKEEP one\nKEEP two"
expected = "KEEP one\n..."
"#,
    )
    .unwrap();

    let trust = Command::new(&syn)
        .args(["filters", "trust"])
        .env("XDG_DATA_HOME", data_home.path())
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        trust.status.success(),
        "filters trust failed: {}",
        String::from_utf8_lossy(&trust.stderr)
    );

    let verify = Command::new(&syn)
        .args([
            "filters",
            "verify",
            "--filter",
            "local-clean",
            "--require-all",
        ])
        .env("XDG_DATA_HOME", data_home.path())
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        verify.status.success(),
        "filters verify failed: {}\nstdout: {}",
        String::from_utf8_lossy(&verify.stderr),
        String::from_utf8_lossy(&verify.stdout)
    );
    let stdout = String::from_utf8_lossy(&verify.stdout);
    assert!(
        stdout.contains("PASS local-clean::keeps-first-line")
            && stdout.contains("Filter verification passed"),
        "verify should report inline test pass: {stdout}"
    );
}
// END_test_filters_verify_cli_runs_inline_tests

// START_CONTRACT_test_proxy_evidence_hint_writes_raw_output
// PURPOSE: Verify syn proxy --evidence writes full raw output artifact while stdout remains filtered
// SIDE_EFFECTS: creates isolated project filter, tracking/evidence data dir, and raw evidence file
// LINKS:
//   → M-PROXY-RUNNER (depends) - raw evidence artifact writer
//   → M-CLI-RUNTIME-COMMANDS (depends) - --evidence hint rendering
//   ← V-M-PROXY-RUNNER (verified_by) - evidence artifact assertion
// START_test_proxy_evidence_hint_writes_raw_output
#[test]
fn test_proxy_evidence_hint_writes_raw_output() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");
    let filter_dir = dir.path().join(".synapse");
    std::fs::create_dir(&filter_dir).unwrap();
    std::fs::write(
        filter_dir.join("filters.toml"),
        "[[filters]]\nmatch_command = \"printf\"\nmax_lines = 1\n",
    )
    .unwrap();
    let noisy = (1..=6).map(|i| format!("line {i}\n")).collect::<String>();

    let out = Command::new(&syn)
        .args(["proxy", "--evidence", "--", "printf", "%s"])
        .arg(&noisy)
        .env("XDG_DATA_HOME", data_home.path())
        .env("SYNAPSE_TRUST_PROJECT_FILTERS", "1")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "proxy --evidence failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("line 1") && !stdout.contains("line 6"),
        "stdout should stay filtered: {stdout}"
    );
    let evidence_path = stdout
        .lines()
        .find_map(|line| {
            line.strip_prefix("Raw output evidence: ")
                .and_then(|rest| rest.split_once(" ("))
                .map(|(path, _)| path.to_string())
        })
        .expect("evidence path in stdout");
    let evidence = std::fs::read_to_string(&evidence_path).unwrap();
    assert!(
        evidence.contains("line 1") && evidence.contains("line 6"),
        "evidence should contain full raw output: {evidence}"
    );
}
// END_test_proxy_evidence_hint_writes_raw_output

// START_CONTRACT_test_rtk_read_shortcut_filters_and_preserves_evidence
// PURPOSE: Verify syn read delegates to the proxy filter/evidence path as an RTK-style shortcut
// SIDE_EFFECTS: creates an isolated data dir, fixture file, and raw evidence file
// LINKS:
//   → M-CLI-RTK-COMMANDS (depends) - first-class read shortcut
//   → M-PROXY (depends) - built-in cat filter
//   → M-PROXY-RUNNER (depends) - raw evidence artifact writer
// START_test_rtk_read_shortcut_filters_and_preserves_evidence
#[test]
fn test_rtk_read_shortcut_filters_and_preserves_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");
    let fixture = dir.path().join("long.txt");
    let payload = (1..=60)
        .map(|i| format!("shortcut line {i:02}\n"))
        .collect::<String>();
    std::fs::write(&fixture, &payload).unwrap();

    let out = Command::new(&syn)
        .args(["read", "--evidence"])
        .arg(&fixture)
        .env("XDG_DATA_HOME", data_home.path())
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "syn read failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("shortcut line 01") && !stdout.contains("shortcut line 60"),
        "syn read stdout should use cat filter: {stdout}"
    );
    let evidence_path = stdout
        .lines()
        .find_map(|line| {
            line.strip_prefix("Raw output evidence: ")
                .and_then(|rest| rest.split_once(" ("))
                .map(|(path, _)| path.to_string())
        })
        .expect("evidence path in stdout");
    let evidence = std::fs::read_to_string(&evidence_path).unwrap();
    assert!(
        evidence.contains("shortcut line 01") && evidence.contains("shortcut line 60"),
        "evidence should preserve full cat output: {evidence}"
    );
}
// END_test_rtk_read_shortcut_filters_and_preserves_evidence

// START_CONTRACT_test_rewrite_cli_delegates_to_router
// PURPOSE: Verify syn rewrite prints routeable proxy commands and skips unsupported commands for thin hooks
// SIDE_EFFECTS: runs syn rewrite twice without executing rewritten commands
// LINKS:
//   → M-CLI-RTK-COMMANDS (depends) - rewrite command implementation
//   → M-PROXY-ROUTER (depends) - route decision source of truth
//   → M-HOOK-OPENCODE-REWRITE (verified_by) - hook delegation contract
// START_test_rewrite_cli_delegates_to_router
#[test]
fn test_rewrite_cli_delegates_to_router() {
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    let out = Command::new(&syn)
        .args(["rewrite", "git", "status"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "syn rewrite routeable command failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "syn proxy -- git status"
    );

    let out = Command::new(&syn)
        .args(["rewrite", "htop"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "unsupported command should skip");
    assert!(
        out.stdout.is_empty(),
        "unsupported command should not emit rewrite"
    );
}
// END_test_rewrite_cli_delegates_to_router

// START_CONTRACT_test_proxy_route_preview_for_rtk_like_command
// PURPOSE: Verify syn proxy --route reports an RTK-style adapter decision without executing the command
// SIDE_EFFECTS: runs syn CLI route preview
// LINKS:
//   → M-PROXY-ROUTER (depends) - route preview command classification
//   ← V-M-PROXY-ROUTER (verified_by) - integration route preview assertion
// START_test_proxy_route_preview_for_rtk_like_command
#[test]
fn test_proxy_route_preview_for_rtk_like_command() {
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    let out = Command::new(&syn)
        .args(["proxy", "--route", "--", "python", "-m", "pytest"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "route preview failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("adapter:      python-pytest"),
        "route preview should report pytest adapter: {stdout}"
    );
    assert!(
        stdout.contains("route_key:    python -m pytest"),
        "route preview should report stable route key: {stdout}"
    );
}
// END_test_proxy_route_preview_for_rtk_like_command

// START_CONTRACT_test_tracking_is_scoped_by_canonical_project_path
// PURPOSE: Verify tracking stats do not mix projects that share the same directory basename
// SIDE_EFFECTS: creates isolated temp project and XDG data directories, runs syn proxy/gain
// START_test_tracking_is_scoped_by_canonical_project_path
#[test]
fn test_tracking_is_scoped_by_canonical_project_path() {
    let parent_one = tempfile::tempdir().unwrap();
    let parent_two = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let project_one = parent_one.path().join("app");
    let project_two = parent_two.path().join("app");
    std::fs::create_dir(&project_one).unwrap();
    std::fs::create_dir(&project_two).unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    for project in [&project_one, &project_two] {
        let out = Command::new(&syn)
            .args(["proxy", "--", "echo", "tracked"])
            .env("XDG_DATA_HOME", data_home.path())
            .current_dir(project)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "proxy failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let out = Command::new(&syn)
        .arg("gain")
        .env("XDG_DATA_HOME", data_home.path())
        .current_dir(&project_one)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "gain failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Commands tracked:    1"),
        "tracking should be scoped by canonical path: {}",
        stdout
    );
}
// END_test_tracking_is_scoped_by_canonical_project_path

#[test]
fn test_doctor_and_hooks() {
    let dir = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    Command::new(&syn)
        .arg("init")
        .current_dir(&dir)
        .output()
        .unwrap();

    // Doctor
    let out = Command::new(&syn)
        .arg("doctor")
        .current_dir(&dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("PASS") || stdout.contains("FAIL"),
        "doctor: {}",
        stdout
    );

    // Hooks status
    let out = Command::new(&syn)
        .args(["hooks", "status"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn test_skills_cli() {
    let dir = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    let out = Command::new(&syn)
        .arg("init")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success());

    let out = Command::new(&syn)
        .args(["skills", "list"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("grace_init"), "skills list: {}", stdout);

    let out = Command::new(&syn)
        .args(["skills", "show", "grace_status"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("grace_status"), "skills show: {}", stdout);

    let out = Command::new(&syn)
        .args(["skills", "run", "grace_status", "detail_level=summary"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("grace_status"), "skills run: {}", stdout);
}

#[test]
fn test_scoped_and_json_cli() {
    let dir = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    let out = Command::new(&syn)
        .arg("init")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success());

    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/main.rs"),
        "// MODULE_CONTRACT\n// MODULE_ID: M-TEST\n// PURPOSE: Integration test\n// START_MODULE_MAP\n// main — entry\n// END_MODULE_MAP\n// START_CHANGE_SUMMARY\n// LAST_CHANGE: [v1.0 — test]\n// END_CHANGE_SUMMARY\n// START_CONTRACT_main\n// PURPOSE: Entry\n// START_main\nfn main() {}\n// END_main",
    ).unwrap();

    let out = Command::new(&syn)
        .arg("index")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "index failed in scoped test");

    let out = Command::new(&syn)
        .args(["refresh", "--fix"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "refresh --fix failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = Command::new(&syn)
        .args(["verify", "--json", "--mod", "M-TEST"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "verify --json failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.trim().is_empty(), "verify --json empty");

    let out = Command::new(&syn)
        .args(["status", "--json", "--mod", "M-TEST"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "status --json failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.trim().is_empty(), "status --json empty");
}

#[test]
fn test_init_from_existing() {
    let dir = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn hello() {}\n").unwrap();

    let out = Command::new(&syn)
        .args(["init", "--from-existing"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "init --from-existing failed");
    assert!(dir.path().join("docs/graph-index.xml").exists());
    assert!(dir.path().join("docs/plan-index.xml").exists());
    assert!(dir.path().join("docs/verification-index.xml").exists());
    let graph = std::fs::read_to_string(dir.path().join("docs/graph-index.xml")).unwrap();
    assert!(
        !graph.contains("M-MOD"),
        "from-existing created duplicate-prone M-MOD"
    );
    assert!(
        graph.contains("docs/modules/M-LIB.xml"),
        "from-existing should point graph entries at module shards: {}",
        graph
    );
}

#[test]
fn test_ci_commands() {
    let dir = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    let out = Command::new(&syn)
        .arg("init")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success());

    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/main.rs"),
        "// MODULE_CONTRACT\n// MODULE_ID: M-TEST\n// PURPOSE: Integration test\n// START_MODULE_MAP\n// main — entry\n// END_MODULE_MAP\n// START_CHANGE_SUMMARY\n// LAST_CHANGE: [v1.0 — test]\n// END_CHANGE_SUMMARY\n// START_CONTRACT_main\n// PURPOSE: Entry\n// START_main\nfn main() {}\n// END_main",
    ).unwrap();

    let out = Command::new(&syn)
        .args(["refresh", "--fix"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "refresh --fix failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = Command::new(&syn)
        .args(["ci", "verify", "--json"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "ci verify failed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("module-local") || stdout.contains("phase"),
        "ci verify: {}",
        stdout
    );
}

#[test]
fn test_compress_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    std::fs::write(dir.path().join("test.md"), "I think this is a very important document. Furthermore, it should be noted that this contains many filler words. However, we can compress it. Therefore, let's do that.").unwrap();

    let out = Command::new(&syn)
        .args(["compress", "test.md"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(dir.path().join("test.original.md").exists());

    let compressed = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
    assert!(
        compressed.len() < 200,
        "should be compressed: {}",
        compressed.len()
    );

    let out = Command::new(&syn)
        .args(["compress", "test.md", "--restore"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    let restored = std::fs::read_to_string(dir.path().join("test.md")).unwrap();
    assert!(restored.contains("Furthermore"), "should be restored");
}
