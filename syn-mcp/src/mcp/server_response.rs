// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-RESPONSE
// PURPOSE: MCP JSON-RPC response helpers, token-budget trimming, cache metadata, and verification failure suggestions
// SCOPE: JSON-RPC result/error/text envelopes, cache metadata/ETags, response trim options/metadata, text token-budget trimming, generic terse text previews, FailurePacket, suggest_fix
// DEPENDS: M-UTILS
// LINKS:
//   -> docs/modules/M-MCP-SERVER.xml (depends) - MCP server response envelopes
//   -> docs/modules/M-UTILS.xml (depends) - token estimation and UTF-8 safe truncation

// START_MODULE_MAP
// result — Wrap a successful MCP tool payload in JSON-RPC format
// error — Wrap an MCP error in JSON-RPC format
// text_result — Wrap MCP text content with response economy metadata
// CacheMetadata — Observable MCP cache hint metadata
// CacheMetadata::to_json — Serializes cache metadata
// cache_ttl_for_tool — Returns short-lived cache TTLs for cache-hinted tools
// is_tool_cacheable — Checks whether a tool can satisfy _if_none_match
// cache_key_for_tool_call — Builds deterministic cache keys from normalized tool arguments
// compute_etag — Builds stable weak ETags from normalized request/response JSON
// result_with_cache — Wrap a result and attach _meta.cache metadata
// with_cache_metadata — Attach _meta.cache metadata to an existing JSON-RPC result
// not_modified_result — Build a minimal _not_modified JSON-RPC result
// ResponseStyle — Handler output verbosity style
// ResponseStyle::parse — Parses style arguments
// ResponseTrimOptions — max_tokens and style options shared by MCP handlers
// ResponseTrimOptions::from_args — Parses response economy options from tool arguments
// ResponseTrimMetadata — Observable token economy metadata for one response
// ResponseTrimMetadata::to_json — Serializes trim metadata
// TrimmedText — Text plus response trim metadata
// trim_text_to_budget — Applies progressive token-budget trimming to final text content
// compact_terse_text — Builds a generic compact line preview for terse text responses
// FailurePacket — Human-readable verification failure packet
// suggest_fix — Maps verification check names to suggested fixes
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.6.1 - Kept cache result helper clippy-clean for release gate]
// END_CHANGE_SUMMARY

use syn_core::utils::{estimate_tokens, truncate_chars};
use serde_json::{Map, Value};
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

// START_CONTRACT_text_result
// PURPOSE: Build a JSON-RPC text content response with max_tokens/style response economy metadata
// INPUTS: { id: Option<serde_json::Value> }, { text: impl Into<String> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_text_result
pub(crate) fn text_result(
    id: Option<serde_json::Value>,
    text: impl Into<String>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let options = ResponseTrimOptions::from_args(args);
    let text = match options.style {
        ResponseStyle::Full => text.into(),
        ResponseStyle::Terse => compact_terse_text(&text.into()),
    };
    let trimmed = trim_text_to_budget(&text, options.max_tokens);
    result(
        id,
        serde_json::json!({
            "content": [{"type": "text", "text": trimmed.text}],
            "isError": false,
            "style": options.style.label(),
            "was_trimmed": trimmed.metadata.was_trimmed,
            "tokens": trimmed.metadata.to_json()
        }),
    )
}
// END_text_result

// START_CacheMetadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheMetadata {
    pub(crate) etag: String,
    pub(crate) ttl_secs: u64,
}
// END_CacheMetadata

impl CacheMetadata {
    // START_CONTRACT_CacheMetadata::new
    // PURPOSE: Build cache metadata from a stable ETag and short-lived TTL
    // INPUTS: { etag: String }, { ttl_secs: u64 }
    // OUTPUTS: { CacheMetadata }
    // START_cache_metadata_new
    pub(crate) fn new(etag: String, ttl_secs: u64) -> Self {
        Self { etag, ttl_secs }
    }
    // END_cache_metadata_new

