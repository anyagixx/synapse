// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-TOOLS
// PURPOSE: MCP tool definition registry for Synapse built-in and MyGRACE skill tools
// SCOPE: Static JSON schema definitions for tools/list including tools/recommend, response economy parameters, cache validator hints, semantic_search filters, GraphRAG impact/Mermaid options, LSP content override options, progressive tool disclosure profiles, GRACE profiles, self_heal, advance_phase, pre_commit_check, diagnose_failure, repair_contract, requirements/technology/development-plan generation, traceability reporting, cascade updates, and agent-based testing
// DEPENDS: M-SKILLS-REGISTRY
// LINKS:
//   -> docs/modules/M-MCP-SERVER.xml (depends) - MCP server parent module
//   -> UC-002 (implements) - exposes verified AI engineering workflows to MCP clients
//   -> NFR-003 (traces_to) - schema discovery reduces repeated context reconstruction

// START_MODULE_MAP
// tool_definitions — Builds the tools/list payload
// ToolProfile — Workflow-oriented MCP tool visibility profile
// ToolProfile::parse — Parses profile arguments for tools/list
// ToolSchemaStyle — MCP tools/list schema verbosity style
// ToolSchemaStyle::parse — Parses tools/list style arguments
// tool_profile_membership — Returns workflow profiles for one tool
// tool_matches_profile — Checks whether a tool is visible for a requested profile
// tool_definitions_for_profile — Builds profile-filtered tool definitions
// apply_tool_schema_style — Applies full or terse schema style
// terse_tool_definition — Removes description fields recursively
// tool_definitions_json_bytes — Estimates serialized schema size
// response_economy_tool_names — Returns tools that support max_tokens and style
// tool_supports_response_economy — Checks response economy schema membership
// add_response_economy_schemas — Adds max_tokens/style schema fields to selected tools
// add_response_economy_schema — Adds response economy fields to one tool definition
// add_cache_hint_schema — Adds one optional cache validator field to one tool definition
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v4.9.0 - Added tools/recommend schema]
// END_CHANGE_SUMMARY

use crate::skills::registry::SKILL_DEFS;

const RESPONSE_ECONOMY_TOOL_NAMES: &[&str] = &[
    "semantic_search",
    "graphrag_query",
    "view_signatures",
    "lsp_hover",
    "lsp_references",
    "project_status",
    "verify_project",
    "review_code",
    "analyze_logs",
    "mental_test_run",
    "traceability_report",
    "token_savings",
    "refresh_project",
    "cascade_impact",
    "cascade_execute",
];

const CACHE_HINT_TOOL_NAMES: &[&str] = &[
    "graphrag_query",
    "view_signatures",
    "lsp_hover",
    "lsp_references",
    "project_status",
    "traceability_report",
];

// START_public_api

// START_ToolProfile
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolProfile {
    All,
    Verification,
    Planning,
    Implementation,
    Debugging,
    Minimal,
    Custom(Vec<String>),
}
// END_ToolProfile

impl ToolProfile {
    // START_CONTRACT_ToolProfile::parse
    // PURPOSE: Parse a tools/list profile argument into a stable workflow profile
    // INPUTS: { value: &str }
    // OUTPUTS: { ToolProfile }
    // START_tool_profile_parse
    pub(crate) fn parse(value: &str) -> Self {
        let trimmed = value.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "" | "all" => Self::All,
            "verify" | "verification" => Self::Verification,
            "plan" | "planning" => Self::Planning,
            "impl" | "implementation" => Self::Implementation,
            "debug" | "debugging" => Self::Debugging,
            "minimal" => Self::Minimal,
            custom if custom.starts_with("custom:") => {
                let tools = trimmed["custom:".len()..]
                    .split(',')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(ToOwned::to_owned)
                    .collect();
                Self::Custom(tools)
            }
            _ => Self::All,
        }
    }
    // END_tool_profile_parse
}

// START_ToolSchemaStyle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolSchemaStyle {
    Full,
    Terse,
}
// END_ToolSchemaStyle

impl ToolSchemaStyle {
    // START_CONTRACT_ToolSchemaStyle::parse
    // PURPOSE: Parse a tools/list style argument into a stable schema verbosity style
    // INPUTS: { value: &str }
    // OUTPUTS: { ToolSchemaStyle }
    // START_tool_schema_style_parse
    pub(crate) fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "terse" => Self::Terse,
            _ => Self::Full,
        }
    }
    // END_tool_schema_style_parse

    // START_CONTRACT_ToolSchemaStyle::label
    // PURPOSE: Return response metadata label for a tools/list schema style
    // OUTPUTS: { &'static str }
    // START_tool_schema_style_label
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Terse => "terse",
        }
    }
    // END_tool_schema_style_label
}

