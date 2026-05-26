// MODULE_CONTRACT
// MODULE_ID: M-CAPABILITIES
// PURPOSE: Machine-readable capability registry — shipped commands, MCP tools, GRACE skill tools, verify checks, review modes, platforms
// SCOPE: Command list including RTK shortcuts, local RTK adapters, .NET artifact adapters, core RTK adapters, session/economics adoption analytics, tracking-backed discover/learn diagnostics, hook processors and multi-agent hook install/audit targets, rewrite, and filters, 32-tool core MCP list, GRACE skill tool list, requirements/technology/development-plan/mental-test/traceability/cascade/agent-testing/run-control/token-economy/non-human pattern/typed LINKS/belief-state/anchor syntax/profile-aware verify/review check list, supported platforms
// DEPENDS: M-CLI, M-MCP, M-SKILLS-REGISTRY
// LINKS: README.md, docs/COMMANDS.md, docs/phases/Phase-27.xml, docs/phases/Phase-28.xml, docs/phases/Phase-49.xml, docs/phases/Phase-54.xml, docs/phases/Phase-56.xml

// START_MODULE_MAP
// COMMANDS — All shipped CLI commands
// MCP_TOOLS — All registered MCP tools including 16 GRACE skills
// CORE_MCP_TOOLS — Base code/verification tools
// GRACE_SKILL_TOOLS — 16 first-class GRACE workflow tools
// VERIFY_CHECKS — All verification check names
// RTK_DISCOVERY_CAPABILITIES — Discover/learn diagnostics surfaced to CLI, MCP, and status consumers
// REVIEW_MODES — All review modes
// PLATFORMS — Supported OS/arch targets
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.37.0 - Synchronized MCP capability counts to 48 tools]
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
    ("gt", "Run Graphite CLI through the token-saving proxy"),
    ("cargo", "Run cargo through the token-saving proxy"),
    ("npm", "Run npm through the token-saving proxy"),
    ("pnpm", "Run pnpm through the token-saving proxy"),
    ("npx", "Run npx through the token-saving proxy"),
    ("pytest", "Run pytest through the token-saving proxy"),
    ("ruff", "Run ruff through the token-saving proxy"),
    ("mypy", "Run mypy through the token-saving proxy"),
    (
        "basedpyright",
        "Run basedpyright through the token-saving proxy",
    ),
    ("pip", "Run pip through the token-saving proxy"),
    ("uv", "Run uv through the token-saving proxy"),
    ("next", "Run Next.js tooling through the token-saving proxy"),
    (
        "playwright",
        "Run Playwright through the token-saving proxy",
    ),
    ("prettier", "Run Prettier through the token-saving proxy"),
    ("prisma", "Run Prisma through the token-saving proxy"),
    (
        "tsc",
        "Run TypeScript compiler through the token-saving proxy",
    ),
    ("vitest", "Run Vitest through the token-saving proxy"),
    ("jest", "Run Jest through the token-saving proxy"),
    ("lint", "Run ESLint through the token-saving proxy"),
    ("format", "Run Prettier through the token-saving proxy"),
    ("gh", "Run GitHub CLI through the token-saving proxy"),
    ("glab", "Run GitLab CLI through the token-saving proxy"),
    ("aws", "Run AWS CLI through the token-saving proxy"),
    ("psql", "Run psql through the token-saving proxy"),
    ("curl", "Run curl through the token-saving proxy"),
    ("wget", "Run wget through the token-saving proxy"),
    ("jq", "Run jq through the token-saving proxy"),
    ("go", "Run Go tooling through the token-saving proxy"),
    (
        "golangci",
        "Run golangci-lint through the token-saving proxy",
    ),
    ("dotnet", "Run dotnet through the token-saving proxy"),
    ("rake", "Run rake through the token-saving proxy"),
    ("rspec", "Run rspec through the token-saving proxy"),
    ("rubocop", "Run rubocop through the token-saving proxy"),
    ("gradle", "Run Gradle through the token-saving proxy"),
    (
        "gradlew",
        "Run local Gradle wrapper through the token-saving proxy",
    ),
    ("make", "Run make through the token-saving proxy"),
    ("just", "Run just through the token-saving proxy"),
    ("helm", "Run Helm through the token-saving proxy"),
    ("kubectl", "Run kubectl through the token-saving proxy"),
    ("docker", "Run Docker through the token-saving proxy"),
    ("podman", "Run Podman through the token-saving proxy"),
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
    ("err", "Run a command and show only errors and warnings"),
    ("test", "Run tests and show compact failure output"),
    ("diff", "Summarize file or unified diff output"),
    ("summary", "Summarize text from a file or stdin"),
    ("pipe", "Filter stdin through Synapse RTK filters"),
    (
        "log",
        "Deduplicate and summarize log output from a file or stdin",
    ),
    (
        "smart",
        "Summarize source file structure without printing full code",
    ),
    (
        "discover",
        "Discover routeable token-heavy commands, local missed-route history, and Synapse replacements",
    ),
    (
        "learn",
        "Show measured RTK learning guidance for recurring misses",
    ),
    ("binlog", "Summarize MSBuild binary log diagnostics"),
    (
        "dotnet-format-report",
        "Summarize dotnet format JSON reports",
    ),
    ("dotnet-trx", "Summarize dotnet TRX test result files"),
    (
        "session",
        "Show RTK adoption, route coverage, and token savings by tracked Synapse session",
    ),
    (
        "cc-economics",
        "Show local Claude Code token economics and route adoption from Synapse tracking",
    ),
    (
        "rtk-parity",
        "Report machine-checkable RTK parity inventory",
    ),
    (
        "rewrite",
        "Rewrite a shell command to its Synapse proxy form for agent hooks",
    ),
    (
        "hook",
        "Process RTK-style agent hook JSON or dry-run rewrites",
    ),
    ("proxy", "Run command through token-saving proxy"),
    ("filters", "Verify and trust token-saving proxy filters"),
    ("gain", "View token savings analytics"),
    ("compress", "Compress files for AI context"),
    ("mcp", "Start MCP server"),
    ("config", "Manage configuration"),
    ("graphrag", "Query the code knowledge graph"),
    (
        "hooks",
        "Manage Synapse hooks for OpenCode, Claude, Cursor, Gemini, Copilot, or all agents",
    ),
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

