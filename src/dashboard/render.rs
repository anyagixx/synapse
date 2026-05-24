// MODULE_CONTRACT
// MODULE_ID: M-DASHBOARD
// PURPOSE: Dashboard rendering helpers — shared HTML shell, navigation, JSON formatting, and escaping utilities
// SCOPE: page shell rendering, stable navigation links, status badges, JSON pre blocks, HTML escaping, URL component encoding
// DEPENDS: M-DASHBOARD
// LINKS:
//   -> V-M-DASHBOARD (verified_by) - dashboard rendering and route tests

// START_MODULE_MAP
// page_shell — Wraps one dashboard page in shared HTML and CSS
// primary_nav — Renders stable dashboard navigation links
// json_pre — Renders JSON as escaped pretty-print HTML
// status_badge — Renders pass/fail badge HTML
// html_escape — Escapes text for safe HTML rendering
// url_component — Encodes simple artifact ids for path links
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Extracted shared dashboard render helpers]
// END_CHANGE_SUMMARY

use serde_json::Value;

// START_public_api

// START_CONTRACT_page_shell
// PURPOSE: Wrap dashboard page body in shared HTML and styles
// INPUTS: { title: &str }, { body: &str }
// OUTPUTS: { String }
// START_page_shell
pub(super) fn page_shell(title: &str, body: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><title>{}</title><meta charset="utf-8"><style>body{{font-family:system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;max-width:1120px;margin:2em auto;padding:1em;background:#111;color:#eee}}a{{color:#8cb9ff}}h1{{color:#5c9cf5}}h2{{margin-top:0}}.grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:1rem}}.card{{background:#1a1a1a;border:1px solid #2d2d2d;border-radius:8px;padding:1em;margin:1em 0}}pre{{background:#0a0a0a;padding:1em;border-radius:4px;overflow-x:auto;white-space:pre-wrap}}table{{width:100%;border-collapse:collapse}}th,td{{border-bottom:1px solid #333;padding:.55em;text-align:left;vertical-align:top}}nav{{display:flex;flex-wrap:wrap;gap:.75rem;margin:1rem 0 1.25rem}}.pass{{color:#4caf50}}.fail{{color:#f44336}}.badge{{display:inline-block;border-radius:999px;padding:.15em .55em;background:#2b2b2b}}label{{display:block;margin:.7em 0}}input{{background:#0a0a0a;color:#eee;border:1px solid #444;border-radius:4px;padding:.45em;width:min(100%,38rem)}}button{{background:#2f6fed;color:white;border:0;border-radius:4px;padding:.55em .8em;cursor:pointer}}</style></head><body><h1>{}</h1>{}</body></html>"#,
        html_escape(title),
        html_escape(title),
        body
    )
}
// END_page_shell

// START_CONTRACT_primary_nav
// PURPOSE: Render stable dashboard navigation links
// OUTPUTS: { String }
// START_primary_nav
pub(super) fn primary_nav() -> String {
    let links = [
        ("/api/status", "Status API"),
        ("/api/graph", "Graph API"),
        ("/api/tokens", "Tokens API"),
        ("/search-explain", "Search Explain"),
        ("/runs", "Runs"),
        ("/belief-states", "Belief States"),
        ("/mental-tests", "Mental Tests"),
        ("/traceability/REQ-001", "Traceability"),
        ("/cascade/preview", "Cascade Preview"),
        ("/cascade/history", "Cascade History"),
    ]
    .iter()
    .map(|(href, label)| format!("<a href=\"{}\">{}</a>", href, html_escape(label)))
    .collect::<Vec<_>>()
    .join("");
    format!("<nav>{links}</nav>")
}
// END_primary_nav

// START_CONTRACT_json_pre
// PURPOSE: Render JSON as escaped pretty-print HTML
// INPUTS: { value: &Value }
// OUTPUTS: { String }
// START_json_pre
pub(super) fn json_pre(value: &Value) -> String {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into());
    format!("<pre>{}</pre>", html_escape(&text))
}
// END_json_pre

// START_CONTRACT_status_badge
// PURPOSE: Render boolean status as pass/fail HTML badge
// INPUTS: { passed: bool }
// OUTPUTS: { String }
// START_status_badge
pub(super) fn status_badge(passed: bool) -> String {
    if passed {
        "<span class=\"badge pass\">valid</span>".into()
    } else {
        "<span class=\"badge fail\">invalid</span>".into()
    }
}
// END_status_badge

// START_CONTRACT_html_escape
// PURPOSE: Escape text for safe dashboard HTML rendering
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_html_escape
pub(super) fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
// END_html_escape

// START_CONTRACT_url_component
// PURPOSE: Encode a simple artifact id for path links
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_url_component
pub(super) fn url_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':') {
                ch.to_string()
            } else {
                format!("%{:02X}", ch as u32)
            }
        })
        .collect()
}
// END_url_component

// END_public_api