// START_CONTRACT_tool_profile_membership
// PURPOSE: Return explicit workflow profiles where a tool is useful
// INPUTS: { tool_name: &str }
// OUTPUTS: { &'static [ToolProfile] }
// START_tool_profile_membership
pub(crate) fn tool_profile_membership(tool_name: &str) -> &'static [ToolProfile] {
    use ToolProfile::*;
    match tool_name {
        "verify_project"
        | "review_code"
        | "project_status"
        | "traceability_report"
        | "pre_commit_check"
        | "refresh_project"
        | "grace_reviewer"
        | "grace_lint" => &[Verification],
        "analyze_logs" => &[Verification, Debugging],
        "mental_test_run" => &[Verification, Planning, Implementation],

        "generate_requirements"
        | "generate_technology"
        | "generate_development_plan"
        | "grace_init"
        | "grace_plan"
        | "grace_verification" => &[Planning],
        "cascade_impact" => &[Planning, Implementation],
        "extract_belief_state" => &[Planning, Implementation],

        "semantic_search"
        | "graphrag_query"
        | "view_signatures"
        | "suggest_contract"
        | "lsp_hover"
        | "lsp_references"
        | "grace_execute"
        | "grace_multiagent_execute"
        | "grace_refactor"
        | "grace_ask"
        | "grace_explainer"
        | "grace_cli"
        | "grace_setup_subagents"
        | "grace_status" => &[Implementation],
        "cascade_execute" => &[Planning, Implementation],
        "compress_text" | "token_savings" | "grace_refresh" | "grace_run_history" => {
            &[Implementation]
        }

        "run_test_guide" | "submit_test_report" | "self_heal" | "diagnose_failure"
        | "repair_contract" | "grace_fix" => &[Debugging, Implementation],
        "advance_phase" => &[Implementation],
        "tools/recommend" => &[Planning, Implementation, Debugging],

        _ => &[],
    }
}
// END_tool_profile_membership

// START_CONTRACT_tool_matches_profile
// PURPOSE: Check whether one tool should be visible under a requested disclosure profile
// INPUTS: { tool_name: &str }, { profile: &ToolProfile }
// OUTPUTS: { bool }
// START_tool_matches_profile
pub(crate) fn tool_matches_profile(tool_name: &str, profile: &ToolProfile) -> bool {
    match profile {
        ToolProfile::All => true,
        ToolProfile::Minimal => matches!(
            tool_name,
            "semantic_search"
                | "verify_project"
                | "graphrag_query"
                | "project_status"
                | "review_code"
                | "mental_test_run"
        ),
        ToolProfile::Custom(tools) => tools.iter().any(|candidate| candidate == tool_name),
        requested => tool_profile_membership(tool_name).contains(requested),
    }
}
// END_tool_matches_profile

// START_CONTRACT_tool_definitions_for_profile
// PURPOSE: Build tool definitions visible under one disclosure profile
// INPUTS: { profile: &ToolProfile }
// OUTPUTS: { Vec<serde_json::Value> }
// START_tool_definitions_for_profile
pub(crate) fn tool_definitions_for_profile(profile: &ToolProfile) -> Vec<serde_json::Value> {
    tool_definitions()
        .into_iter()
        .filter(|tool| {
            tool["name"]
                .as_str()
                .is_some_and(|name| tool_matches_profile(name, profile))
        })
        .collect()
}
// END_tool_definitions_for_profile

// START_CONTRACT_apply_tool_schema_style
// PURPOSE: Apply full or terse schema style to already-selected tool definitions
// INPUTS: { tools: Vec<serde_json::Value> }, { style: ToolSchemaStyle }
// OUTPUTS: { Vec<serde_json::Value> }
// START_apply_tool_schema_style
pub(crate) fn apply_tool_schema_style(
    tools: Vec<serde_json::Value>,
    style: ToolSchemaStyle,
) -> Vec<serde_json::Value> {
    match style {
        ToolSchemaStyle::Full => tools,
        ToolSchemaStyle::Terse => tools.into_iter().map(terse_tool_definition).collect(),
    }
}
// END_apply_tool_schema_style

// START_CONTRACT_terse_tool_definition
// PURPOSE: Remove human-facing schema prose from one tool definition while preserving callable argument shape
// INPUTS: { tool: serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_terse_tool_definition
pub(crate) fn terse_tool_definition(mut tool: serde_json::Value) -> serde_json::Value {
    strip_description_fields(&mut tool);
    if let Some(input_schema) = tool
        .get_mut("inputSchema")
        .and_then(serde_json::Value::as_object_mut)
    {
        input_schema.remove("type");
    }
    tool
}
// END_terse_tool_definition

