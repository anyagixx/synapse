// MODULE_CONTRACT
// MODULE_ID: M-MCP-PIPELINE
// PURPOSE: Bounded MCP stdio pipeline that decouples request reads, concurrent handler execution, and single-writer response serialization
// SCOPE: Pipeline config, bounded JSON-RPC line reads, initialize-before-concurrency ordering, handler task spawning, panic isolation, response queue writing, EOF shutdown
// DEPENDS: M-MCP-SERVER-RESPONSE
// LINKS:
//   -> M-MCP-SERVER (depends) - stdio server entry point will route through this pipeline
//   -> M-MCP-SERVER-RESPONSE (depends) - JSON-RPC error construction

// START_MODULE_MAP
// McpPipelineConfig — Runtime bounds for response queue, concurrency, and message size
// PipelineHandler — Async JSON-RPC line handler abstraction used by McpServer
// run_stdio_pipeline — Reads requests, dispatches handlers, and writes JSON-RPC responses
// read_bounded_json_rpc_line — Reads one bounded JSON-RPC line
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added bounded MCP stdio pipeline runtime]
// END_CHANGE_SUMMARY

use super::server_response;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Semaphore};

pub const DEFAULT_RESPONSE_QUEUE_CAPACITY: usize = 128;
pub const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 16;
pub const MAX_JSON_RPC_MESSAGE_BYTES: usize = 10_485_760;

// START_public_api

// START_PipelineHandler
pub type PipelineHandler =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Option<Value>> + Send>> + Send + Sync>;
// END_PipelineHandler

// START_McpPipelineConfig
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct McpPipelineConfig {
    pub response_queue_capacity: usize,
    pub max_concurrent_requests: usize,
    pub max_message_bytes: usize,
}
// END_McpPipelineConfig

impl Default for McpPipelineConfig {
    // START_CONTRACT_McpPipelineConfig::default
    // PURPOSE: Return release-safe MCP pipeline bounds
    // OUTPUTS: { McpPipelineConfig }
    // START_mcp_pipeline_config_default
    fn default() -> Self {
        Self {
            response_queue_capacity: DEFAULT_RESPONSE_QUEUE_CAPACITY,
            max_concurrent_requests: DEFAULT_MAX_CONCURRENT_REQUESTS,
            max_message_bytes: MAX_JSON_RPC_MESSAGE_BYTES,
        }
    }
    // END_mcp_pipeline_config_default
}

impl McpPipelineConfig {
    // START_CONTRACT_McpPipelineConfig::normalized
    // PURPOSE: Replace zero config bounds with release-safe defaults
    // OUTPUTS: { McpPipelineConfig }
    // START_mcp_pipeline_config_normalized
    fn normalized(self) -> Self {
        Self {
            response_queue_capacity: non_zero_or_default(
                self.response_queue_capacity,
                DEFAULT_RESPONSE_QUEUE_CAPACITY,
            ),
            max_concurrent_requests: non_zero_or_default(
                self.max_concurrent_requests,
                DEFAULT_MAX_CONCURRENT_REQUESTS,
            ),
            max_message_bytes: non_zero_or_default(
                self.max_message_bytes,
                MAX_JSON_RPC_MESSAGE_BYTES,
            ),
        }
    }
    // END_mcp_pipeline_config_normalized
}

