// MODULE_CONTRACT
// MODULE_ID: M-TESTS-MCP
// PURPOSE: MCP and skills integration tests — verify registry, discovery, tester-agent guidance, and representative skill behavior
// SCOPE: Tool count parity including agent-based testing tools, grace skill visibility, tester subagent setup, basic CLI skills commands
// DEPENDS: M-CLI, M-MCP, M-SKILLS, M-CAPABILITIES

// START_MODULE_MAP
// test_skill_registry_count — 15 GRACE skills exposed
// test_capabilities_include_skills — Capabilities registry includes skill tools
// test_skill_engine_executes_init — Skill engine can run grace_init
// test_skill_engine_setup_subagents_includes_tester — Skill output includes Tester agent setup
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.20.0 — Updated MCP capability count for agent-based testing tools]
// END_CHANGE_SUMMARY

use std::sync::{Mutex, OnceLock};
use syn::capabilities;
use syn::skills::{registry::SKILL_DEFS, SkillEngine, SkillRequest};

// START_CONTRACT_cwd_lock
// PURPOSE: Return global lock for tests that mutate process current directory
// OUTPUTS: { &'static Mutex<()> }
fn cwd_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
// START_CONTRACT_test_skill_registry_count
// PURPOSE: Verify the GRACE skill registry exposes the expected skill count
// SIDE_EFFECTS: test assertion
fn test_skill_registry_count() {
    assert_eq!(SKILL_DEFS.len(), 15, "Expected 15 GRACE skills");
}

#[test]
// START_CONTRACT_test_capabilities_include_skills
// PURPOSE: Verify capability registry exposes skills and MCP tool counts
// SIDE_EFFECTS: test assertion
fn test_capabilities_include_skills() {
    assert!(capabilities::COMMANDS.iter().any(|(n, _)| *n == "skills"));
    assert_eq!(capabilities::GRACE_SKILL_TOOL_COUNT, 15);
    assert_eq!(capabilities::CORE_MCP_TOOL_COUNT, 21);
    assert_eq!(capabilities::MCP_TOOLS.len(), 36);
}

#[test]
// START_CONTRACT_test_skill_engine_executes_init
// PURPOSE: Verify SkillEngine can execute grace_init in an isolated temp project
// SIDE_EFFECTS: changes cwd during test and writes temp docs
fn test_skill_engine_executes_init() {
    let _guard = cwd_lock().lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let old = std::env::current_dir().unwrap();
    std::env::set_current_dir(tmp.path()).unwrap();

    let config = syn::config::Config::load_or_default();
    let engine = SkillEngine::new(&config);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime
        .block_on(engine.execute(SkillRequest {
            name: "grace_init".into(),
            arguments: serde_json::json!({}),
        }))
        .unwrap();

    assert_eq!(result.title, "grace_init");
    assert!(tmp.path().join("docs/graph-index.xml").exists());
    assert!(tmp.path().join("docs/plan-index.xml").exists());
    assert!(tmp.path().join("docs/verification-index.xml").exists());
    assert!(tmp.path().join("docs/belief-states").exists());
    assert!(tmp.path().join("docs/mental-tests").exists());
    assert!(tmp.path().join("docs/tests/guides").exists());
    assert!(tmp.path().join("docs/tests/results").exists());

    std::env::set_current_dir(old).unwrap();
}

#[test]
// START_CONTRACT_test_skill_engine_setup_subagents_includes_tester
// PURPOSE: Verify grace_setup_subagents recommends the Tester agent and testing MCP tools
// SIDE_EFFECTS: executes skill engine in current project
fn test_skill_engine_setup_subagents_includes_tester() {
    let config = syn::config::Config::load_or_default();
    let engine = SkillEngine::new(&config);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime
        .block_on(engine.execute(SkillRequest {
            name: "grace_setup_subagents".into(),
            arguments: serde_json::json!({"platform": "opencode"}),
        }))
        .unwrap();

    assert!(result.body.contains("Tester Agent"));
    assert!(result.body.contains("run_test_guide"));
    assert!(result.body.contains("submit_test_report"));
}