// START_CONTRACT_strip_description_fields
// PURPOSE: Recursively remove schema description keys without deleting parameters named description
// INPUTS: { value: &mut serde_json::Value }
// SIDE_EFFECTS: mutates value in place
// START_strip_description_fields
fn strip_description_fields(value: &mut serde_json::Value) {
    strip_description_fields_in_context(value, false);
}
// END_strip_description_fields

// START_CONTRACT_strip_description_fields_in_context
// PURPOSE: Remove description metadata while preserving property maps that may contain a description argument
// INPUTS: { value: &mut serde_json::Value }, { is_properties_map: bool }
// SIDE_EFFECTS: mutates value in place
// START_strip_description_fields_in_context
fn strip_description_fields_in_context(value: &mut serde_json::Value, is_properties_map: bool) {
    match value {
        serde_json::Value::Object(object) => {
            if !is_properties_map {
                object.remove("description");
            }
            for (key, child) in object.iter_mut() {
                let child_is_properties_map = !is_properties_map && key == "properties";
                strip_description_fields_in_context(child, child_is_properties_map);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_description_fields_in_context(item, false);
            }
        }
        _ => {}
    }
}
// END_strip_description_fields_in_context

// START_CONTRACT_tool_definitions_json_bytes
// PURPOSE: Count compact JSON bytes for one tools/list payload subset
// INPUTS: { tools: &[serde_json::Value] }
// OUTPUTS: { usize }
// START_tool_definitions_json_bytes
pub(crate) fn tool_definitions_json_bytes(tools: &[serde_json::Value]) -> usize {
    serde_json::to_vec(tools).map_or(0, |bytes| bytes.len())
}
// END_tool_definitions_json_bytes

// START_CONTRACT_response_economy_tool_names
// PURPOSE: Return built-in tools that expose max_tokens and style response economy params
// OUTPUTS: { &'static [&'static str] }
// START_response_economy_tool_names
pub(crate) fn response_economy_tool_names() -> &'static [&'static str] {
    RESPONSE_ECONOMY_TOOL_NAMES
}
// END_response_economy_tool_names

// START_CONTRACT_tool_supports_response_economy
// PURPOSE: Check whether a tool should expose max_tokens and style schema parameters
// INPUTS: { tool_name: &str }
// OUTPUTS: { bool }
// START_tool_supports_response_economy
fn tool_supports_response_economy(tool_name: &str) -> bool {
    RESPONSE_ECONOMY_TOOL_NAMES.contains(&tool_name)
}
// END_tool_supports_response_economy

// START_CONTRACT_add_response_economy_schemas
// PURPOSE: Add response economy schema fields to all selected tool definitions
// INPUTS: { tools: &mut [serde_json::Value] }
// SIDE_EFFECTS: mutates tool inputSchema properties
// START_add_response_economy_schemas
fn add_response_economy_schemas(tools: &mut [serde_json::Value]) {
    for tool in tools {
        let Some(name) = tool.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if tool_supports_response_economy(name) {
            add_response_economy_schema(tool);
        }
    }
}
// END_add_response_economy_schemas

// START_CONTRACT_add_response_economy_schema
// PURPOSE: Add max_tokens and style properties to one tool input schema
// INPUTS: { tool: &mut serde_json::Value }
// SIDE_EFFECTS: mutates tool inputSchema properties
// START_add_response_economy_schema
fn add_response_economy_schema(tool: &mut serde_json::Value) {
    let Some(properties) = tool
        .get_mut("inputSchema")
        .and_then(|schema| schema.get_mut("properties"))
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };

    properties.insert(
        "max_tokens".into(),
        serde_json::json!({
            "type": "number",
            "default": 0,
            "description": "Maximum estimated response tokens; 0 keeps the full response"
        }),
    );
    properties.insert(
        "style".into(),
        serde_json::json!({
            "type": "string",
            "enum": ["full", "terse"],
            "default": "full",
            "description": "Response verbosity style"
        }),
    );
}
// END_add_response_economy_schema

