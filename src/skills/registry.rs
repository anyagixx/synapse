// MODULE_CONTRACT
// MODULE_ID: M-SKILLS-REGISTRY
// PURPOSE: Skill registry — declares 15 first-class GRACE skill tools for MCP exposure
// SCOPE: Skill metadata constants, built-in MCP tool descriptions, and lookup helpers
// DEPENDS: M-SKILLS-TYPES
// LINKS: M-SKILLS

// START_MODULE_MAP
// SKILL_DEFS — Static list of all GRACE skill tools
// find_skill — Lookup skill metadata by name
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.13.0 — Documented extract_belief_state MCP capability]
// END_CHANGE_SUMMARY

use super::types::{SkillArg, SkillDef};

// START_public_api

const QUESTION_ARG: &[SkillArg] = &[SkillArg {
    name: "question",
    description: "Question to answer from project artifacts and code",
    required: true,
}];
const DETAIL_LEVEL_ARG: &[SkillArg] = &[SkillArg {
    name: "detail_level",
    description: "Output detail level: summary | standard | deep",
    required: false,
}];
const SCOPE_ARG: &[SkillArg] = &[SkillArg {
    name: "scope",
    description: "Target scope such as module, phase, or project",
    required: false,
}];
const TARGET_ARG: &[SkillArg] = &[SkillArg {
    name: "target",
    description: "Target module, phase, file, or subsystem",
    required: false,
}];
const ISSUE_ARG: &[SkillArg] = &[
    SkillArg {
        name: "issue",
        description: "Bug, failure, or integrity problem to diagnose",
        required: true,
    },
    SkillArg {
        name: "module_hint",
        description: "Optional module hint like M-MCP or M-CLI",
        required: false,
    },
];
const INIT_ARGS: &[SkillArg] = &[
    SkillArg {
        name: "project_name",
        description: "Project name",
        required: false,
    },
    SkillArg {
        name: "description",
        description: "Short project description",
        required: false,
    },
    SkillArg {
        name: "language",
        description: "Primary language such as rust, ts, python, or go",
        required: false,
    },
];
const PLAN_ARGS: &[SkillArg] = &[
    SkillArg {
        name: "goal",
        description: "Architecture or delivery goal",
        required: true,
    },
    SkillArg {
        name: "constraints",
        description: "Important constraints or boundaries",
        required: false,
    },
];
const EXECUTE_ARGS: &[SkillArg] = &[
    SkillArg {
        name: "phase",
        description: "Phase ID to execute",
        required: false,
    },
    SkillArg {
        name: "module_id",
        description: "Module ID to execute",
        required: false,
    },
    SkillArg {
        name: "objective",
        description: "Immediate objective for this execution step",
        required: false,
    },
];
const MULTIAGENT_ARGS: &[SkillArg] = &[
    SkillArg {
        name: "phase",
        description: "Phase ID",
        required: false,
    },
    SkillArg {
        name: "modules",
        description: "Comma-separated module IDs",
        required: false,
    },
    SkillArg {
        name: "execution_policy",
        description: "Execution policy for multiple workers",
        required: false,
    },
];
const VERIFICATION_ARGS: &[SkillArg] = &[
    SkillArg {
        name: "module_id",
        description: "Module ID for verification planning",
        required: false,
    },
    SkillArg {
        name: "priority",
        description: "Verification priority",
        required: false,
    },
];
const REFRESH_ARGS: &[SkillArg] = &[SkillArg {
    name: "sync_mode",
    description: "Sync mode: report | repair | full",
    required: false,
}];
const REFACTOR_ARGS: &[SkillArg] = &[
    SkillArg {
        name: "target",
        description: "Refactor target",
        required: true,
    },
    SkillArg {
        name: "intent",
        description: "Refactor intent",
        required: false,
    },
];
const CLI_ARGS: &[SkillArg] = &[SkillArg {
    name: "topic",
    description: "CLI topic, command, or workflow to explain",
    required: false,
}];
const SUBAGENT_ARGS: &[SkillArg] = &[
    SkillArg {
        name: "roles",
        description: "Requested role set or worker roles",
        required: false,
    },
    SkillArg {
        name: "platform",
        description: "Target agent platform, such as opencode",
        required: false,
    },
];

