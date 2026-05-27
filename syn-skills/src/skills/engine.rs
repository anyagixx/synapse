// MODULE_CONTRACT
// MODULE_ID: M-SKILLS-ENGINE
// PURPOSE: Skill execution engine — dispatches 16 GRACE skill tools to deterministic project-aware summaries
// SCOPE: SkillEngine state, execute logic, helper formatters for sharded layout, requirements, pending/concrete technology decisions, development plan, mental tests, traceability, tester-agent workflow, belief state, and project workflows
// DEPENDS: M-CONFIG, M-GRACE-DEVELOPMENT-PLAN, M-GRACE-MENTAL-TEST, M-GRACE-TRACEABILITY, M-GRACE-TESTING, M-GRACE-LAYOUT, M-GRACE-REQUIREMENTS, M-GRACE-TECHNOLOGY, M-SKILLS-REGISTRY, M-SKILLS-TYPES
// LINKS:
//   -> M-SKILLS (depends) - skill runtime facade

// START_MODULE_MAP
// SkillEngine — Main skill runtime facade
// SkillEngine::new_with_root — Construct engine with explicit root for tests and embedding
// execute — Dispatch skill by name to structured response
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.24.0 - Teach planning guidance to handle pending technology decisions]
// END_CHANGE_SUMMARY

use super::registry::{find_skill, SKILL_DEFS};
use super::types::{SkillContext, SkillRequest, SkillResponse};
use syn_core::config::Config;
use syn_engine::grace::layout::DocsLayout;
use syn_engine::grace::refresh::Refresher;
use syn_engine::grace::status::StatusCollector;
use syn_engine::grace::verify::Verifier;
use syn_run::{RunManager, RunRecord};
use syn_core::tracking::Tracker;

// START_public_api

// START_SkillEngine
#[derive(Debug, Clone)]
pub struct SkillEngine {
    context: SkillContext,
}
// END_SkillEngine

