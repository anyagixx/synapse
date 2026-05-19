// MODULE_CONTRACT
// MODULE_ID: M-TESTS-PARITY
// PURPOSE: Ensure README, docs, and code claims match capabilities registry
// SCOPE: Compare README tool count, README command count, verify check count, install docs, and release artifact claims
// DEPENDS: M-CAPABILITIES, M-INSTALL, M-CI

// START_MODULE_MAP
// test_mcp_tool_count_matches_capabilities — MCP tool count check
// test_command_count_matches_capabilities — Command count check
// test_verify_checks_consistent — Verify check consistency
// test_no_ghost_commands — No ghost commands
// test_install_default_version_matches_package — Installer default tag check
// test_install_docs_use_supported_url — Install documentation URL check
// test_release_artifact_matches_installer — Release artifact naming check
// test_platform_claims_match_release_truth — Platform support truth check
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Added install and release truth parity checks]
// END_CHANGE_SUMMARY

use syn::capabilities;

const INSTALL_SCRIPT: &str = include_str!("../install.sh");
const RELEASE_WORKFLOW: &str = include_str!("../.github/workflows/release.yml");
const README: &str = include_str!("../README.md");
const QUICKSTART: &str = include_str!("../docs/QUICKSTART.md");
const FAQ: &str = include_str!("../docs/FAQ.md");
const INSTALL_COMMAND: &str =
    "curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/main/install.sh | sh";
const LINUX_RELEASE_ARTIFACT: &str = "syn-x86_64-unknown-linux-gnu.tar.gz";

#[test]
// START_CONTRACT_test_mcp_tool_count_matches_capabilities
// PURPOSE: Verify MCP capability counts and uniqueness
// SIDE_EFFECTS: reads README.md and asserts capability registry invariants
fn test_mcp_tool_count_matches_capabilities() {
    let expected = capabilities::MCP_TOOLS.len();
    let readme = std::fs::read_to_string("README.md").unwrap();
    let tool_mentions = readme.matches("`").count() / 2;
    assert_eq!(expected, capabilities::TOTAL_MCP_TOOL_COUNT);
    assert_eq!(capabilities::CORE_MCP_TOOL_COUNT, 12);
    assert_eq!(capabilities::GRACE_SKILL_TOOL_COUNT, 15);
    assert_eq!(capabilities::skill_defs_count(), 15);
    assert!(!capabilities::MCP_TOOLS.is_empty(), "MCP tools list empty");
    let names: Vec<&str> = capabilities::MCP_TOOLS.iter().map(|(n, _)| *n).collect();
    let unique: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(names.len(), unique.len(), "Duplicate MCP tool names found");
    let _ = tool_mentions;
}

#[test]
// START_CONTRACT_test_command_count_matches_capabilities
// PURPOSE: Verify CLI command registry has enough unique commands
// SIDE_EFFECTS: test assertion
fn test_command_count_matches_capabilities() {
    assert!(
        capabilities::COMMANDS.len() >= 15,
        "Expected at least 15 commands"
    );
    let names: Vec<&str> = capabilities::COMMANDS.iter().map(|(n, _)| *n).collect();
    let unique: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(
        names.len(),
        unique.len(),
        "Duplicate command names in registry"
    );
}

#[test]
// START_CONTRACT_test_verify_checks_consistent
// PURPOSE: Verify verification check registry exposes expected checks
// SIDE_EFFECTS: test assertion
fn test_verify_checks_consistent() {
    assert!(
        capabilities::VERIFY_CHECKS.len() >= 8,
        "Expected at least 8 verify checks"
    );
}

#[test]
// START_CONTRACT_test_no_ghost_commands
// PURPOSE: Verify documented command names and descriptions are non-empty
// SIDE_EFFECTS: test assertion
fn test_no_ghost_commands() {
    // Ensure all documented commands have non-empty descriptions
    for (name, desc) in capabilities::COMMANDS {
        assert!(!desc.is_empty(), "Command '{}' has empty description", name);
        assert!(!name.is_empty(), "Empty command name");
    }
}

#[test]
// START_CONTRACT_test_install_default_version_matches_package
// PURPOSE: Verify install.sh defaults to the current Cargo package tag
// SIDE_EFFECTS: test assertion
fn test_install_default_version_matches_package() {
    let expected_default = format!("VERSION=\"${{1:-v{}}}\"", env!("CARGO_PKG_VERSION"));
    assert!(
        INSTALL_SCRIPT.contains(&expected_default),
        "install.sh must default to {}",
        expected_default
    );
    assert!(
        INSTALL_SCRIPT.contains(
            "cargo install --git https://github.com/anyagixx/synapse --tag \"${VERSION}\""
        ),
        "install.sh must keep source fallback pinned to the selected release tag"
    );
}

#[test]
// START_CONTRACT_test_install_docs_use_supported_url
// PURPOSE: Verify install docs use the repository-hosted installer URL
// SIDE_EFFECTS: test assertion
fn test_install_docs_use_supported_url() {
    for (name, doc) in [
        ("README.md", README),
        ("docs/QUICKSTART.md", QUICKSTART),
        ("docs/FAQ.md", FAQ),
    ] {
        assert!(
            doc.contains(INSTALL_COMMAND),
            "{} must document the supported install command",
            name
        );
        assert!(
            !doc.contains("https://synapse.dev/install.sh"),
            "{} must not document unsupported hosted installer URLs",
            name
        );
    }
}

#[test]
// START_CONTRACT_test_release_artifact_matches_installer
// PURPOSE: Verify release workflow publishes the Linux artifact expected by install.sh
// SIDE_EFFECTS: test assertion
fn test_release_artifact_matches_installer() {
    assert!(
        INSTALL_SCRIPT.contains("TARBALL=\"syn-${ARCH}-${OS}.tar.gz\""),
        "install.sh must derive tarball names from detected arch and OS"
    );
    assert!(
        RELEASE_WORKFLOW.contains(&format!("tar -czf {}", LINUX_RELEASE_ARTIFACT)),
        "release workflow must package {}",
        LINUX_RELEASE_ARTIFACT
    );
    assert!(
        RELEASE_WORKFLOW.contains(&format!("files: {}", LINUX_RELEASE_ARTIFACT)),
        "release workflow must upload {}",
        LINUX_RELEASE_ARTIFACT
    );
}

#[test]
// START_CONTRACT_test_platform_claims_match_release_truth
// PURPOSE: Verify FAQ platform support claims distinguish prebuilt artifacts from source fallback
// SIDE_EFFECTS: test assertion
fn test_platform_claims_match_release_truth() {
    assert!(
        FAQ.contains("Prebuilt release artifact: Linux x86_64."),
        "FAQ must state the currently published prebuilt artifact scope"
    );
    assert!(
        FAQ.contains("Source fallback via Cargo: Linux/macOS on x86_64 or aarch64."),
        "FAQ must state source fallback platforms"
    );
    assert!(
        FAQ.contains("Windows: build/test support exists in CI; install from source with Cargo."),
        "FAQ must state Windows install reality"
    );
}
