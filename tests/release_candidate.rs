// MODULE_CONTRACT
// MODULE_ID: M-TESTS-PARITY
// PURPOSE: Ensure release-candidate automation remains a dry-run truth gate before publishing.
// SCOPE: Validate RC workflow permissions, candidate ref checkout, release metadata checks, release-context freshness policy, full RTK gate policy, fresh install smoke, installer matrix smoke, GitHub step-summary evidence, and CI integration.
// DEPENDS: M-CI, M-INSTALL, M-CI-RELEASE-SMOKE, M-RTK-FULL-PARITY
// LINKS: scripts/release_candidate_dry_run.sh, scripts/rtk_full_release_gate.sh, .github/workflows/release-candidate.yml

// START_MODULE_MAP
// test_release_candidate_workflow_is_dry_run_only - Workflow must not publish releases
// test_release_candidate_checks_out_candidate_ref - Workflow must checkout the requested candidate tag
// test_release_candidate_script_checks_release_truth - Script must validate tag, freshness, notes, checksums, installer mapping, and full RTK policy
// test_release_candidate_fresh_install_evidence - Workflow must run public installer fresh smoke on Linux/macOS
// test_ci_invokes_release_candidate_gate - Local CI must include the lightweight RC policy gate
// test_release_candidate_script_executes_without_publishing - Script dry-run must pass with the package tag
// test_release_candidate_step_summary_is_written - Script must write maintainer-readable evidence summary
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.7.0 - Restored Intel macOS release-candidate artifact assertions]
// END_CHANGE_SUMMARY

const RELEASE_CANDIDATE_WORKFLOW: &str = include_str!("../.github/workflows/release-candidate.yml");
const RELEASE_WORKFLOW: &str = include_str!("../.github/workflows/release.yml");
const CI_SCRIPT: &str = include_str!("../scripts/ci.sh");
const RELEASE_CANDIDATE_SCRIPT: &str = include_str!("../scripts/release_candidate_dry_run.sh");
const RTK_FULL_RELEASE_GATE_SCRIPT: &str = include_str!("../scripts/rtk_full_release_gate.sh");
const FRESH_INSTALL_SCRIPT: &str = include_str!("../scripts/fresh_install_smoke.sh");

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
    assert!(
        RELEASE_CANDIDATE_WORKFLOW.contains("Summarize smoke evidence")
            && RELEASE_CANDIDATE_WORKFLOW.contains("$GITHUB_STEP_SUMMARY"),
        "release candidate workflow must expose smoke evidence in GitHub Step Summary"
    );
    for publishing_marker in ["action-gh-release", "gh release create", "draft: false"] {
        assert!(
            !RELEASE_CANDIDATE_WORKFLOW.contains(publishing_marker),
            "release candidate workflow must not contain publishing marker {publishing_marker}"
        );
    }
}

