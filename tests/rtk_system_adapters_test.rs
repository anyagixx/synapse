// MODULE_CONTRACT
// MODULE_ID: M-TESTS-INTEGRATION
// PURPOSE: End-to-end integration coverage for local RTK system adapters
// SCOPE: syn pipe, syn log, and syn smart CLI behavior without external command dependencies
// DEPENDS: M-CLI, M-CLI-RTK-COMMANDS, M-TRACKING
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - local RTK system adapter implementation
//   -> M-TRACKING (depends) - adapter-level savings recording
//   <- V-M-TESTS-INTEGRATION (verified_by) - integration coverage

// START_MODULE_MAP
// test_rtk_system_adapters_compact_outputs - Verifies pipe/log/smart local system adapters
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added RTK system adapter integration coverage]
// END_CHANGE_SUMMARY

use std::io::Write as _;
use std::process::{Command, Stdio};

// START_CONTRACT_test_rtk_system_adapters_compact_outputs
// PURPOSE: Verify local RTK system adapters compact stdin, log files, and source files without external command dependencies
// SIDE_EFFECTS: creates isolated fixture files and tracking data dir
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - pipe/log/smart adapters
//   -> M-TRACKING (depends) - adapter-level token economy recording
// START_test_rtk_system_adapters_compact_outputs
#[test]
fn test_rtk_system_adapters_compact_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let syn = std::env::current_dir().unwrap().join("target/debug/syn");

    let mut pipe = Command::new(&syn)
        .args(["pipe", "--filter", "make"])
        .env("XDG_DATA_HOME", data_home.path())
        .current_dir(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    pipe.stdin
        .as_mut()
        .unwrap()
        .write_all(b"make[1]: Entering directory '/tmp'\nmake[1]: Leaving directory '/tmp'\n")
        .unwrap();
    let pipe = pipe.wait_with_output().unwrap();
    assert!(
        pipe.status.success(),
        "syn pipe failed: {}",
        String::from_utf8_lossy(&pipe.stderr)
    );
    let stdout = String::from_utf8_lossy(&pipe.stdout);
    assert_eq!(stdout.trim(), "make: ok");

    let log_path = dir.path().join("app.log");
    std::fs::write(
        &log_path,
        "2026-01-01 10:00:00 ERROR failed /tmp/a id=12345\n2026-01-01 10:00:01 ERROR failed /tmp/b id=67890\n",
    )
    .unwrap();
    let log = Command::new(&syn)
        .arg("log")
        .arg(&log_path)
        .env("XDG_DATA_HOME", data_home.path())
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        log.status.success(),
        "syn log failed: {}",
        String::from_utf8_lossy(&log.stderr)
    );
    let stdout = String::from_utf8_lossy(&log.stdout);
    assert!(
        stdout.contains("errors: 2 unique: 1"),
        "log output: {stdout}"
    );

    let source_path = dir.path().join("sample.rs");
    std::fs::write(
        &source_path,
        "use std::fs;\npub struct Config { value: String }\npub async fn run() {}\n",
    )
    .unwrap();
    let smart = Command::new(&syn)
        .arg("smart")
        .arg(&source_path)
        .env("XDG_DATA_HOME", data_home.path())
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        smart.status.success(),
        "syn smart failed: {}",
        String::from_utf8_lossy(&smart.stderr)
    );
    let stdout = String::from_utf8_lossy(&smart.stdout);
    assert!(stdout.contains("language: Rust"), "smart output: {stdout}");
    assert!(stdout.contains("struct Config"), "smart output: {stdout}");
    assert!(
        !stdout.contains("value: String"),
        "smart output leaked body detail: {stdout}"
    );
}
// END_test_rtk_system_adapters_compact_outputs
