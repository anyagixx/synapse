// MODULE_CONTRACT
// MODULE_ID: M-MCP-LSP
// PURPOSE: LSP client bridge — sends guarded hover, go-to-definition, and references requests through persistent configurable language-server connections
// SCOPE: LspClient, LspHoverResult, LspDefinitionResult, LspReferenceResult, safe LSP positions, configurable LSP command detection, didOpen preparation, persistent LSP manager integration
// DEPENDS: M-CONFIG, M-UTILS
// LINKS:
//   -> M-MCP-LSP (depends) - persistent LSP manager
//   -> Phase-59 (implements) - persistent configurable LSP runtime

// START_MODULE_MAP
// LspHoverResult — Hover information from LSP
// LspDefinitionResult — Go-to-definition result
// LspReferenceResult — References result
// LspClient — LSP protocol client bridge
// detect_lsp_command_for — Resolve configured or built-in language-server command
// protocol_position — Converts user-facing positions to LSP positions without underflow
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v4.0.0 — Replaced per-request LSP spawning with persistent configurable manager]
// END_CHANGE_SUMMARY

use crate::config::Config;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

// START_public_api

// START_LspHoverResult
pub struct LspHoverResult {
    pub contents: String,
    pub range: Option<(usize, usize)>,
}
// END_LspHoverResult

// START_LspDefinitionResult
pub struct LspDefinitionResult {
    pub uri: String,
    pub range: (usize, usize, usize, usize), // start_line, start_col, end_line, end_col
}
// END_LspDefinitionResult

// START_LspReferenceResult
pub struct LspReferenceResult {
    pub uri: String,
    pub ranges: Vec<(usize, usize, usize, usize)>,
}
// END_LspReferenceResult

// START_LspClient
pub struct LspClient {
    config: Config,
}
// END_LspClient

struct PreparedLspRequest {
    connection: Arc<crate::mcp::lsp_manager::LspConnection>,
    uri: String,
    line: u32,
    character: u32,
}

impl LspClient {
    // START_CONTRACT_LspClient::new
    // PURPOSE: Create a new LspClient
    // OUTPUTS: { Self }
    // START_lsp_client_new
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }
    // END_lsp_client_new

    // START_CONTRACT_LspClient::hover
    // PURPOSE: Send textDocument/hover request to language server
    // INPUTS: { file: &str }, { line: u32 }, { column: u32 }
    // OUTPUTS: { anyhow::Result<LspHoverResult> }
    // START_lsp_client_hover
    pub fn hover(&self, file: &str, line: u32, column: u32) -> anyhow::Result<LspHoverResult> {
        let prepared = self.prepare_request(file, line, column)?;
        let response = prepared.connection.request(
            "textDocument/hover",
            &serde_json::json!({
                "textDocument": { "uri": prepared.uri },
                "position": { "line": prepared.line, "character": prepared.character }
            }),
        )?;
        let contents = hover_contents(&response["result"]["contents"]);
        Ok(LspHoverResult {
            contents: contents.chars().take(500).collect(),
            range: None,
        })
    }
    // END_lsp_client_hover

    // START_CONTRACT_LspClient::go_to_def
    // PURPOSE: Send textDocument/definition request
    // INPUTS: { file: &str }, { line: u32 }, { column: u32 }
    // OUTPUTS: { anyhow::Result<Vec<LspDefinitionResult>> }
    // START_lsp_client_go_to_def
    pub fn go_to_def(
        &self,
        file: &str,
        line: u32,
        column: u32,
    ) -> anyhow::Result<Vec<LspDefinitionResult>> {
        let prepared = self.prepare_request(file, line, column)?;
        let response = prepared.connection.request(
            "textDocument/definition",
            &serde_json::json!({
                "textDocument": { "uri": prepared.uri },
                "position": { "line": prepared.line, "character": prepared.character }
            }),
        )?;
        let mut results = Vec::new();
        if let Some(arr) = response["result"].as_array() {
            for entry in arr {
                push_definition_result(&mut results, entry);
            }
        } else if response["result"].is_object() {
            push_definition_result(&mut results, &response["result"]);
        }
        Ok(results)
    }
    // END_lsp_client_go_to_def

    // START_CONTRACT_LspClient::references
    // PURPOSE: Send textDocument/references request
    // INPUTS: { file: &str }, { line: u32 }, { column: u32 }
    // OUTPUTS: { anyhow::Result<Vec<LspReferenceResult>> }
    // START_lsp_client_references
    pub fn references(
        &self,
        file: &str,
        line: u32,
        column: u32,
    ) -> anyhow::Result<Vec<LspReferenceResult>> {
        let prepared = self.prepare_request(file, line, column)?;
        let response = prepared.connection.request(
            "textDocument/references",
            &serde_json::json!({
                "textDocument": { "uri": prepared.uri },
                "position": { "line": prepared.line, "character": prepared.character },
                "context": { "includeDeclaration": false }
            }),
        )?;
        let mut ranges_by_uri: BTreeMap<String, Vec<(usize, usize, usize, usize)>> =
            BTreeMap::new();
        if let Some(arr) = response["result"].as_array() {
            for entry in arr {
                let uri = entry["uri"].as_str().unwrap_or("").to_string();
                ranges_by_uri.entry(uri).or_default().push(lsp_range(entry));
            }
        }
        Ok(ranges_by_uri
            .into_iter()
            .map(|(uri, ranges)| LspReferenceResult { uri, ranges })
            .collect())
    }
    // END_lsp_client_references

    fn prepare_request(
        &self,
        file: &str,
        line: u32,
        column: u32,
    ) -> anyhow::Result<PreparedLspRequest> {
        let file_path = std::fs::canonicalize(file)?;
        let uri = format!("file://{}", file_path.display());
        let (language, ext) = Self::detect_language(file)?;
        let cmd = Self::detect_lsp_command_for(&language, &ext, &self.config)?;
        let root_path = std::env::current_dir()?;
        let root_path = root_path.canonicalize().unwrap_or(root_path);
        let root = root_path.to_string_lossy().to_string();
        let connection =
            crate::mcp::lsp_manager::global_lsp_manager().get_or_spawn(&root, &language, cmd)?;
        let text = std::fs::read_to_string(&file_path)?;
        connection.ensure_opened(&uri, &text)?;
        let (line, character) = Self::protocol_position(line, column);
        Ok(PreparedLspRequest {
            connection,
            uri,
            line,
            character,
        })
    }

    fn detect_language(file: &str) -> anyhow::Result<(String, String)> {
        let ext = Path::new(file)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        match ext {
            "rs" => Ok(("rust".into(), ext.into())),
            "py" => Ok(("python".into(), ext.into())),
            "ts" | "tsx" => Ok(("typescript".into(), ext.into())),
            "js" | "jsx" => Ok(("javascript".into(), ext.into())),
            "go" => Ok(("go".into(), ext.into())),
            _ => anyhow::bail!("No LSP server configured for .{} files", ext),
        }
    }

    fn detect_lsp_command_for(
        language: &str,
        ext: &str,
        config: &Config,
    ) -> anyhow::Result<Vec<String>> {
        if let Some(cmd) = config
            .lsp
            .servers
            .get(ext)
            .or_else(|| config.lsp.servers.get(language))
        {
            if cmd.is_empty() {
                anyhow::bail!("configured LSP command for {} is empty", ext);
            }
            return Ok(cmd.clone());
        }
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

    // START_CONTRACT_LspClient::protocol_position
    // PURPOSE: Convert one-based user line input into zero-based LSP position without underflow
    // INPUTS: { line: u32 }, { column: u32 }
    // OUTPUTS: { (u32, u32) }
    // START_lsp_client_protocol_position
    fn protocol_position(line: u32, column: u32) -> (u32, u32) {
        (line.saturating_sub(1), column)
    }
    // END_lsp_client_protocol_position
}