impl SkillEngine {
    // START_CONTRACT_SkillEngine::new
    // PURPOSE: Create a new SkillEngine bound to current project directory
    // INPUTS: { config: &Config — runtime configuration }
    // OUTPUTS: { SkillEngine }
    // START_skill_engine_new
    pub fn new(config: &Config) -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        Self::new_with_root(config, root)
    }
    // END_skill_engine_new

    // START_CONTRACT_SkillEngine::new_with_root
    // PURPOSE: Create a SkillEngine bound to an explicit project directory
    // INPUTS: { config: &Config }, { root: PathBuf }
    // OUTPUTS: { SkillEngine }
    // START_skill_engine_new_with_root
    pub fn new_with_root(config: &Config, root: std::path::PathBuf) -> Self {
        Self {
            context: SkillContext {
                root,
                config: config.clone(),
            },
        }
    }
    // END_skill_engine_new_with_root

    // START_CONTRACT_SkillEngine::defs
    // PURPOSE: Return all registered skill definitions
    // OUTPUTS: { &'static [SkillDef] — skill metadata slice }
    // START_skill_engine_defs
    pub fn defs(&self) -> &'static [super::types::SkillDef] {
        SKILL_DEFS
    }
    // END_skill_engine_defs

    // START_CONTRACT_SkillEngine::execute
    // PURPOSE: Execute a named skill with JSON arguments and return structured text result
    // INPUTS: { request: SkillRequest — skill invocation payload }
    // OUTPUTS: { anyhow::Result<SkillResponse> — deterministic skill output }
    // SIDE_EFFECTS: may create sharded docs layout for grace_init
    // START_skill_engine_execute
    pub async fn execute(&self, request: SkillRequest) -> anyhow::Result<SkillResponse> {
        let skill = find_skill(&request.name)
            .ok_or_else(|| anyhow::anyhow!("Unknown skill: {}", request.name))?;
        let layout = DocsLayout::new(&self.context.root);

        let body = match skill.name {
            "grace_init" => {
                layout.ensure_initialized()?;
                format!(
                    "Initialized sharded GRACE layout at {}\n\nPrimary indexes:\n- {}\n- {}\n- {}\n\nShard dirs:\n- {}\n- {}\n- {}\n\nCompatibility docs preserved under docs/*.xml.",
                    self.context.root.display(),
                    rel(&self.context.root, &layout.graph_index_path()),
                    rel(&self.context.root, &layout.plan_index_path()),
                    rel(&self.context.root, &layout.verification_index_path()),
                    rel(&self.context.root, &layout.modules_dir()),
                    rel(&self.context.root, &layout.phases_dir()),
                    rel(&self.context.root, &layout.verification_dir()),
                )
            }
            "grace_plan" => {
                let refresh = Refresher::refresh(&self.context.root).ok();
                let status = StatusCollector::collect(&self.context.root).await.ok();
                let requirements =
                    syn_engine::grace::requirements::validate_requirements(&self.context.root).ok();
                let technology =
                    syn_engine::grace::technology::validate_technology(&self.context.root).ok();
                let development_plan =
                    syn_engine::grace::development_plan::validate_development_plan(&self.context.root)
                        .ok();
                let mental_tests =
                    syn_engine::grace::mental_test::scan_project_mental_tests(&self.context.root).ok();
                let active_phase = read_active_phase(&layout).unwrap_or_else(|| "Phase-0".into());
                let drift_hint = refresh
                    .as_ref()
                    .map(|r| format!("{} modules missing from graph, {} missing from verification", r.not_in_graph.len(), r.not_in_verification.len()))
                    .unwrap_or_else(|| "no drift report available".into());
                let next_hint = status
                    .as_ref()
                    .and_then(|s| s.next_actions.first().cloned())
                    .unwrap_or_else(|| "define modules and implementation order".into());
                let requirements_hint = requirements
                    .as_ref()
                    .map(|r| {
                        format!(
                            "entities={}, use_cases={}, valid={}",
                            r.entities.len(),
                            r.use_cases.len(),
                            r.valid
                        )
                    })
                    .unwrap_or_else(|| "no requirements report available".into());
                let technology_hint = technology
                    .as_ref()
                    .map(|t| {
                        if t.decision_pending {
                            format!(
                                "status={}, valid={}, action=recommend stack from requirements before implementation",
                                t.status, t.valid
                            )
                        } else {
                            format!(
                                "status={}, components={}, compatibility_checks={}, valid={}",
                                t.status,
                                t.components.len(),
                                t.compatibility_checks.len(),
                                t.valid
                            )
                        }
                    })
                    .unwrap_or_else(|| "no technology report available".into());
                let technology_step = technology
                    .as_ref()
                    .map(|t| {
                        if t.decision_pending {
                            "read docs/technology.xml; if status=needs-decision, recommend stack from requirements and constraints before implementation"
                        } else {
                            "read docs/technology.xml and confirm exact pinned versions plus compatibility checks"
                        }
                    })
                    .unwrap_or(
                        "read docs/technology.xml and either resolve needs-decision or confirm exact pinned versions",
                    );
                let development_plan_hint = development_plan
                    .as_ref()
                    .map(|p| {
                        format!(
                            "data_flows={}, generation_modules={}, valid={}",
                            p.data_flows.len(),
                            p.generation_modules.len(),
                            p.valid
                        )
                    })
                    .unwrap_or_else(|| "no development plan report available".into());
                let mental_tests_hint = mental_tests
                    .as_ref()
                    .map(|m| {
                        format!(
                            "total={}, passed={}, failed={}, required={}",
                            m.total,
                            m.passed,
                            m.failed,
                            m.required_targets.len()
                        )
                    })
                    .unwrap_or_else(|| "no mental test report available".into());
                format!(
                    "Planning skill ready.\n\nActive phase: {}\nGoal: {}\nConstraints: {}\nRequirements: {}\nTechnology: {}\nDevelopmentPlan: {}\nMentalTests: {}\nDrift: {}\nNext action: {}\n\nPlan in this order:\n1. read docs/requirements.xml and confirm Goals, DomainModel, Actors, UseCases, NFRs, Constraints, and Glossary\n2. {}\n3. read docs/development-plan.xml and confirm DataFlows, GenerationOrder, and which algorithms need MentalTests\n4. scaffold or update <MentalTests> before code for critical/complex modules\n5. confirm module boundaries in docs/modules/\n6. confirm phase order in docs/phases/\n7. confirm verification coverage in docs/verification/\n8. implement only next bounded module step after mental tests pass",
                    active_phase,
                    string_arg(&request.arguments, "goal", "Define architecture and delivery plan"),
                    string_arg(&request.arguments, "constraints", "none provided"),
                    requirements_hint,
                    technology_hint,
                    development_plan_hint,
                    mental_tests_hint,
                    drift_hint,
                    next_hint,
                    technology_step,
                )
            }
            "grace_verification" => format!(
                "Verification skill ready. Verification source of truth:\n- {}\n- {}\n\nModule: {}\nPriority: {}\n\nExpected checks:\n- module-local\n- wave\n- phase\n- cross-ref integrity",
                rel(&self.context.root, &layout.verification_index_path()),
                rel(&self.context.root, &layout.verification_dir()),
                string_arg(&request.arguments, "module_id", "all modules"),
                string_arg(&request.arguments, "priority", "standard"),
            ),
            "grace_execute" => {
                let active_phase = string_arg(
                    &request.arguments,
                    "phase",
                    &read_active_phase(&layout).unwrap_or_else(|| "Phase-1".into()),
                );
                let module = string_arg(
                    &request.arguments,
                    "module_id",
                    &first_module_id(&layout).unwrap_or_else(|| "M-CORE".into()),
                );
                let verify = Verifier::verify_all(&self.context.root).await.ok();
                let plan =
                    syn_engine::grace::development_plan::validate_development_plan(&self.context.root)
                        .ok();
                let mental_tests =
                    syn_engine::grace::mental_test::scan_project_mental_tests(&self.context.root).ok();
                let status = StatusCollector::collect(&self.context.root).await.ok();
                let failing = verify
                    .as_ref()
                    .map(|results| results.iter().filter(|r| !r.passed).count())
                    .unwrap_or(0);
                let next_plan_module = plan
                    .as_ref()
                    .and_then(|p| p.generation_modules.first())
                    .map(|m| m.id.clone())
                    .unwrap_or_else(|| module.clone());
                let flow_count = plan.as_ref().map(|p| p.data_flows.len()).unwrap_or(0);
                let mental_gate = mental_tests
                    .as_ref()
                    .map(|m| {
                        if m.mental_tests_defined()
                            && m.mental_tests_passed()
                            && m.mental_test_no_drift()
                        {
                            format!("pass ({} tests)", m.total)
                        } else {
                            format!(
                                "block: failed={} not_run={} missing_required={}",
                                m.failed,
                                m.not_run,
                                m.missing_required_targets.len()
                            )
                        }
                    })
                    .unwrap_or_else(|| "unknown".into());
                let mut run = RunRecord::new(
                    string_arg(&request.arguments, "objective", "complete next bounded implementation step"),
                    active_phase.clone(),
                    module.clone(),
                    string_arg(&request.arguments, "objective", "complete next bounded implementation step"),
                );
                if let Some(report) = status {
                    let manager = RunManager::new(&self.context.root);
                    let (created_run, _) = manager
                        .create_run(
                            &string_arg(
                                &request.arguments,
                                "objective",
                                "complete next bounded implementation step",
                            ),
                            &active_phase,
                            &module,
                            &string_arg(
                                &request.arguments,
                                "objective",
                                "complete next bounded implementation step",
                            ),
                            &report,
                        )
                        .unwrap_or_else(|_| {
                            let policy = manager.build_gate_policy(&active_phase, &module, &report);
                            let decision = manager.evaluate_gate_policy(&policy);
                            run.policy = Some(policy);
                            run.status = if decision.blocked {
                                syn_run::RunStatus::Blocked
                            } else {
                                syn_run::RunStatus::Ready
                            };
                            (run.clone(), decision)
                        });
                    run = created_run;
                }
                format!(
                    "Execution guidance:\n- phase: {}\n- module: {}\n- generation-order next: {}\n- data flows in plan: {}\n- mental test gate: {}\n- objective: {}\n- failing verification groups: {}\n- run_id: {}\n- run_status: {:?}\n\nNext bounded step:\n1. read docs/development-plan.xml GenerationOrder, DataFlows, and MentalTests for {}\n2. run mental_test_run for {} if a matching MentalTest exists\n3. read shard docs for {}\n4. call extract_belief_state for {} and inspect docs/belief-states/{}.xml\n5. inspect affected source files for {}\n6. add/update LINKS to requirements or use cases and run traceability_report for {}\n7. implement smallest safe change for objective only after mental test PASS\n8. run verify_project\n9. if verify fails, switch to grace_fix",
                    active_phase,
                    module,
                    next_plan_module,
                    flow_count,
                    mental_gate,
                    string_arg(&request.arguments, "objective", "complete next bounded implementation step"),
                    failing,
                    run.run_id,
                    run.status,
                    module,
                    module,
                    module,
                    module,
                    module,
                    module,
                    module,
                )
            }
            "grace_multiagent_execute" => format!(
                "Multi-agent execution split:\n- phase: {}\n- modules: {}\n- policy: {}\n\nSuggested roles:\n- planner\n- implementer\n- tester\n- reviewer\n- verifier\n- fixer\n\nTester workflow:\n1. read docs/tests/guides/\n2. run run_test_guide against the app or mock console\n3. submit_test_report when failures include LOG evidence\n\nConstraint: one worker per module boundary unless dependencies force sequence.",
                string_arg(&request.arguments, "phase", "active phase"),
                string_arg(&request.arguments, "modules", "derived from active plan phase"),
                string_arg(&request.arguments, "execution_policy", "one worker per module"),
            ),
            "grace_reviewer" => format!(
                "Reviewer skill wraps integrity review.\n\nScope: {}\nRecommended sequence:\n1. review_code scoped or full\n2. inspect shard/index consistency\n3. verify contract coverage\n4. confirm verification refs and phase refs",
                string_arg(&request.arguments, "scope", "project"),
            ),
            "grace_refresh" => format!(
                "Refresh skill sync target:\n- graph index: {}\n- plan index: {}\n- verification index: {}\n\nMode: {}\n\nRefresh should detect missing shards, orphaned refs, and stale compatibility docs.",
                rel(&self.context.root, &layout.graph_index_path()),
                rel(&self.context.root, &layout.plan_index_path()),
                rel(&self.context.root, &layout.verification_index_path()),
                string_arg(&request.arguments, "sync_mode", "report"),
            ),
            "grace_refactor" => format!(
                "Refactor planning skill:\n- target: {}\n- intent: {}\n\nRequired before changes:\n1. module shard update plan\n2. verification impact review\n3. dependency and cross-link check\n4. post-refactor verify_project + review_code",
                string_arg(&request.arguments, "target", "unspecified target"),
                string_arg(&request.arguments, "intent", "improve structure without architectural drift"),
            ),
            "grace_fix" => {
                let refresh = Refresher::refresh(&self.context.root).ok();
                let module_hint = string_arg(&request.arguments, "module_hint", "none");
                let missing_graph = refresh.as_ref().map(|r| r.not_in_graph.len()).unwrap_or(0);
                let missing_verification = refresh
                    .as_ref()
                    .map(|r| r.not_in_verification.len())
                    .unwrap_or(0);
                format!(
                    "Fix workflow:\n- issue: {}\n- module hint: {}\n- graph drift: {}\n- verification drift: {}\n\nDebug path:\n1. identify failing module or contract\n2. inspect any docs/tests/results failure report and LOG evidence refs\n3. run or add a MentalTest that reproduces the expected fix behavior\n4. inspect shard docs for module hint or active module\n5. inspect source files and graph refs\n6. patch smallest cause, not symptoms\n7. run run_test_guide if a guide exists, then verify_project and review_code before done",
                    string_arg(&request.arguments, "issue", "unspecified issue"),
                    module_hint,
                    missing_graph,
                    missing_verification,
                )
            }
            "grace_status" => format!(
                "Status skill overview:\n- root: {}\n- detail level: {}\n- skill count: {}\n\nPrimary model:\n- {}\n- {}\n- {}\n- {}\n\nUse project_status for machine report, traceability_report for requirement/code chains, and grace_lint for structural integrity.",
                self.context.root.display(),
                string_arg(&request.arguments, "detail_level", "standard"),
                SKILL_DEFS.len(),
                rel(&self.context.root, &layout.graph_index_path()),
                rel(&self.context.root, &layout.plan_index_path()),
                rel(&self.context.root, &layout.verification_index_path()),
                rel(&self.context.root, &layout.traceability_index_path()),
            ),
            "grace_run_history" => {
                let detail_level = string_arg(&request.arguments, "detail_level", "standard");
                let run_id = string_arg(&request.arguments, "run_id", "");
                let tracker = Tracker::new(&self.context.config);
                let runs = RunManager::new(&self.context.root);
                let latest_run = runs.list().ok().and_then(|mut list| list.pop());
                let selected_run = if run_id.is_empty() {
                    latest_run.clone()
                } else {
                    runs.load(&run_id).ok().or(latest_run.clone())
                };
                let events = if let Some(run) = &selected_run {
                    tracker.get_run_events(Some(&run.run_id)).await.ok().unwrap_or_default()
                } else {
                    Vec::new()
                };
                let mut body = String::new();
                body.push_str(&format!(
                    "Run history view\n- detail level: {}\n- run selected: {}\n- event count: {}\n\n",
                    detail_level,
                    selected_run
                        .as_ref()
                        .map(|r| r.run_id.as_str())
                        .unwrap_or("none"),
                    events.len()
                ));
                if let Some(run) = selected_run {
                    body.push_str(&format!(
                        "Current run\n- phase: {}\n- module: {}\n- status: {:?}\n- step: {}\n- blocked: {}\n- escalation: {}\n- traceability: {}\n- retries: {}\n\n",
                        run.phase,
                        run.module_id,
                        run.status,
                        run.current_step,
                        run.blocked_reason.as_deref().unwrap_or("none"),
                        run.escalation_reason.as_deref().unwrap_or("none"),
                        run.metadata
                            .get("traceability_summary")
                            .map(|s| s.as_str())
                            .unwrap_or("none"),
                        run.metadata
                            .get("retry_count")
                            .map(|s| s.as_str())
                            .unwrap_or("0")
                    ));
                    if detail_level != "summary" {
                        for event in events.iter().rev().take(10) {
                            body.push_str(&format!(
                                "- {} | {} | {} | {} | {}\n",
                                event.timestamp, event.event_type, event.phase, event.status, event.detail
                            ));
                        }
                    }
                } else {
                    body.push_str("No run records found.\n");
                }
                body
            }

            "grace_explainer" => format!(
                "Explainer target: {}\n\nExplain using:\n- architecture shard\n- verification shard\n- code signatures\n- semantic search results\n- graph relationships",
                string_arg(&request.arguments, "target", "module or subsystem"),
            ),
            "grace_cli" => format!(
                "CLI usage guide topic: {}\n\nCore commands:\n- syn init\n- syn verify\n- syn review\n- syn refresh\n- syn status\n- syn mcp\n\nOpenCode path:\n- opencode loads synapse MCP from opencode.jsonc\n- 16 grace tools exposed over MCP\n- shell commands proxied through syn proxy",
                string_arg(&request.arguments, "topic", "general workflow"),
            ),
            "grace_setup_subagents" => format!(
                "Recommended subagent setup for {}:\n- planner\n- implementer\n- tester\n- reviewer\n- verifier\n- fixer\n\nTester Agent:\n- role: natural-language test executor\n- tools: run_test_guide, submit_test_report, analyze_logs, semantic_search\n- knowledge: docs/tests/guides/, application API, structured LOG format\n- workflow: run guide, capture logs, submit XML failure report, hand evidence to developer\n\nRoles input: {}\n\nPolicy: planner owns architecture, implementer owns one module, tester owns guide execution and failure reports, reviewer/verifier gate completion, fixer handles failures.",
                string_arg(&request.arguments, "platform", "opencode"),
                string_arg(&request.arguments, "roles", "default GRACE roles"),
            ),
            "grace_lint" => format!(
                "Lint scope: {}\n\nChecks to run:\n- graph-index.xml exists\n- plan-index.xml exists\n- verification-index.xml exists\n- shard directories exist\n- compatibility docs present\n- refs are not orphaned\n- module and verification coverage align",
                string_arg(&request.arguments, "scope", "project"),
            ),
            other => anyhow::bail!("Unknown skill: {}", other),
        };

        Ok(SkillResponse {
            title: skill.name.to_string(),
            body,
        })
    }
    // END_skill_engine_execute
}

