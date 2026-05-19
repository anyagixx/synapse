// MODULE_CONTRACT
// MODULE_ID: M-TESTS-MCP
// PURPOSE: MCP and skills integration tests — verify registry, discovery, and representative skill behavior
// SCOPE: Tool count parity, grace skill visibility, basic CLI skills commands
// DEPENDS: M-CLI, M-MCP, M-SKILLS, M-CAPABILITIES

// START_MODULE_MAP
// test_skill_registry_count — 15 GRACE skills exposed
// test_capabilities_include_skills — Capabilities registry includes skill tools
// test_skill_engine_executes_init — Skill engine can run grace_init
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.6.0 — Added test function contracts]
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
    assert_eq!(capabilities::CORE_MCP_TOOL_COUNT, 12);
    assert_eq!(capabilities::MCP_TOOLS.len(), 27);
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

    std::env::set_current_dir(old).unwrap();
}
