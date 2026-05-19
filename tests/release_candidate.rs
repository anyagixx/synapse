// MODULE_CONTRACT
// MODULE_ID: M-TESTS-PARITY
// PURPOSE: Ensure release-candidate automation remains a dry-run truth gate before publishing.
// SCOPE: Validate RC workflow permissions, release metadata checks, installer matrix smoke, and CI integration.
// DEPENDS: M-CI, M-INSTALL, M-CI-RELEASE-SMOKE
// LINKS: scripts/release_candidate_dry_run.sh, .github/workflows/release-candidate.yml

// START_MODULE_MAP
// test_release_candidate_workflow_is_dry_run_only - Workflow must not publish releases
// test_release_candidate_script_checks_release_truth - Script must validate tag, notes, checksums, and installer mapping
// test_ci_invokes_release_candidate_gate - Local CI must include the lightweight RC policy gate
// test_release_candidate_script_executes_without_publishing - Script dry-run must pass with the package tag
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added Phase 9 release candidate automation parity tests]
// END_CHANGE_SUMMARY

const RELEASE_CANDIDATE_WORKFLOW: &str = include_str!("../.github/workflows/release-candidate.yml");
const RELEASE_WORKFLOW: &str = include_str!("../.github/workflows/release.yml");
const CI_SCRIPT: &str = include_str!("../scripts/ci.sh");
const RELEASE_CANDIDATE_SCRIPT: &str = include_str!("../scripts/release_candidate_dry_run.sh");

#[test]
// START_CONTRACT_test_release_candidate_workflow_is_dry_run_only
// PURPOSE: Verify the release-candidate workflow can validate a candidate without publishing artifacts.
fn test_release_candidate_workflow_is_dry_run_only() {
    assert!(
        RELEASE_CANDIDATE_WORKFLOW.contains("workflow_dispatch"),
        "release candidate workflow must be manually runnable"
    );
    assert!(
        RELEASE_CANDIDATE_WORKFLOW.contains("contents: read"),
        "release candidate workflow must not request write permissions"
    );
    assert!(
        !RELEASE_CANDIDATE_WORKFLOW.contains("contents: write"),
        "release candidate workflow must not publish releases"
    );
    for publishing_marker in ["action-gh-release", "gh release create", "draft: false"] {
        assert!(
            !RELEASE_CANDIDATE_WORKFLOW.contains(publishing_marker),
            "release candidate workflow must not contain publishing marker {publishing_marker}"
        );
    }
}

#[test]
// START_CONTRACT_test_release_candidate_script_checks_release_truth
// PURPOSE: Verify the release-candidate gate covers tag, notes, checksums, and Linux/macOS install truth.
fn test_release_candidate_script_checks_release_truth() {
    assert!(
        RELEASE_CANDIDATE_SCRIPT.contains("bash scripts/release_version_guard.sh"),
        "release candidate script must run the tag/version guard"
    );
    assert!(
        RELEASE_WORKFLOW.contains("bash scripts/release_candidate_dry_run.sh"),
        "release workflow must run the candidate dry-run before publishing"
    );
    for marker in [
        "generate_release_notes: true",
        "sort -k2 > dist/SHA256SUMS",
        "sha256sum -c SHA256SUMS",
        "SYN_INSTALL_UNAME_S",
        "scripts/release_install_smoke.sh",
    ] {
        assert!(
            RELEASE_CANDIDATE_SCRIPT.contains(marker),
            "release candidate script must validate {marker}"
        );
    }
    for artifact in [
        "syn-x86_64-unknown-linux-gnu.tar.gz",
        "syn-aarch64-unknown-linux-gnu.tar.gz",
        "syn-x86_64-apple-darwin.tar.gz",
        "syn-aarch64-apple-darwin.tar.gz",
    ] {
        assert!(
            RELEASE_CANDIDATE_SCRIPT.contains(artifact)
                && RELEASE_CANDIDATE_WORKFLOW.contains(artifact)
                && RELEASE_WORKFLOW.contains(artifact),
            "release candidate, release workflow, and publishing workflow must agree on {artifact}"
        );
    }
}

#[test]
// START_CONTRACT_test_ci_invokes_release_candidate_gate
// PURPOSE: Verify local CI includes release-candidate validation without duplicating matrix smoke.
fn test_ci_invokes_release_candidate_gate() {
    assert!(
        CI_SCRIPT.contains("SYN_RC_SKIP_SMOKE=1 bash scripts/release_candidate_dry_run.sh"),
        "scripts/ci.sh must run the lightweight release candidate gate"
    );
    assert!(
        CI_SCRIPT.contains("[CI][run_ci_gate][RELEASE_CANDIDATE]"),
        "scripts/ci.sh must emit a stable release-candidate log marker"
    );
}

#[test]
// START_CONTRACT_test_release_candidate_script_executes_without_publishing
// PURPOSE: Verify the dry-run script passes with the package tag and skip-smoke mode.
fn test_release_candidate_script_executes_without_publishing() {
    let expected_tag = format!("v{}", env!("CARGO_PKG_VERSION"));
    let output = std::process::Command::new("bash")
        .arg("scripts/release_candidate_dry_run.sh")
        .env("SYN_RELEASE_TAG", expected_tag)
        .env("SYN_RC_SKIP_SMOKE", "1")
        .output()
        .expect("release candidate script should execute");

    assert!(
        output.status.success(),
        "release candidate script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("[CI][release_candidate][PASS] Release candidate dry-run passed"),
        "release candidate script must emit pass marker"
    );
}
