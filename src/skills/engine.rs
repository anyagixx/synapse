// MODULE_CONTRACT
// MODULE_ID: M-SKILLS-ENGINE
// PURPOSE: Skill execution engine — dispatches 15 GRACE skill tools to deterministic project-aware summaries
// SCOPE: SkillEngine state, execute logic, helper formatters for sharded layout, requirements, technology, belief state, and project workflows
// DEPENDS: M-CONFIG, M-GRACE-LAYOUT, M-GRACE-REQUIREMENTS, M-GRACE-TECHNOLOGY, M-SKILLS-REGISTRY, M-SKILLS-TYPES
// LINKS: M-SKILLS

// START_MODULE_MAP
// SkillEngine — Main skill runtime facade
// execute — Dispatch skill by name to structured response
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.16.0 — Added Technology awareness to grace_plan guidance]
// END_CHANGE_SUMMARY

use super::registry::{find_skill, SKILL_DEFS};
use super::types::{SkillContext, SkillRequest, SkillResponse};
use crate::config::Config;
use crate::grace::layout::DocsLayout;
use crate::grace::refresh::Refresher;
use crate::grace::status::StatusCollector;
use crate::grace::verify::Verifier;

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
        Self {
            context: SkillContext {
                root,
                config: config.clone(),
            },
        }
    }
    // END_skill_engine_new

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
                    crate::grace::requirements::validate_requirements(&self.context.root).ok();
                let technology =
                    crate::grace::technology::validate_technology(&self.context.root).ok();
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
                        format!(
                            "components={}, compatibility_checks={}, valid={}",
                            t.components.len(),
                            t.compatibility_checks.len(),
                            t.valid
                        )
                    })
                    .unwrap_or_else(|| "no technology report available".into());
                format!(
                    "Planning skill ready.\n\nActive phase: {}\nGoal: {}\nConstraints: {}\nRequirements: {}\nTechnology: {}\nDrift: {}\nNext action: {}\n\nPlan in this order:\n1. read docs/requirements.xml and confirm Goals, DomainModel, Actors, UseCases, NFRs, Constraints, and Glossary\n2. read docs/technology.xml and confirm exact pinned versions plus compatibility checks\n3. confirm module boundaries in docs/modules/\n4. confirm phase order in docs/phases/\n5. confirm verification coverage in docs/verification/\n6. implement only next bounded module step",
                    active_phase,
                    string_arg(&request.arguments, "goal", "Define architecture and delivery plan"),
                    string_arg(&request.arguments, "constraints", "none provided"),
                    requirements_hint,
                    technology_hint,
                    drift_hint,
                    next_hint,
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
                let failing = verify
                    .as_ref()
                    .map(|results| results.iter().filter(|r| !r.passed).count())
                    .unwrap_or(0);
                format!(
                    "Execution guidance:\n- phase: {}\n- module: {}\n- objective: {}\n- failing verification groups: {}\n\nNext bounded step:\n1. read shard docs for {}\n2. call extract_belief_state for {} and inspect docs/belief-states/{}.xml\n3. inspect affected source files for {}\n4. implement smallest safe change for objective\n5. run verify_project\n6. if verify fails, switch to grace_fix",
                    active_phase,
                    module,
                    string_arg(&request.arguments, "objective", "complete next bounded implementation step"),
                    failing,
                    module,
                    module,
                    module,
                    module,
                )
            }
            "grace_multiagent_execute" => format!(
                "Multi-agent execution split:\n- phase: {}\n- modules: {}\n- policy: {}\n\nSuggested roles:\n- planner\n- implementer\n- reviewer\n- verifier\n- fixer\n\nConstraint: one worker per module boundary unless dependencies force sequence.",
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
                    "Fix workflow:\n- issue: {}\n- module hint: {}\n- graph drift: {}\n- verification drift: {}\n\nDebug path:\n1. identify failing module or contract\n2. inspect shard docs for module hint or active module\n3. inspect source files and graph refs\n4. patch smallest cause, not symptoms\n5. run verify_project and review_code before done",
                    string_arg(&request.arguments, "issue", "unspecified issue"),
                    module_hint,
                    missing_graph,
                    missing_verification,
                )
            }
            "grace_status" => format!(
                "Status skill overview:\n- root: {}\n- detail level: {}\n- skill count: {}\n\nPrimary model:\n- {}\n- {}\n- {}\n\nUse project_status for machine report and grace_lint for structural integrity.",
                self.context.root.display(),
                string_arg(&request.arguments, "detail_level", "standard"),
                SKILL_DEFS.len(),
                rel(&self.context.root, &layout.graph_index_path()),
                rel(&self.context.root, &layout.plan_index_path()),
                rel(&self.context.root, &layout.verification_index_path()),
            ),
            "grace_ask" => format!(
                "Artifact-aware answer flow prepared.\n\nQuestion: {}\n\nUse sources in order:\n1. docs/plan-index.xml\n2. docs/graph-index.xml\n3. docs/verification-index.xml\n4. relevant shards under docs/modules, docs/phases, docs/verification\n5. indexed code search",
                string_arg(&request.arguments, "question", "no question provided"),
            ),
            "grace_explainer" => format!(
                "Explainer target: {}\n\nExplain using:\n- architecture shard\n- verification shard\n- code signatures\n- semantic search results\n- graph relationships",
                string_arg(&request.arguments, "target", "module or subsystem"),
            ),
            "grace_cli" => format!(
                "CLI usage guide topic: {}\n\nCore commands:\n- syn init\n- syn verify\n- syn review\n- syn refresh\n- syn status\n- syn mcp\n\nOpenCode path:\n- opencode loads synapse MCP from opencode.jsonc\n- 15 grace tools exposed over MCP\n- shell commands proxied through syn proxy",
                string_arg(&request.arguments, "topic", "general workflow"),
            ),
            "grace_setup_subagents" => format!(
                "Recommended subagent setup for {}:\n- planner\n- implementer\n- reviewer\n- verifier\n- fixer\n\nRoles input: {}\n\nPolicy: planner owns architecture, implementer owns one module, reviewer/verifier gate completion, fixer handles failures.",
                string_arg(&request.arguments, "platform", "opencode"),
                string_arg(&request.arguments, "roles", "default GRACE roles"),
            ),
            "grace_lint" => format!(
                "Lint scope: {}\n\nChecks to run:\n- graph-index.xml exists\n- plan-index.xml exists\n- verification-index.xml exists\n- shard directories exist\n- compatibility docs present\n- refs are not orphaned\n- module and verification coverage align",
                string_arg(&request.arguments, "scope", "project"),
            ),
            _ => unreachable!(),
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

// END_public_api
