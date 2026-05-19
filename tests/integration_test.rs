// MODULE_CONTRACT
// MODULE_ID: M-TESTS-INTEGRATION
// PURPOSE: End-to-end integration tests for Synapse CLI commands
// SCOPE: init, index, search, verify, status, proxy, gain, doctor, hooks, compress
// DEPENDS: M-CLI, M-INDEXER, M-GRACE

use std::process::Command;

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
