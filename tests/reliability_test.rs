// MODULE_CONTRACT
// MODULE_ID: M-TESTS-RELIABILITY
// PURPOSE: Runtime reliability integration tests for degraded storage, index lifecycle, bootstrap diagnostics, and config merge behavior
// SCOPE: corrupted index visibility, stale index pruning, gitignore toggle, clean doctor defaults, OpenCode config merge
// DEPENDS: M-CLI, M-INDEXER, M-INDEXER-STORAGE, M-CLI-RUNTIME-COMMANDS, M-HOOKS
// LINKS: docs/phases/Phase-7.xml

// START_MODULE_MAP
// test_corrupted_index_is_reported — Verifies corrupted blocks.json is visible to users
// test_index_rebuild_removes_deleted_files — Verifies deleted files disappear from search after re-index
// test_index_no_git_indexes_gitignored_files — Verifies --no-git indexes gitignored files
// test_clean_init_doctor_accepts_default_config — Verifies doctor accepts missing user config defaults
// test_init_merges_existing_opencode_jsonc — Verifies syn init preserves existing OpenCode config siblings
// syn_bin — Resolves the compiled syn binary path
// run_syn — Runs syn in an isolated project and platform data home
// run_syn_with_config — Runs syn with isolated project, data home, and config home
// write_contract_source — Writes a minimal contract-bearing Rust source file
// corrupt_first_blocks_json — Replaces generated index storage with invalid JSON
// find_first_blocks_json — Finds generated block storage under the isolated data home
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.2.0 — Added Phase 7 protocol-adjacent bootstrap and index lifecycle regressions]
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

// START_CONTRACT_test_index_rebuild_removes_deleted_files
// PURPOSE: Verify a full re-index removes stale search entries for files deleted from disk
// SIDE_EFFECTS: creates isolated temp project and index storage
// START_test_index_rebuild_removes_deleted_files
#[test]
fn test_index_rebuild_removes_deleted_files() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        "pub fn phase7_stale_marker() -> bool { true }\n",
    )
    .unwrap();

    let out = run_syn(["index"], dir.path(), data_home.path());
    assert!(
        out.status.success(),
        "initial index failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = run_syn(
        ["search", "phase7_stale_marker"],
        dir.path(),
        data_home.path(),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("phase7_stale_marker"), "search: {}", stdout);

    std::fs::remove_file(src_dir.join("lib.rs")).unwrap();
    let out = run_syn(["index"], dir.path(), data_home.path());
    assert!(
        out.status.success(),
        "second index failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = run_syn(
        ["search", "phase7_stale_marker"],
        dir.path(),
        data_home.path(),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No results"),
        "deleted file must not remain searchable: {}",
        stdout
    );
}
// END_test_index_rebuild_removes_deleted_files

// START_CONTRACT_test_index_no_git_indexes_gitignored_files
// PURPOSE: Verify syn index --no-git disables .gitignore filtering for source discovery
// SIDE_EFFECTS: creates isolated temp project and index storage
// START_test_index_no_git_indexes_gitignored_files
#[test]
fn test_index_no_git_indexes_gitignored_files() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    std::fs::write(dir.path().join(".gitignore"), "src/ignored.rs\n").unwrap();
    std::fs::write(
        src_dir.join("ignored.rs"),
        "pub fn phase7_no_git_marker() -> bool { true }\n",
    )
    .unwrap();

    let out = run_syn(["index"], dir.path(), data_home.path());
    assert!(
        out.status.success(),
        "default index failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = run_syn(
        ["search", "phase7_no_git_marker"],
        dir.path(),
        data_home.path(),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No results"),
        "default index should respect .gitignore: {}",
        stdout
    );

    let out = run_syn(["index", "--no-git"], dir.path(), data_home.path());
    assert!(
        out.status.success(),
        "index --no-git failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = run_syn(
        ["search", "phase7_no_git_marker"],
        dir.path(),
        data_home.path(),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("phase7_no_git_marker"),
        "--no-git should index gitignored source files: {}",
        stdout
    );
}
// END_test_index_no_git_indexes_gitignored_files

// START_CONTRACT_test_clean_init_doctor_accepts_default_config
// PURPOSE: Verify syn init followed by syn doctor is clean when no user config file exists
// SIDE_EFFECTS: creates isolated temp project, config, and data directories
// START_test_clean_init_doctor_accepts_default_config
#[test]
fn test_clean_init_doctor_accepts_default_config() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let config_home = tempfile::tempdir().unwrap();

    let out = run_syn_with_config(["init"], dir.path(), data_home.path(), config_home.path());
    assert!(
        out.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = run_syn_with_config(["doctor"], dir.path(), data_home.path(), config_home.path());
    assert!(
        out.status.success(),
        "doctor failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("PASS config"),
        "doctor should pass default config state: {}",
        stdout
    );
    assert!(
        !stdout.contains("FAIL config"),
        "doctor must not fail config after clean init: {}",
        stdout
    );
}
// END_test_clean_init_doctor_accepts_default_config

// START_CONTRACT_test_init_merges_existing_opencode_jsonc
// PURPOSE: Verify syn init adds mcp.synapse without dropping existing OpenCode config keys
// SIDE_EFFECTS: creates isolated temp project with existing opencode.jsonc
// START_test_init_merges_existing_opencode_jsonc
#[test]
fn test_init_merges_existing_opencode_jsonc() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let config_home = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("opencode.jsonc"),
        r#"{
          // user-owned config
          "mcp": {
            "other": {"type": "local", "command": ["other"], "enabled": true,},
          },
          "theme": "dark",
        }"#,
    )
    .unwrap();

    let out = run_syn_with_config(["init"], dir.path(), data_home.path(), config_home.path());
    assert!(
        out.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let merged = std::fs::read_to_string(dir.path().join("opencode.jsonc")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&merged).expect("merged config JSON");
    assert!(value["mcp"]["other"].is_object());
    assert_eq!(value["mcp"]["synapse"]["command"][0], "syn");
    assert_eq!(value["theme"], "dark");
}
// END_test_init_merges_existing_opencode_jsonc

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

// START_CONTRACT_run_syn_with_config
// PURPOSE: Run syn with isolated project root, platform data home, and platform config home
// INPUTS: { args: impl IntoIterator<Item = &'static str> }, { cwd: &Path }, { data_home: &Path }, { config_home: &Path }
// OUTPUTS: { Output }
// SIDE_EFFECTS: executes the compiled syn binary
// START_run_syn_with_config
fn run_syn_with_config<const N: usize>(
    args: [&str; N],
    cwd: &Path,
    data_home: &Path,
    config_home: &Path,
) -> Output {
    Command::new(syn_bin())
        .args(args)
        .env("XDG_DATA_HOME", data_home)
        .env("XDG_CONFIG_HOME", config_home)
        .env("HOME", data_home)
        .env("APPDATA", data_home)
        .env("LOCALAPPDATA", data_home)
        .current_dir(cwd)
        .output()
        .unwrap()
}
// END_run_syn_with_config

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