    // START_CONTRACT_CacheMetadata::to_json
    // PURPOSE: Serialize cache metadata into the MCP _meta.cache response shape
    // OUTPUTS: { serde_json::Value }
    // START_cache_metadata_to_json
    pub(crate) fn to_json(&self) -> Value {
        serde_json::json!({
            "etag": self.etag,
            "ttl_secs": self.ttl_secs
        })
    }
    // END_cache_metadata_to_json
}

// START_CONTRACT_cache_ttl_for_tool
// PURPOSE: Return bounded TTLs for tools whose repeated responses can safely use MCP cache hints
// INPUTS: { tool_name: &str }
// OUTPUTS: { u64 }
// START_cache_ttl_for_tool
pub(crate) fn cache_ttl_for_tool(tool_name: &str) -> u64 {
    match tool_name {
        "project_status" => 15,
        "graphrag_query" => 120,
        "traceability_report" => 30,
        "view_signatures" => 120,
        "lsp_hover" | "lsp_references" => 60,
        _ => 0,
    }
}
// END_cache_ttl_for_tool

// START_CONTRACT_is_tool_cacheable
// PURPOSE: Check whether a tool may satisfy _if_none_match from the in-memory ETag cache
// INPUTS: { tool_name: &str }
// OUTPUTS: { bool }
// START_is_tool_cacheable
pub(crate) fn is_tool_cacheable(tool_name: &str) -> bool {
    cache_ttl_for_tool(tool_name) > 0
}
// END_is_tool_cacheable

// START_CONTRACT_cache_key_for_tool_call
// PURPOSE: Build a deterministic cache key from tool name and normalized arguments
// INPUTS: { tool_name: &str }, { args: &serde_json::Value }
// OUTPUTS: { String }
// START_cache_key_for_tool_call
pub(crate) fn cache_key_for_tool_call(tool_name: &str, args: &Value) -> String {
    format!(
        "{tool_name}:{}",
        canonical_json(&normalized_cache_args(args))
    )
}
// END_cache_key_for_tool_call

// START_CONTRACT_compute_etag
// PURPOSE: Build a stable weak ETag from normalized tool arguments and result payload
// INPUTS: { tool_name: &str }, { args: &serde_json::Value }, { result_payload: &serde_json::Value }
// OUTPUTS: { String }
// START_compute_etag
pub(crate) fn compute_etag(tool_name: &str, args: &Value, result_payload: &Value) -> String {
    let mut hash = FNV_OFFSET_BASIS;
    update_hash(&mut hash, tool_name.as_bytes());
    update_hash(&mut hash, b"\0");
    update_hash(
        &mut hash,
        canonical_json(&normalized_cache_args(args)).as_bytes(),
    );
    update_hash(&mut hash, b"\0");
    update_hash(
        &mut hash,
        canonical_json(&normalized_cache_result(result_payload)).as_bytes(),
    );
    format!("W/\"syn-{hash:016x}\"")
}
// END_compute_etag

// START_CONTRACT_result_with_cache
// PURPOSE: Build a JSON-RPC result response and attach _meta.cache metadata
// INPUTS: { id: Option<serde_json::Value> }, { result_payload: serde_json::Value }, { tool_name: &str }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_result_with_cache
#[allow(dead_code)]
pub(crate) fn result_with_cache(
    id: Option<Value>,
    result_payload: Value,
    tool_name: &str,
    args: &Value,
) -> Value {
    let mut response = result(id, result_payload);
    with_cache_metadata(&mut response, tool_name, args);
    response
}
// END_result_with_cache

// START_CONTRACT_with_cache_metadata
// PURPOSE: Attach _meta.cache metadata to an existing JSON-RPC result response
// INPUTS: { response: &mut serde_json::Value }, { tool_name: &str }, { args: &serde_json::Value }
// OUTPUTS: { Option<CacheMetadata> }
// START_with_cache_metadata
pub(crate) fn with_cache_metadata(
    response: &mut Value,
    tool_name: &str,
    args: &Value,
) -> Option<CacheMetadata> {
    let result_payload = response.get("result")?.clone();
    let metadata = CacheMetadata::new(
        compute_etag(tool_name, args, &result_payload),
        cache_ttl_for_tool(tool_name),
    );
    attach_cache_metadata(response, &metadata)?;
    Some(metadata)
}
// END_with_cache_metadata