fn string_arg(arguments: &serde_json::Value, key: &str, default: &str) -> String {
    arguments
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

fn rel(root: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn read_active_phase(layout: &DocsLayout) -> Option<String> {
    std::fs::read_to_string(layout.plan_index_path())
        .ok()
        .and_then(|content| {
            regex::Regex::new(r#"<ACTIVE_PHASE>([^<]+)</ACTIVE_PHASE>"#)
                .ok()
                .and_then(|re| re.captures(&content).map(|c| c[1].to_string()))
        })
}

fn first_module_id(layout: &DocsLayout) -> Option<String> {
    let content = std::fs::read_to_string(layout.graph_index_path()).ok()?;
    regex::Regex::new(r#"<MODULE id=\"([^\"]+)\""#)
        .ok()
        .and_then(|re| re.captures(&content).map(|c| c[1].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn grace_plan_guidance_flags_pending_technology_decision() {
        let dir = tempfile::tempdir().unwrap();
        let layout = DocsLayout::new(dir.path());
        layout.ensure_initialized().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let response = engine
            .execute(SkillRequest {
                name: "grace_plan".into(),
                arguments: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("status=needs-decision"));
        assert!(response.body.contains("recommend stack from requirements"));
    }

    #[tokio::test]
    async fn grace_run_history_reports_current_run_state() {
        let dir = tempfile::tempdir().unwrap();
        let layout = DocsLayout::new(dir.path());
        layout.ensure_initialized().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let tracker = Tracker::new(&config);
        let run_manager = RunManager::new(dir.path());
        let report = syn_engine::grace::status::StatusCollector::collect(dir.path())
            .await
            .unwrap();
        let (record, _) = run_manager
            .create_run(
                "test goal",
                "Phase-17",
                "M-RUNNER",
                "test objective",
                &report,
            )
            .unwrap();
        tracker
            .record_run_event(
                &record.run_id,
                "create_run",
                &record.module_id,
                &record.phase,
                "ready",
                "created test run",
            )
            .await
            .unwrap();
        let response = engine
            .execute(SkillRequest {
                name: "grace_run_history".into(),
                arguments: serde_json::json!({"run_id": record.run_id, "detail_level": "deep"}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("Run history view"));
        assert!(response.body.contains("create_run"));
    }

    // ── Format-output skill tests (Phase-98 coverage hardening) ──

    #[tokio::test]
    async fn grace_lint_produces_structural_checks() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let response = engine
            .execute(SkillRequest {
                name: "grace_lint".into(),
                arguments: serde_json::json!({"scope": "project"}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("Lint scope: project"));
        assert!(response.body.contains("graph-index.xml exists"));
        assert!(response.body.contains("verification-index.xml exists"));
    }

    #[tokio::test]
    async fn grace_explainer_produces_target_guidance() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let response = engine
            .execute(SkillRequest {
                name: "grace_explainer".into(),
                arguments: serde_json::json!({"target": "M-CONFIG"}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("Explainer target: M-CONFIG"));
        assert!(response.body.contains("architecture shard"));
        assert!(response.body.contains("semantic search results"));
    }

    #[tokio::test]
    async fn grace_cli_produces_usage_guide() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let response = engine
            .execute(SkillRequest {
                name: "grace_cli".into(),
                arguments: serde_json::json!({"topic": "verify workflow"}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("CLI usage guide topic: verify workflow"));
        assert!(response.body.contains("syn verify"));
        assert!(response.body.contains("syn refresh"));
    }

    #[tokio::test]
    async fn grace_setup_subagents_produces_role_guidance() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let response = engine
            .execute(SkillRequest {
                name: "grace_setup_subagents".into(),
                arguments: serde_json::json!({"platform": "opencode", "roles": "planner tester"}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("Recommended subagent setup for opencode"));
        assert!(response.body.contains("tester"));
        assert!(response.body.contains("planner"));
        assert!(response.body.contains("implementer"));
    }

    #[tokio::test]
    async fn grace_status_produces_overview() {
        let dir = tempfile::tempdir().unwrap();
        let layout = DocsLayout::new(dir.path());
        layout.ensure_initialized().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let response = engine
            .execute(SkillRequest {
                name: "grace_status".into(),
                arguments: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("Status skill overview"));
        assert!(response.body.contains("graph-index.xml"));
        assert!(response.body.contains("plan-index.xml"));
    }

    #[tokio::test]
    async fn grace_refresh_produces_sync_guidance() {
        let dir = tempfile::tempdir().unwrap();
        let layout = DocsLayout::new(dir.path());
        layout.ensure_initialized().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let response = engine
            .execute(SkillRequest {
                name: "grace_refresh".into(),
                arguments: serde_json::json!({"sync_mode": "fix"}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("Refresh skill sync target"));
        assert!(response.body.contains("Mode: fix"));
        assert!(response.body.contains("graph-index.xml"));
    }

    #[tokio::test]
    async fn grace_reviewer_produces_sequence() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let response = engine
            .execute(SkillRequest {
                name: "grace_reviewer".into(),
                arguments: serde_json::json!({"scope": "full"}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("Reviewer skill wraps integrity review"));
        assert!(response.body.contains("review_code"));
        assert!(response.body.contains("contract coverage"));
    }

    #[tokio::test]
    async fn grace_refactor_produces_planning_guidance() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let response = engine
            .execute(SkillRequest {
                name: "grace_refactor".into(),
                arguments: serde_json::json!({"target": "M-CONFIG", "intent": "extract validation"}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("Refactor planning skill"));
        assert!(response.body.contains("M-CONFIG"));
        assert!(response.body.contains("extract validation"));
        assert!(response.body.contains("module shard update plan"));
    }

    #[tokio::test]
    async fn grace_multiagent_execute_produces_role_split() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let response = engine
            .execute(SkillRequest {
                name: "grace_multiagent_execute".into(),
                arguments: serde_json::json!({"phase": "Phase-17", "modules": "M-CORE"}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("Multi-agent execution split"));
        assert!(response.body.contains("Phase-17"));
        assert!(response.body.contains("planner"));
        assert!(response.body.contains("tester"));
    }

    #[tokio::test]
    async fn grace_fix_produces_debug_workflow() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let response = engine
            .execute(SkillRequest {
                name: "grace_fix".into(),
                arguments: serde_json::json!({"issue": "missing module shard", "module_hint": "M-CORE"}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("Fix workflow"));
        assert!(response.body.contains("missing module shard"));
        assert!(response.body.contains("module hint: M-CORE"));
        assert!(response.body.contains("MentalTest"));
    }

    #[tokio::test]
    async fn grace_verification_produces_execution_guidance() {
        let dir = tempfile::tempdir().unwrap();
        let layout = DocsLayout::new(dir.path());
        layout.ensure_initialized().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let response = engine
            .execute(SkillRequest {
                name: "grace_verification".into(),
                arguments: serde_json::json!({"phase": "Phase-0", "module": "M-CORE", "objective": "init"}),
            })
            .await
            .unwrap();
        assert!(response.body.contains("Verification skill ready"));
        assert!(response.body.contains("module-local"));
        assert!(response.body.contains("wave"));
    }

    #[tokio::test]
    async fn unknown_skill_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let engine = SkillEngine::new_with_root(&config, dir.path().to_path_buf());
        let result = engine
            .execute(SkillRequest {
                name: "nonexistent_skill".into(),
                arguments: serde_json::json!({}),
            })
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown skill"));
    }
}

// END_public_api