#[test]
// START_CONTRACT_test_release_candidate_checks_out_candidate_ref
// PURPOSE: Verify release-candidate workflow validates the requested candidate ref, not the default branch.
fn test_release_candidate_checks_out_candidate_ref() {
    assert!(
        RELEASE_CANDIDATE_WORKFLOW.contains("ref: ${{ inputs.version_tag }}"),
        "release candidate workflow must checkout the requested candidate tag"
    );
    assert!(
        RELEASE_CANDIDATE_WORKFLOW.contains("git rev-parse HEAD")
            && RELEASE_CANDIDATE_SCRIPT.contains("candidate_commit="),
        "release candidate evidence must include the checked-out commit SHA"
    );
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
        "scripts/release_freshness_guard.sh",
        "[CI][release_candidate][FRESHNESS]",
        "SYN_RC_SKIP_FRESHNESS",
        "sort -k2 > dist/SHA256SUMS",
        "sha256sum -c SHA256SUMS",
        "SYN_INSTALL_UNAME_S",
        "scripts/release_install_smoke.sh",
        "scripts/rtk_full_release_gate.sh",
        "[CI][release_candidate][RTK_FULL]",
        "SYN_RC_SKIP_FULL_RTK",
        "GITHUB_STEP_SUMMARY",
        "[CI][release_candidate][SUMMARY]",
    ] {
        assert!(
            RELEASE_CANDIDATE_SCRIPT.contains(marker),
            "release candidate script must validate {marker}"
        );
    }
    for marker in [
        "SYNAPSE_RTK_SOURCE_REF",
        "https://github.com/rtk-ai/rtk",
        "v0.34.3",
        "git clone --depth 1 --branch",
    ] {
        assert!(
            RTK_FULL_RELEASE_GATE_SCRIPT.contains(marker),
            "full RTK release gate must support hosted source fallback marker {marker}"
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
    assert!(
        !RELEASE_CANDIDATE_WORKFLOW.contains("SYN_RC_SKIP_FULL_RTK")
            && !RELEASE_WORKFLOW.contains("SYN_RC_SKIP_FULL_RTK"),
        "release workflows must not skip the full RTK gate"
    );
    assert!(
        !RELEASE_CANDIDATE_WORKFLOW.contains("SYN_RC_SKIP_FRESHNESS")
            && !RELEASE_WORKFLOW.contains("SYN_RC_SKIP_FRESHNESS"),
        "release workflows must not skip the freshness guard"
    );
}

#[test]
// START_CONTRACT_test_release_candidate_fresh_install_evidence
// PURPOSE: Verify release-candidate workflow captures public installer fresh-machine evidence for Linux/macOS.
fn test_release_candidate_fresh_install_evidence() {
    for marker in [
        "fresh-install:",
        "bash scripts/fresh_install_smoke.sh",
        "SYN_INSTALL_SCRIPT_URL",
        "raw.githubusercontent.com/anyagixx/synapse/${{ inputs.version_tag }}/install.sh",
        "fetch-depth: 0",
        "ubuntu-latest",
        "macos-latest",
        "Fresh install smoke",
        "Install dir: temporary",
    ] {
        assert!(
            RELEASE_CANDIDATE_WORKFLOW.contains(marker),
            "release candidate workflow must declare fresh install marker {marker}"
        );
    }
    for marker in [
        "curl -fsSL",
        "SYN_INSTALL_DIR",
        "syn --version",
        "syn --help",
        "[CI][fresh_install_smoke][PASS]",
    ] {
        assert!(
            FRESH_INSTALL_SCRIPT.contains(marker),
            "fresh install script must validate {marker}"
        );
    }
}

#[test]
// START_CONTRACT_test_ci_invokes_release_candidate_gate
// PURPOSE: Verify local CI includes release-candidate validation without duplicating matrix smoke.
fn test_ci_invokes_release_candidate_gate() {
    assert!(
        CI_SCRIPT.contains("SYN_RC_SKIP_SMOKE=1 SYN_RC_SKIP_FULL_RTK=1 SYN_RC_SKIP_FRESHNESS=1 bash scripts/release_candidate_dry_run.sh"),
        "scripts/ci.sh must run the lightweight release candidate gate"
    );
    assert!(
        CI_SCRIPT.contains("[CI][run_ci_gate][RELEASE_CANDIDATE]"),
        "scripts/ci.sh must emit a stable release-candidate log marker"
    );
    assert!(
        CI_SCRIPT.contains("[CI][run_ci_gate][RTK_FULL]"),
        "scripts/ci.sh must emit a stable full RTK gate marker"
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
        .env("SYN_RC_SKIP_FULL_RTK", "1")
        .env("SYN_RC_SKIP_FRESHNESS", "1")
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

#[test]
// START_CONTRACT_test_release_candidate_step_summary_is_written
// PURPOSE: Verify the dry-run script writes a maintainer-readable GitHub Step Summary.
fn test_release_candidate_step_summary_is_written() {
    let expected_tag = format!("v{}", env!("CARGO_PKG_VERSION"));
    let summary_path =
        std::env::temp_dir().join(format!("synapse-rc-summary-{}.md", std::process::id()));
    let _ = std::fs::remove_file(&summary_path);

    let output = std::process::Command::new("bash")
        .arg("scripts/release_candidate_dry_run.sh")
        .env("SYN_RELEASE_TAG", expected_tag)
        .env("SYN_RC_SKIP_SMOKE", "1")
        .env("SYN_RC_SKIP_FULL_RTK", "1")
        .env("SYN_RC_SKIP_FRESHNESS", "1")
        .env("GITHUB_STEP_SUMMARY", &summary_path)
        .output()
        .expect("release candidate script should execute with summary path");

    assert!(
        output.status.success(),
        "release candidate script failed while writing summary"
    );
    let summary = std::fs::read_to_string(&summary_path)
        .expect("release candidate script should write summary file");
    let _ = std::fs::remove_file(&summary_path);

    for marker in [
        "## Release candidate dry-run",
        "Commit",
        "Cargo version",
        "SHA256SUMS aggregation and verification checked",
        "Installer matrix: Linux x86_64, Linux aarch64, macOS x86_64, macOS arm64",
        "Full RTK gate: skipped by SYN_RC_SKIP_FULL_RTK",
        "Publishing: not performed by this dry-run",
    ] {
        assert!(
            summary.contains(marker),
            "summary missing marker {marker}: {summary}"
        );
    }
}