// START_CONTRACT_run_stdio_pipeline
// PURPOSE: Run a bounded MCP stdio pipeline with concurrent handler tasks and a single response writer
// INPUTS: { input: impl AsyncRead }, { output: impl AsyncWrite }, { handler: PipelineHandler }, { config: McpPipelineConfig }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: reads JSON-RPC lines, writes newline-delimited JSON-RPC responses, spawns handler tasks
// START_run_stdio_pipeline
pub async fn run_stdio_pipeline<R, W>(
    input: R,
    output: W,
    handler: PipelineHandler,
    config: McpPipelineConfig,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Send + Unpin + 'static,
{
    let config = config.normalized();
    let (tx, rx) = mpsc::channel::<PipelineResponse>(config.response_queue_capacity);
    let writer_task = tokio::spawn(write_response_loop(output, rx));
    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_requests));
    let mut reader = BufReader::new(input);
    let mut initialized = false;
    let mut sequence = 0_u64;

    loop {
        let line = match read_bounded_json_rpc_line(&mut reader, config.max_message_bytes).await {
            Ok(Some(line)) => line,
            Ok(None) => {
                tracing::info!("[McpPipeline][run_stdio_pipeline][EOF] stdin closed");
                break;
            }
            Err(error) => {
                let response =
                    server_response::error(None, -32600, format!("Invalid request: {}", error));
                send_response(&tx, sequence, response).await;
                break;
            }
        };

        let line = line.trim().to_string();
        if line.is_empty() || !line.starts_with('{') {
            continue;
        }

        let current_sequence = sequence;
        sequence = sequence.saturating_add(1);
        let is_initialize = is_initialize_message(&line);
        if !initialized {
            if let Some(response) = run_handler_with_panic_guard(handler.clone(), line).await {
                send_response(&tx, current_sequence, response).await;
            }
            if is_initialize {
                initialized = true;
            }
            continue;
        }

        spawn_handler(
            current_sequence,
            line,
            handler.clone(),
            tx.clone(),
            semaphore.clone(),
        );
    }

    drop(tx);
    writer_task
        .await
        .map_err(|error| anyhow::anyhow!("MCP response writer task failed: {}", error))??;
    Ok(())
}
// END_run_stdio_pipeline

// START_CONTRACT_read_bounded_json_rpc_line
// PURPOSE: Read one JSON-RPC stdio line without allowing unbounded allocation
// INPUTS: { reader: &mut impl AsyncBufRead + Unpin }, { max_bytes: usize }
// OUTPUTS: { anyhow::Result<Option<String>> }
// START_read_bounded_json_rpc_line
pub(crate) async fn read_bounded_json_rpc_line<R>(
    reader: &mut R,
    max_bytes: usize,
) -> anyhow::Result<Option<String>>
where
    R: AsyncBufRead + Unpin,
{
    let mut buf = Vec::new();
    let max_bytes = non_zero_or_default(max_bytes, MAX_JSON_RPC_MESSAGE_BYTES);
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            if buf.is_empty() {
                return Ok(None);
            }
            return String::from_utf8(buf)
                .map(Some)
                .map_err(|err| anyhow::anyhow!("JSON-RPC message is not valid UTF-8: {}", err));
        }

        if let Some(newline_pos) = available.iter().position(|byte| *byte == b'\n') {
            if buf.len().saturating_add(newline_pos) > max_bytes {
                reader.consume(newline_pos + 1);
                anyhow::bail!("JSON-RPC message exceeds max size of {} bytes", max_bytes);
            }
            buf.extend_from_slice(&available[..newline_pos]);
            reader.consume(newline_pos + 1);
            return String::from_utf8(buf)
                .map(Some)
                .map_err(|err| anyhow::anyhow!("JSON-RPC message is not valid UTF-8: {}", err));
        }

        if buf.len().saturating_add(available.len()) > max_bytes {
            let consumed = available.len();
            reader.consume(consumed);
            anyhow::bail!("JSON-RPC message exceeds max size of {} bytes", max_bytes);
        }
        let consumed = available.len();
        buf.extend_from_slice(available);
        reader.consume(consumed);
    }
}
// END_read_bounded_json_rpc_line

// END_public_api

// START_PipelineResponse
struct PipelineResponse {
    sequence: u64,
    response: Value,
}
// END_PipelineResponse

// START_CONTRACT_spawn_handler
// PURPOSE: Spawn one bounded handler task and send its response to the single-writer queue
// INPUTS: { sequence: u64 }, { line: String }, { handler: PipelineHandler }, { tx: mpsc::Sender<PipelineResponse> }, { semaphore: Arc<Semaphore> }
// SIDE_EFFECTS: spawns a tokio task
// START_spawn_handler
fn spawn_handler(
    sequence: u64,
    line: String,
    handler: PipelineHandler,
    tx: mpsc::Sender<PipelineResponse>,
    semaphore: Arc<Semaphore>,
) {
    tokio::spawn(async move {
        let permit = match semaphore.acquire_owned().await {
            Ok(permit) => permit,
            Err(error) => {
                tracing::warn!(
                    "[McpPipeline][spawn_handler][SEMAPHORE] cannot acquire permit: {}",
                    error
                );
                return;
            }
        };
        let response = run_handler_with_panic_guard(handler, line).await;
        drop(permit);
        if let Some(response) = response {
            send_response(&tx, sequence, response).await;
        }
    });
}
// END_spawn_handler

