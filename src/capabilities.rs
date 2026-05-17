// MODULE_CONTRACT
// MODULE_ID: M-CAPABILITIES
// PURPOSE: Single source of truth — machine-readable registry of shipped commands, MCP tools, platforms
// SCOPE: Command list, MCP tool list, verify check list, review mode list, supported platforms
// DEPENDS: M-CLI, M-MCP
// LINKS: README.md, docs/COMMANDS.md

// START_MODULE_MAP
// COMMANDS — All shipped CLI commands
// MCP_TOOLS — All registered MCP tools
// VERIFY_CHECKS — All verification check names
// REVIEW_MODES — All review modes
// PLATFORMS — Supported OS/arch targets
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.2.0 — Initial capability registry]
// END_CHANGE_SUMMARY

// START_public_api

/// Shipped CLI commands (exactly matches Command enum)
pub const COMMANDS: &[(&str, &str)] = &[
    ("init", "Install Synapse hooks into current project"),
    ("index", "Index codebase for semantic search"),
    ("search", "Semantic code search"),
    ("view", "View file signatures"),
    ("verify", "Run GRACE verification suite"),
    ("review", "GRACE integrity review"),
    ("status", "Project health report"),
    ("proxy", "Run command through token-saving proxy"),
    ("gain", "View token savings analytics"),
    ("compress", "Compress files for AI context"),
    ("mcp", "Start MCP server"),
    ("graphrag", "Query the code knowledge graph"),
    ("config", "Manage configuration"),
    ("hooks", "Manage Synapse hooks for AI agents"),
    ("doctor", "Run diagnostic checks"),
    (
        "refresh",
        "Sync knowledge graph and verification plan with code",
    ),
    ("history", "Search git history for code changes"),
    ("serve", "Start web dashboard"),
];

/// Registered MCP tools (exactly matches server tool registry)
pub const MCP_TOOLS: &[(&str, &str)] = &[
    ("semantic_search", "Search codebase by natural language"),
    (
        "view_signatures",
        "View function and class signatures in a file",
    ),
    ("graphrag_query", "Query the code knowledge graph"),
    ("verify_project", "Run GRACE verification checks"),
    ("review_code", "Run GRACE integrity review"),
    ("project_status", "Full project health report"),
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

/// Verification checks in module-local level
pub const VERIFY_CHECKS: &[&str] = &[
    "contract-exists",
    "contract-valid",
    "module-map",
    "change-summary",
    "function-contracts",
    "semantic-blocks",
    "unique-block-names",
    "500-token-rule",
    "trace-assertions",
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

// END_public_api