// START_CONTRACT_not_modified_result
// PURPOSE: Build a minimal JSON-RPC result for a matching _if_none_match request
// INPUTS: { id: Option<serde_json::Value> }, { metadata: &CacheMetadata }
// OUTPUTS: { serde_json::Value }
// START_not_modified_result
pub(crate) fn not_modified_result(id: Option<Value>, metadata: &CacheMetadata) -> Value {
    result(
        id,
        serde_json::json!({
            "_not_modified": true,
            "_meta": {
                "cache": metadata.to_json()
            }
        }),
    )
}
// END_not_modified_result

// START_CONTRACT_attach_cache_metadata
// PURPOSE: Insert _meta.cache into a JSON-RPC result object while preserving existing _meta fields
// INPUTS: { response: &mut serde_json::Value }, { metadata: &CacheMetadata }
// OUTPUTS: { Option<()> }
// START_attach_cache_metadata
fn attach_cache_metadata(response: &mut Value, metadata: &CacheMetadata) -> Option<()> {
    let result_object = response.get_mut("result")?.as_object_mut()?;
    let meta = result_object
        .entry("_meta")
        .or_insert_with(|| Value::Object(Map::new()));
    if !meta.is_object() {
        *meta = Value::Object(Map::new());
    }
    meta.as_object_mut()?
        .insert("cache".to_string(), metadata.to_json());
    Some(())
}
// END_attach_cache_metadata

// START_CONTRACT_normalized_cache_args
// PURPOSE: Remove transport-only cache hints before computing cache keys or ETags
// INPUTS: { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_normalized_cache_args
fn normalized_cache_args(args: &Value) -> Value {
    let mut normalized = args.clone();
    if let Some(object) = normalized.as_object_mut() {
        object.remove("_if_none_match");
    }
    normalized
}
// END_normalized_cache_args

// START_CONTRACT_normalized_cache_result
// PURPOSE: Remove previous cache metadata before hashing a result payload
// INPUTS: { result_payload: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_normalized_cache_result
fn normalized_cache_result(result_payload: &Value) -> Value {
    let mut normalized = result_payload.clone();
    if let Some(object) = normalized.as_object_mut() {
        if let Some(meta) = object.get_mut("_meta").and_then(Value::as_object_mut) {
            meta.remove("cache");
        }
        if object
            .get("_meta")
            .and_then(Value::as_object)
            .is_some_and(Map::is_empty)
        {
            object.remove("_meta");
        }
    }
    normalized
}
// END_normalized_cache_result

// START_CONTRACT_canonical_json
// PURPOSE: Serialize JSON deterministically by sorting object keys recursively
// INPUTS: { value: &serde_json::Value }
// OUTPUTS: { String }
// START_canonical_json
fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();
            let body = keys
                .into_iter()
                .map(|key| format!("{}:{}", json_string(key), canonical_json(&object[key])))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{body}}}")
        }
        Value::Array(items) => {
            let body = items
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{body}]")
        }
        Value::String(text) => json_string(text),
        _ => value.to_string(),
    }
}
// END_canonical_json

// START_CONTRACT_json_string
// PURPOSE: Serialize a string using JSON escaping for deterministic canonical output
// INPUTS: { text: &str }
// OUTPUTS: { String }
// START_json_string
fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}
// END_json_string

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001b3;

// START_CONTRACT_update_hash
// PURPOSE: Apply FNV-1a bytes to the running stable cache hash
// INPUTS: { hash: &mut u64 }, { bytes: &[u8] }
// OUTPUTS: { () }
// START_update_hash
fn update_hash(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}
// END_update_hash

// START_ResponseStyle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResponseStyle {
    Full,
    Terse,
}
// END_ResponseStyle