fn hover_contents(contents: &serde_json::Value) -> String {
    contents["value"]
        .as_str()
        .or_else(|| contents.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| contents.to_string())
}

fn push_definition_result(results: &mut Vec<LspDefinitionResult>, entry: &serde_json::Value) {
    results.push(LspDefinitionResult {
        uri: entry["uri"].as_str().unwrap_or("").to_string(),
        range: lsp_range(entry),
    });
}

fn lsp_range(entry: &serde_json::Value) -> (usize, usize, usize, usize) {
    (
        entry["range"]["start"]["line"].as_u64().unwrap_or(0) as usize + 1,
        entry["range"]["start"]["character"].as_u64().unwrap_or(0) as usize,
        entry["range"]["end"]["line"].as_u64().unwrap_or(0) as usize + 1,
        entry["range"]["end"]["character"].as_u64().unwrap_or(0) as usize,
    )
}

#[cfg(test)]
mod tests {
    use super::LspClient;
    use crate::config::Config;

    // START_CONTRACT_test_protocol_position_zero_line_is_safe
    // PURPOSE: Verify line zero cannot underflow before an LSP request is sent
    // START_test_protocol_position_zero_line_is_safe
    #[test]
    fn test_protocol_position_zero_line_is_safe() {
        assert_eq!(LspClient::protocol_position(0, 7), (0, 7));
        assert_eq!(LspClient::protocol_position(1, 7), (0, 7));
        assert_eq!(LspClient::protocol_position(9, 7), (8, 7));
    }
    // END_test_protocol_position_zero_line_is_safe

    #[test]
    fn test_detect_lsp_command_uses_config_override() {
        let mut config = Config::default();
        config.lsp.servers.insert(
            "py".into(),
            vec!["pyright-langserver".into(), "--stdio".into()],
        );
        let command = LspClient::detect_lsp_command_for("python", "py", &config).unwrap();
        assert_eq!(
            command,
            vec!["pyright-langserver".to_string(), "--stdio".to_string()]
        );
    }

    #[test]
    fn test_detect_lsp_command_falls_back_to_builtin() {
        let config = Config::default();
        let command = LspClient::detect_lsp_command_for("rust", "rs", &config).unwrap();
        assert_eq!(command, vec!["rust-analyzer".to_string()]);
    }
}
// END_public_api
