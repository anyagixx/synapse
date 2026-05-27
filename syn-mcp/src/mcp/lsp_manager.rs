// MODULE_CONTRACT
// MODULE_ID: M-MCP-LSP
// PURPOSE: Persistent LSP connection manager — pools language-server stdio processes by root, language, and command
// SCOPE: LspManager, LspConnection, LSP message framing, initialize/initialized lifecycle, didOpen tracking, didClose cleanup, request serialization, shutdown cleanup
// DEPENDS: M-CONFIG
// LINKS:
//   -> M-MCP-LSP (depends) - runtime used by LspClient
//   -> Phase-59 (implements) - persistent configurable LSP runtime

// START_MODULE_MAP
// LspManager — Pool of persistent LSP connections
// LspConnection — One long-lived language-server process with serialized stdio and opened-document state
// global_lsp_manager — Process-wide LSP manager singleton
// write_lsp_message — Write one Content-Length framed LSP JSON message
// read_lsp_message — Read one Content-Length framed LSP JSON message
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 - Added didClose cleanup for opened LSP documents]
// END_CHANGE_SUMMARY

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};

// START_public_api

// START_LspManager
pub struct LspManager {
    connections: Mutex<HashMap<String, Arc<LspConnection>>>,
}
// END_LspManager

impl LspManager {
    // START_CONTRACT_LspManager::new
    // PURPOSE: Create an empty LSP connection pool
    // OUTPUTS: { Self }
    // START_lsp_manager_new
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
        }
    }
    // END_lsp_manager_new

    // START_CONTRACT_LspManager::get_or_spawn
    // PURPOSE: Return a pooled LSP connection or spawn and initialize a new one
    // INPUTS: { root: &str }, { language: &str }, { cmd: Vec<String> }
    // OUTPUTS: { anyhow::Result<Arc<LspConnection>> }
    // SIDE_EFFECTS: may spawn a language-server process
    // START_lsp_manager_get_or_spawn
    pub fn get_or_spawn(
        &self,
        root: &str,
        language: &str,
        cmd: Vec<String>,
    ) -> anyhow::Result<Arc<LspConnection>> {
        let key = format!("{}|{}|{}", root, language, cmd.join("\u{1f}"));
        let mut connections = self
            .connections
            .lock()
            .map_err(|_| anyhow::anyhow!("LSP manager lock poisoned"))?;
        if let Some(connection) = connections.get(&key) {
            return Ok(Arc::clone(connection));
        }
        let connection = Arc::new(LspConnection::spawn(root, language, &cmd)?);
        connections.insert(key, Arc::clone(&connection));
        Ok(connection)
    }
    // END_lsp_manager_get_or_spawn
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new()
    }
}

// START_LspConnection
pub struct LspConnection {
    child: Mutex<Child>,
    io: Mutex<LspIo>,
    language: String,
}
// END_LspConnection

struct LspIo {
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    next_id: u64,
    opened_files: HashSet<String>,
}

