// MODULE_CONTRACT
// MODULE_ID: M-MCP-USER-TOOLS
// PURPOSE: Local user-defined MCP command tools loaded from JSON definitions.
// SCOPE: ~/.synapse/tools discovery, JSON schema validation, built-in override protection, MCP tool metadata rendering, command interpolation, allowed path checks, timeout-bounded execution, ANSI stripping, and output token trimming.
// DEPENDS: M-MCP-SERVER, M-MCP-SERVER-TOOLS, M-MCP-SERVER-RESPONSE, M-UTILS
// LINKS:
//   -> docs/phases/Phase-92.xml (implements) - UPGRADE_5 local MCP tools
//   -> NFR-002 (traces_to) - bounded local command execution
//   -> NFR-003 (traces_to) - token-capped dynamic tool output
//   <- V-M-MCP-USER-TOOLS (verified_by) - user tool loader and executor tests

// START_MODULE_MAP
// UserToolDefinition - JSON-backed user tool model
// UserToolHandler - Command handler settings
// UserToolLoadReport - Load result with valid tools and warnings
// UserToolMergeReport - tools/list merge metadata
// UserToolValidationReport - One-file validation report
// default_tools_dir - Resolve ~/.synapse/tools or SYNAPSE_TOOLS_DIR
// builtin_tool_names - Extract protected built-in MCP tool names
// current_builtin_tool_names - Return protected names from the live built-in registry
// load_from_dir - Load and validate user tool JSON files
// load_visible_definitions - Render profile-visible user tools for tools/list
// merge_visible_definitions - Append profile-visible user tools to an MCP tools/list vector
// validate_file - Validate one user tool JSON file
// validate_definition - Validate one already parsed user tool definition
// handle_call - Render JSON-RPC response for a user tool call or unknown tool
// execute_default_tool - Locate and execute a user tool from the default directory
// execute_tool_definition - Execute one validated command tool
// new_tool_template - Build a starter user tool JSON definition
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.1 - Serialized shell-spawning user tool tests against global cwd changes]
// END_CHANGE_SUMMARY

use super::server_response::{self, trim_text_to_budget};
use super::server_tools::ToolProfile;
use syn_core::utils::{strip_ansi, truncate_chars};
use globset::{Glob, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_TIMEOUT_SECS: u64 = 300;
const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 2_000;

// START_public_api

// START_UserToolDefinition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserToolDefinition {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    pub handler: UserToolHandler,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}
// END_UserToolDefinition

// START_UserToolHandler
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserToolHandler {
    #[serde(rename = "type")]
    pub kind: String,
    pub command: Option<String>,
    pub fix_command: Option<String>,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub strip_ansi: bool,
    pub max_output_tokens: Option<u32>,
}
// END_UserToolHandler

// START_UserToolLoadReport
#[derive(Debug, Clone, Serialize)]
pub struct UserToolLoadReport {
    pub directory: PathBuf,
    pub tools: Vec<UserToolDefinition>,
    pub warnings: Vec<String>,
}
// END_UserToolLoadReport

// START_UserToolVisibleReport
#[derive(Debug, Clone, Serialize)]
pub struct UserToolVisibleReport {
    pub tools: Vec<Value>,
    pub warnings: Vec<String>,
    pub total_valid: usize,
}
// END_UserToolVisibleReport

// START_UserToolMergeReport
#[derive(Debug, Clone, Serialize)]
pub struct UserToolMergeReport {
    pub warnings: Vec<String>,
    pub total_valid: usize,
    pub total_available: usize,
}
// END_UserToolMergeReport

// START_UserToolValidationReport
#[derive(Debug, Clone, Serialize)]
pub struct UserToolValidationReport {
    pub path: PathBuf,
    pub valid: bool,
    pub tool_name: Option<String>,
    pub errors: Vec<String>,
}
// END_UserToolValidationReport

// START_UserToolError
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserToolError {
    InvalidArguments(String),
    Io(String),
    TimedOut(u64),
}
// END_UserToolError

impl fmt::Display for UserToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArguments(message) => write!(f, "{message}"),
            Self::Io(message) => write!(f, "{message}"),
            Self::TimedOut(secs) => write!(f, "user tool timed out after {secs}s"),
        }
    }
}