impl ResponseStyle {
    // START_CONTRACT_ResponseStyle::parse
    // PURPOSE: Parse a handler style argument into a stable response verbosity style
    // INPUTS: { value: &str }
    // OUTPUTS: { ResponseStyle }
    // START_response_style_parse
    pub(crate) fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "terse" => Self::Terse,
            _ => Self::Full,
        }
    }
    // END_response_style_parse

    // START_CONTRACT_ResponseStyle::label
    // PURPOSE: Return a response metadata label for the style
    // OUTPUTS: { &'static str }
    // START_response_style_label
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Terse => "terse",
        }
    }
    // END_response_style_label

    // START_CONTRACT_ResponseStyle::is_terse
    // PURPOSE: Check whether compact handler formatting is requested
    // OUTPUTS: { bool }
    // START_response_style_is_terse
    pub(crate) fn is_terse(self) -> bool {
        self == Self::Terse
    }
    // END_response_style_is_terse
}

// START_ResponseTrimOptions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResponseTrimOptions {
    pub(crate) max_tokens: u32,
    pub(crate) style: ResponseStyle,
}
// END_ResponseTrimOptions

impl ResponseTrimOptions {
    // START_CONTRACT_ResponseTrimOptions::from_args
    // PURPOSE: Parse max_tokens and style from MCP tool arguments with backward-compatible defaults
    // INPUTS: { args: &serde_json::Value }
    // OUTPUTS: { ResponseTrimOptions }
    // START_response_trim_options_from_args
    pub(crate) fn from_args(args: &serde_json::Value) -> Self {
        let max_tokens = args
            .get("max_tokens")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(0);
        let style = args
            .get("style")
            .and_then(serde_json::Value::as_str)
            .map(ResponseStyle::parse)
            .unwrap_or(ResponseStyle::Full);
        Self { max_tokens, style }
    }
    // END_response_trim_options_from_args
}

impl Default for ResponseTrimOptions {
    // START_CONTRACT_ResponseTrimOptions::default
    // PURPOSE: Return backward-compatible unlimited full response options
    // OUTPUTS: { ResponseTrimOptions }
    // START_response_trim_options_default
    fn default() -> Self {
        Self {
            max_tokens: 0,
            style: ResponseStyle::Full,
        }
    }
    // END_response_trim_options_default
}

// START_ResponseTrimMetadata
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResponseTrimMetadata {
    pub(crate) original_tokens: u32,
    pub(crate) shown_tokens: u32,
    pub(crate) saved_tokens: u32,
    pub(crate) was_trimmed: bool,
}
// END_ResponseTrimMetadata

impl ResponseTrimMetadata {
    // START_CONTRACT_ResponseTrimMetadata::new
    // PURPOSE: Build response trim metadata from original/shown token counts and trim status
    // INPUTS: { original_tokens: u32 }, { shown_tokens: u32 }, { was_trimmed: bool }
    // OUTPUTS: { ResponseTrimMetadata }
    // START_response_trim_metadata_new
    pub(crate) fn new(original_tokens: u32, shown_tokens: u32, was_trimmed: bool) -> Self {
        Self {
            original_tokens,
            shown_tokens,
            saved_tokens: original_tokens.saturating_sub(shown_tokens),
            was_trimmed,
        }
    }
    // END_response_trim_metadata_new

    // START_CONTRACT_ResponseTrimMetadata::to_json
    // PURPOSE: Serialize trim metadata into the MCP response economy shape
    // OUTPUTS: { serde_json::Value }
    // START_response_trim_metadata_to_json
    pub(crate) fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "original": self.original_tokens,
            "shown": self.shown_tokens,
            "saved": self.saved_tokens,
            "was_trimmed": self.was_trimmed
        })
    }
    // END_response_trim_metadata_to_json
}

// START_TrimmedText
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrimmedText {
    pub(crate) text: String,
    pub(crate) metadata: ResponseTrimMetadata,
}
// END_TrimmedText