pub const CORE_MCP_TOOL_COUNT: usize = CORE_MCP_TOOLS.len();
pub const GRACE_SKILL_TOOL_COUNT: usize = 16;
pub const TOTAL_MCP_TOOL_COUNT: usize = CORE_MCP_TOOL_COUNT + GRACE_SKILL_TOOL_COUNT;

pub const RTK_DISCOVERY_CAPABILITIES: &[(&str, &str)] = &[
    (
        "discover",
        "Route-aware missed-opportunity diagnostics for raw commands and built-in adapter examples.",
    ),
    (
        "learn",
        "Bounded learning guidance for recurring token-saving misses without scraping private session history.",
    ),
];

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
    (
        "self_heal",
        "Run one bounded self-heal iteration for a persisted autonomous run",
    ),
    (
        "advance_phase",
        "Check active MyGRACE phase gates and optionally advance to the next planned phase",
    ),
    (
        "pre_commit_check",
        "Run pre-commit verification for a persisted bounded run before final completion",
    ),
    ("token_savings", "View token savings analytics"),
    ("compress_text", "Compress text for AI context efficiency"),
    (
        "refresh_project",
        "Report or fix canonical MyGRACE artifact drift",
    ),
    (
        "diagnose_failure",
        "Parse tester-agent failure evidence and return a bounded fix diagnosis",
    ),
    (
        "repair_contract",
        "Generate or apply a safe language-aware MODULE_CONTRACT repair",
    ),
    (
        "suggest_contract",
        "Generate a language-aware MODULE_CONTRACT template",
    ),
    ("lsp_hover", "Get type/signature information via LSP"),
    ("lsp_references", "Find all references to a symbol via LSP"),
    (
        "tools/recommend",
        "Recommend a bounded context-relevant MCP tool subset before listing schemas",
    ),
    (
        "compact_evidence",
        "Deduplicate, alias, and truncate evidence refs for one persisted run",
    ),
    (
        "check_budget",
        "Check current session token budget and estimated-token affordability",
    ),
    (
        "context_pressure",
        "Check current session context-window pressure and new-session recommendation",
    ),
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

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_capabilities_include_rtk_discover_learn_metadata
    // PURPOSE: Verify Phase-49 discover/learn commands and metadata are published together.
    // OUTPUTS: { () }
    // START_capabilities_include_rtk_discover_learn_metadata
    #[test]
    fn capabilities_include_rtk_discover_learn_metadata() {
        let commands = COMMANDS.iter().map(|(name, _)| *name).collect::<Vec<_>>();
        assert!(commands.contains(&"discover"));
        assert!(commands.contains(&"learn"));
        assert!(commands.contains(&"docker"));
        assert!(commands.contains(&"podman"));

        let capabilities = RTK_DISCOVERY_CAPABILITIES
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>();
        assert_eq!(capabilities, vec!["discover", "learn"]);
    }
    // END_capabilities_include_rtk_discover_learn_metadata
}

// END_public_api
