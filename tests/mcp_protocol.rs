// MODULE_CONTRACT
// MODULE_ID: M-TESTS-MCP-PROTOCOL
// PURPOSE: MCP protocol tests — verify initialize, tools/list shape, and representative grace tool exposure
// SCOPE: Direct handler tests for initialize and tools/list protocol behavior
// DEPENDS: M-MCP-SERVER, M-SKILLS, M-CAPABILITIES

// START_MODULE_MAP
// test_initialize_then_list_tools — MCP handler lists all 27 tools after initialize
// test_tools_list_contains_grace_and_core_tools — Tool list contains representative core and grace tools
// test_tools_call_grace_status_returns_text — Representative grace tool call returns MCP content envelope
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.6.0 — Added test function contracts]
// END_CHANGE_SUMMARY

use syn::mcp::server::SynapseHandler;

#[tokio::test]
// START_CONTRACT_test_initialize_then_list_tools
// PURPOSE: Verify MCP initialize followed by tools/list returns the full tool registry
// SIDE_EFFECTS: creates in-process MCP handler
async fn test_initialize_then_list_tools() {
    let mut handler = SynapseHandler::new();
    let init = handler
        .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
        .await;
    assert!(init.get("result").is_some());

    let list = handler
        .handle_message(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#)
        .await;
    let tools = list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 27);
}

#[tokio::test]
// START_CONTRACT_test_tools_call_grace_status_returns_text
// PURPOSE: Verify a representative grace tool call returns a text content envelope
// SIDE_EFFECTS: creates in-process MCP handler
async fn test_tools_call_grace_status_returns_text() {
    let mut handler = SynapseHandler::new();
    handler
        .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
        .await;
    let resp = handler
        .handle_message(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"grace_status","arguments":{"detail_level":"summary"}}}"#)
        .await;
    let text = resp["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("grace_status") || text.contains("Status skill overview"));
}
