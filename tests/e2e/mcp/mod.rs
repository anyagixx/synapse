// MODULE_CONTRACT
// MODULE_ID: M-TEST-MCP-REGRESSION
// PURPOSE: Real stdio JSON-RPC regression helper and tests for the Synapse MCP server.
// SCOPE: Server spawn, request/notification helpers, timeout-bounded stdout reads, response ID checks, token economy smoke coverage, and child cleanup.
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
// tools_list_names - Extract tool names from a tools/list response
// contains_schema_description_key - Detect schema description metadata in tools/list payloads
// mcp_regression_tools_list_profiles_and_terse_style - Real stdio tools/list profile and terse coverage
// mcp_regression_token_economy_gate - Real stdio token economy smoke coverage
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.8.0 - Added token economy e2e smoke gate]
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

    // START_CONTRACT_tools_list_names
    // PURPOSE: Extract ordered tool names from a tools/list response
    // INPUTS: { response: &Value }
    // OUTPUTS: { Vec<String> }
    // START_tools_list_names
    fn tools_list_names(response: &Value) -> Vec<String> {
        response["result"]["tools"]
            .as_array()
            .expect("tools/list array")
            .iter()
            .filter_map(|tool| tool["name"].as_str().map(ToOwned::to_owned))
            .collect()
    }
    // END_tools_list_names

    // START_CONTRACT_contains_schema_description_key
    // PURPOSE: Detect schema description metadata without flagging parameters named description
    // INPUTS: { value: &Value }
    // OUTPUTS: { bool }
    // START_contains_schema_description_key
    fn contains_schema_description_key(value: &Value) -> bool {
        contains_schema_description_key_in_context(value, false)
    }
    // END_contains_schema_description_key

    // START_CONTRACT_contains_schema_description_key_in_context
    // PURPOSE: Context-aware recursive implementation for schema description detection
    // INPUTS: { value: &Value }, { is_properties_map: bool }
    // OUTPUTS: { bool }
    // START_contains_schema_description_key_in_context
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
    // END_contains_schema_description_key_in_context

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
    // START_CONTRACT_mcp_regression_tools_list_profiles_and_terse_style
    // PURPOSE: Verify real stdio tools/list supports all profile and terse schema combinations
    // SIDE_EFFECTS: spawns syn mcp with a Minimal fixture
    fn mcp_regression_tools_list_profiles_and_terse_style() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let mut server = McpTestServer::spawn_for_fixture(&fixture).expect("spawn mcp server");
        server.initialize().expect("initialize response");
        server
            .notify("notifications/initialized", &json!({}))
            .expect("send notification");

        let all = server
            .request("tools/list", &json!({}))
            .expect("all tools/list response");
        let verification = server
            .request("tools/list", &json!({"profile": "verification"}))
            .expect("verification tools/list response");
        let minimal = server
            .request("tools/list", &json!({"profile": "minimal"}))
            .expect("minimal tools/list response");
        let custom = server
            .request(
                "tools/list",
                &json!({"profile": "custom:semantic_search,verify_project"}),
            )
            .expect("custom tools/list response");
        let terse = server
            .request("tools/list", &json!({"style": "terse"}))
            .expect("terse tools/list response");
        let verification_terse = server
            .request(
                "tools/list",
                &json!({"profile": "verification", "style": "terse"}),
            )
            .expect("profile terse tools/list response");

        let all_names = tools_list_names(&all);
        let verification_names = tools_list_names(&verification);
        let minimal_names = tools_list_names(&minimal);
        let custom_names = tools_list_names(&custom);
        let verification_terse_names = tools_list_names(&verification_terse);

        assert_eq!(all_names.len(), 48);
        assert_eq!(all["result"]["profile"], "all");
        assert_eq!(all["result"]["style"], "full");
        assert_eq!(all["result"]["total_visible"], 48);
        assert!(contains_schema_description_key(&all["result"]["tools"]));

        assert!(verification_names.len() <= 10, "{verification}");
        assert!(verification_names.contains(&"verify_project".to_string()));
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
        assert_eq!(custom["result"]["total_visible"], 2);

        assert_eq!(terse["result"]["style"], "terse");
        assert!(!contains_schema_description_key(&terse["result"]["tools"]));
        assert!(terse["result"]["schema_economy"]["savings_pct"]
            .as_f64()
            .is_some_and(|pct| pct >= 60.0));

        assert_eq!(verification_terse["result"]["profile"], "verification");
        assert_eq!(verification_terse["result"]["style"], "terse");
        assert!(verification_terse_names.len() <= 10, "{verification_terse}");
        assert!(!contains_schema_description_key(
            &verification_terse["result"]["tools"]
        ));
    }

    #[test]
    // START_CONTRACT_mcp_regression_token_economy_gate
    // PURPOSE: Verify real stdio MCP covers the UPGRADE_4 token economy surface.
    // SIDE_EFFECTS: spawns syn mcp with a Minimal fixture
    fn mcp_regression_token_economy_gate() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");
        let mut server = McpTestServer::spawn_for_fixture(&fixture).expect("spawn mcp server");
        server.initialize().expect("initialize response");

        let terse_tools = server
            .request(
                "tools/list",
                &json!({"profile": "verification", "style": "terse"}),
            )
            .expect("terse tools/list response");
        assert_eq!(terse_tools["result"]["profile"], "verification");
        assert_eq!(terse_tools["result"]["style"], "terse");
        assert!(!contains_schema_description_key(
            &terse_tools["result"]["tools"]
        ));

        let recommendation = call_tool(
            &mut server,
            "tools/recommend",
            json!({"context": "verify contracts", "max_tools": 4}),
        )
        .expect("tools/recommend response");
        let recommended = recommendation["result"]["recommended_tools"]
            .as_array()
            .expect("recommended tools");
        assert!(recommended
            .iter()
            .any(|tool| tool["name"] == "verify_project"));

        let budget = call_tool(
            &mut server,
            "check_budget",
            json!({"estimated_tokens": 1024}),
        )
        .expect("check_budget response");
        assert_eq!(budget["result"]["budget"]["status"], "normal");

        let pressure = call_tool(&mut server, "context_pressure", json!({}))
            .expect("context_pressure response");
        assert_eq!(pressure["result"]["context_window_limit"], 200_000);

        let status =
            call_tool(&mut server, "project_status", json!({})).expect("project_status response");
        let etag = status["result"]["_meta"]["cache"]["etag"]
            .as_str()
            .expect("project_status cache etag")
            .to_string();
        let not_modified = call_tool(
            &mut server,
            "project_status",
            json!({"_if_none_match": etag}),
        )
        .expect("project_status not modified response");
        assert_eq!(not_modified["result"]["_not_modified"], true);
    }
    // END_mcp_regression_token_economy_gate

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
