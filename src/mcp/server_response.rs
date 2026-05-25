// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-RESPONSE
// PURPOSE: MCP JSON-RPC response helpers, token-budget trimming, and verification failure suggestions
// SCOPE: JSON-RPC result/error/text envelopes, response trim options/metadata, text token-budget trimming, generic terse text previews, FailurePacket, suggest_fix
// DEPENDS: M-UTILS
// LINKS:
//   -> docs/modules/M-MCP-SERVER.xml (depends) - MCP server response envelopes
//   -> docs/modules/M-UTILS.xml (depends) - token estimation and UTF-8 safe truncation

// START_MODULE_MAP
// result — Wrap a successful MCP tool payload in JSON-RPC format
// error — Wrap an MCP error in JSON-RPC format
// text_result — Wrap MCP text content with response economy metadata
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
// LAST_CHANGE: [v2.5.0 - Added shared text_result response economy envelope]
// END_CHANGE_SUMMARY

use crate::utils::{estimate_tokens, truncate_chars};
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
}

// END_public_api