impl LspConnection {
    // START_CONTRACT_LspConnection::spawn
    // PURPOSE: Spawn a language server, send initialize, wait for initialize response, and send initialized notification
    // INPUTS: { root: &str }, { language: &str }, { cmd: &[String] }
    // OUTPUTS: { anyhow::Result<Self> }
    // SIDE_EFFECTS: starts a child process
    // START_lsp_connection_spawn
    pub fn spawn(root: &str, language: &str, cmd: &[String]) -> anyhow::Result<Self> {
        let (program, args) = cmd
            .split_first()
            .ok_or_else(|| anyhow::anyhow!("empty LSP command for {}", language))?;
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                anyhow::anyhow!(
                    "failed to start LSP server '{}' for {}: {}. Configure [lsp.servers] if the binary is not on PATH",
                    program,
                    language,
                    error
                )
            })?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("LSP stdin pipe unavailable for {}", program))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("LSP stdout pipe unavailable for {}", program))?;
        let mut reader = BufReader::new(stdout);

        let init = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "capabilities": {},
                "rootUri": format!("file://{}", root)
            }
        });
        write_lsp_message(&mut stdin, &init)?;
        read_matching_response(&mut reader, 0)?;
        let initialized = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });
        write_lsp_message(&mut stdin, &initialized)?;

        Ok(Self {
            child: Mutex::new(child),
            io: Mutex::new(LspIo {
                stdin,
                reader,
                next_id: 1,
                opened_files: HashSet::new(),
            }),
            language: language.to_string(),
        })
    }
    // END_lsp_connection_spawn

    // START_CONTRACT_LspConnection::ensure_opened
    // PURPOSE: Send textDocument/didOpen once for a file on this connection
    // INPUTS: { uri: &str }, { text: &str }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes a notification to the LSP process stdin
    // START_lsp_connection_ensure_opened
    pub fn ensure_opened(&self, uri: &str, text: &str) -> anyhow::Result<()> {
        let mut io = self
            .io
            .lock()
            .map_err(|_| anyhow::anyhow!("LSP connection lock poisoned"))?;
        let LspIo {
            stdin,
            opened_files,
            ..
        } = &mut *io;
        write_did_open_if_needed(stdin, opened_files, &self.language, uri, text)?;
        Ok(())
    }
    // END_lsp_connection_ensure_opened

    // START_CONTRACT_LspConnection::close_opened
    // PURPOSE: Send textDocument/didClose for a previously opened file on this connection
    // INPUTS: { uri: &str }
    // OUTPUTS: { anyhow::Result<bool> }
    // SIDE_EFFECTS: may write a notification to the LSP process stdin
    // LINKS:
    //   -> NFR-002 (traces_to) - LSP lifecycle cleanup should not leave stale server state
    // START_lsp_connection_close_opened
    pub fn close_opened(&self, uri: &str) -> anyhow::Result<bool> {
        let mut io = self
            .io
            .lock()
            .map_err(|_| anyhow::anyhow!("LSP connection lock poisoned"))?;
        let LspIo {
            stdin,
            opened_files,
            ..
        } = &mut *io;
        write_did_close_if_opened(stdin, opened_files, uri)
    }
    // END_lsp_connection_close_opened

    // START_CONTRACT_LspConnection::request
    // PURPOSE: Send one serialized LSP request and read the matching response
    // INPUTS: { method: &str }, { params: &serde_json::Value }
    // OUTPUTS: { anyhow::Result<serde_json::Value> }
    // SIDE_EFFECTS: writes to and reads from the LSP process stdio
    // START_lsp_connection_request
    pub fn request(
        &self,
        method: &str,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let mut io = self
            .io
            .lock()
            .map_err(|_| anyhow::anyhow!("LSP connection lock poisoned"))?;
        let id = io.next_id;
        io.next_id = io.next_id.saturating_add(1);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        write_lsp_message(&mut io.stdin, &request)?;
        read_matching_response(&mut io.reader, id)
    }
    // END_lsp_connection_request
}

impl Drop for LspConnection {
    fn drop(&mut self) {
        if let Ok(mut io) = self.io.lock() {
            let LspIo {
                stdin,
                opened_files,
                ..
            } = &mut *io;
            let _ = write_did_close_all(stdin, opened_files);
            let shutdown = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 999_999_u64,
                "method": "shutdown",
                "params": null
            });
            let _ = write_lsp_message(&mut io.stdin, &shutdown);
            let exit = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "exit",
                "params": null
            });
            let _ = write_lsp_message(&mut io.stdin, &exit);
        }
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

// START_CONTRACT_global_lsp_manager
// PURPOSE: Return the process-wide LSP manager singleton
// OUTPUTS: { &'static LspManager }
// START_global_lsp_manager
pub fn global_lsp_manager() -> &'static LspManager {
    static LSP_MANAGER: OnceLock<LspManager> = OnceLock::new();
    LSP_MANAGER.get_or_init(LspManager::new)
}
// END_global_lsp_manager

// END_public_api

fn read_matching_response<R: BufRead>(
    reader: &mut R,
    id: u64,
) -> anyhow::Result<serde_json::Value> {
    loop {
        let response = read_lsp_message(reader)?;
        if response.get("id").and_then(|value| value.as_u64()) == Some(id) {
            return Ok(response);
        }
    }
}

// START_CONTRACT_write_did_open_if_needed
// PURPOSE: Write didOpen once per URI using the provided document text
// INPUTS: { writer: &mut W }, { opened_files: &mut HashSet<String> }, { language: &str }, { uri: &str }, { text: &str }
// OUTPUTS: { anyhow::Result<bool> }
// SIDE_EFFECTS: writes framed LSP notification when the URI was not already opened
// LINKS:
//   -> NFR-002 (traces_to) - first LSP request must use current document content
// START_write_did_open_if_needed
fn write_did_open_if_needed<W: Write>(
    writer: &mut W,
    opened_files: &mut HashSet<String>,
    language: &str,
    uri: &str,
    text: &str,
) -> anyhow::Result<bool> {
    if opened_files.contains(uri) {
        return Ok(false);
    }
    write_lsp_message(writer, &did_open_notification(language, uri, text))?;
    opened_files.insert(uri.to_string());
    Ok(true)
}
// END_write_did_open_if_needed