impl std::error::Error for UserToolError {}

// START_CONTRACT_default_tools_dir
// PURPOSE: Resolve the local user-tool directory, allowing tests and local automation to override it.
// OUTPUTS: { PathBuf }
// START_default_tools_dir
pub fn default_tools_dir() -> PathBuf {
    std::env::var_os("SYNAPSE_TOOLS_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".synapse").join("tools")))
        .unwrap_or_else(|| PathBuf::from(".synapse").join("tools"))
}
// END_default_tools_dir

// START_CONTRACT_builtin_tool_names
// PURPOSE: Extract protected MCP tool names from built-in tool definitions.
// INPUTS: { tools: &[serde_json::Value] }
// OUTPUTS: { BTreeSet<String> }
// START_builtin_tool_names
pub fn builtin_tool_names(tools: &[Value]) -> BTreeSet<String> {
    tools
        .iter()
        .filter_map(|tool| tool["name"].as_str().map(ToOwned::to_owned))
        .collect()
}
// END_builtin_tool_names

// START_CONTRACT_current_builtin_tool_names
// PURPOSE: Return protected MCP tool names from the live built-in registry.
// OUTPUTS: { BTreeSet<String> }
// START_current_builtin_tool_names
pub fn current_builtin_tool_names() -> BTreeSet<String> {
    builtin_tool_names(&super::server_tools::tool_definitions())
}
// END_current_builtin_tool_names

// START_CONTRACT_load_from_dir
// PURPOSE: Load valid user tool definitions from a directory while collecting non-fatal warnings.
// INPUTS: { directory: &Path }, { builtin_names: &BTreeSet<String> }
// OUTPUTS: { UserToolLoadReport }
// SIDE_EFFECTS: reads JSON files from disk
// START_load_from_dir
pub fn load_from_dir(
    directory: &Path,
    builtin_names: &BTreeSet<String>,
) -> UserToolLoadReport {
    let mut report = UserToolLoadReport {
        directory: directory.to_path_buf(),
        tools: Vec::new(),
        warnings: Vec::new(),
    };
    if !directory.exists() {
        return report;
    }

    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            report.warnings.push(format!(
                "{}: cannot read directory: {}",
                directory.display(),
                error
            ));
            return report;
        }
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();

    let mut seen = BTreeSet::new();
    for path in paths {
        let validation = validate_file(&path, builtin_names);
        if !validation.valid {
            report.warnings.extend(
                validation
                    .errors
                    .iter()
                    .map(|error| format!("{}: {}", path.display(), error)),
            );
            continue;
        }
        let Ok(tool) = read_tool_file(&path) else {
            report.warnings.push(format!(
                "{}: failed to reread validated tool",
                path.display()
            ));
            continue;
        };
        if !seen.insert(tool.name.clone()) {
            report.warnings.push(format!(
                "{}: duplicate user tool name '{}'",
                path.display(),
                tool.name
            ));
            continue;
        }
        report.tools.push(tool);
    }
    report
}
// END_load_from_dir

// START_CONTRACT_load_visible_definitions
// PURPOSE: Render profile-visible user tools with MCP source metadata.
// INPUTS: { directory: &Path }, { profile: &ToolProfile }, { builtin_names: &BTreeSet<String> }
// OUTPUTS: { UserToolVisibleReport }
// SIDE_EFFECTS: reads user tool JSON files from disk
// START_load_visible_definitions
pub fn load_visible_definitions(
    directory: &Path,
    profile: &ToolProfile,
    builtin_names: &BTreeSet<String>,
) -> UserToolVisibleReport {
    let report = load_from_dir(directory, builtin_names);
    let tools = report
        .tools
        .iter()
        .filter(|tool| tool_visible_for_profile(&tool.name, profile))
        .map(to_mcp_tool_definition)
        .collect();
    UserToolVisibleReport {
        tools,
        warnings: report.warnings,
        total_valid: report.tools.len(),
    }
}
// END_load_visible_definitions

