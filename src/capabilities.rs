// MODULE_CONTRACT
// MODULE_ID: M-CAPABILITIES
// PURPOSE: Machine-readable capability registry — shipped commands, MCP tools, GRACE skill tools, verify checks, review modes, platforms
// SCOPE: Command list including RTK shortcuts, local RTK adapters, rewrite, and filters, core MCP tool list, GRACE skill tool list, requirements/technology/development-plan/mental-test/traceability/cascade/agent-testing/non-human pattern/typed LINKS/belief-state/anchor syntax/profile-aware verify/review check list, supported platforms
// DEPENDS: M-CLI, M-MCP, M-SKILLS-REGISTRY
// LINKS: README.md, docs/COMMANDS.md, docs/phases/Phase-27.xml, docs/phases/Phase-28.xml

// START_MODULE_MAP
// COMMANDS — All shipped CLI commands
// MCP_TOOLS — All registered MCP tools including 16 GRACE skills
// CORE_MCP_TOOLS — Base code/verification tools
// GRACE_SKILL_TOOLS — 16 first-class GRACE workflow tools
// VERIFY_CHECKS — All verification check names
// REVIEW_MODES — All review modes
// PLATFORMS — Supported OS/arch targets
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.26.0 - Added filters command capability parity]
// END_CHANGE_SUMMARY

use crate::skills::registry::{CORE_MCP_TOOLS, SKILL_DEFS};

// START_public_api

/// Shipped CLI commands (exactly matches Command enum)
pub const COMMANDS: &[(&str, &str)] = &[
    ("init", "Install Synapse hooks into current project"),
    ("index", "Index codebase for semantic search"),
    ("search", "Semantic code search"),
    ("view", "View file signatures"),
    ("verify", "Run profile-aware GRACE verification suite"),
    ("review", "Profile-aware GRACE integrity review"),
    ("status", "Project health report"),
    ("read", "Read files through the token-saving proxy"),
    (
        "ls",
        "List directory contents through the token-saving proxy",
    ),
    ("tree", "Show directory tree through the token-saving proxy"),
    ("find", "Find files through the token-saving proxy"),
    ("rg", "Search with ripgrep through the token-saving proxy"),
    ("grep", "Search with grep through the token-saving proxy"),
    ("git", "Run git through the token-saving proxy"),
    ("cargo", "Run cargo through the token-saving proxy"),
    ("npm", "Run npm through the token-saving proxy"),
    ("pnpm", "Run pnpm through the token-saving proxy"),
    ("npx", "Run npx through the token-saving proxy"),
    ("pytest", "Run pytest through the token-saving proxy"),
    (
        "json",
        "Inspect JSON with compact values or keys-only schema",
    ),
    (
        "deps",
        "Summarize dependency manifests without dumping full files",
    ),
    (
        "env",
        "Show filtered environment variables with secrets masked",
    ),
    ("wc", "Count text locally with compact wc-style output"),
    (
        "rewrite",
        "Rewrite a shell command to its Synapse proxy form for agent hooks",
    ),
    ("proxy", "Run command through token-saving proxy"),
    ("filters", "Verify and trust token-saving proxy filters"),
    ("gain", "View token savings analytics"),
    ("compress", "Compress files for AI context"),
    ("mcp", "Start MCP server"),
    ("config", "Manage configuration"),
    ("graphrag", "Query the code knowledge graph"),
    ("hooks", "Manage Synapse hooks for AI agents"),
    ("doctor", "Run diagnostic and dependency checks"),
    ("refresh", "Report or fix canonical MyGRACE artifact drift"),
    ("skills", "List and run GRACE workflow skills"),
    (
        "ci",
        "Run CI-friendly verification, review, and status commands",
    ),
    ("history", "Search git history for code changes"),
    ("serve", "Start web dashboard"),
];

pub const CORE_MCP_TOOL_COUNT: usize = 23;
pub const GRACE_SKILL_TOOL_COUNT: usize = 16;
pub const TOTAL_MCP_TOOL_COUNT: usize = CORE_MCP_TOOL_COUNT + GRACE_SKILL_TOOL_COUNT;