// START_CONTRACT_run_handler_with_panic_guard
// PURPOSE: Convert handler panics into JSON-RPC internal errors without terminating the pipeline
// INPUTS: { handler: PipelineHandler }, { line: String }
// OUTPUTS: { Option<Value> }
// SIDE_EFFECTS: spawns and awaits a handler task
// START_run_handler_with_panic_guard
async fn run_handler_with_panic_guard(handler: PipelineHandler, line: String) -> Option<Value> {
    let id = request_id_from_line(&line);
    let join = tokio::spawn(async move { (handler)(line).await });
    match join.await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!("[McpPipeline][spawn_handler][PANIC] {}", error);
            Some(server_response::error(
                id,
                -32603,
                "MCP handler task failed",
            ))
        }
    }
}
// END_run_handler_with_panic_guard

// START_CONTRACT_write_response_loop
// PURPOSE: Serialize JSON-RPC responses through a single output writer
// INPUTS: { output: impl AsyncWrite }, { rx: mpsc::Receiver<PipelineResponse> }
// OUTPUTS: { anyhow::Result<usize> — number of written responses }
// SIDE_EFFECTS: writes and flushes newline-delimited JSON to output
// START_write_response_loop
async fn write_response_loop<W>(
    mut output: W,
    mut rx: mpsc::Receiver<PipelineResponse>,
) -> anyhow::Result<usize>
where
    W: AsyncWrite + Unpin,
{
    let mut written = 0usize;
    while let Some(item) = rx.recv().await {
        let msg = serde_json::to_string(&item.response)?;
        tracing::debug!(
            "[McpPipeline][write_response_loop][WRITE] sequence={} bytes={}",
            item.sequence,
            msg.len()
        );
        output.write_all(msg.as_bytes()).await?;
        output.write_all(b"\n").await?;
        output.flush().await?;
        written += 1;
    }
    Ok(written)
}
// END_write_response_loop

// START_CONTRACT_send_response
// PURPOSE: Send a response to the writer queue without panicking when the receiver has closed
// INPUTS: { tx: &mpsc::Sender<PipelineResponse> }, { sequence: u64 }, { response: Value }
// SIDE_EFFECTS: sends through mpsc channel
// START_send_response
async fn send_response(tx: &mpsc::Sender<PipelineResponse>, sequence: u64, response: Value) {
    if let Err(error) = tx.send(PipelineResponse { sequence, response }).await {
        tracing::warn!(
            "[McpPipeline][send_response][CLOSED] response queue closed: {}",
            error
        );
    }
}
// END_send_response