// START_CONTRACT_merge_visible_definitions
// PURPOSE: Append profile-visible user tools to a tools/list vector and return count/warning metadata.
// INPUTS: { profile: &ToolProfile }, { tools: &mut Vec<serde_json::Value> }
// OUTPUTS: { UserToolMergeReport }
// SIDE_EFFECTS: reads user tool JSON files from disk
// START_merge_visible_definitions
pub fn merge_visible_definitions(
    profile: &ToolProfile,
    tools: &mut Vec<Value>,
) -> UserToolMergeReport {
    let builtins = super::server_tools::tool_definitions();
    let report = load_visible_definitions(
        &default_tools_dir(),
        profile,
        &builtin_tool_names(&builtins),
    );
    let total_available = builtins.len() + report.total_valid;
    tools.extend(report.tools);
    UserToolMergeReport {
        warnings: report.warnings,
        total_valid: report.total_valid,
        total_available,
    }
}
// END_merge_visible_definitions

// START_CONTRACT_validate_file
// PURPOSE: Validate one user tool JSON file against schema, command safety, and built-in names.
// INPUTS: { path: &Path }, { builtin_names: &BTreeSet<String> }
// OUTPUTS: { UserToolValidationReport }
// SIDE_EFFECTS: reads one JSON file
// START_validate_file
pub fn validate_file(
    path: &Path,
    builtin_names: &BTreeSet<String>,
) -> UserToolValidationReport {
    match read_tool_file(path) {
        Ok(tool) => {
            let errors = validation_errors(&tool, builtin_names);
            UserToolValidationReport {
                path: path.to_path_buf(),
                valid: errors.is_empty(),
                tool_name: Some(tool.name),
                errors,
            }
        }
        Err(error) => UserToolValidationReport {
            path: path.to_path_buf(),
            valid: false,
            tool_name: None,
            errors: vec![error],
        },
    }
}
// END_validate_file

// START_CONTRACT_validate_definition
// PURPOSE: Validate an already parsed user tool definition for CLI authoring flows.
// INPUTS: { tool: &UserToolDefinition }, { builtin_names: &BTreeSet<String> }
// OUTPUTS: { Vec<String> }
// START_validate_definition
pub fn validate_definition(
    tool: &UserToolDefinition,
    builtin_names: &BTreeSet<String>,
) -> Vec<String> {
    validation_errors(tool, builtin_names)
}
// END_validate_definition

// START_CONTRACT_handle_call
// PURPOSE: Execute an unknown MCP tools/call name as a validated user tool or return the standard unknown-tool error.
// INPUTS: { id: Option<serde_json::Value> }, { name: &str }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// SIDE_EFFECTS: may execute a local user-defined command tool
// START_handle_call
pub async fn handle_call(id: Option<Value>, name: &str, args: &Value) -> Value {
    match execute_default_tool(name, args, &current_builtin_tool_names()).await {
        Ok(Some(text)) => server_response::text_result(id, text, args),
        Ok(None) => server_response::error(id, -32601, format!("Unknown tool: {name}")),
        Err(error) => server_response::error(id, -32603, error.to_string()),
    }
}
// END_handle_call

// START_CONTRACT_execute_default_tool
// PURPOSE: Locate and execute a user tool from the default tool directory.
// INPUTS: { name: &str }, { args: &serde_json::Value }, { builtin_names: &BTreeSet<String> }
// OUTPUTS: { Result<Option<String>, UserToolError> }
// SIDE_EFFECTS: reads user tool files and may execute a local shell command
// START_execute_default_tool
pub async fn execute_default_tool(
    name: &str,
    args: &Value,
    builtin_names: &BTreeSet<String>,
) -> Result<Option<String>, UserToolError> {
    if builtin_names.contains(name) {
        return Ok(None);
    }
    let report = load_from_dir(&default_tools_dir(), builtin_names);
    for warning in &report.warnings {
        tracing::warn!("[UserTools][execute_default_tool][LOAD] {}", warning);
    }
    match report.tools.into_iter().find(|tool| tool.name == name) {
        Some(tool) => execute_tool_definition(&tool, args).await.map(Some),
        None => Ok(None),
    }
}
// END_execute_default_tool

