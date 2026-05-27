// MODULE_CONTRACT
// MODULE_ID: M-TESTS-INTEGRATION
// PURPOSE: Integration smoke tests for Synapse multi-project workspace CLI commands.
// SCOPE: workspace init, list, and sequential verify against an existing MyGRACE member project.
// DEPENDS: M-CLI, M-WORKSPACE, M-GRACE-VERIFY
// LINKS:
//   -> Phase-91 (implements) - workspace CLI integration coverage
//   <- V-M-TESTS-INTEGRATION (verified_by) - workspace command smoke

// START_MODULE_MAP
// test_workspace_cli_init_list_and_verify - Verifies workspace CLI command flow
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added Phase-91 workspace CLI smoke test]
// END_CHANGE_SUMMARY

use std::process::Command;

// START_CONTRACT_test_workspace_cli_init_list_and_verify
// PURPOSE: Verify workspace init/list/verify route through the Synapse binary.
// SIDE_EFFECTS: creates an isolated workspace root with Synapse.toml
// LINKS:
//   -> Phase-91 (implements) - workspace CLI
//   -> NFR-002 (traces_to) - reliable release verification commands
//   <- V-M-TESTS-INTEGRATION (verified_by) - workspace CLI smoke
// START_test_workspace_cli_init_list_and_verify
#[test]
fn test_workspace_cli_init_list_and_verify() {
    let workspace_root = tempfile::tempdir().unwrap();
    let project_root = std::env::current_dir().unwrap();
    let syn = project_root.join("target/debug/syn");
    let member = project_root.display().to_string();

    let init = Command::new(&syn)
        .args(["workspace", "init", &member])
        .current_dir(&workspace_root)
        .output()
        .unwrap();
    assert!(init.status.success());
    assert!(workspace_root.path().join("Synapse.toml").exists());

    let list = Command::new(&syn)
        .args(["workspace", "list"])
        .current_dir(&workspace_root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(list.status.success());
    assert!(stdout.contains("Members (1)"));

    let verify = Command::new(&syn)
        .args(["workspace", "verify", "--sequential", "--profile", "strict"])
        .current_dir(&workspace_root)
        .output()
        .unwrap();
    assert!(
        verify.status.success(),
        "workspace verify failed: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
}
// END_test_workspace_cli_init_list_and_verify