/// Registered base MCP tools (server registry source)
pub const CORE_TOOLS: &[(&str, &str)] = CORE_MCP_TOOLS;

/// Registered GRACE workflow skill tools
pub const GRACE_SKILL_TOOLS: &[(&str, &str)] = &[
    (
        "grace_init",
        "Initialize MyGrace-style sharded architecture artifacts for current project.",
    ),
    (
        "grace_plan",
        "Plan modules, phases, and architecture using sharded GRACE artifacts.",
    ),
    (
        "grace_verification",
        "Design or inspect verification strategy for modules and phases.",
    ),
    (
        "grace_execute",
        "Generate bounded execution guidance for active phase or module.",
    ),
    (
        "grace_multiagent_execute",
        "Produce multi-agent execution split for phase modules and responsibilities.",
    ),
    (
        "grace_reviewer",
        "Run or summarize GRACE review scope and integrity risks.",
    ),
    (
        "grace_refresh",
        "Refresh project artifacts against source code and surface drift.",
    ),
    (
        "grace_refactor",
        "Prepare refactor plan tied to architecture and verification artifacts.",
    ),
    (
        "grace_fix",
        "Diagnose a failure using modules, graph, verification, and code context.",
    ),
    (
        "grace_status",
        "Return project health, phase progress, and artifact coverage summary.",
    ),
    (
        "grace_run_history",
        "Return bounded autonomous run history and provenance events.",
    ),
    (
        "grace_ask",
        "Answer questions using project artifacts and indexed code context.",
    ),
    (
        "grace_explainer",
        "Explain code or architecture areas using artifacts and index context.",
    ),
    (
        "grace_cli",
        "Explain how to use Synapse and OpenCode CLI workflows for project tasks.",
    ),
    (
        "grace_setup_subagents",
        "Recommend planner, implementer, reviewer, verifier, and fixer subagent setup.",
    ),
    (
        "grace_lint",
        "Check sharded artifact integrity, consistency, and structural completeness.",
    ),
];

/// Registered MCP tools (exactly matches server tool registry)
pub const MCP_TOOLS: &[(&str, &str)] = &[
    ("semantic_search", "Search codebase by natural language"),
    (
        "view_signatures",
        "View function and class signatures in a file",
    ),
    (
        "graphrag_query",
        "Query the code knowledge graph with typed LINKS filters",
    ),
    (
        "verify_project",
        "Run profile-aware GRACE verification checks",
    ),
    ("review_code", "Run profile-aware GRACE integrity review"),
    ("project_status", "Full project health report"),
    (
        "analyze_logs",
        "Analyze structured GRACE LOG files for LDD trajectory and anomalies",
    ),
    (
        "extract_belief_state",
        "Create and validate an observable AI belief state artifact",
    ),
    (
        "generate_requirements",
        "Generate and validate a complete RequirementsAnalysis artifact",
    ),
    (
        "generate_technology",
        "Generate and validate a complete exact-version Technology artifact",
    ),
    (
        "generate_development_plan",
        "Generate and validate a complete DevelopmentPlan with DataFlows, GenerationOrder, and MentalTests",
    ),
    (
        "mental_test_run",
        "Run a DevelopmentPlan MentalTest before code generation",
    ),
    (
        "traceability_report",
        "Generate an end-to-end traceability matrix and gap report",
    ),
    (
        "cascade_impact",
        "Preview downstream artifact impact for a requirement, contract, interface, or implementation change",
    ),
    (
        "cascade_execute",
        "Execute a cached cascade preview and record proposals plus changelog",
    ),
    (
        "run_test_guide",
        "Run a natural-language GRACE testing guide and persist tester-agent reports",
    ),
    (
        "submit_test_report",
        "Submit a tester-agent XML failure report to the developer agent",
    ),
    ("token_savings", "View token savings analytics"),
    ("compress_text", "Compress text for AI context efficiency"),
    (
        "refresh_project",
        "Report or fix canonical MyGRACE artifact drift",
    ),
    (
        "suggest_contract",
        "Generate a language-aware MODULE_CONTRACT template",
    ),
    ("lsp_hover", "Get type/signature information via LSP"),
    ("lsp_references", "Find all references to a symbol via LSP"),
    (
        "grace_init",
        "Initialize MyGrace-style sharded architecture artifacts for current project.",
    ),
    (
        "grace_plan",
        "Plan modules, phases, and architecture using sharded GRACE artifacts.",
    ),
    (
        "grace_verification",
        "Design or inspect verification strategy for modules and phases.",
    ),
    (
        "grace_execute",
        "Generate bounded execution guidance for active phase or module.",
    ),
    (
        "grace_multiagent_execute",
        "Produce multi-agent execution split for phase modules and responsibilities.",
    ),
    (
        "grace_reviewer",
        "Run or summarize GRACE review scope and integrity risks.",
    ),
    (
        "grace_refresh",
        "Refresh project artifacts against source code and surface drift.",
    ),
    (
        "grace_refactor",
        "Prepare refactor plan tied to architecture and verification artifacts.",
    ),
    (
        "grace_fix",
        "Diagnose a failure using modules, graph, verification, and code context.",
    ),
    (
        "grace_status",
        "Return project health, phase progress, and artifact coverage summary.",
    ),
    (
        "grace_run_history",
        "Return bounded autonomous run history and provenance events.",
    ),
    (
        "grace_ask",
        "Answer questions using project artifacts and indexed code context.",
    ),
    (
        "grace_explainer",
        "Explain code or architecture areas using artifacts and index context.",
    ),
    (
        "grace_cli",
        "Explain how to use Synapse and OpenCode CLI workflows for project tasks.",
    ),
    (
        "grace_setup_subagents",
        "Recommend planner, implementer, reviewer, verifier, and fixer subagent setup.",
    ),
    (
        "grace_lint",
        "Check sharded artifact integrity, consistency, and structural completeness.",
    ),
];