// START_CONTRACT_trim_text_to_budget
// PURPOSE: Apply progressive token-budget trimming to final MCP text content
// INPUTS: { text: &str }, { max_tokens: u32 }
// OUTPUTS: { TrimmedText }
// START_trim_text_to_budget
pub(crate) fn trim_text_to_budget(text: &str, max_tokens: u32) -> TrimmedText {
    let original_tokens = estimate_tokens(text);
    if max_tokens == 0 || original_tokens <= max_tokens {
        return TrimmedText {
            text: text.to_string(),
            metadata: ResponseTrimMetadata::new(original_tokens, original_tokens, false),
        };
    }

    let trimmed = if original_tokens <= max_tokens.saturating_mul(2) {
        trim_near_budget_response(text, original_tokens, max_tokens)
    } else {
        summarize_large_response(text, original_tokens, max_tokens)
    };
    let shown_tokens = estimate_tokens(&trimmed);
    TrimmedText {
        text: trimmed,
        metadata: ResponseTrimMetadata::new(original_tokens, shown_tokens, true),
    }
}
// END_trim_text_to_budget

// START_CONTRACT_trim_near_budget_response
// PURPOSE: Keep a bounded prefix plus trim notice for responses near the token budget
// INPUTS: { text: &str }, { original_tokens: u32 }, { max_tokens: u32 }
// OUTPUTS: { String }
// START_trim_near_budget_response
fn trim_near_budget_response(text: &str, original_tokens: u32, max_tokens: u32) -> String {
    let notice = format!(
        "\n\n[Response trimmed: {} tokens saved. Use max_tokens to adjust.]",
        original_tokens.saturating_sub(max_tokens)
    );
    fit_prefix_with_notice(text, &notice, max_tokens)
}
// END_trim_near_budget_response

// START_CONTRACT_summarize_large_response
// PURPOSE: Replace very large responses with a compact summary and first lines
// INPUTS: { text: &str }, { original_tokens: u32 }, { max_tokens: u32 }
// OUTPUTS: { String }
// START_summarize_large_response
fn summarize_large_response(text: &str, original_tokens: u32, max_tokens: u32) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let header = format!(
        "[Large response: {} lines, ~{} tokens. Use max_tokens to expand or request a narrower scope.]",
        lines.len(),
        original_tokens
    );
    if lines.is_empty() {
        return header;
    }

    let preview = lines.iter().take(3).copied().collect::<Vec<_>>().join("\n");
    let notice = format!("\n\n{header}");
    fit_prefix_with_notice(&preview, &notice, max_tokens)
}
// END_summarize_large_response

// START_CONTRACT_fit_prefix_with_notice
// PURPOSE: Fit a UTF-8 safe prefix plus notice into a best-effort token budget
// INPUTS: { text: &str }, { notice: &str }, { max_tokens: u32 }
// OUTPUTS: { String }
// START_fit_prefix_with_notice
fn fit_prefix_with_notice(text: &str, notice: &str, max_tokens: u32) -> String {
    if max_tokens == 0 {
        return text.to_string();
    }
    let notice_tokens = estimate_tokens(notice);
    if notice_tokens >= max_tokens {
        return notice.trim().to_string();
    }

    let max_chars = text.chars().count();
    let mut low = 0_usize;
    let mut high = max_chars.min(max_tokens as usize * 6);
    let mut best = notice.trim().to_string();
    while low <= high {
        let mid = low + (high - low) / 2;
        let candidate = format!("{}{}", truncate_chars(text, mid), notice);
        let candidate_tokens = estimate_tokens(&candidate);
        if candidate_tokens <= max_tokens {
            best = candidate;
            low = mid.saturating_add(1);
        } else if mid == 0 {
            break;
        } else {
            high = mid - 1;
        }
    }
    best
}
// END_fit_prefix_with_notice

