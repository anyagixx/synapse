// MODULE_CONTRACT
// MODULE_ID: M-TESTS-RELIABILITY
// PURPOSE: Runtime reliability integration tests for degraded storage behavior
// SCOPE: corrupted index visibility through search errors and doctor diagnostics
// DEPENDS: M-CLI, M-INDEXER, M-INDEXER-STORAGE, M-CLI-RUNTIME-COMMANDS
// LINKS: docs/phases/Phase-3.xml

// START_MODULE_MAP
// test_corrupted_index_is_reported — Verifies corrupted blocks.json is visible to users
// syn_bin — Resolves the compiled syn binary path
// run_syn — Runs syn in an isolated project and platform data home
// write_contract_source — Writes a minimal contract-bearing Rust source file
// corrupt_first_blocks_json — Replaces generated index storage with invalid JSON
// find_first_blocks_json — Finds generated block storage under the isolated data home
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 — Made corrupted index test portable across Linux/macOS]
// END_CHANGE_SUMMARY

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

// START_CONTRACT_test_corrupted_index_is_reported
// PURPOSE: Verify corrupted JSON index storage is visible through search errors and doctor diagnostics
// SIDE_EFFECTS: creates isolated temp project and XDG data directory, corrupts blocks.json
// START_test_corrupted_index_is_reported
#[test]
fn test_corrupted_index_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    write_contract_source(dir.path());

    let out = run_syn(["index"], dir.path(), data_home.path());
    assert!(
        out.status.success(),
        "index failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    corrupt_first_blocks_json(data_home.path());

    let out = run_syn(["search", "main"], dir.path(), data_home.path());
    assert!(
        !out.status.success(),
        "search should fail visibly for corrupted index"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("index storage is unreadable") && stderr.contains("syn index"),
        "search corruption error should be actionable: {}",
        stderr
    );

    let out = run_syn(["doctor"], dir.path(), data_home.path());
    assert!(
        out.status.success(),
        "doctor failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("index storage unreadable") && stdout.contains("syn index"),
        "doctor should report corrupted index: {}",
        stdout
    );
}
// END_test_corrupted_index_is_reported

// START_CONTRACT_syn_bin
// PURPOSE: Resolve the compiled syn binary path used by integration tests
// OUTPUTS: { PathBuf }
// START_syn_bin
fn syn_bin() -> PathBuf {
    let exe = if cfg!(windows) { "syn.exe" } else { "syn" };
    std::env::current_dir()
        .unwrap()
        .join("target")
        .join("debug")
        .join(exe)
}
// END_syn_bin

// START_CONTRACT_run_syn
// PURPOSE: Run syn with an isolated project root and platform data home
// INPUTS: { args: impl IntoIterator<Item = &'static str> }, { cwd: &Path }, { data_home: &Path }
// OUTPUTS: { Output }
// SIDE_EFFECTS: executes the compiled syn binary
// START_run_syn
fn run_syn<const N: usize>(args: [&str; N], cwd: &Path, data_home: &Path) -> Output {
    Command::new(syn_bin())
        .args(args)
        .env("XDG_DATA_HOME", data_home)
        .env("HOME", data_home)
        .env("APPDATA", data_home)
        .env("LOCALAPPDATA", data_home)
        .current_dir(cwd)
        .output()
        .unwrap()
}
// END_run_syn

// START_CONTRACT_write_contract_source
// PURPOSE: Create a minimal source file that the indexer can parse
// INPUTS: { root: &Path }
// SIDE_EFFECTS: writes src/main.rs
// START_write_contract_source
fn write_contract_source(root: &Path) {
    let src_dir = root.join("src");
    std::fs::create_dir(&src_dir).unwrap();
    let source = "// MODULE_CONTRACT\n// MODULE_ID: M-CORRUPT\n// PURPOSE: Corruption regression\n// START_MODULE_MAP\n// main — entry\n// END_MODULE_MAP\n// START_CHANGE_SUMMARY\n// LAST_CHANGE: [v1.0 — test]\n// END_CHANGE_SUMMARY\n// START_CONTRACT_main\n// PURPOSE: Entry\n// START_main\nfn main() {}\n// END_main";
    std::fs::write(src_dir.join("main.rs"), source).unwrap();
}
// END_write_contract_source

// START_CONTRACT_corrupt_first_blocks_json
// PURPOSE: Replace the first generated index file with invalid JSON
// INPUTS: { data_home: &Path }
// SIDE_EFFECTS: writes invalid JSON to blocks.json
// START_corrupt_first_blocks_json
fn corrupt_first_blocks_json(data_home: &Path) {
    let blocks_path =
        find_first_blocks_json(data_home).expect("blocks.json should exist after indexing");
    std::fs::write(blocks_path, "{ invalid json").unwrap();
}
// END_corrupt_first_blocks_json

// START_CONTRACT_find_first_blocks_json
// PURPOSE: Find generated block storage without assuming Linux-only data directory layout
// INPUTS: { root: &Path }
// OUTPUTS: { Option<PathBuf> }
// SIDE_EFFECTS: reads directory entries under root
// START_find_first_blocks_json
fn find_first_blocks_json(root: &Path) -> Option<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str()) == Some("blocks.json") {
                return Some(path);
            }
            if path.is_dir() {
                pending.push(path);
            }
        }
    }
    None
}
// END_find_first_blocks_json
