// MODULE_CONTRACT
// MODULE_ID: M-TESTS-MCP-PROTOCOL
// PURPOSE: MCP protocol tests — verify initialize, tools/list shape, response economy/cache schemas, pipelined response IDs, stdio cleanliness, notification silence, and representative grace tool exposure
// SCOPE: Direct handler tests, response economy/cache schema assertions, cache metadata behavior, and binary stdio protocol behavior
// DEPENDS: M-MCP-SERVER, M-MCP-SERVER-TOOLS, M-SKILLS, M-CAPABILITIES

// START_MODULE_MAP
// test_initialize_then_list_tools — MCP handler lists all 44 tools after initialize
// initialized_handler — Creates an initialized in-process MCP handler
// initialized_tools_list — Creates a handler, initializes MCP, and returns tools/list
// tools_call — Executes one initialized tools/call request against a handler
// tool_names — Extracts tool names from a tools/list response
// contains_schema_description_key — Detects schema description metadata in tools/list payloads
// test_tools_list_contains_grace_and_core_tools — Tool list contains representative core and grace tools
// test_tools_list_profiles_return_expected_counts — tools/list profiles expose bounded tool sets
// test_tools_list_full_and_terse_styles — tools/list style controls schema verbosity and economy metadata
// test_tools_list_profile_and_terse_style_compose — tools/list profile and terse style combine
// test_tools_list_response_economy_schema_covers_core_tools — tools/list exposes max_tokens/style on 10+ tools
// test_tools_list_cache_hint_schema_covers_cacheable_tools — tools/list exposes optional cache validators
// test_tools_call_cache_metadata_and_not_modified — project_status emits _meta.cache and honors _if_none_match
// test_tools_call_volatile_semantic_search_reports_zero_ttl — semantic_search stays volatile
// test_tools_call_grace_status_returns_text — Representative grace tool call returns MCP content envelope
// test_concurrent_requests_keep_response_ids — Pipelined stdio requests preserve JSON-RPC response IDs
// test_stdio_keeps_logs_off_stdout_and_notifications_silent — Binary stdio emits only request responses on stdout
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.31.0 - Added MCP cache hint protocol coverage]
// END_CHANGE_SUMMARY

use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};
use syn::mcp::server::SynapseHandler;

// START_CONTRACT_initialized_handler
// PURPOSE: Create an initialized in-process MCP handler for protocol tests
// OUTPUTS: { SynapseHandler }
// SIDE_EFFECTS: creates in-process MCP handler
async fn initialized_handler() -> SynapseHandler {
    let handler = SynapseHandler::new();
    handler
        .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
        .await
        .expect("initialize request should produce a response");
    handler
}

// START_CONTRACT_initialized_tools_list
// PURPOSE: Initialize an in-process MCP handler and execute one tools/list request
// INPUTS: { params: &str }
// OUTPUTS: { serde_json::Value }
// SIDE_EFFECTS: creates in-process MCP handler
async fn initialized_tools_list(params: &str) -> Value {
    let handler = initialized_handler().await;
    let request = format!(r#"{{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{params}}}"#);
    handler
        .handle_message(&request)
        .await
        .expect("tools/list request should produce a response")
}

// START_CONTRACT_tools_call
// PURPOSE: Execute one tools/call request against an initialized in-process handler
// INPUTS: { handler: &SynapseHandler }, { id: u64 }, { name: &str }, { arguments: serde_json::Value }
// OUTPUTS: { serde_json::Value }
// SIDE_EFFECTS: invokes MCP tool handler
async fn tools_call(handler: &SynapseHandler, id: u64, name: &str, arguments: Value) -> Value {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments
        }
    })
    .to_string();
    handler
        .handle_message(&request)
        .await
        .expect("tools/call request should produce a response")
}

