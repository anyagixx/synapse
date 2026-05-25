// MODULE_CONTRACT
// MODULE_ID: M-UTILS
// PURPOSE: Token estimation, ANSI stripping, savings formatting, and test synchronization — utility functions
// SCOPE: ANSI escape stripping, Unicode-safe truncation, naive token estimation (chars/4), savings percentage formatting, test-only cwd locking
// DEPENDS: N/A
// LINKS: N/A

// START_MODULE_MAP
// strip_ansi — Remove ANSI escape sequences from a string
// truncate_chars — Truncate a string by Unicode scalar values without splitting UTF-8
// estimate_tokens — Estimate token count as text length / 4
// format_savings — Format token savings as percentage string
// test_cwd_lock — Return a shared test-only lock for current-directory mutation
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.4.0 — Added ANSI stripping regression coverage for shared filter utility]
// END_CHANGE_SUMMARY

use std::sync::OnceLock;

static ANSI_RE: OnceLock<Result<regex::Regex, String>> = OnceLock::new();
const ANSI_PATTERN: &str = "\x1b\\[[0-9;]*m";

// START_public_api

// START_CONTRACT_strip_ansi
// PURPOSE: Remove ANSI escape sequences from a string
// INPUTS: { s: &str — input string possibly containing ANSI codes }
// OUTPUTS: { String — cleaned string without ANSI }
// START_strip_ansi
pub fn strip_ansi(s: &str) -> String {
    match ANSI_RE.get_or_init(|| regex::Regex::new(ANSI_PATTERN).map_err(|e| e.to_string())) {
        Ok(re) => re.replace_all(s, "").to_string(),
        Err(_) => s.to_string(),
    }
}
// END_strip_ansi

// START_CONTRACT_truncate_chars
// PURPOSE: Truncate a string by character count without splitting UTF-8 code points
// INPUTS: { s: &str — input string }, { max_chars: usize — maximum Unicode scalar values before ellipsis }
// OUTPUTS: { String — original or truncated string with ellipsis }
// START_truncate_chars
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}
// END_truncate_chars

// START_CONTRACT_estimate_tokens
// PURPOSE: Naive token estimation — text length divided by 4
// INPUTS: { text: &str — text to estimate }
// OUTPUTS: { u32 — estimated token count }
// START_estimate_tokens
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() / 4) as u32
}
// END_estimate_tokens

// START_CONTRACT_format_savings
// PURPOSE: Format token savings as a rounded percentage string
// INPUTS: { input: u32 — input tokens }, { output: u32 — output tokens }
// OUTPUTS: { String — formatted percentage like "42%" }
// START_format_savings
pub fn format_savings(input: u32, output: u32) -> String {
    if input == 0 {
        return "0%".into();
    }
    let pct = (input.saturating_sub(output) as f64 / input as f64 * 100.0).round() as u32;
    format!("{}%", pct.min(100))
}
// END_format_savings

// START_CONTRACT_test_cwd_lock
// PURPOSE: Return the process-wide async lock used by tests that mutate current directory
// OUTPUTS: { &'static tokio::sync::Mutex<()> }
// SIDE_EFFECTS: Initializes a test-only static mutex once
// START_test_cwd_lock
#[cfg(test)]
pub fn test_cwd_lock() -> &'static tokio::sync::Mutex<()> {
    static TEST_CWD_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    TEST_CWD_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}
// END_test_cwd_lock

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_chars_preserves_utf8_boundaries() {
        let text = format!("{}😀x", "я".repeat(999));
        let truncated = truncate_chars(&text, 1000);

        assert!(truncated.ends_with("..."));
        assert!(truncated.is_char_boundary(truncated.len()));
        assert!(truncated.contains('😀'));
    }

    #[test]
    fn test_strip_ansi_removes_common_sgr_sequences() {
        let input = "\x1b[1mbold\x1b[0m \x1b[31mred\x1b[0m \x1b[38;5;196mbright\x1b[0m";

        assert_eq!(strip_ansi(input), "bold red bright");
    }
}
// END_public_api