// START_CONTRACT_write_did_close_if_opened
// PURPOSE: Write didClose for one URI only when it is tracked as opened
// INPUTS: { writer: &mut W }, { opened_files: &mut HashSet<String> }, { uri: &str }
// OUTPUTS: { anyhow::Result<bool> }
// SIDE_EFFECTS: writes framed LSP notification when the URI was opened
// LINKS:
//   -> NFR-002 (traces_to) - explicit LSP close avoids stale opened document state
// START_write_did_close_if_opened
fn write_did_close_if_opened<W: Write>(
    writer: &mut W,
    opened_files: &mut HashSet<String>,
    uri: &str,
) -> anyhow::Result<bool> {
    if !opened_files.remove(uri) {
        return Ok(false);
    }
    write_lsp_message(writer, &did_close_notification(uri))?;
    Ok(true)
}
// END_write_did_close_if_opened

// START_CONTRACT_write_did_close_all
// PURPOSE: Write didClose notifications for all tracked opened documents
// INPUTS: { writer: &mut W }, { opened_files: &mut HashSet<String> }
// OUTPUTS: { anyhow::Result<usize> }
// SIDE_EFFECTS: writes framed LSP notifications and clears opened file state
// LINKS:
//   -> NFR-002 (traces_to) - connection drop closes all opened LSP documents
// START_write_did_close_all
fn write_did_close_all<W: Write>(
    writer: &mut W,
    opened_files: &mut HashSet<String>,
) -> anyhow::Result<usize> {
    let mut uris = opened_files.iter().cloned().collect::<Vec<_>>();
    uris.sort();
    for uri in &uris {
        write_lsp_message(writer, &did_close_notification(uri))?;
    }
    opened_files.clear();
    Ok(uris.len())
}
// END_write_did_close_all

fn did_open_notification(language: &str, uri: &str, text: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": language,
                "version": 1,
                "text": text
            }
        }
    })
}

fn did_close_notification(uri: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didClose",
        "params": {
            "textDocument": { "uri": uri }
        }
    })
}

fn write_lsp_message<W: Write>(writer: &mut W, value: &serde_json::Value) -> anyhow::Result<()> {
    let body = value.to_string();
    write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    writer.flush()?;
    Ok(())
}

fn read_lsp_message<R: BufRead>(reader: &mut R) -> anyhow::Result<serde_json::Value> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            anyhow::bail!("LSP server closed stdout");
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse::<usize>()?);
        }
    }
    let len =
        content_length.ok_or_else(|| anyhow::anyhow!("LSP response missing Content-Length"))?;
    let mut body = vec![0_u8; len];
    reader.read_exact(&mut body)?;
    let response = serde_json::from_slice(&body)?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_lsp_message_round_trip() {
        let value = serde_json::json!({"jsonrpc":"2.0","id":7,"result":{"ok":true}});
        let mut bytes = Vec::new();
        write_lsp_message(&mut bytes, &value).expect("write message");
        let mut cursor = Cursor::new(bytes);
        let parsed = read_lsp_message(&mut cursor).expect("read message");
        assert_eq!(parsed["id"].as_u64(), Some(7));
        assert_eq!(parsed["result"]["ok"].as_bool(), Some(true));
    }

    #[test]
    fn test_did_open_is_written_once_with_provided_content() {
        let mut opened = HashSet::new();
        let mut bytes = Vec::new();
        assert!(write_did_open_if_needed(
            &mut bytes,
            &mut opened,
            "rust",
            "file:///tmp/a.rs",
            "memory content"
        )
        .expect("first didOpen"));
        assert!(!write_did_open_if_needed(
            &mut bytes,
            &mut opened,
            "rust",
            "file:///tmp/a.rs",
            "disk content"
        )
        .expect("second didOpen"));

        let mut cursor = Cursor::new(bytes);
        let parsed = read_lsp_message(&mut cursor).expect("read didOpen");
        assert_eq!(parsed["method"], "textDocument/didOpen");
        assert_eq!(parsed["params"]["textDocument"]["text"], "memory content");
    }

    #[test]
    fn test_did_close_all_writes_and_clears_opened_documents() {
        let mut opened = HashSet::from([
            "file:///tmp/b.rs".to_string(),
            "file:///tmp/a.rs".to_string(),
        ]);
        let mut bytes = Vec::new();
        let count = write_did_close_all(&mut bytes, &mut opened).expect("didClose all");

        assert_eq!(count, 2);
        assert!(opened.is_empty());
        let mut cursor = Cursor::new(bytes);
        let first = read_lsp_message(&mut cursor).expect("first close");
        let second = read_lsp_message(&mut cursor).expect("second close");
        assert_eq!(first["method"], "textDocument/didClose");
        assert_eq!(second["method"], "textDocument/didClose");
        assert_eq!(first["params"]["textDocument"]["uri"], "file:///tmp/a.rs");
        assert_eq!(second["params"]["textDocument"]["uri"], "file:///tmp/b.rs");
    }
}
