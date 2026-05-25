// MODULE_CONTRACT
// MODULE_ID: M-TEST-MCP-REGRESSION
// PURPOSE: Real stdio JSON-RPC regression helper and tests for the Synapse MCP server.
// SCOPE: Server spawn, request/notification helpers, timeout-bounded stdout reads, response ID checks, and child cleanup.
// DEPENDS: M-MCP-SERVER, M-MCP-SERVER-TOOLS, M-MCP-SERVER-GRACE-TOOLS, M-TEST-FIXTURE
// LINKS:
//   -> Phase-78 (implements) - MCP regression suite
//   -> NFR-002 (traces_to) - reliable MCP protocol regression evidence
//   <- V-M-TEST-MCP-REGRESSION (verified_by) - MCP JSON-RPC verification

// START_MODULE_MAP
// mcp_regression_timeouts - Timeout constants for MCP stdio regression helpers
// McpTestServer - Running syn mcp child process with timeout-bounded JSON-RPC helpers
// spawn_for_fixture - Spawn server with fixture root and isolated XDG homes
// request - Send JSON-RPC request and wait for matching response ID
// notify - Send JSON-RPC notification without waiting for a response
// read_any_response - Read the next JSON-RPC response regardless of id
// read_response - Read responses while skipping notifications
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.2.0 - Added strict MyGRACE contract anchor for MCP regression helper constants]
// END_CHANGE_SUMMARY

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use syn::test::fixture::TestFixture;

// START_CONTRACT_mcp_regression_timeouts
// PURPOSE: Define bounded wait durations used by MCP regression process helpers
// OUTPUTS: { Duration constants }
// SIDE_EFFECTS: none
// LINKS:
//   -> M-MCP-SERVER (depends) - stdio server lifecycle timing
//   -> NFR-002 (traces_to) - regression waits must remain bounded
// START_mcp_regression_timeouts
const MCP_RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);
const MCP_KILL_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
// END_mcp_regression_timeouts

// START_public_api

// START_McpTestServer
pub struct McpTestServer {
    child: Child,
    stdin: ChildStdin,
    stdout_rx: Receiver<String>,
    reader_thread: Option<JoinHandle<()>>,
    next_id: u64,
    timeout: Duration,
}
// END_McpTestServer

