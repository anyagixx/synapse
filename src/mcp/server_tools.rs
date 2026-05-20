// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-TOOLS
// PURPOSE: MCP tool definition registry for Synapse built-in and MyGRACE skill tools
// SCOPE: Static JSON schema definitions for tools/list including GRACE profiles and requirements/technology/development-plan generation
// DEPENDS: M-SKILLS-REGISTRY
// LINKS: docs/modules/M-MCP-SERVER.xml

// START_MODULE_MAP
// tool_definitions — Builds the tools/list payload
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.18.0 — Added mental_test_run MCP tool schema]
// END_CHANGE_SUMMARY

use crate::skills::registry::SKILL_DEFS;

// START_public_api

// START_CONTRACT_tool_definitions
// PURPOSE: Build MCP tools/list definitions for built-in tools and registered grace_* skills
// OUTPUTS: { Vec<serde_json::Value> }
// START_tool_definitions
pub(crate) fn tool_definitions() -> Vec<serde_json::Value> {
    let mut tools = vec![
        serde_json::json!({
            "name": "semantic_search",
            "description": "Search codebase by natural language. Returns code blocks with paths and line numbers.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" },
                    "max_results": { "type": "number", "default": 10 }
                },
                "required": ["query"]
            }
        }),
        serde_json::json!({
            "name": "view_signatures",
            "description": "View function and class signatures in a file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" }
                },
                "required": ["path"]
            }
        }),
        serde_json::json!({
            "name": "graphrag_query",
            "description": "Query the code knowledge graph. Supports: search, get-node, get-relationships, find-path, dependents, tracedown, overview",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "operation": { "type": "string", "description": "Operation: search | get-node | get-relationships | find-path | dependents | tracedown | overview" },
                    "query": { "type": "string", "description": "Search query" },
                    "node_id": { "type": "string", "description": "Node ID" },
                    "from": { "type": "string", "description": "Source node ID" },
                    "to": { "type": "string", "description": "Target node ID" },
                    "target": { "type": "string", "description": "Target artifact for dependents/tracedown" },
                    "link_type": { "type": "string", "description": "Typed LINKS filter: implements | depends | refines | traces_to | verified_by | manages | uses" }
                },
                "required": ["operation"]
            }
        }),
        serde_json::json!({
            "name": "verify_project",
            "description": "Run GRACE verification checks (module-local, wave, phase). Returns pass/fail status with details. Use profile=lite|balanced|strict to tune contract strictness.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "level": { "type": "string", "description": "Verification level: module-local | wave | phase | all" },
                    "profile": { "type": "string", "description": "Strictness profile: lite | balanced | strict" }
                }
            }
        }),
        serde_json::json!({
            "name": "review_code",
            "description": "Run GRACE integrity review — checks semantic markup, contracts, naming, sensitive data. Returns issues list. Use profile=lite|balanced|strict for small projects.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "mode": { "type": "string", "description": "Review mode: scoped | wave-audit | full" },
                    "profile": { "type": "string", "description": "Strictness profile: lite | balanced | strict" }
                }
            }
        }),
        serde_json::json!({
            "name": "project_status",
            "description": "Full project health report — contracts, semantic markup, verification, token economy, system info.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        serde_json::json!({
            "name": "analyze_logs",
            "description": "Analyze structured GRACE LOG files in trajectory, anomaly, or compare mode.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "log_path": { "type": "string", "description": "Path to a structured LOG file" },
                    "mode": { "type": "string", "description": "trajectory | anomaly | compare", "default": "trajectory" },
                    "contract_ref": { "type": "string", "description": "Optional module id or function contract name filter" }
                },
                "required": ["log_path"]
            }
        }),
        serde_json::json!({
            "name": "extract_belief_state",
            "description": "Create and validate a docs/belief-states artifact for a module before code generation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "module_id": { "type": "string", "description": "Module ID such as M-ORDER-SERVICE" },
                    "context": { "type": "string", "description": "Execution context read before generating code" }
                },
                "required": ["module_id"]
            }
        }),
        serde_json::json!({
            "name": "generate_requirements",
            "description": "Generate and validate a complete RequirementsAnalysis.xml artifact with AAG use cases.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_description": { "type": "string", "description": "Brief description of the project" },
                    "domain": { "type": "string", "description": "Domain such as retail, developer tooling, finance" },
                    "detail_level": { "type": "string", "description": "quick | standard | detailed", "default": "standard" }
                },
                "required": ["project_description"]
            }
        }),
        serde_json::json!({
            "name": "generate_technology",
            "description": "Generate and validate a complete Technology.xml artifact with exact versions and compatibility checks.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_path": { "type": "string", "description": "Project path to scan", "default": "." },
                    "detect_existing": { "type": "boolean", "description": "Scan dependency manifests such as Cargo.toml, package.json, requirements.txt, and go.mod", "default": true },
                    "compatibility_check": { "type": "boolean", "description": "Emit compatibility matrix checks", "default": true }
                }
            }
        }),
        serde_json::json!({
            "name": "generate_development_plan",
            "description": "Generate and validate a complete DevelopmentPlan.xml artifact with DataFlows, GenerationOrder, and MentalTests.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from_requirements": { "type": "boolean", "description": "Read RequirementsAnalysis when generating the plan", "default": true },
                    "data_flow_analysis": { "type": "string", "description": "auto | guided | manual", "default": "auto" },
                    "generation_order": { "type": "string", "description": "topological | phase | manual", "default": "topological" }
                }
            }
        }),
        serde_json::json!({
            "name": "mental_test_run",
            "description": "Run a DevelopmentPlan MentalTest before code generation and persist a trace artifact.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "module_id": { "type": "string", "description": "Target module id such as M-GRACE-MENTAL-TEST" },
                    "mental_test_id": { "type": "string", "description": "Mental test id such as MT-001" },
                    "step_by_step": { "type": "boolean", "description": "Return step-by-step trace behavior", "default": true }
                },
                "required": ["module_id", "mental_test_id"]
            }
        }),
        serde_json::json!({
            "name": "token_savings",
            "description": "View token savings analytics — total commands, tokens saved, average savings %, estimated cost saved.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        serde_json::json!({
            "name": "compress_text",
            "description": "Compress text for AI context efficiency. Levels: lite (remove filler), full (also pleasantries), ultra (telegraphic).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to compress" },
                    "level": { "type": "string", "description": "Compression level: lite | full | ultra" }
                },
                "required": ["text"]
            }
        }),
        serde_json::json!({
            "name": "refresh_project",
            "description": "Report or fix canonical MyGRACE drift between code contracts and sharded artifacts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "fix": { "type": "boolean", "description": "When true, rewrite canonical indexes and shards from source MODULE_ID contracts" }
                }
            }
        }),
        serde_json::json!({
            "name": "suggest_contract",
            "description": "Generate a language-aware MODULE_CONTRACT template for a new module. Provide module name, purpose, and optional language.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "module_name": { "type": "string", "description": "Module name (e.g. 'auth')" },
                    "purpose": { "type": "string", "description": "What the module does" },
                    "language": { "type": "string", "description": "Language or extension: rust | python | sql | ts | go" }
                },
                "required": ["module_name", "purpose"]
            }
        }),
        serde_json::json!({
            "name": "lsp_hover",
            "description": "Get type/signature information at a position via LSP.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path" },
                    "line": { "type": "number", "description": "Line (1-based)" },
                    "column": { "type": "number", "description": "Column (0-based)" }
                },
                "required": ["file", "line", "column"]
            }
        }),
        serde_json::json!({
            "name": "lsp_references",
            "description": "Find all references to a symbol at a position via LSP.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path" },
                    "line": { "type": "number", "description": "Line (1-based)" },
                    "column": { "type": "number", "description": "Column (0-based)" }
                },
                "required": ["file", "line", "column"]
            }
        }),
    ];

    for skill in SKILL_DEFS {
        tools.push(serde_json::json!({
            "name": skill.name,
            "description": skill.description,
            "inputSchema": skill.input_schema(),
        }));
    }

    tools
}
// END_tool_definitions

// END_public_api
