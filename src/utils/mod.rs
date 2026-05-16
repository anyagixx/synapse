// MODULE_CONTRACT
// MODULE_ID: M-UTILS
// PURPOSE: Token estimation, ANSI stripping, savings formatting — utility functions
// SCOPE: ANSI escape stripping, naive token estimation (chars/4), savings percentage formatting
// DEPENDS: N/A
// LINKS: N/A

// START_MODULE_MAP
// strip_ansi — Remove ANSI escape sequences from a string
// estimate_tokens — Estimate token count as text length / 4
// format_savings — Format token savings as percentage string
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY

// START_public_api

// START_CONTRACT_strip_ansi
// PURPOSE: Remove ANSI escape sequences from a string
// INPUTS: { s: &str — input string possibly containing ANSI codes }
// OUTPUTS: { String — cleaned string without ANSI }
// START_strip_ansi
pub fn strip_ansi(s: &str) -> String {
    regex::Regex::new("\x1b\\[[0-9;]*m")
        .unwrap()
        .replace_all(s, "")
        .to_string()
}
// END_strip_ansi

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
    let pct = ((input - output) as f64 / input as f64 * 100.0).round() as u32;
    format!("{}%", pct.min(100))
}
// END_format_savings
// END_public_api