// START_CONTRACT_execute_tool_definition
// PURPOSE: Execute one validated command-backed user tool with safety and token-economy controls.
// INPUTS: { tool: &UserToolDefinition }, { args: &serde_json::Value }
// OUTPUTS: { Result<String, UserToolError> }
// SIDE_EFFECTS: spawns a local shell command through sh -c
// START_execute_tool_definition
pub async fn execute_tool_definition(
    tool: &UserToolDefinition,
    args: &Value,
) -> Result<String, UserToolError> {
    ensure_arguments_object(args)?;
    validate_required_args(tool, args)?;
    let command_template = select_command_template(tool, args)?;
    let command = interpolate_command(command_template, args, &tool.handler.allowed_paths)?;
    let output = execute_shell_command(&command, handler_timeout_secs(&tool.handler)).await?;
    let mut text = render_command_output(&output);
    if tool.handler.strip_ansi {
        text = strip_ansi(&text);
    }
    let max_tokens = tool
        .handler
        .max_output_tokens
        .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS);
    Ok(trim_text_to_budget(&text, max_tokens).text)
}
// END_execute_tool_definition

// START_CONTRACT_new_tool_template
// PURPOSE: Build a starter command tool JSON value for local authoring.
// INPUTS: { name: &str }
// OUTPUTS: { serde_json::Value }
// START_new_tool_template
pub fn new_tool_template(name: &str) -> Value {
    serde_json::json!({
        "name": name,
        "description": format!("Local command tool: {name}"),
        "version": "0.1.0",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Project-relative path allowed by handler.allowed_paths"
                },
                "fix": {
                    "type": "boolean",
                    "description": "Use handler.fix_command when available"
                }
            },
            "required": ["path"]
        },
        "handler": {
            "type": "command",
            "command": "echo ${path}",
            "fix_command": "echo ${path}",
            "timeout_secs": DEFAULT_TIMEOUT_SECS,
            "allowed_paths": ["src/**", "tests/**", "docs/**"],
            "strip_ansi": true,
            "max_output_tokens": 1200
        }
    })
}
// END_new_tool_template

// END_public_api

// START_CONTRACT_read_tool_file
// PURPOSE: Read and deserialize one user tool JSON definition.
// INPUTS: { path: &Path }
// OUTPUTS: { Result<UserToolDefinition, String> }
// SIDE_EFFECTS: reads one file from disk
// START_read_tool_file
fn read_tool_file(path: &Path) -> Result<UserToolDefinition, String> {
    let raw =
        std::fs::read_to_string(path).map_err(|error| format!("cannot read tool file: {error}"))?;
    serde_json::from_str(&raw).map_err(|error| format!("invalid tool JSON: {error}"))
}
// END_read_tool_file

// START_CONTRACT_validation_errors
// PURPOSE: Collect validation errors for one parsed user tool definition.
// INPUTS: { tool: &UserToolDefinition }, { builtin_names: &BTreeSet<String> }
// OUTPUTS: { Vec<String> }
// START_validation_errors
fn validation_errors(tool: &UserToolDefinition, builtin_names: &BTreeSet<String>) -> Vec<String> {
    let mut errors = Vec::new();
    if !is_valid_tool_name(&tool.name) {
        errors.push("name must use letters, numbers, underscore, dot, or dash".into());
    }
    if builtin_names.contains(&tool.name) {
        errors.push(format!(
            "name '{}' overrides a built-in MCP tool",
            tool.name
        ));
    }
    if tool.description.trim().is_empty() {
        errors.push("description must not be empty".into());
    }
    if tool.version.trim().is_empty() {
        errors.push("version must not be empty".into());
    }
    if !tool.input_schema.is_object() {
        errors.push("inputSchema must be a JSON object".into());
    }
    if tool.handler.kind != "command" {
        errors.push("handler.type must be 'command'".into());
    }
    if tool
        .handler
        .command
        .as_deref()
        .is_none_or(|command| command.trim().is_empty())
    {
        errors.push("handler.command must not be empty".into());
    }
    if !(1..=MAX_TIMEOUT_SECS).contains(&handler_timeout_secs(&tool.handler)) {
        errors.push(format!(
            "handler.timeout_secs must be between 1 and {MAX_TIMEOUT_SECS}"
        ));
    }
    if tool.handler.max_output_tokens == Some(0) {
        errors.push("handler.max_output_tokens must be greater than zero".into());
    }
    if let Err(error) = compile_allowed_paths(&tool.handler.allowed_paths) {
        errors.push(error);
    }
    errors
}
// END_validation_errors

