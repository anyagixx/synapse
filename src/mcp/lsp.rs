use std::process::{Command, Stdio};

#[derive(serde::Serialize)]
pub struct LspHoverResult {
    pub contents: String,
    pub range: Option<(usize, usize)>,
}

#[derive(serde::Serialize)]
pub struct LspDefinitionResult {
    pub uri: String,
    pub range: (usize, usize, usize, usize), // start_line, start_col, end_line, end_col
}

#[derive(serde::Serialize)]
pub struct LspReferenceResult {
    pub uri: String,
    pub ranges: Vec<(usize, usize, usize, usize)>,
}

pub struct LspClient;

impl LspClient {
    pub fn new() -> Self {
        Self
    }

    pub fn hover(&self, file: &str, line: u32, column: u32) -> anyhow::Result<LspHoverResult> {
        let cmd = Self::detect_lsp_command(file)?;
        let uri = format!("file://{}", std::fs::canonicalize(file)?.display());
        let request = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line - 1, "character": column }
            }
        });
        let response = Self::send_lsp_request(&cmd, &request)?;
        let contents = response["result"]["contents"]["value"]
            .as_str()
            .unwrap_or(&response["result"]["contents"].to_string())
            .to_string();
        Ok(LspHoverResult {
            contents: contents.chars().take(500).collect(),
            range: None,
        })
    }

    pub fn go_to_def(
        &self,
        file: &str,
        line: u32,
        column: u32,
    ) -> anyhow::Result<Vec<LspDefinitionResult>> {
        let cmd = Self::detect_lsp_command(file)?;
        let uri = format!("file://{}", std::fs::canonicalize(file)?.display());
        let request = serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line - 1, "character": column }
            }
        });
        let response = Self::send_lsp_request(&cmd, &request)?;
        let mut results = Vec::new();
        if let Some(arr) = response["result"].as_array() {
            for entry in arr {
                results.push(LspDefinitionResult {
                    uri: entry["uri"].as_str().unwrap_or("").to_string(),
                    range: (
                        entry["range"]["start"]["line"].as_u64().unwrap_or(0) as usize + 1,
                        entry["range"]["start"]["character"].as_u64().unwrap_or(0) as usize,
                        entry["range"]["end"]["line"].as_u64().unwrap_or(0) as usize + 1,
                        entry["range"]["end"]["character"].as_u64().unwrap_or(0) as usize,
                    ),
                });
            }
        }
        Ok(results)
    }

    pub fn references(
        &self,
        file: &str,
        line: u32,
        column: u32,
    ) -> anyhow::Result<Vec<LspReferenceResult>> {
        let cmd = Self::detect_lsp_command(file)?;
        let uri = format!("file://{}", std::fs::canonicalize(file)?.display());
        let request = serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line - 1, "character": column },
                "context": { "includeDeclaration": false }
            }
        });
        let response = Self::send_lsp_request(&cmd, &request)?;
        let mut ranges = Vec::new();
        if let Some(arr) = response["result"].as_array() {
            for entry in arr {
                ranges.push((
                    entry["range"]["start"]["line"].as_u64().unwrap_or(0) as usize + 1,
                    entry["range"]["start"]["character"].as_u64().unwrap_or(0) as usize,
                    entry["range"]["end"]["line"].as_u64().unwrap_or(0) as usize + 1,
                    entry["range"]["end"]["character"].as_u64().unwrap_or(0) as usize,
                ));
            }
        }
        Ok(vec![LspReferenceResult {
            uri: uri.clone(),
            ranges,
        }])
    }

    fn detect_lsp_command(file: &str) -> anyhow::Result<Vec<String>> {
        let ext = std::path::Path::new(file)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        match ext {
            "rs" => Ok(vec!["rust-analyzer".into()]),
            "py" => Ok(vec!["pylsp".into()]),
            "ts" | "tsx" | "js" | "jsx" => {
                Ok(vec!["typescript-language-server".into(), "--stdio".into()])
            }
            "go" => Ok(vec!["gopls".into()]),
            _ => anyhow::bail!("No LSP server configured for .{} files", ext),
        }
    }

    fn send_lsp_request(
        cmd: &[String],
        request: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let program = &cmd[0];
        let args: Vec<&str> = cmd[1..].iter().map(|s| s.as_str()).collect();

        // Initialize LSP
        let init = serde_json::json!({
            "jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "capabilities": {},
                "rootUri": format!("file://{}", std::env::current_dir()?.display())
            }
        });

        let mut child = Command::new(program)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        use std::io::Write;
        let stdin = child.stdin.as_mut().unwrap();
        let init_str = format!("Content-Length: {}\r\n\r\n{}", init.to_string().len(), init);
        stdin.write_all(init_str.as_bytes())?;

        let req_str = format!(
            "Content-Length: {}\r\n\r\n{}",
            request.to_string().len(),
            request
        );
        stdin.write_all(req_str.as_bytes())?;
        stdin.flush()?;

        let output = child.wait_with_output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse LSP header + body
        if let Some(body_start) = stdout.find("\r\n\r\n") {
            let body = &stdout[body_start + 4..];
            // Skip Content-Length header of the response
            if let Some(inner_start) = body.find("\r\n\r\n") {
                let json = &body[inner_start + 4..];
                if let Ok(val) = serde_json::from_str(json) {
                    return Ok(val);
                }
            }
            if let Ok(val) = serde_json::from_str(body) {
                return Ok(val);
            }
        }
        anyhow::bail!(
            "Failed to parse LSP response: {}",
            &stdout[..200.min(stdout.len())]
        )
    }
}