// START_CONTRACT_tool_names
// PURPOSE: Extract ordered tool names from a tools/list response
// INPUTS: { response: &serde_json::Value }
// OUTPUTS: { Vec<String> }
fn tool_names(response: &Value) -> Vec<String> {
    response["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["name"].as_str().map(ToOwned::to_owned))
        .collect()
}

// START_CONTRACT_contains_schema_description_key
// PURPOSE: Detect schema description metadata without flagging parameters named description
// INPUTS: { value: &serde_json::Value }
// OUTPUTS: { bool }
fn contains_schema_description_key(value: &Value) -> bool {
    contains_schema_description_key_in_context(value, false)
}

// START_CONTRACT_contains_schema_description_key_in_context
// PURPOSE: Context-aware recursive implementation for schema description detection
// INPUTS: { value: &serde_json::Value }, { is_properties_map: bool }
// OUTPUTS: { bool }
fn contains_schema_description_key_in_context(value: &Value, is_properties_map: bool) -> bool {
    match value {
        Value::Object(object) => {
            (!is_properties_map && object.contains_key("description"))
                || object.iter().any(|(key, child)| {
                    let child_is_properties_map = !is_properties_map && key == "properties";
                    contains_schema_description_key_in_context(child, child_is_properties_map)
                })
        }
        Value::Array(items) => items
            .iter()
            .any(|item| contains_schema_description_key_in_context(item, false)),
        _ => false,
    }
}

#[tokio::test]
// START_CONTRACT_test_initialize_then_list_tools
// PURPOSE: Verify MCP initialize followed by tools/list returns the full tool registry
// SIDE_EFFECTS: creates in-process MCP handler
async fn test_initialize_then_list_tools() {
    let handler = SynapseHandler::new();
    let init = handler
        .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
        .await
        .expect("initialize request should produce a response");
    assert!(init.get("result").is_some());

    let list = handler
        .handle_message(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#)
        .await
        .expect("tools/list request should produce a response");
    let tools = list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 44);
    assert_eq!(list["result"]["profile"], "all");
    assert_eq!(list["result"]["style"], "full");
    assert_eq!(list["result"]["total_available"], 44);
    assert_eq!(list["result"]["total_visible"], 44);
}

#[tokio::test]
// START_CONTRACT_test_tools_list_profiles_return_expected_counts
// PURPOSE: Verify tools/list profile filtering exposes all, verification, minimal, and custom sets
// SIDE_EFFECTS: creates in-process MCP handlers
async fn test_tools_list_profiles_return_expected_counts() {
    let all = initialized_tools_list("{}").await;
    let verification = initialized_tools_list(r#"{"profile":"verification"}"#).await;
    let minimal = initialized_tools_list(r#"{"profile":"minimal"}"#).await;
    let custom =
        initialized_tools_list(r#"{"profile":"custom:semantic_search,verify_project"}"#).await;

    let all_names = tool_names(&all);
    let verification_names = tool_names(&verification);
    let minimal_names = tool_names(&minimal);
    let custom_names = tool_names(&custom);

    assert_eq!(all_names.len(), 44);
    assert!(verification_names.len() <= 10, "{verification:?}");
    assert!(verification_names.contains(&"verify_project".to_string()));
    assert!(verification_names.contains(&"review_code".to_string()));
    assert!(!verification_names.contains(&"semantic_search".to_string()));
    assert_eq!(
        minimal_names,
        vec![
            "semantic_search",
            "graphrag_query",
            "verify_project",
            "review_code",
            "project_status",
            "mental_test_run"
        ]
    );
    assert_eq!(custom_names, vec!["semantic_search", "verify_project"]);
    assert_eq!(custom["result"]["profile"], "custom");
    assert_eq!(custom["result"]["total_available"], 44);
    assert_eq!(custom["result"]["total_visible"], 2);
}

#[tokio::test]
// START_CONTRACT_test_tools_list_full_and_terse_styles
// PURPOSE: Verify tools/list full keeps descriptions and terse removes them with at least 60 percent savings
// SIDE_EFFECTS: creates in-process MCP handlers
async fn test_tools_list_full_and_terse_styles() {
    let full = initialized_tools_list(r#"{"style":"full"}"#).await;
    let terse = initialized_tools_list(r#"{"style":"terse"}"#).await;

    assert_eq!(full["result"]["style"], "full");
    assert_eq!(terse["result"]["style"], "terse");
    assert!(contains_schema_description_key(&full["result"]["tools"]));
    assert!(!contains_schema_description_key(&terse["result"]["tools"]));
    assert_eq!(terse["result"]["total_visible"], 44);
    assert!(terse["result"]["schema_economy"]["savings_pct"]
        .as_f64()
        .is_some_and(|pct| pct >= 60.0));
}

#[tokio::test]
// START_CONTRACT_test_tools_list_profile_and_terse_style_compose
// PURPOSE: Verify tools/list combines profile filtering and terse schema style in one request
// SIDE_EFFECTS: creates in-process MCP handler
async fn test_tools_list_profile_and_terse_style_compose() {
    let response = initialized_tools_list(r#"{"profile":"verification","style":"terse"}"#).await;
    let names = tool_names(&response);

    assert_eq!(response["result"]["profile"], "verification");
    assert_eq!(response["result"]["style"], "terse");
    assert!(names.len() <= 10, "{response:?}");
    assert!(names.contains(&"verify_project".to_string()));
    assert!(!names.contains(&"semantic_search".to_string()));
    assert!(!contains_schema_description_key(
        &response["result"]["tools"]
    ));
    assert!(response["result"]["schema_economy"]["savings_pct"]
        .as_f64()
        .is_some_and(|pct| pct > 0.0));
}

#[tokio::test]
// START_CONTRACT_test_tools_list_response_economy_schema_covers_core_tools
// PURPOSE: Verify tools/list exposes max_tokens and style on at least ten high-output MCP tools
// SIDE_EFFECTS: creates in-process MCP handler
async fn test_tools_list_response_economy_schema_covers_core_tools() {
    let response = initialized_tools_list("{}").await;
    let tools = response["result"]["tools"].as_array().expect("tools array");
    let expected = [
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
    let mut covered = 0_usize;

    for name in expected {
        let tool = tools
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} tool"));
        let properties = tool["inputSchema"]["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("{name} properties"));
        assert!(properties.contains_key("max_tokens"), "{name} max_tokens");
        assert!(properties.contains_key("style"), "{name} style");
        assert_eq!(properties["style"]["enum"][1], "terse");
        covered += 1;
    }

    assert!(covered >= 10, "covered={covered}");
}

#[tokio::test]
// START_CONTRACT_test_tools_list_cache_hint_schema_covers_cacheable_tools
// PURPOSE: Verify tools/list exposes optional _if_none_match only for cacheable tools
// SIDE_EFFECTS: creates in-process MCP handler
async fn test_tools_list_cache_hint_schema_covers_cacheable_tools() {
    let response = initialized_tools_list("{}").await;
    let tools = response["result"]["tools"].as_array().expect("tools array");
    let expected = [
        "graphrag_query",
        "view_signatures",
        "lsp_hover",
        "lsp_references",
        "project_status",
        "traceability_report",
    ];

    for name in expected {
        let tool = tools
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} tool"));
        let properties = tool["inputSchema"]["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("{name} properties"));
        let required = tool["inputSchema"]["required"].as_array();
        assert!(properties.contains_key("_if_none_match"), "{name} cache");
        assert!(
            required
                .map(|required| {
                    !required
                        .iter()
                        .any(|entry| entry.as_str() == Some("_if_none_match"))
                })
                .unwrap_or(true),
            "{name} required _if_none_match"
        );
    }

    let semantic_search = tools
        .iter()
        .find(|tool| tool["name"] == "semantic_search")
        .expect("semantic_search tool");
    let properties = semantic_search["inputSchema"]["properties"]
        .as_object()
        .expect("semantic_search properties");
    assert!(!properties.contains_key("_if_none_match"));
}

#[tokio::test]
// START_CONTRACT_test_tools_call_cache_metadata_and_not_modified
// PURPOSE: Verify project_status emits _meta.cache and matching _if_none_match returns _not_modified
// SIDE_EFFECTS: creates in-process MCP handler
async fn test_tools_call_cache_metadata_and_not_modified() {
    let handler = initialized_handler().await;
    let status = tools_call(&handler, 2, "project_status", serde_json::json!({})).await;
    let status_cache = &status["result"]["_meta"]["cache"];
    let etag = status_cache["etag"].as_str().expect("project_status etag");

    assert!(etag.starts_with("W/\"syn-"));
    assert_eq!(status_cache["ttl_secs"], 15);

    let not_modified = tools_call(
        &handler,
        3,
        "project_status",
        serde_json::json!({"_if_none_match": etag}),
    )
    .await;
    assert_eq!(not_modified["result"]["_not_modified"], true);
    assert_eq!(not_modified["result"]["_meta"]["cache"]["etag"], etag);
}

#[tokio::test]
// START_CONTRACT_test_tools_call_volatile_semantic_search_reports_zero_ttl
// PURPOSE: Verify semantic_search reports ttl_secs=0 and does not return _not_modified
// SIDE_EFFECTS: creates in-process MCP handler and queries local index storage
async fn test_tools_call_volatile_semantic_search_reports_zero_ttl() {
    let handler = initialized_handler().await;
    let first = tools_call(
        &handler,
        2,
        "semantic_search",
        serde_json::json!({"query": "Phase-85", "max_results": 1}),
    )
    .await;
    let etag = first["result"]["_meta"]["cache"]["etag"]
        .as_str()
        .expect("semantic_search etag");
    assert_eq!(first["result"]["_meta"]["cache"]["ttl_secs"], 0);

    let second = tools_call(
        &handler,
        3,
        "semantic_search",
        serde_json::json!({
            "query": "Phase-85",
            "max_results": 1,
            "_if_none_match": etag
        }),
    )
    .await;
    assert_ne!(second["result"]["_not_modified"], true);
    assert_eq!(second["result"]["_meta"]["cache"]["ttl_secs"], 0);
}

#[tokio::test]
// START_CONTRACT_test_tools_call_grace_status_returns_text
// PURPOSE: Verify a representative grace tool call returns a text content envelope
// SIDE_EFFECTS: creates in-process MCP handler
async fn test_tools_call_grace_status_returns_text() {
    let handler = SynapseHandler::new();
    let _ = handler
        .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
        .await;
    let resp = handler
        .handle_message(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"grace_status","arguments":{"detail_level":"summary"}}}"#)
        .await
        .expect("tools/call request should produce a response");
    let text = resp["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("grace_status") || text.contains("Status skill overview"));
}

#[tokio::test]
// START_CONTRACT_test_notifications_return_no_response
// PURPOSE: Verify JSON-RPC notifications are consumed without emitting responses
// SIDE_EFFECTS: creates in-process MCP handler
async fn test_notifications_return_no_response() {
    let handler = SynapseHandler::new();
    let notification = handler
        .handle_message(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
        .await;
    assert!(
        notification.is_none(),
        "notifications must not produce JSON-RPC responses"
    );
}
// END_test_notifications_return_no_response

#[test]
// START_CONTRACT_test_stdio_keeps_logs_off_stdout_and_notifications_silent
// PURPOSE: Verify syn mcp stdout contains only JSON-RPC responses while logs go to stderr
// SIDE_EFFECTS: spawns target/debug/syn mcp with piped stdio
fn test_stdio_keeps_logs_off_stdout_and_notifications_silent() {
    let syn = std::env::current_dir()
        .expect("current dir")
        .join("target/debug/syn");
    let mut child = Command::new(&syn)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn syn mcp");

    {
        let mut stdin = child.stdin.take().expect("child stdin");
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{}}}}"#
        )
        .expect("write initialize");
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
        )
        .expect("write notification");
    }

    let output = child.wait_with_output().expect("wait syn mcp");
    assert!(
        output.status.success(),
        "syn mcp failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "stdout must contain exactly one response line, got: {}",
        stdout
    );
    let response: serde_json::Value =
        serde_json::from_str(lines[0]).expect("stdout line must be JSON-RPC");
    assert_eq!(response["id"], 1);
    assert!(
        response.get("result").is_some(),
        "initialize response must contain result: {}",
        response
    );
}
// END_test_stdio_keeps_logs_off_stdout_and_notifications_silent

#[test]
// START_CONTRACT_test_concurrent_requests_keep_response_ids
// PURPOSE: Verify pipelined stdio requests keep their JSON-RPC response IDs under the MCP pipeline
// SIDE_EFFECTS: spawns target/debug/syn mcp with piped stdio
fn test_concurrent_requests_keep_response_ids() {
    let syn = std::env::current_dir()
        .expect("current dir")
        .join("target/debug/syn");
    let mut child = Command::new(&syn)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn syn mcp");

    {
        let mut stdin = child.stdin.take().expect("child stdin");
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{}}}}"#
        )
        .expect("write initialize");
        for id in [10, 11, 12] {
            writeln!(
                stdin,
                r#"{{"jsonrpc":"2.0","id":{},"method":"tools/list","params":{{}}}}"#,
                id
            )
            .expect("write tools/list");
        }
    }

    let output = child.wait_with_output().expect("wait syn mcp");
    assert!(
        output.status.success(),
        "syn mcp failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ids: Vec<i64> = stdout
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("json response"))
        .filter_map(|response| response["id"].as_i64())
        .collect();
    ids.sort_unstable();

    assert_eq!(ids, vec![1, 10, 11, 12], "stdout was: {}", stdout);
}
// END_test_concurrent_requests_keep_response_ids