// START_CONTRACT_is_valid_tool_name
// PURPOSE: Validate a conservative MCP user tool name.
// INPUTS: { name: &str }
// OUTPUTS: { bool }
// START_is_valid_tool_name
fn is_valid_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
}
// END_is_valid_tool_name

// START_CONTRACT_tool_visible_for_profile
// PURPOSE: Decide whether a dynamic user tool should be shown for a tools/list profile.
// INPUTS: { name: &str }, { profile: &ToolProfile }
// OUTPUTS: { bool }
// START_tool_visible_for_profile
fn tool_visible_for_profile(name: &str, profile: &ToolProfile) -> bool {
    match profile {
        ToolProfile::All => true,
        ToolProfile::Custom(names) => names.iter().any(|candidate| candidate == name),
        _ => false,
    }
}
// END_tool_visible_for_profile

// START_CONTRACT_to_mcp_tool_definition
// PURPOSE: Convert one user tool into the MCP tools/list JSON shape with source metadata.
// INPUTS: { tool: &UserToolDefinition }
// OUTPUTS: { serde_json::Value }
// START_to_mcp_tool_definition
fn to_mcp_tool_definition(tool: &UserToolDefinition) -> Value {
    let mut value = serde_json::json!({
        "name": tool.name,
        "description": tool.description,
        "inputSchema": tool.input_schema,
        "_source": "user",
        "_version": tool.version
    });
    if let Some(ttl) = tool.cache_ttl_secs {
        value["_cache_ttl_secs"] = Value::from(ttl);
    }
    value
}
// END_to_mcp_tool_definition

// START_CONTRACT_ensure_arguments_object
// PURPOSE: Reject non-object tool arguments before interpolation.
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { Result<(), UserToolError> }
// START_ensure_arguments_object
fn ensure_arguments_object(args: &Value) -> Result<(), UserToolError> {
    if args.is_object() || args.is_null() {
        Ok(())
    } else {
        Err(UserToolError::InvalidArguments(
            "user tool arguments must be a JSON object".into(),
        ))
    }
}
// END_ensure_arguments_object

// START_CONTRACT_validate_required_args
// PURPOSE: Enforce inputSchema.required fields before command interpolation.
// INPUTS: { tool: &UserToolDefinition }, { args: &serde_json::Value }
// OUTPUTS: { Result<(), UserToolError> }
// START_validate_required_args
fn validate_required_args(tool: &UserToolDefinition, args: &Value) -> Result<(), UserToolError> {
    let required = tool
        .input_schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str);
    for key in required {
        if args.get(key).is_none_or(Value::is_null) {
            return Err(UserToolError::InvalidArguments(format!(
                "missing required argument '{key}'"
            )));
        }
    }
    Ok(())
}
// END_validate_required_args

// START_CONTRACT_select_command_template
// PURPOSE: Select fix_command when fix=true and a fix template is available.
// INPUTS: { tool: &UserToolDefinition }, { args: &serde_json::Value }
// OUTPUTS: { Result<&str, UserToolError> }
// START_select_command_template
fn select_command_template<'a>(
    tool: &'a UserToolDefinition,
    args: &Value,
) -> Result<&'a str, UserToolError> {
    if args.get("fix").and_then(Value::as_bool).unwrap_or(false) {
        if let Some(fix_command) = tool.handler.fix_command.as_deref() {
            return Ok(fix_command);
        }
    }
    tool.handler
        .command
        .as_deref()
        .ok_or_else(|| UserToolError::InvalidArguments("handler.command must not be empty".into()))
}
// END_select_command_template

