// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-TOOLS
// PURPOSE: MCP tool definition registry for Synapse built-in and MyGRACE skill tools
// SCOPE: Static JSON schema definitions for tools/list including semantic_search filters, GRACE profiles, requirements/technology/development-plan generation, traceability reporting, cascade updates, and agent-based testing
// DEPENDS: M-SKILLS-REGISTRY
// LINKS: docs/modules/M-MCP-SERVER.xml

// START_MODULE_MAP
// tool_definitions — Builds the tools/list payload
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.8.0 - Added semantic_search filter schema]
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
                    "max_results": { "type": "number", "default": 10 },
                    "language": { "type": "string", "description": "Optional language filter such as rust, python, typescript, or javascript" },
                    "path": { "type": "string", "description": "Optional project-relative path prefix filter" },
                    "path_contains": { "type": "string", "description": "Optional project-relative path substring filter" }
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
            "name": "traceability_report",
            "description": "Generate an end-to-end traceability report from requirements/use cases to modules, functions, blocks, and LOG evidence.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "scope": { "type": "string", "description": "project | module | requirement", "default": "project" },
                    "target": { "type": "string", "description": "Required for module or requirement scope, such as M-ORDER or REQ-001" },
                    "direction": { "type": "string", "description": "up | down", "default": "up" }
                }
            }
        }),
        serde_json::json!({
            "name": "cascade_impact",
            "description": "Preview downstream artifact impact for a requirement, contract, interface, or implementation change.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "changed_artifact": { "type": "string", "description": "Changed artifact id such as UC-001, REQ-001, M-ORDER, or M-ORDER::place_order" },
                    "change_description": { "type": "string", "description": "Short description of what changed" },
                    "preview_only": { "type": "boolean", "description": "When false, execute the cascade immediately after preview", "default": true }
                },
                "required": ["changed_artifact"]
            }
        }),
        serde_json::json!({
            "name": "cascade_execute",
            "description": "Execute a cached cascade preview, write proposals, and record a cascade changelog.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cascade_id": { "type": "string", "description": "Cascade id returned by cascade_impact, such as CSC-1234ABCD" },
                    "auto_apply_contracts": { "type": "boolean", "description": "Allow safe contract proposal updates to be marked applied", "default": true },
                    "auto_apply_code": { "type": "boolean", "description": "Allow code regeneration to be marked applied. Defaults to false for human review.", "default": false },
                    "apply_to_phases": { "type": "array", "items": { "type": "string" }, "description": "Optional phase ids to scope proposal output" }
                },
                "required": ["cascade_id"]
            }
        }),
        serde_json::json!({
            "name": "run_test_guide",
            "description": "Run a natural-language GRACE testing guide and persist tester-agent summary/failure artifacts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guide_path": { "type": "string", "description": "Path to docs/tests/guides/*.md" },
                    "application_url": { "type": "string", "description": "Application URL or mock://pass|mock://fail", "default": "mock://pass" },
                    "agent_console_url": { "type": "string", "description": "Optional embedded agent console URL" },
                    "collect_logs": { "type": "boolean", "description": "Collect LOG evidence placeholders", "default": true },
                    "output_report": { "type": "boolean", "description": "Write XML failure report when deviations are found", "default": true }
                },
                "required": ["guide_path"]
            }
        }),
        serde_json::json!({
            "name": "submit_test_report",
            "description": "Submit a tester-agent XML failure report to the developer agent and highlight LOG evidence refs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "report": { "type": "string", "description": "Path to a docs/tests/results/* failure XML report" },
                    "to": { "type": "string", "description": "Recipient agent name", "default": "developer" }
                },
                "required": ["report"]
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

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_semantic_search_schema_exposes_filters
    // PURPOSE: Verify tools/list declares semantic_search language and path filters for MCP clients
    // START_test_semantic_search_schema_exposes_filters
    #[test]
    fn test_semantic_search_schema_exposes_filters() {
        let tools = tool_definitions();
        let semantic_search = tools
            .iter()
            .find(|tool| tool["name"] == "semantic_search")
            .expect("semantic_search tool");
        let properties = semantic_search["inputSchema"]["properties"]
            .as_object()
            .expect("properties");

        assert!(properties.contains_key("language"));
        assert!(properties.contains_key("path"));
        assert!(properties.contains_key("path_contains"));
    }
    // END_test_semantic_search_schema_exposes_filters
}

// END_public_api