pub const SKILL_DEFS: &[SkillDef] = &[
    SkillDef {
        name: "grace_init",
        description: "Initialize MyGrace-style sharded architecture artifacts for current project.",
        args: INIT_ARGS,
    },
    SkillDef {
        name: "grace_plan",
        description: "Plan modules, phases, and architecture using sharded GRACE artifacts.",
        args: PLAN_ARGS,
    },
    SkillDef {
        name: "grace_verification",
        description: "Design or inspect verification strategy for modules and phases.",
        args: VERIFICATION_ARGS,
    },
    SkillDef {
        name: "grace_execute",
        description: "Generate bounded execution guidance for active phase or module.",
        args: EXECUTE_ARGS,
    },
    SkillDef {
        name: "grace_multiagent_execute",
        description: "Produce multi-agent execution split for phase modules and responsibilities.",
        args: MULTIAGENT_ARGS,
    },
    SkillDef {
        name: "grace_reviewer",
        description: "Run or summarize GRACE review scope and integrity risks.",
        args: SCOPE_ARG,
    },
    SkillDef {
        name: "grace_refresh",
        description: "Refresh project artifacts against source code and surface drift.",
        args: REFRESH_ARGS,
    },
    SkillDef {
        name: "grace_refactor",
        description: "Prepare refactor plan tied to architecture and verification artifacts.",
        args: REFACTOR_ARGS,
    },
    SkillDef {
        name: "grace_fix",
        description: "Diagnose a failure using modules, graph, verification, and code context.",
        args: ISSUE_ARG,
    },
    SkillDef {
        name: "grace_status",
        description: "Return project health, phase progress, and artifact coverage summary.",
        args: DETAIL_LEVEL_ARG,
    },
    SkillDef {
        name: "grace_ask",
        description: "Answer questions using project artifacts and indexed code context.",
        args: QUESTION_ARG,
    },
    SkillDef {
        name: "grace_explainer",
        description: "Explain code or architecture areas using artifacts and index context.",
        args: TARGET_ARG,
    },
    SkillDef {
        name: "grace_cli",
        description: "Explain how to use Synapse and OpenCode CLI workflows for project tasks.",
        args: CLI_ARGS,
    },
    SkillDef {
        name: "grace_setup_subagents",
        description:
            "Recommend planner, implementer, reviewer, verifier, and fixer subagent setup.",
        args: SUBAGENT_ARGS,
    },
    SkillDef {
        name: "grace_lint",
        description: "Check sharded artifact integrity, consistency, and structural completeness.",
        args: SCOPE_ARG,
    },
];

// START_CONTRACT_find_skill
// PURPOSE: Find skill definition by exact name
// INPUTS: { name: &str — skill name }
// OUTPUTS: { Option<SkillDef> — matching skill metadata }
// START_find_skill
pub fn find_skill(name: &str) -> Option<SkillDef> {
    SKILL_DEFS.iter().copied().find(|skill| skill.name == name)
}
// END_find_skill

pub const GRACE_TOOL_COUNT: usize = 15;

pub const CORE_MCP_TOOLS: &[(&str, &str)] = &[
    ("semantic_search", "Search codebase by natural language"),
    (
        "view_signatures",
        "View function and class signatures in a file",
    ),
    (
        "graphrag_query",
        "Query the code knowledge graph with typed LINKS filters",
    ),
    ("verify_project", "Run GRACE verification checks"),
    ("review_code", "Run GRACE integrity review"),
    ("project_status", "Full project health report"),
    (
        "analyze_logs",
        "Analyze structured GRACE LOG files for LDD trajectory and anomalies",
    ),
    (
        "extract_belief_state",
        "Create and validate an observable AI belief state artifact",
    ),
    ("token_savings", "View token savings analytics"),
    ("compress_text", "Compress text for AI context efficiency"),
    (
        "refresh_project",
        "Sync knowledge graph and verification plan with code",
    ),
    ("suggest_contract", "Generate a MODULE_CONTRACT template"),
    ("lsp_hover", "Get type/signature information via LSP"),
    ("lsp_references", "Find all references to a symbol via LSP"),
];

// END_public_api