impl McpTestServer {
    // START_CONTRACT_McpTestServer::spawn
    // PURPOSE: Spawn syn mcp in a fixture root with default environment
    // INPUTS: { fixture_root: &Path }
    // OUTPUTS: { anyhow::Result<McpTestServer> }
    // SIDE_EFFECTS: starts a child process and stdout reader thread
    // LINKS:
    //   -> M-MCP-SERVER (depends) - binary stdio server under test
    //   -> NFR-002 (traces_to) - real MCP server regression coverage
    // <LOG id="mcp_regression_spawned" level="INFO" ref="mcp-spawn" module="M-TEST-MCP-REGRESSION" contract="McpTestServer::spawn">
    //   EVENT: mcp_regression_spawned
    //   EXPECTATION: helper starts syn mcp with piped stdin/stdout
    //   DECISION: stderr is inherited so stdout remains JSON-RPC-only
    //   RESULT: success
    //   TRACEABILITY: NFR-002
    // </LOG>
    // START_mcp_test_server_spawn
    pub fn spawn(fixture_root: &Path) -> anyhow::Result<Self> {
        Self::spawn_with_env(fixture_root, std::iter::empty::<(&'static str, &Path)>())
    }
    // END_mcp_test_server_spawn

    // START_CONTRACT_McpTestServer::spawn_for_fixture
    // PURPOSE: Spawn syn mcp in a TestFixture with isolated config and data homes
    // INPUTS: { fixture: &TestFixture }
    // OUTPUTS: { anyhow::Result<McpTestServer> }
    // SIDE_EFFECTS: starts a child process and stdout reader thread
    // LINKS:
    //   -> M-TEST-FIXTURE (depends) - fixture root and isolated XDG homes
    //   -> NFR-002 (traces_to) - regression server must not use developer checkout state
    // START_mcp_test_server_spawn_for_fixture
    pub fn spawn_for_fixture(fixture: &TestFixture) -> anyhow::Result<Self> {
        Self::spawn_with_env(fixture.root(), fixture.command_env())
    }
    // END_mcp_test_server_spawn_for_fixture

    // START_CONTRACT_McpTestServer::request
    // PURPOSE: Send a JSON-RPC request and read the matching response ID
    // INPUTS: { method: &str }, { params: &Value }
    // OUTPUTS: { anyhow::Result<Value> }
    // SIDE_EFFECTS: writes to child stdin and reads child stdout
    // LINKS:
    //   -> M-MCP-SERVER (depends) - JSON-RPC request/response behavior
    //   -> NFR-002 (traces_to) - response IDs must be preserved
    // <LOG id="mcp_regression_request_completed" level="INFO" ref="mcp-request" module="M-TEST-MCP-REGRESSION" contract="McpTestServer::request">
    //   EVENT: mcp_regression_request_completed
    //   EXPECTATION: helper returns only the response matching the sent request ID
    //   DECISION: notification lines are skipped because they do not contain an id
    //   RESULT: success
    //   TRACEABILITY: NFR-002
    // </LOG>
    // START_mcp_test_server_request
    pub fn request(&mut self, method: &str, params: &Value) -> anyhow::Result<Value> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.write_message(&request)?;
        self.read_response(id)
    }
    // END_mcp_test_server_request

    // START_CONTRACT_McpTestServer::initialize
    // PURPOSE: Send a standard MCP initialize request
    // OUTPUTS: { anyhow::Result<Value> }
    // SIDE_EFFECTS: writes initialize request and reads response
    // LINKS:
    //   -> M-MCP-SERVER (depends) - initialize handshake
    //   -> NFR-002 (traces_to) - MCP protocol startup regression
    // START_mcp_test_server_initialize
    pub fn initialize(&mut self) -> anyhow::Result<Value> {
        self.request(
            "initialize",
            &json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "synapse-mcp-regression", "version": "1.0" }
            }),
        )
    }
    // END_mcp_test_server_initialize

    // START_CONTRACT_McpTestServer::notify
    // PURPOSE: Send a JSON-RPC notification without expecting a response
    // INPUTS: { method: &str }, { params: &Value }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes to child stdin
    // LINKS:
    //   -> M-MCP-SERVER (depends) - notification handling
    //   -> NFR-002 (traces_to) - notifications must be response-silent
    // START_mcp_test_server_notify
    pub fn notify(&mut self, method: &str, params: &Value) -> anyhow::Result<()> {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))
    }
    // END_mcp_test_server_notify

    // START_CONTRACT_McpTestServer::send_raw
    // PURPOSE: Send raw JSON-RPC text for malformed input regression tests
    // INPUTS: { raw: &str }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes to child stdin
    // LINKS:
    //   -> M-MCP-SERVER (depends) - malformed JSON handling
    //   -> NFR-002 (traces_to) - protocol errors must not panic
    // START_mcp_test_server_send_raw
    pub fn send_raw(&mut self, raw: &str) -> anyhow::Result<()> {
        self.stdin.write_all(raw.as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;
        Ok(())
    }
    // END_mcp_test_server_send_raw

    // START_CONTRACT_McpTestServer::read_any_response
    // PURPOSE: Read the next JSON-RPC response regardless of request ID
    // OUTPUTS: { anyhow::Result<Value> }
    // SIDE_EFFECTS: reads child stdout
    // LINKS:
    //   -> M-MCP-SERVER (depends) - malformed JSON and protocol error responses
    //   -> NFR-002 (traces_to) - negative-path reads must remain timeout bounded
    // START_mcp_test_server_read_any_response
    pub fn read_any_response(&mut self) -> anyhow::Result<Value> {
        self.read_next_json()
    }
    // END_mcp_test_server_read_any_response

    // START_CONTRACT_McpTestServer::spawn_with_env
    // PURPOSE: Spawn syn mcp with caller-provided environment overrides
    // INPUTS: { fixture_root: &Path }, { env: impl IntoIterator<Item = (&'static str, &Path)> }
    // OUTPUTS: { anyhow::Result<McpTestServer> }
    // SIDE_EFFECTS: starts a child process and stdout reader thread
    // LINKS:
    //   -> M-MCP-SERVER (depends) - server process management
    //   -> NFR-002 (traces_to) - deterministic test process setup
    // START_mcp_test_server_spawn_with_env
    fn spawn_with_env<'a, I>(fixture_root: &Path, env: I) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = (&'static str, &'a Path)>,
    {
        let mut command = Command::new(syn_binary_path());
        command
            .arg("mcp")
            .current_dir(fixture_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        for (key, value) in env {
            command.env(key, value);
        }

        let mut child = command.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to open MCP child stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to open MCP child stdout"))?;
        let (stdout_tx, stdout_rx) = mpsc::channel();
        let reader_thread = std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if stdout_tx.send(line).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Self {
            child,
            stdin,
            stdout_rx,
            reader_thread: Some(reader_thread),
            next_id: 1,
            timeout: MCP_RESPONSE_TIMEOUT,
        })
    }
    // END_mcp_test_server_spawn_with_env

    // START_CONTRACT_McpTestServer::write_message
    // PURPOSE: Serialize and write one JSON-RPC message line
    // INPUTS: { message: &Value }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes to child stdin
    // LINKS:
    //   -> M-MCP-SERVER (depends) - newline-delimited stdio protocol
    //   -> NFR-002 (traces_to) - request writes are explicit and flush
    // START_mcp_test_server_write_message
    fn write_message(&mut self, message: &Value) -> anyhow::Result<()> {
        let mut text = serde_json::to_string(message)?;
        text.push('\n');
        self.stdin.write_all(text.as_bytes())?;
        self.stdin.flush()?;
        Ok(())
    }
    // END_mcp_test_server_write_message

    // START_CONTRACT_McpTestServer::read_response
    // PURPOSE: Read stdout until the requested response ID is observed or timeout expires
    // INPUTS: { id: u64 }
    // OUTPUTS: { anyhow::Result<Value> }
    // SIDE_EFFECTS: reads from stdout channel
    // LINKS:
    //   -> M-MCP-SERVER (depends) - JSON-RPC response stream
    //   -> NFR-002 (traces_to) - timeout-bounded protocol reads
    // START_mcp_test_server_read_response
    fn read_response(&mut self, id: u64) -> anyhow::Result<Value> {
        let started = Instant::now();
        loop {
            let response =
                self.read_next_json_with_deadline(started, &format!("response id {id}"))?;
            let Some(response_id) = response.get("id") else {
                continue;
            };
            if response_id != &json!(id) {
                anyhow::bail!("MCP response id mismatch: expected {id}, got {response_id}");
            }
            return Ok(response);
        }
    }
    // END_mcp_test_server_read_response

    // START_CONTRACT_McpTestServer::read_next_json
    // PURPOSE: Read the next non-empty stdout line as JSON within the helper timeout
    // OUTPUTS: { anyhow::Result<Value> }
    // SIDE_EFFECTS: reads child stdout
    // LINKS:
    //   -> M-MCP-SERVER (depends) - stdout must remain JSON-RPC-only
    // START_mcp_test_server_read_next_json
    fn read_next_json(&mut self) -> anyhow::Result<Value> {
        self.read_next_json_with_deadline(Instant::now(), "next MCP response")
    }
    // END_mcp_test_server_read_next_json

    // START_CONTRACT_McpTestServer::read_next_json_with_deadline
    // PURPOSE: Read one non-empty JSON line before the shared deadline expires
    // INPUTS: { started: Instant }, { label: &str }
    // OUTPUTS: { anyhow::Result<Value> }
    // SIDE_EFFECTS: reads child stdout
    // LINKS:
    //   -> M-MCP-SERVER (depends) - protocol response stream
    // START_mcp_test_server_read_next_json_with_deadline
    fn read_next_json_with_deadline(
        &mut self,
        started: Instant,
        label: &str,
    ) -> anyhow::Result<Value> {
        loop {
            let Some(remaining) = self.timeout.checked_sub(started.elapsed()) else {
                anyhow::bail!("timed out waiting for MCP {label}");
            };
            let line = match self.stdout_rx.recv_timeout(remaining) {
                Ok(line) => line,
                Err(RecvTimeoutError::Timeout) => {
                    anyhow::bail!("timed out waiting for MCP {label}");
                }
                Err(RecvTimeoutError::Disconnected) => {
                    anyhow::bail!("MCP stdout closed while waiting for {label}");
                }
            };
            if line.trim().is_empty() {
                continue;
            }
            return serde_json::from_str(line.trim()).map_err(Into::into);
        }
    }
    // END_mcp_test_server_read_next_json_with_deadline
}

