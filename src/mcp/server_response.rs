// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-RESPONSE
// PURPOSE: MCP JSON-RPC response helpers and verification failure suggestions
// SCOPE: JSON-RPC result/error envelopes, FailurePacket, suggest_fix
// DEPENDS: N/A
// LINKS: docs/modules/M-MCP-SERVER.xml

// START_MODULE_MAP
// result — Wrap a successful MCP tool payload in JSON-RPC format
// error — Wrap an MCP error in JSON-RPC format
// FailurePacket — Human-readable verification failure packet
// suggest_fix — Maps verification check names to suggested fixes
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.2.0 — Extracted response helpers from M-MCP-SERVER]
// END_CHANGE_SUMMARY

use std::fmt::Display;

// START_public_api

// START_CONTRACT_result
// PURPOSE: Build a JSON-RPC result response with an optional request id
// INPUTS: { id: Option<serde_json::Value> }, { result: serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_result
pub(crate) fn result(
    id: Option<serde_json::Value>,
    result: serde_json::Value,
) -> serde_json::Value {
    let mut resp = serde_json::json!({
        "jsonrpc": "2.0",
        "result": result
    });
    if let Some(ref id_val) = id {
        resp["id"] = id_val.clone();
    }
    resp
}
// END_result

// START_CONTRACT_error
// PURPOSE: Build a JSON-RPC error response with an optional request id
// INPUTS: { id: Option<serde_json::Value> }, { code: i32 }, { message: impl Display }
// OUTPUTS: { serde_json::Value }
// START_error
pub(crate) fn error(
    id: Option<serde_json::Value>,
    code: i32,
    message: impl Display,
) -> serde_json::Value {
    let mut resp = serde_json::json!({
        "jsonrpc": "2.0",
        "error": {
            "code": code,
            "message": message.to_string()
        }
    });
    if let Some(ref id_val) = id {
        resp["id"] = id_val.clone();
    }
    resp
}
// END_error

// START_FailurePacket
pub(crate) struct FailurePacket {
    pub(crate) check: String,
    pub(crate) details: String,
    pub(crate) suggested: String,
}
// END_FailurePacket

// START_CONTRACT_suggest_fix
// PURPOSE: Return a short remediation suggestion for a named verification check
// INPUTS: { check: &str }
// OUTPUTS: { String }
// START_suggest_fix
pub(crate) fn suggest_fix(check: &str) -> String {
    match check {
        "contract-exists" => "Add MODULE_CONTRACT headers to source files".into(),
        "contract-valid" => "Add PURPOSE field to MODULE_CONTRACT".into(),
        "module-map" => "Add START_MODULE_MAP / END_MODULE_MAP".into(),
        "change-summary" => "Add START_CHANGE_SUMMARY / END_CHANGE_SUMMARY".into(),
        "function-contracts" => {
            "Add START_CONTRACT_name blocks with PURPOSE, INPUTS, OUTPUTS".into()
        }
        "semantic-blocks" => "Close all START_/END_ pairs".into(),
        "unique-block-names" => "Rename duplicate blocks to be unique per file".into(),
        "500-token-rule" => "Split large files into smaller units".into(),
        "no-todos" => "Resolve TODO/FIXME or convert to tracked issues".into(),
        "file-size-limit" => "Split files exceeding 500 lines into modules".into(),
        "trace-assertions" => "Add [Module][function][BLOCK_NAME] log markers".into(),
        _ => "Review the check details and fix the reported issue".into(),
    }
}
// END_suggest_fix

// END_public_api