// START_CONTRACT_interpolate_command
// PURPOSE: Replace ${arg} placeholders with shell-escaped argument values after allowlist checks.
// INPUTS: { template: &str }, { args: &serde_json::Value }, { allowed_paths: &[String] }
// OUTPUTS: { Result<String, UserToolError> }
// START_interpolate_command
fn interpolate_command(
    template: &str,
    args: &Value,
    allowed_paths: &[String],
) -> Result<String, UserToolError> {
    let re = regex::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}")
        .map_err(|error| UserToolError::InvalidArguments(error.to_string()))?;
    let mut rendered = String::new();
    let mut last = 0;
    for captures in re.captures_iter(template) {
        let Some(matched) = captures.get(0) else {
            continue;
        };
        let key = captures.get(1).map_or("", |value| value.as_str());
        rendered.push_str(&template[last..matched.start()]);
        let value = argument_value(args.get(key).unwrap_or(&Value::Null))?;
        validate_allowed_argument(key, &value, allowed_paths)?;
        rendered.push_str(&shell_escape(&value)?);
        last = matched.end();
    }
    rendered.push_str(&template[last..]);
    Ok(rendered)
}
// END_interpolate_command

// START_CONTRACT_argument_value
// PURPOSE: Convert scalar JSON values into shell argument strings.
// INPUTS: { value: &serde_json::Value }
// OUTPUTS: { Result<String, UserToolError> }
// START_argument_value
fn argument_value(value: &Value) -> Result<String, UserToolError> {
    match value {
        Value::Null => Ok(String::new()),
        Value::String(value) => Ok(value.clone()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        _ => Err(UserToolError::InvalidArguments(
            "user tool placeholders only support scalar arguments".into(),
        )),
    }
}
// END_argument_value

// START_CONTRACT_validate_allowed_argument
// PURPOSE: Validate path-like placeholder values against handler.allowed_paths.
// INPUTS: { key: &str }, { value: &str }, { allowed_paths: &[String] }
// OUTPUTS: { Result<(), UserToolError> }
// START_validate_allowed_argument
fn validate_allowed_argument(
    key: &str,
    value: &str,
    allowed_paths: &[String],
) -> Result<(), UserToolError> {
    if allowed_paths.is_empty() || value.is_empty() || !is_path_like_argument(key, value) {
        return Ok(());
    }
    if path_has_parent_dir(value) {
        return Err(UserToolError::InvalidArguments(format!(
            "argument '{key}' must not contain parent directory segments"
        )));
    }
    let set = compile_allowed_paths(allowed_paths).map_err(UserToolError::InvalidArguments)?;
    let normalized = normalize_path_value(value);
    if set.is_match(&normalized) {
        return Ok(());
    }
    Err(UserToolError::InvalidArguments(format!(
        "argument '{key}' path '{}' is outside allowed_paths",
        truncate_chars(value, 120)
    )))
}
// END_validate_allowed_argument

// START_CONTRACT_compile_allowed_paths
// PURPOSE: Compile allowed path glob patterns once per validation or execution.
// INPUTS: { patterns: &[String] }
// OUTPUTS: { Result<globset::GlobSet, String> }
// START_compile_allowed_paths
fn compile_allowed_paths(patterns: &[String]) -> Result<globset::GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(
            Glob::new(pattern)
                .map_err(|error| format!("invalid allowed_paths pattern '{pattern}': {error}"))?,
        );
    }
    builder
        .build()
        .map_err(|error| format!("invalid allowed_paths: {error}"))
}
// END_compile_allowed_paths

// START_CONTRACT_is_path_like_argument
// PURPOSE: Detect placeholder values that should be constrained by allowed_paths.
// INPUTS: { key: &str }, { value: &str }
// OUTPUTS: { bool }
// START_is_path_like_argument
fn is_path_like_argument(key: &str, value: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("path")
        || key.contains("file")
        || key.contains("dir")
        || value.contains('/')
        || value.starts_with('.')
        || value.starts_with('~')
}
// END_is_path_like_argument

// START_CONTRACT_path_has_parent_dir
// PURPOSE: Reject path traversal before glob matching.
// INPUTS: { value: &str }
// OUTPUTS: { bool }
// START_path_has_parent_dir
fn path_has_parent_dir(value: &str) -> bool {
    Path::new(value)
        .components()
        .any(|component| matches!(component, Component::ParentDir))
}
// END_path_has_parent_dir

// START_CONTRACT_normalize_path_value
// PURPOSE: Normalize common project-relative path spelling before globset matching.
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_normalize_path_value
fn normalize_path_value(value: &str) -> String {
    value
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}
// END_normalize_path_value