// START_CONTRACT_add_cache_hint_schema
// PURPOSE: Add _if_none_match as an optional cache validator property to one tool schema
// INPUTS: { tool: &mut serde_json::Value }
// SIDE_EFFECTS: mutates tool inputSchema properties
// START_add_cache_hint_schema
fn add_cache_hint_schema(tool: &mut serde_json::Value) {
    let Some(properties) = tool
        .get_mut("inputSchema")
        .and_then(|schema| schema.get_mut("properties"))
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };

    properties.insert(
        "_if_none_match".into(),
        serde_json::json!({
            "type": "string",
            "description": "Previous _meta.cache.etag; matching fresh cache validators return _not_modified"
        }),
    );
}
// END_add_cache_hint_schema

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
            "description": "Query the code knowledge graph. Supports: search, get-node, get-relationships, find-path, dependents, tracedown, impact, overview, mermaid",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "operation": { "type": "string", "description": "Operation: search | get-node | get-relationships | find-path | dependents | tracedown | impact | overview | mermaid" },
                    "query": { "type": "string", "description": "Search query" },
                    "node_id": { "type": "string", "description": "Node ID" },
                    "from": { "type": "string", "description": "Source node ID" },
                    "to": { "type": "string", "description": "Target node ID" },
                    "target": { "type": "string", "description": "Target artifact for dependents/tracedown" },
                    "link_type": { "type": "string", "description": "Typed LINKS filter: implements | depends | refines | traces_to | verified_by | manages | uses" },
                    "depth": { "type": "number", "description": "Impact traversal depth for operation=impact", "default": 2 },
                    "include_tests": { "type": "boolean", "description": "Include verification/test targets for operation=impact", "default": true },
                    "subset": { "type": "string", "description": "Mermaid subset for operation=mermaid: modules | symbols | relations", "default": "relations" },
                    "focus": { "type": "string", "description": "Optional Mermaid focus node id" },
                    "focus_ids": { "type": "array", "items": { "type": "string" }, "description": "Optional Mermaid focus node ids" },
                    "max_nodes": { "type": "number", "description": "Maximum Mermaid graph nodes", "default": 80 }
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
            "name": "self_heal",
            "description": "Run one bounded self-heal iteration for a persisted autonomous run. Verifies the project, stores diagnoses in run metadata, and escalates when retry budget is exhausted.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "run_id": { "type": "string", "description": "Persisted run id from docs/runs/<run_id>.json" },
                    "profile": { "type": "string", "description": "GRACE profile: lite | balanced | strict", "default": "strict" },
                    "project_root": { "type": "string", "description": "Optional project root; defaults to current working directory" }
                },
                "required": ["run_id"]
            }
        }),
        serde_json::json!({
            "name": "advance_phase",
            "description": "Check active MyGRACE phase gates and optionally advance to the next planned phase. Defaults to dry_run=true.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "dry_run": { "type": "boolean", "description": "Preview without writing when true", "default": true },
                    "project_root": { "type": "string", "description": "Optional project root; defaults to current working directory" }
                }
            }
        }),
        serde_json::json!({
            "name": "pre_commit_check",
            "description": "Run pre-commit verification for a persisted bounded run before final completion.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "run_id": { "type": "string", "description": "Persisted run id from docs/runs/<run_id>.json" },
                    "project_root": { "type": "string", "description": "Optional project root; defaults to current working directory" }
                },
                "required": ["run_id"]
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
            "name": "diagnose_failure",
            "description": "Parse tester-agent XML or plain text failure evidence, extract identifiers, search exact source matches, and return an EnhancedFixResult.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "report": { "type": "string", "description": "Failure XML or plain text failure report" },
                    "failure_xml": { "type": "string", "description": "Alias for report" },
                    "description": { "type": "string", "description": "Plain text failure description alias" },
                    "project_root": { "type": "string", "description": "Optional project root; defaults to current working directory" }
                }
            }
        }),
        serde_json::json!({
            "name": "repair_contract",
            "description": "Generate or apply a safe language-aware MODULE_CONTRACT repair. Defaults to dry_run=true.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Project-relative source file path to repair" },
                    "file": { "type": "string", "description": "Alias for file_path" },
                    "module_id": { "type": "string", "description": "Optional MODULE_ID such as M-SAMPLE" },
                    "purpose": { "type": "string", "description": "Optional generated contract purpose" },
                    "scope": { "type": "string", "description": "Optional generated contract scope" },
                    "depends": { "type": "array", "items": { "type": "string" }, "description": "Optional DEPENDS entries" },
                    "links": { "type": "array", "items": { "type": "string" }, "description": "Optional typed LINKS entries" },
                    "language": { "type": "string", "description": "Optional language override" },
                    "dry_run": { "type": "boolean", "description": "When true, return preview without writing", "default": true },
                    "project_root": { "type": "string", "description": "Optional project root; defaults to current working directory" }
                },
                "required": ["file_path"]
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
                    "column": { "type": "number", "description": "Column (0-based)" },
                    "content": { "type": "string", "description": "Optional current file content for recently edited unsaved buffers" }
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
                    "column": { "type": "number", "description": "Column (0-based)" },
                    "content": { "type": "string", "description": "Optional current file content for recently edited unsaved buffers" }
                },
                "required": ["file", "line", "column"]
            }
        }),
    ];

    tools.push(serde_json::json!({"name":"tools/recommend","description":"Recommend up to eight context-relevant tools before listing schemas.","inputSchema":{"type":"object","properties":{"context":{"type":"string","description":"Current task or surrounding agent context"},"run_id":{"type":"string","description":"Optional persisted run id for run-state-aware recommendations"},"max_tools":{"type":"number","default":8,"description":"Maximum tools to recommend; clamped to 1..8"}}}}));
    add_response_economy_schemas(&mut tools);
    for tool in &mut tools {
        if tool
            .get("name")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|name| CACHE_HINT_TOOL_NAMES.contains(&name))
        {
            add_cache_hint_schema(tool);
        }
    }

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
    use std::collections::BTreeSet;

    // START_CONTRACT_contains_description_key
    // PURPOSE: Detect description keys recursively in JSON values for terse schema tests
    // INPUTS: { value: &serde_json::Value }
    // OUTPUTS: { bool }
    // START_contains_description_key
    fn contains_description_key(value: &serde_json::Value) -> bool {
        contains_description_key_in_context(value, false)
    }
    // END_contains_description_key

    // START_CONTRACT_contains_description_key_in_context
    // PURPOSE: Detect schema description metadata without flagging arguments named description
    // INPUTS: { value: &serde_json::Value }, { is_properties_map: bool }
    // OUTPUTS: { bool }
    // START_contains_description_key_in_context
    fn contains_description_key_in_context(
        value: &serde_json::Value,
        is_properties_map: bool,
    ) -> bool {
        match value {
            serde_json::Value::Object(object) => {
                (!is_properties_map && object.contains_key("description"))
                    || object.iter().any(|(key, child)| {
                        let child_is_properties_map = !is_properties_map && key == "properties";
                        contains_description_key_in_context(child, child_is_properties_map)
                    })
            }
            serde_json::Value::Array(items) => items
                .iter()
                .any(|item| contains_description_key_in_context(item, false)),
            _ => false,
        }
    }
    // END_contains_description_key_in_context

    // START_CONTRACT_test_tool_profile_parse_supports_workflow_aliases
    // PURPOSE: Verify tools/list profile aliases parse into stable workflow profiles
    // START_test_tool_profile_parse_supports_workflow_aliases
    #[test]
    fn test_tool_profile_parse_supports_workflow_aliases() {
        assert_eq!(ToolProfile::parse(""), ToolProfile::All);
        assert_eq!(ToolProfile::parse("verify"), ToolProfile::Verification);
        assert_eq!(ToolProfile::parse("planning"), ToolProfile::Planning);
        assert_eq!(ToolProfile::parse("impl"), ToolProfile::Implementation);
        assert_eq!(ToolProfile::parse("debugging"), ToolProfile::Debugging);
        assert_eq!(ToolProfile::parse("minimal"), ToolProfile::Minimal);
        assert_eq!(
            ToolProfile::parse("custom:semantic_search, verify_project"),
            ToolProfile::Custom(vec!["semantic_search".into(), "verify_project".into()])
        );
    }
    // END_test_tool_profile_parse_supports_workflow_aliases

    // START_CONTRACT_test_tool_schema_style_parse_supports_terse
    // PURPOSE: Verify tools/list style parsing defaults to full and accepts terse
    // START_test_tool_schema_style_parse_supports_terse
    #[test]
    fn test_tool_schema_style_parse_supports_terse() {
        assert_eq!(ToolSchemaStyle::parse(""), ToolSchemaStyle::Full);
        assert_eq!(ToolSchemaStyle::parse("full"), ToolSchemaStyle::Full);
        assert_eq!(ToolSchemaStyle::parse(" TERSE "), ToolSchemaStyle::Terse);
        assert_eq!(ToolSchemaStyle::Full.label(), "full");
        assert_eq!(ToolSchemaStyle::Terse.label(), "terse");
    }
    // END_test_tool_schema_style_parse_supports_terse

    // START_CONTRACT_test_minimal_profile_matches_exactly_six_tools
    // PURPOSE: Verify the minimal disclosure profile exposes the six core tools planned by Phase-83
    // START_test_minimal_profile_matches_exactly_six_tools
    #[test]
    fn test_minimal_profile_matches_exactly_six_tools() {
        let minimal: Vec<_> = tool_definitions()
            .into_iter()
            .filter_map(|tool| tool["name"].as_str().map(ToOwned::to_owned))
            .filter(|name| tool_matches_profile(name, &ToolProfile::Minimal))
            .collect();

        assert_eq!(
            minimal,
            vec![
                "semantic_search",
                "graphrag_query",
                "verify_project",
                "review_code",
                "project_status",
                "mental_test_run"
            ]
        );
    }
    // END_test_minimal_profile_matches_exactly_six_tools

    // START_CONTRACT_test_verification_profile_is_bounded_and_relevant
    // PURPOSE: Verify the verification disclosure profile stays under the Phase-83 token-economy cap
    // START_test_verification_profile_is_bounded_and_relevant
    #[test]
    fn test_verification_profile_is_bounded_and_relevant() {
        let verification: BTreeSet<_> = tool_definitions()
            .into_iter()
            .filter_map(|tool| tool["name"].as_str().map(ToOwned::to_owned))
            .filter(|name| tool_matches_profile(name, &ToolProfile::Verification))
            .collect();

        assert!(verification.len() <= 10);
        assert!(verification.contains("verify_project"));
        assert!(verification.contains("review_code"));
        assert!(verification.contains("project_status"));
        assert!(verification.contains("traceability_report"));
        assert!(!verification.contains("semantic_search"));
    }
    // END_test_verification_profile_is_bounded_and_relevant

    // START_CONTRACT_test_custom_profile_matches_only_named_tools
    // PURPOSE: Verify custom disclosure profiles expose exactly caller-requested tools
    // START_test_custom_profile_matches_only_named_tools
    #[test]
    fn test_custom_profile_matches_only_named_tools() {
        let profile = ToolProfile::parse("custom:semantic_search,verify_project");
        let custom: Vec<_> = tool_definitions()
            .into_iter()
            .filter_map(|tool| tool["name"].as_str().map(ToOwned::to_owned))
            .filter(|name| tool_matches_profile(name, &profile))
            .collect();

        assert_eq!(custom, vec!["semantic_search", "verify_project"]);
    }
    // END_test_custom_profile_matches_only_named_tools

    // START_CONTRACT_test_all_current_tools_have_profile_membership
    // PURPOSE: Verify every current built-in and skill tool is classified before profile filtering is wired into tools/list
    // START_test_all_current_tools_have_profile_membership
    #[test]
    fn test_all_current_tools_have_profile_membership() {
        let unclassified: Vec<_> = tool_definitions()
            .into_iter()
            .filter_map(|tool| tool["name"].as_str().map(ToOwned::to_owned))
            .filter(|name| tool_profile_membership(name).is_empty())
            .collect();

        assert!(unclassified.is_empty(), "{unclassified:?}");
    }
    // END_test_all_current_tools_have_profile_membership

    // START_CONTRACT_test_response_economy_schema_exposes_max_tokens_and_style
    // PURPOSE: Verify high-output MCP tools expose max_tokens and style schema fields
    // START_test_response_economy_schema_exposes_max_tokens_and_style
    #[test]
    fn test_response_economy_schema_exposes_max_tokens_and_style() {
        let tools = tool_definitions();

        for expected in response_economy_tool_names() {
            let tool = tools
                .iter()
                .find(|tool| tool["name"] == *expected)
                .unwrap_or_else(|| panic!("{expected} tool"));
            let properties = tool["inputSchema"]["properties"]
                .as_object()
                .unwrap_or_else(|| panic!("{expected} properties"));

            assert!(
                properties.contains_key("max_tokens"),
                "{expected} missing max_tokens"
            );
            assert!(properties.contains_key("style"), "{expected} missing style");
            assert_eq!(properties["max_tokens"]["default"], 0);
            assert_eq!(properties["style"]["enum"][1], "terse");
        }
    }
    // END_test_response_economy_schema_exposes_max_tokens_and_style

    // START_CONTRACT_test_response_economy_schema_covers_at_least_ten_tools
    // PURPOSE: Verify Phase-84 covers at least ten high-output MCP tools with response economy params
    // START_test_response_economy_schema_covers_at_least_ten_tools
    #[test]
    fn test_response_economy_schema_covers_at_least_ten_tools() {
        let covered = tool_definitions()
            .into_iter()
            .filter_map(|tool| tool["name"].as_str().map(ToOwned::to_owned))
            .filter(|name| tool_supports_response_economy(name))
            .count();

        assert!(covered >= 10, "covered={covered}");
        assert_eq!(covered, response_economy_tool_names().len());
    }
    // END_test_response_economy_schema_covers_at_least_ten_tools

    // START_CONTRACT_test_cache_hint_schema_exposes_if_none_match
    // PURPOSE: Verify cacheable tools expose optional _if_none_match schema fields
    // START_test_cache_hint_schema_exposes_if_none_match
    #[test]
    fn test_cache_hint_schema_exposes_if_none_match() {
        let tools = tool_definitions();

        for expected in CACHE_HINT_TOOL_NAMES {
            let tool = tools
                .iter()
                .find(|tool| tool["name"] == *expected)
                .unwrap_or_else(|| panic!("{expected} tool"));
            let properties = tool["inputSchema"]["properties"]
                .as_object()
                .unwrap_or_else(|| panic!("{expected} properties"));
            let required = tool["inputSchema"]["required"].as_array();

            assert!(
                properties.contains_key("_if_none_match"),
                "{expected} missing _if_none_match"
            );
            assert!(
                required
                    .map(|required| {
                        !required
                            .iter()
                            .any(|entry| entry.as_str() == Some("_if_none_match"))
                    })
                    .unwrap_or(true),
                "{expected} must not require _if_none_match"
            );
        }

        let semantic_search = tools
            .iter()
            .find(|tool| tool["name"] == "semantic_search")
            .expect("semantic_search tool");
        let properties = semantic_search["inputSchema"]["properties"]
            .as_object()
            .expect("properties");

        assert!(!properties.contains_key("_if_none_match"));
        assert!(!CACHE_HINT_TOOL_NAMES.contains(&"semantic_search"));
    }
    // END_test_cache_hint_schema_exposes_if_none_match

    // START_CONTRACT_test_terse_tool_definition_preserves_machine_schema
    // PURPOSE: Verify terse schemas remove descriptions recursively while preserving JSON schema shape
    // START_test_terse_tool_definition_preserves_machine_schema
    #[test]
    fn test_terse_tool_definition_preserves_machine_schema() {
        let tool = serde_json::json!({
            "name": "sample",
            "description": "Human-facing text",
            "inputSchema": {
                "type": "object",
                "description": "Schema text",
                "properties": {
                    "mode": {
                        "type": "string",
                        "description": "Mode text",
                        "default": "fast",
                        "enum": ["fast", "safe"]
                    },
                    "items": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "description": "Nested item text",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Nested path text"
                                }
                            },
                            "required": ["path"]
                        }
                    },
                    "description": {
                        "type": "string",
                        "description": "A legitimate argument named description"
                    }
                },
                "required": ["mode"]
            }
        });

        let terse = terse_tool_definition(tool);

        assert!(!contains_description_key(&terse));
        assert_eq!(terse["name"], "sample");
        assert!(terse["inputSchema"].get("type").is_none());
        assert_eq!(terse["inputSchema"]["required"][0], "mode");
        assert_eq!(
            terse["inputSchema"]["properties"]["mode"]["default"],
            "fast"
        );
        assert_eq!(
            terse["inputSchema"]["properties"]["mode"]["enum"][1],
            "safe"
        );
        assert_eq!(
            terse["inputSchema"]["properties"]["items"]["items"]["properties"]["path"]["type"],
            "string"
        );
        assert_eq!(
            terse["inputSchema"]["properties"]["items"]["items"]["type"],
            "object"
        );
        assert_eq!(
            terse["inputSchema"]["properties"]["items"]["items"]["required"][0],
            "path"
        );
        assert_eq!(
            terse["inputSchema"]["properties"]["description"]["type"],
            "string"
        );
    }
    // END_test_terse_tool_definition_preserves_machine_schema

    // START_CONTRACT_test_terse_tool_definitions_strip_all_current_descriptions
    // PURPOSE: Verify terse style removes every description key from the current tool registry
    // START_test_terse_tool_definitions_strip_all_current_descriptions
    #[test]
    fn test_terse_tool_definitions_strip_all_current_descriptions() {
        let full_tools = tool_definitions_for_profile(&ToolProfile::All);
        let terse_tools = apply_tool_schema_style(full_tools, ToolSchemaStyle::Terse);

        assert!(!terse_tools.iter().any(contains_description_key));
    }
    // END_test_terse_tool_definitions_strip_all_current_descriptions

    // START_CONTRACT_test_terse_tool_definitions_save_at_least_sixty_percent
    // PURPOSE: Verify terse style reaches the Phase-83 schema economy target for the full registry
    // START_test_terse_tool_definitions_save_at_least_sixty_percent
    #[test]
    fn test_terse_tool_definitions_save_at_least_sixty_percent() {
        let full_tools = tool_definitions_for_profile(&ToolProfile::All);
        let full_bytes = tool_definitions_json_bytes(&full_tools);
        let terse_tools = apply_tool_schema_style(full_tools, ToolSchemaStyle::Terse);
        let terse_bytes = tool_definitions_json_bytes(&terse_tools);
        let saved_pct = (full_bytes.saturating_sub(terse_bytes) as f64 / full_bytes as f64) * 100.0;

        assert!(
            saved_pct >= 60.0,
            "terse saved {saved_pct:.1}% ({full_bytes} -> {terse_bytes} bytes)"
        );
    }
    // END_test_terse_tool_definitions_save_at_least_sixty_percent

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

    // START_CONTRACT_test_graphrag_schema_exposes_mermaid_options
    // PURPOSE: Verify tools/list declares GraphRAG Mermaid operation options for MCP clients
    // START_test_graphrag_schema_exposes_mermaid_options
    #[test]
    fn test_graphrag_schema_exposes_mermaid_options() {
        let tools = tool_definitions();
        let graphrag = tools
            .iter()
            .find(|tool| tool["name"] == "graphrag_query")
            .expect("graphrag_query tool");
        let properties = graphrag["inputSchema"]["properties"]
            .as_object()
            .expect("properties");

        assert!(graphrag["description"]
            .as_str()
            .is_some_and(|description| description.contains("mermaid")));
        assert!(properties.contains_key("subset"));
        assert!(properties.contains_key("focus_ids"));
        assert!(properties.contains_key("max_nodes"));
    }
    // END_test_graphrag_schema_exposes_mermaid_options

    // START_CONTRACT_test_graphrag_schema_exposes_impact_options
    // PURPOSE: Verify tools/list declares GraphRAG impact operation options for MCP clients
    // START_test_graphrag_schema_exposes_impact_options
    #[test]
    fn test_graphrag_schema_exposes_impact_options() {
        let tools = tool_definitions();
        let graphrag = tools
            .iter()
            .find(|tool| tool["name"] == "graphrag_query")
            .expect("graphrag_query tool");
        let properties = graphrag["inputSchema"]["properties"]
            .as_object()
            .expect("properties");

        assert!(graphrag["description"]
            .as_str()
            .is_some_and(|description| description.contains("impact")));
        assert!(
            graphrag["inputSchema"]["properties"]["operation"]["description"]
                .as_str()
                .is_some_and(|description| description.contains("impact"))
        );
        assert!(properties.contains_key("depth"));
        assert!(properties.contains_key("include_tests"));
    }
    // END_test_graphrag_schema_exposes_impact_options

    // START_CONTRACT_test_lsp_schema_exposes_content_override
    // PURPOSE: Verify tools/list declares optional content override for LSP hover and references
    // LINKS:
    //   -> M-MCP-LSP (depends) - content override reaches didOpen
    //   -> NFR-002 (traces_to) - avoids stale LSP reads after edits
    // START_test_lsp_schema_exposes_content_override
    #[test]
    fn test_lsp_schema_exposes_content_override() {
        let tools = tool_definitions();
        for name in ["lsp_hover", "lsp_references"] {
            let tool = tools
                .iter()
                .find(|tool| tool["name"] == name)
                .unwrap_or_else(|| panic!("{name} tool"));
            let properties = tool["inputSchema"]["properties"]
                .as_object()
                .expect("properties");

            assert!(properties.contains_key("content"));
        }
    }
    // END_test_lsp_schema_exposes_content_override

    // START_CONTRACT_test_self_heal_schema_exposes_run_id_and_profile
    // PURPOSE: Verify tools/list declares self_heal run_id and profile arguments
    // START_test_self_heal_schema_exposes_run_id_and_profile
    #[test]
    fn test_self_heal_schema_exposes_run_id_and_profile() {
        let tools = tool_definitions();
        let self_heal = tools
            .iter()
            .find(|tool| tool["name"] == "self_heal")
            .expect("self_heal tool");
        let properties = self_heal["inputSchema"]["properties"]
            .as_object()
            .expect("properties");

        assert!(properties.contains_key("run_id"));
        assert!(properties.contains_key("profile"));
        assert_eq!(self_heal["inputSchema"]["required"][0], "run_id");
    }
    // END_test_self_heal_schema_exposes_run_id_and_profile

    // START_CONTRACT_test_phase_and_pre_commit_schemas_are_exposed
    // PURPOSE: Verify tools/list declares advance_phase and pre_commit_check schemas.
    // START_test_phase_and_pre_commit_schemas_are_exposed
    #[test]
    fn test_phase_and_pre_commit_schemas_are_exposed() {
        let tools = tool_definitions();
        let advance_phase = tools
            .iter()
            .find(|tool| tool["name"] == "advance_phase")
            .expect("advance_phase tool");
        let pre_commit = tools
            .iter()
            .find(|tool| tool["name"] == "pre_commit_check")
            .expect("pre_commit_check tool");

        assert!(advance_phase["inputSchema"]["properties"]
            .as_object()
            .is_some_and(|properties| properties.contains_key("dry_run")));
        assert_eq!(pre_commit["inputSchema"]["required"][0], "run_id");
    }
    // END_test_phase_and_pre_commit_schemas_are_exposed

    // START_CONTRACT_test_diagnose_and_repair_schemas_are_exposed
    // PURPOSE: Verify tools/list declares diagnose_failure and repair_contract schemas
    // START_test_diagnose_and_repair_schemas_are_exposed
    #[test]
    fn test_diagnose_and_repair_schemas_are_exposed() {
        let tools = tool_definitions();
        let diagnose = tools
            .iter()
            .find(|tool| tool["name"] == "diagnose_failure")
            .expect("diagnose_failure tool");
        let repair = tools
            .iter()
            .find(|tool| tool["name"] == "repair_contract")
            .expect("repair_contract tool");

        assert!(diagnose["inputSchema"]["properties"]
            .as_object()
            .expect("diagnose properties")
            .contains_key("report"));
        assert_eq!(repair["inputSchema"]["required"][0], "file_path");
    }
    // END_test_diagnose_and_repair_schemas_are_exposed
}

// END_public_api