impl Drop for McpTestServer {
    // START_CONTRACT_McpTestServer::drop
    // PURPOSE: Kill and wait on the MCP child process during test cleanup
    // SIDE_EFFECTS: may terminate child process and join stdout reader thread
    // LINKS:
    //   -> M-MCP-SERVER (depends) - child process cleanup
    //   -> NFR-002 (traces_to) - regression tests must not leak servers
    // START_mcp_test_server_drop
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                let _ = self.child.kill();
                let started = Instant::now();
                while started.elapsed() < MCP_KILL_WAIT_TIMEOUT {
                    if matches!(self.child.try_wait(), Ok(Some(_))) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                let _ = self.child.wait();
            }
        }
        if let Some(reader_thread) = self.reader_thread.take() {
            let _ = reader_thread.join();
        }
    }
    // END_mcp_test_server_drop
}

// END_public_api

// START_CONTRACT_syn_binary_path
// PURPOSE: Resolve the syn binary path for integration tests
// OUTPUTS: { PathBuf }
// LINKS:
//   -> M-TEST-MCP-REGRESSION (depends) - test process spawn
//   -> NFR-002 (traces_to) - binary resolution must be deterministic under cargo test
// START_syn_binary_path
fn syn_binary_path() -> PathBuf {
    option_env!("CARGO_BIN_EXE_syn")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("target/debug/syn")
        })
}
// END_syn_binary_path