// START_CONTRACT_shell_escape
// PURPOSE: Quote one interpolated scalar so user arguments cannot alter shell syntax.
// INPUTS: { value: &str }
// OUTPUTS: { Result<String, UserToolError> }
// START_shell_escape
fn shell_escape(value: &str) -> Result<String, UserToolError> {
    if value.chars().any(|ch| matches!(ch, '\0' | '\n' | '\r')) {
        return Err(UserToolError::InvalidArguments(
            "user tool arguments must not contain control newlines".into(),
        ));
    }
    if value.is_empty() {
        return Ok("''".into());
    }
    Ok(format!("'{}'", value.replace('\'', r#"'\''"#)))
}
// END_shell_escape

// START_CONTRACT_handler_timeout_secs
// PURPOSE: Resolve a handler timeout with a conservative default.
// INPUTS: { handler: &UserToolHandler }
// OUTPUTS: { u64 }
// START_handler_timeout_secs
fn handler_timeout_secs(handler: &UserToolHandler) -> u64 {
    handler.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS)
}
// END_handler_timeout_secs

// START_CONTRACT_execute_shell_command
// PURPOSE: Run a rendered shell command with timeout and kill-on-drop behavior.
// INPUTS: { command: &str }, { timeout_secs: u64 }
// OUTPUTS: { Result<std::process::Output, UserToolError> }
// SIDE_EFFECTS: spawns sh -c
// START_execute_shell_command
async fn execute_shell_command(
    command: &str,
    timeout_secs: u64,
) -> Result<std::process::Output, UserToolError> {
    let mut process = Command::new("sh");
    process.arg("-c").arg(command).kill_on_drop(true);
    match tokio::time::timeout(Duration::from_secs(timeout_secs), process.output()).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => Err(UserToolError::Io(format!(
            "failed to execute user tool command: {error}"
        ))),
        Err(_) => Err(UserToolError::TimedOut(timeout_secs)),
    }
}
// END_execute_shell_command

// START_CONTRACT_render_command_output
// PURPOSE: Combine stdout, stderr, and non-zero exit code into a compact text result.
// INPUTS: { output: &std::process::Output }
// OUTPUTS: { String }
// START_render_command_output
fn render_command_output(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut text = String::new();
    if !stdout.is_empty() {
        text.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !text.ends_with('\n') && !text.is_empty() {
            text.push('\n');
        }
        text.push_str("stderr:\n");
        text.push_str(&stderr);
    }
    if !output.status.success() {
        if !text.ends_with('\n') && !text.is_empty() {
            text.push('\n');
        }
        let code = output
            .status
            .code()
            .map_or("signal".into(), |code| code.to_string());
        text.push_str(&format!("[exit_code: {code}]"));
    }
    text
}
// END_render_command_output

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn builtin_names() -> BTreeSet<String> {
        BTreeSet::from(["semantic_search".to_string(), "verify_project".to_string()])
    }

    fn write_tool(dir: &Path, name: &str, body: Value) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, serde_json::to_string_pretty(&body).unwrap()).unwrap();
        path
    }

    fn valid_tool(name: &str) -> Value {
        serde_json::json!({
            "name": name,
            "description": "Local test tool",
            "version": "0.1.0",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "path"}
                },
                "required": ["path"]
            },
            "handler": {
                "type": "command",
                "command": "printf '\\033[31m%s\\033[0m' ${path}",
                "timeout_secs": 5,
                "allowed_paths": ["src/**"],
                "strip_ansi": true,
                "max_output_tokens": 40
            }
        })
    }

    // START_CONTRACT_loads_valid_tool_with_source_metadata
    // PURPOSE: Verify user tools load and render MCP source metadata.
    // START_loads_valid_tool_with_source_metadata
    #[test]
    fn loads_valid_tool_with_source_metadata() {
        let dir = tempdir().unwrap();
        write_tool(dir.path(), "local_echo.json", valid_tool("local_echo"));

        let report = load_visible_definitions(dir.path(), &ToolProfile::All, &builtin_names());

        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert_eq!(report.total_valid, 1);
        assert_eq!(report.tools[0]["name"], "local_echo");
        assert_eq!(report.tools[0]["_source"], "user");
    }
    // END_loads_valid_tool_with_source_metadata

    // START_CONTRACT_rejects_builtin_override_names
    // PURPOSE: Verify user tool names cannot replace built-in MCP tools.
    // START_rejects_builtin_override_names
    #[test]
    fn rejects_builtin_override_names() {
        let dir = tempdir().unwrap();
        let path = write_tool(dir.path(), "override.json", valid_tool("semantic_search"));

        let validation = validate_file(&path, &builtin_names());

        assert!(!validation.valid);
        assert!(validation
            .errors
            .iter()
            .any(|error| error.contains("overrides a built-in")));
    }
    // END_rejects_builtin_override_names

    // START_CONTRACT_execute_strips_ansi_and_honors_allowed_paths
    // PURPOSE: Verify command execution strips ANSI and accepts allowed project-relative paths.
    // START_execute_strips_ansi_and_honors_allowed_paths
    #[tokio::test]
    async fn execute_strips_ansi_and_honors_allowed_paths() {
        let _cwd = syn_core::utils::test_cwd_lock().lock().await;
        let tool: UserToolDefinition =
            serde_json::from_value(valid_tool("local_echo")).expect("valid tool");

        let output = execute_tool_definition(&tool, &serde_json::json!({"path": "src/main.rs"}))
            .await
            .expect("execute");

        assert_eq!(output, "src/main.rs");
    }
    // END_execute_strips_ansi_and_honors_allowed_paths

    // START_CONTRACT_execute_rejects_disallowed_paths
    // PURPOSE: Verify allowed_paths blocks path arguments outside the configured glob set.
    // START_execute_rejects_disallowed_paths
    #[tokio::test]
    async fn execute_rejects_disallowed_paths() {
        let tool: UserToolDefinition =
            serde_json::from_value(valid_tool("local_echo")).expect("valid tool");

        let error = execute_tool_definition(&tool, &serde_json::json!({"path": "Cargo.toml"}))
            .await
            .expect_err("outside allowed paths");

        assert!(error.to_string().contains("outside allowed_paths"));
    }
    // END_execute_rejects_disallowed_paths

    // START_CONTRACT_execute_trims_long_output
    // PURPOSE: Verify max_output_tokens applies token-budget trimming to command output.
    // START_execute_trims_long_output
    #[tokio::test]
    async fn execute_trims_long_output() {
        let _cwd = syn_core::utils::test_cwd_lock().lock().await;
        let mut value = valid_tool("local_echo");
        value["handler"]["command"] =
            Value::from("printf 'alpha beta gamma delta epsilon zeta eta theta iota kappa'");
        value["handler"]["max_output_tokens"] = Value::from(5);
        let tool: UserToolDefinition = serde_json::from_value(value).expect("valid tool");

        let output = execute_tool_definition(&tool, &serde_json::json!({"path": "src/main.rs"}))
            .await
            .expect("execute");

        assert!(output.contains("Response trimmed") || output.contains("Large response"));
    }
    // END_execute_trims_long_output

    // START_CONTRACT_execute_times_out_long_running_tool
    // PURPOSE: Verify timeout-bounded execution reports a timeout.
    // START_execute_times_out_long_running_tool
    #[tokio::test]
    async fn execute_times_out_long_running_tool() {
        let _cwd = syn_core::utils::test_cwd_lock().lock().await;
        let mut value = valid_tool("slow_tool");
        value["handler"]["command"] = Value::from("sleep 5");
        value["handler"]["timeout_secs"] = Value::from(1);
        let tool: UserToolDefinition = serde_json::from_value(value).expect("valid tool");

        let error = execute_tool_definition(&tool, &serde_json::json!({"path": "src/main.rs"}))
            .await
            .expect_err("timeout");

        assert_eq!(error, UserToolError::TimedOut(1));
    }
    // END_execute_times_out_long_running_tool

    #[test]
    fn merge_visible_definitions_preserves_existing_tools() {
        use crate::mcp::server_tools::ToolProfile;
        let profile = ToolProfile::Minimal;
        let mut tools = vec![serde_json::json!({"name": "existing"})];
        let _report = merge_visible_definitions(&profile, &mut tools);
        // Existing tools should not be removed
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "existing");
    }
}

// END_public_api
