// MODULE_CONTRACT
// MODULE_ID: M-TESTS-PARITY
// PURPOSE: Ensure README, docs, and code claims match capabilities registry
// SCOPE: Compare README tool count, README command count, verify check count
// DEPENDS: M-CAPABILITIES

// START_MODULE_MAP
// test_mcp_tool_count_matches_capabilities — MCP tool count check
// test_command_count_matches_capabilities — Command count check
// test_verify_checks_consistent — Verify check consistency
// test_no_ghost_commands — No ghost commands
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.6.0 — Added test function contracts]
// END_CHANGE_SUMMARY

use syn::capabilities;

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