// START_CONTRACT_is_initialize_message
// PURPOSE: Check whether a raw JSON-RPC line is an initialize request
// INPUTS: { line: &str }
// OUTPUTS: { bool }
// START_is_initialize_message
fn is_initialize_message(line: &str) -> bool {
    serde_json::from_str::<Value>(line)
        .ok()
        .and_then(|message| {
            message
                .get("method")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .as_deref()
        == Some("initialize")
}
// END_is_initialize_message

// START_CONTRACT_request_id_from_line
// PURPOSE: Extract a JSON-RPC request id for panic/error responses
// INPUTS: { line: &str }
// OUTPUTS: { Option<Value> }
// START_request_id_from_line
fn request_id_from_line(line: &str) -> Option<Value> {
    serde_json::from_str::<Value>(line)
        .ok()
        .and_then(|message| message.get("id").cloned())
}
// END_request_id_from_line

// START_CONTRACT_non_zero_or_default
// PURPOSE: Replace a zero runtime bound with its default
// INPUTS: { value: usize }, { default: usize }
// OUTPUTS: { usize }
// START_non_zero_or_default
fn non_zero_or_default(value: usize, default: usize) -> usize {
    if value == 0 {
        default
    } else {
        value
    }
}
// END_non_zero_or_default

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::time::{sleep, Duration};

    fn response_id(response: &Value) -> Option<i64> {
        response.get("id").and_then(Value::as_i64)
    }

    async fn run_pipeline_capture(input: &str, handler: PipelineHandler) -> Vec<Value> {
        let (mut input_writer, input_reader) = tokio::io::duplex(8192);
        let (output, mut read_output) = tokio::io::duplex(8192);
        input_writer
            .write_all(input.as_bytes())
            .await
            .expect("write input");
        drop(input_writer);
        let task = tokio::spawn(run_stdio_pipeline(
            input_reader,
            output,
            handler,
            McpPipelineConfig::default(),
        ));
        let mut captured = String::new();
        read_output
            .read_to_string(&mut captured)
            .await
            .expect("read output");
        task.await.expect("pipeline join").expect("pipeline result");
        captured
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("json response"))
            .collect()
    }

    #[tokio::test]
    async fn test_read_bounded_json_rpc_line_accepts_normal_message() {
        let input = br#"{"jsonrpc":"2.0","id":1}
"#;
        let mut reader = BufReader::new(&input[..]);

        let line = read_bounded_json_rpc_line(&mut reader, MAX_JSON_RPC_MESSAGE_BYTES)
            .await
            .expect("read line")
            .expect("line");

        assert_eq!(line, r#"{"jsonrpc":"2.0","id":1}"#);
    }

    #[tokio::test]
    async fn test_read_bounded_json_rpc_line_rejects_oversized_message() {
        let input = vec![b'a'; MAX_JSON_RPC_MESSAGE_BYTES + 1];
        let mut reader = BufReader::new(&input[..]);

        let err = read_bounded_json_rpc_line(&mut reader, MAX_JSON_RPC_MESSAGE_BYTES)
            .await
            .expect_err("oversized line should fail");

        assert!(err.to_string().contains("exceeds max size"));
    }

    #[tokio::test]
    async fn test_pipeline_processes_initialize_before_concurrent_requests() {
        let initialized = Arc::new(AtomicUsize::new(0));
        let initialized_for_handler = initialized.clone();
        let handler: PipelineHandler = Arc::new(move |line| {
            let initialized = initialized_for_handler.clone();
            Box::pin(async move {
                let message: Value = serde_json::from_str(&line).expect("message");
                let id = message.get("id").cloned();
                let method = message.get("method").and_then(Value::as_str).unwrap_or("");
                if method == "initialize" {
                    initialized.store(1, Ordering::SeqCst);
                    return Some(server_response::result(id, serde_json::json!({"ok": true})));
                }
                Some(server_response::result(
                    id,
                    serde_json::json!({"initialized": initialized.load(Ordering::SeqCst)}),
                ))
            })
        });

        let responses = run_pipeline_capture(
            concat!(
                r#"{"jsonrpc":"2.0","id":0,"method":"initialize"}"#,
                "\n",
                r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
                "\n"
            ),
            handler,
        )
        .await;

        assert_eq!(responses.len(), 2);
        assert_eq!(response_id(&responses[0]), Some(0));
        assert_eq!(responses[1]["result"]["initialized"], 1);
    }

    #[tokio::test]
    async fn test_concurrent_requests_keep_response_ids() {
        let handler: PipelineHandler = Arc::new(move |line| {
            Box::pin(async move {
                let message: Value = serde_json::from_str(&line).expect("message");
                let id = message.get("id").cloned();
                if id.as_ref().and_then(Value::as_i64) == Some(1) {
                    sleep(Duration::from_millis(40)).await;
                }
                Some(server_response::result(id, serde_json::json!({"ok": true})))
            })
        });

        let responses = run_pipeline_capture(
            concat!(
                r#"{"jsonrpc":"2.0","id":0,"method":"initialize"}"#,
                "\n",
                r#"{"jsonrpc":"2.0","id":1,"method":"tools/call"}"#,
                "\n",
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/call"}"#,
                "\n"
            ),
            handler,
        )
        .await;

        let mut ids: Vec<_> = responses.iter().filter_map(response_id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn test_handler_panic_returns_internal_error() {
        let handler: PipelineHandler = Arc::new(move |line| {
            Box::pin(async move {
                let message: Value = serde_json::from_str(&line).expect("message");
                if message.get("id").and_then(Value::as_i64) == Some(7) {
                    panic!("synthetic handler panic");
                }
                Some(server_response::result(
                    message.get("id").cloned(),
                    serde_json::json!({"ok": true}),
                ))
            })
        });

        let responses = run_pipeline_capture(
            concat!(
                r#"{"jsonrpc":"2.0","id":0,"method":"initialize"}"#,
                "\n",
                r#"{"jsonrpc":"2.0","id":7,"method":"tools/call"}"#,
                "\n"
            ),
            handler,
        )
        .await;

        let error = responses
            .iter()
            .find(|response| response_id(response) == Some(7))
            .expect("panic response");
        assert_eq!(error["error"]["code"], -32603);
    }
}

// END_public_api