/// Verification checks in module-local level
pub const VERIFY_CHECKS: &[&str] = &[
    "contract-exists",
    "contract-valid",
    "module-map",
    "change-summary",
    "function-contracts",
    "links-valid-types",
    "links-targets-exist",
    "links-no-dangling",
    "links-format",
    "semantic-blocks",
    "unique-block-names",
    "anchor-syntax-consistent",
    "500-token-rule",
    "trace-assertions",
    "structured-log-format",
    "belief-state-exists",
    "requirements-entities-defined",
    "requirements-use-cases",
    "requirements-glossary",
    "requirements-no-empty-sections",
    "technology-language-defined",
    "technology-dependencies-compatible",
    "technology-no-version-guessing",
    "technology-known-issues",
    "dataflow-sources-exist",
    "dataflow-contracts-exist",
    "dataflow-protocol-consistent",
    "dataflow-errors-documented",
    "genorder-topology-correct",
    "genorder-all-modules",
    "genorder-no-dangling-deps",
    "mental-tests-defined",
    "mental-tests-passed",
    "mental-test-no-drift",
    "traceability-requirements-implemented",
    "traceability-code-traced",
    "traceability-logs-traced",
    "traceability-no-dangling",
    "cascade-no-drift",
    "non-human-explicit-typing",
    "non-human-explicit-flow",
    "non-human-explicit-null",
    "non-human-no-magic-values",
    "non-human-deterministic-iter",
    "sharded-artifacts",
    "artifact-ref-integrity",
    "canonical-mygrace-drift",
];

/// Review modes
pub const REVIEW_MODES: &[&str] = &["scoped", "wave-audit", "full"];

/// Supported platform targets for release
pub const PLATFORMS: &[&str] = &[
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
];

pub const SKILL_NAMES: &[&str] = &[
    "grace_init",
    "grace_plan",
    "grace_verification",
    "grace_execute",
    "grace_multiagent_execute",
    "grace_reviewer",
    "grace_refresh",
    "grace_refactor",
    "grace_fix",
    "grace_status",
    "grace_run_history",
    "grace_ask",
    "grace_explainer",
    "grace_cli",
    "grace_setup_subagents",
    "grace_lint",
];

// START_CONTRACT_skill_defs_count
// PURPOSE: Return the number of registered GRACE skill definitions
// OUTPUTS: { usize — registered skill definition count }
pub fn skill_defs_count() -> usize {
    SKILL_DEFS.len()
}

// END_public_api