#[cfg(test)]
mod tests {
    use super::*;
    use syn::test::fixture::{FixtureTemplate, TestFixture};

    #[test]
    fn mcp_regression_helper_initializes_minimal_fixture() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let mut server = McpTestServer::spawn_for_fixture(&fixture).expect("spawn mcp server");

        let response = server.initialize().expect("initialize response");

        assert_eq!(response["id"], 1);
        assert!(response["result"].is_object(), "{response}");
    }

    #[test]
    fn mcp_regression_helper_skips_notifications_and_preserves_ids() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let mut server = McpTestServer::spawn_for_fixture(&fixture).expect("spawn mcp server");
        server.initialize().expect("initialize response");
        server
            .notify("notifications/initialized", &json!({}))
            .expect("send notification");

        let response = server
            .request("tools/list", &json!({}))
            .expect("tools/list response");

        assert_eq!(response["id"], 2);
        assert!(response["result"]["tools"].is_array(), "{response}");
    }

    #[test]
    fn mcp_regression_tools_list_contains_required_tools() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let mut server = McpTestServer::spawn_for_fixture(&fixture).expect("spawn mcp server");
        server.initialize().expect("initialize response");
        server
            .notify("notifications/initialized", &json!({}))
            .expect("send notification");

        let response = server
            .request("tools/list", &json!({}))
            .expect("tools/list response");
        let tools = response["result"]["tools"]
            .as_array()
            .expect("tools/list array");
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();

        for expected in [
            "semantic_search",
            "verify_project",
            "graphrag_query",
            "project_status",
        ] {
            assert!(
                names.contains(&expected),
                "tools/list missing {expected}: {response}"
            );
        }
    }

    #[test]
    fn mcp_regression_semantic_search_returns_content_envelope() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let mut server = McpTestServer::spawn_for_fixture(&fixture).expect("spawn mcp server");
        server.initialize().expect("initialize response");

        let response = call_tool(
            &mut server,
            "semantic_search",
            json!({
                "query": "do_work",
                "max_results": 3
            }),
        )
        .expect("semantic_search response");

        assert_eq!(response["result"]["isError"], false, "{response}");
        let text = response_content_text(&response);
        assert!(
            text.contains("No results found") || text.contains("src/main.rs"),
            "unexpected semantic_search content: {response}"
        );
    }

    #[test]
    fn mcp_regression_verify_project_reports_broken_fixture_failures() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Broken)
            .build()
            .expect("fixture");
        let mut server = McpTestServer::spawn_for_fixture(&fixture).expect("spawn mcp server");
        server.initialize().expect("initialize response");

        let response = call_tool(
            &mut server,
            "verify_project",
            json!({
                "level": "all",
                "profile": "balanced"
            }),
        )
        .expect("verify_project response");

        assert_eq!(response["result"]["isError"], false, "{response}");
        let text = response_content_text(&response);
        assert!(
            text.contains("[FAIL]") || text.contains("FAILED") || text.contains("FAILURE PACKETS"),
            "broken fixture should report verification failures: {response}"
        );
    }

    #[test]
    fn mcp_regression_malformed_json_returns_json_rpc_error() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let mut server = McpTestServer::spawn_for_fixture(&fixture).expect("spawn mcp server");
        server.initialize().expect("initialize response");

        server.send_raw("{malformed-json").expect("send raw input");
        let response = server.read_any_response().expect("malformed response");

        assert!(response["error"].is_object(), "{response}");
        assert_eq!(response["error"]["code"], -32700, "{response}");
        assert!(response["id"].is_null(), "{response}");
    }

    #[test]
    fn mcp_regression_helper_supports_root_spawn_and_raw_write() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let mut server = McpTestServer::spawn(fixture.root()).expect("spawn mcp server");

        server.send_raw("{malformed-json").expect("send raw input");
    }

    fn call_tool(
        server: &mut McpTestServer,
        name: &str,
        arguments: Value,
    ) -> anyhow::Result<Value> {
        server.request(
            "tools/call",
            &json!({
                "name": name,
                "arguments": arguments,
            }),
        )
    }

    fn response_content_text(response: &Value) -> String {
        response["result"]["content"]
            .as_array()
            .expect("content array")
            .iter()
            .filter_map(|entry| entry["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
