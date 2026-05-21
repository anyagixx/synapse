// MODULE_CONTRACT
// MODULE_ID: M-TESTS-MCP-PROTOCOL
// PURPOSE: MCP protocol tests — verify initialize, tools/list shape, stdio cleanliness, notification silence, and representative grace tool exposure
// SCOPE: Direct handler tests and binary stdio protocol behavior
// DEPENDS: M-MCP-SERVER, M-SKILLS, M-CAPABILITIES

// START_MODULE_MAP
// test_initialize_then_list_tools — MCP handler lists all 38 tools after initialize
// test_tools_list_contains_grace_and_core_tools — Tool list contains representative core and grace tools
// test_tools_call_grace_status_returns_text — Representative grace tool call returns MCP content envelope
// test_stdio_keeps_logs_off_stdout_and_notifications_silent — Binary stdio emits only request responses on stdout
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.22.0 - Updated MCP tool count for cascade tools]
// END_CHANGE_SUMMARY

use std::io::Write;
use std::process::{Command, Stdio};
use syn::mcp::server::SynapseHandler;

#[tokio::test]
// START_CONTRACT_test_initialize_then_list_tools
// PURPOSE: Verify MCP initialize followed by tools/list returns the full tool registry
// SIDE_EFFECTS: creates in-process MCP handler
async fn test_initialize_then_list_tools() {
    let mut handler = SynapseHandler::new();
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
    assert_eq!(tools.len(), 38);
}

#[tokio::test]
// START_CONTRACT_test_tools_call_grace_status_returns_text
// PURPOSE: Verify a representative grace tool call returns a text content envelope
// SIDE_EFFECTS: creates in-process MCP handler
async fn test_tools_call_grace_status_returns_text() {
    let mut handler = SynapseHandler::new();
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
    let mut handler = SynapseHandler::new();
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
        let stdin = child.stdin.as_mut().expect("child stdin");
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