// START_CONTRACT_compact_terse_text
// PURPOSE: Build a generic compact line preview for terse MCP text responses
// INPUTS: { text: &str }
// OUTPUTS: { String }
// START_compact_terse_text
fn compact_terse_text(text: &str) -> String {
    const TERSE_LINES: usize = 8;
    const TERSE_LINE_CHARS: usize = 120;
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.len() <= TERSE_LINES {
        return lines
            .iter()
            .map(|line| truncate_chars(line, TERSE_LINE_CHARS))
            .collect::<Vec<_>>()
            .join("\n");
    }
    let mut compact = lines
        .iter()
        .take(TERSE_LINES)
        .map(|line| truncate_chars(line, TERSE_LINE_CHARS))
        .collect::<Vec<_>>();
    compact.push(format!(
        "[terse: {} more lines hidden]",
        lines.len() - TERSE_LINES
    ));
    compact.join("\n")
}
// END_compact_terse_text

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
        "cascade-no-drift" => "Run cascade_execute for each pending cascade marker".into(),
        _ => "Review the check details and fix the reported issue".into(),
    }
}
// END_suggest_fix

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_response_style_parse_defaults_to_full
    // PURPOSE: Verify response style parsing is backward compatible
    // START_test_response_style_parse_defaults_to_full
    #[test]
    fn test_response_style_parse_defaults_to_full() {
        assert_eq!(ResponseStyle::parse(""), ResponseStyle::Full);
        assert_eq!(ResponseStyle::parse("FULL"), ResponseStyle::Full);
        assert_eq!(ResponseStyle::parse("terse"), ResponseStyle::Terse);
        assert_eq!(ResponseStyle::Full.label(), "full");
        assert!(ResponseStyle::Terse.is_terse());
    }
    // END_test_response_style_parse_defaults_to_full

    // START_CONTRACT_test_response_trim_options_from_args
    // PURPOSE: Verify response trim options parse max_tokens and style from MCP arguments
    // START_test_response_trim_options_from_args
    #[test]
    fn test_response_trim_options_from_args() {
        let options = ResponseTrimOptions::from_args(&serde_json::json!({
            "max_tokens": 42,
            "style": "terse"
        }));

        assert_eq!(options.max_tokens, 42);
        assert_eq!(options.style, ResponseStyle::Terse);
        assert_eq!(ResponseTrimOptions::default().max_tokens, 0);
    }
    // END_test_response_trim_options_from_args

    // START_CONTRACT_test_trim_text_to_budget_keeps_unlimited_full_text
    // PURPOSE: Verify max_tokens=0 preserves backward-compatible unlimited responses
    // START_test_trim_text_to_budget_keeps_unlimited_full_text
    #[test]
    fn test_trim_text_to_budget_keeps_unlimited_full_text() {
        let text = "short response";

        let trimmed = trim_text_to_budget(text, 0);

        assert_eq!(trimmed.text, text);
        assert!(!trimmed.metadata.was_trimmed);
        assert_eq!(trimmed.metadata.saved_tokens, 0);
    }
    // END_test_trim_text_to_budget_keeps_unlimited_full_text

    // START_CONTRACT_test_trim_text_to_budget_adds_notice
    // PURPOSE: Verify trimmed responses include a token-savings notice and metadata
    // START_test_trim_text_to_budget_adds_notice
    #[test]
    fn test_trim_text_to_budget_adds_notice() {
        let text = (0..80)
            .map(|i| format!("line {i}: fn sample_{i}() {{ println!(\"value\"); }}"))
            .collect::<Vec<_>>()
            .join("\n");

        let trimmed = trim_text_to_budget(&text, 40);

        assert!(trimmed.metadata.was_trimmed);
        assert!(trimmed.metadata.original_tokens > trimmed.metadata.shown_tokens);
        assert!(trimmed.metadata.saved_tokens > 0);
        assert!(
            trimmed.text.contains("Response trimmed") || trimmed.text.contains("Large response")
        );
        assert_eq!(trimmed.metadata.to_json()["was_trimmed"], true);
    }
    // END_test_trim_text_to_budget_adds_notice

    // START_CONTRACT_test_text_result_applies_terse_and_tokens
    // PURPOSE: Verify text_result adds response economy metadata and terse formatting
    // START_test_text_result_applies_terse_and_tokens
    #[test]
    fn test_text_result_applies_terse_and_tokens() {
        let text = (0..20)
            .map(|i| format!("line {i}: verbose response content with additional detail"))
            .collect::<Vec<_>>()
            .join("\n");

        let response = text_result(
            Some(serde_json::json!(1)),
            text,
            &serde_json::json!({
                "style": "terse",
                "max_tokens": 0
            }),
        );

        assert_eq!(response["result"]["style"], "terse");
        assert_eq!(response["result"]["was_trimmed"], false);
        assert!(response["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("[terse:")));
    }
    // END_test_text_result_applies_terse_and_tokens

    // START_CONTRACT_test_cache_metadata_helpers_attach_cache_shape
    // PURPOSE: Verify result_with_cache attaches ETag and TTL metadata while normalizing transport hints
    // START_test_cache_metadata_helpers_attach_cache_shape
    #[test]
    fn test_cache_metadata_helpers_attach_cache_shape() {
        let args = serde_json::json!({
            "b": 2,
            "a": 1,
            "_if_none_match": "W/\"stale\""
        });

        let response = result_with_cache(
            Some(serde_json::json!(7)),
            serde_json::json!({"content": [{"type": "text", "text": "ok"}]}),
            "project_status",
            &args,
        );

        assert_eq!(response["id"], 7);
        assert_eq!(
            response["result"]["_meta"]["cache"]["ttl_secs"],
            cache_ttl_for_tool("project_status")
        );
        assert!(response["result"]["_meta"]["cache"]["etag"]
            .as_str()
            .is_some_and(|etag| etag.starts_with("W/\"syn-")));
        assert!(is_tool_cacheable("project_status"));
        assert!(!is_tool_cacheable("semantic_search"));
    }
    // END_test_cache_metadata_helpers_attach_cache_shape

    // START_CONTRACT_test_cache_keys_and_etags_are_stable
    // PURPOSE: Verify cache keys and ETags are stable across object key order and ignored cache hints
    // START_test_cache_keys_and_etags_are_stable
    #[test]
    fn test_cache_keys_and_etags_are_stable() {
        let args_a = serde_json::json!({"b": 2, "a": 1});
        let args_b = serde_json::json!({"_if_none_match": "old", "a": 1, "b": 2});
        let result_a = serde_json::json!({"z": false, "a": [1, 2]});
        let result_b = serde_json::json!({
            "a": [1, 2],
            "z": false,
            "_meta": {"cache": {"etag": "old", "ttl_secs": 99}}
        });

        assert_eq!(
            cache_key_for_tool_call("graphrag_query", &args_a),
            cache_key_for_tool_call("graphrag_query", &args_b)
        );
        assert_eq!(
            compute_etag("graphrag_query", &args_a, &result_a),
            compute_etag("graphrag_query", &args_b, &result_b)
        );
    }
    // END_test_cache_keys_and_etags_are_stable

    // START_CONTRACT_test_volatile_tools_report_zero_ttl
    // PURPOSE: Verify volatile tools can expose cache metadata without enabling not-modified cache hits
    // START_test_volatile_tools_report_zero_ttl
    #[test]
    fn test_volatile_tools_report_zero_ttl() {
        let response = result_with_cache(
            None,
            serde_json::json!({"content": [{"type": "text", "text": "results"}]}),
            "semantic_search",
            &serde_json::json!({"query": "needle"}),
        );

        assert_eq!(response["result"]["_meta"]["cache"]["ttl_secs"], 0);
        assert!(!is_tool_cacheable("semantic_search"));
    }
    // END_test_volatile_tools_report_zero_ttl

    // START_CONTRACT_test_not_modified_result_is_minimal
    // PURPOSE: Verify matching cache validators can return a compact not-modified MCP payload
    // START_test_not_modified_result_is_minimal
    #[test]
    fn test_not_modified_result_is_minimal() {
        let metadata = CacheMetadata::new("W/\"syn-abc\"".to_string(), 15);

        let response = not_modified_result(Some(serde_json::json!("req-1")), &metadata);

        assert_eq!(response["id"], "req-1");
        assert_eq!(response["result"]["_not_modified"], true);
        assert_eq!(
            response["result"]["_meta"]["cache"]["etag"],
            "W/\"syn-abc\""
        );
        assert_eq!(response["result"]["_meta"]["cache"]["ttl_secs"], 15);
    }
    // END_test_not_modified_result_is_minimal
}

// END_public_api
