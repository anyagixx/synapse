// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-GRACE-TOOLS
// PURPOSE: MCP handlers for MyGRACE verification, review, status, refresh, compression, tracking, and skills
// SCOPE: profile-aware verify_project/review_code, project_status, token_savings, compress_text, refresh_project, language-aware suggest_contract, grace_* handlers
// DEPENDS: M-GRACE, M-TRACKING, M-COMPRESS, M-SKILLS-ENGINE, M-MCP-SERVER-RESPONSE
// LINKS: docs/modules/M-MCP-SERVER.xml

// START_MODULE_MAP
// handle_verify — Runs profile-aware MyGRACE verification and formats failure packets
// handle_review — Runs profile-aware MyGRACE review
// handle_status — Returns project status JSON
// handle_gain — Returns token savings stats
// handle_compress — Compresses input text
// handle_refresh — Reports or fixes MyGRACE drift
// handle_suggest_contract — Generates a language-aware MODULE_CONTRACT template
// comment_prefix_for_language — Maps language names to contract comment prefixes
// module_id_from_name — Builds one canonical MODULE_ID from a free-form name
// handle_grace_skill — Delegates grace_* MCP tools to SkillEngine
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.3.0 — Added profiles and language-aware contract suggestions]
// END_CHANGE_SUMMARY

use super::server_response::{error, result, suggest_fix, FailurePacket};
use crate::grace::GraceProfile;
use crate::skills::{SkillEngine, SkillRequest};

// START_public_api

// START_CONTRACT_handle_verify
// PURPOSE: Execute verify_project and format pass/fail checks with failure suggestions
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_verify
pub(crate) async fn handle_verify(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let root = match std::env::current_dir() {
        Ok(r) => r,
        Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
    };
    let level = args["level"].as_str().unwrap_or("all");
    let profile = match parse_grace_profile_arg(args) {
        Ok(profile) => profile,
        Err(message) => return error(id, -32602, message),
    };
    match crate::grace::GraceEngine::verify_project_with_profile(&root, profile).await {
        Ok(results) => {
            let filtered: Vec<_> = if level == "all" {
                results
            } else {
                results.into_iter().filter(|r| r.level == level).collect()
            };
            let mut text = String::new();
            let mut failures = Vec::new();
            for r in &filtered {
                let status = if r.passed { "PASS" } else { "FAIL" };
                text.push_str(&format!("\n[{}] {}\n", status, r.level));
                for c in &r.checks {
                    let mark = if c.passed { "✓" } else { "✗" };
                    text.push_str(&format!("  {} {} — {}\n", mark, c.name, c.details));
                    if !c.passed {
                        failures.push(FailurePacket {
                            check: c.name.clone(),
                            details: c.details.clone(),
                            suggested: suggest_fix(&c.name),
                        });
                    }
                }
            }
            if !failures.is_empty() {
                text.push_str("\n--- FAILURE PACKETS ---\n");
                for (i, fp) in failures.iter().enumerate() {
                    text.push_str(&format!(
                        "\n{}. {} FAILED\n   Observed: {}\n   Suggested: {}\n",
                        i + 1,
                        fp.check,
                        fp.details,
                        fp.suggested
                    ));
                }
            }
            text.push_str(&format!("\nProfile: {}\n", profile.as_str()));
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Verify error: {}", e)),
    }
}
// END_handle_verify

// START_CONTRACT_handle_review
// PURPOSE: Execute review_code and format review sections with issues
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_review
pub(crate) async fn handle_review(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let root = match std::env::current_dir() {
        Ok(r) => r,
        Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
    };
    let mode = args["mode"].as_str().unwrap_or("scoped");
    let profile = match parse_grace_profile_arg(args) {
        Ok(profile) => profile,
        Err(message) => return error(id, -32602, message),
    };
    match crate::grace::review::Reviewer::review_with_profile(&root, mode, profile) {
        Ok(report) => {
            let mut text = format!(
                "=== GRACE Review ({}, profile={}) ===\n",
                report.mode,
                profile.as_str()
            );
            for s in &report.sections {
                let status = if s.passed { "✓" } else { "✗" };
                text.push_str(&format!("{} {} — {}\n", status, s.name, s.details));
                for issue in &s.issues {
                    text.push_str(&format!("  ⚠ {}\n", issue));
                }
            }
            if report.passed {
                text.push_str("\nAll checks passed.");
            }
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Review error: {}", e)),
    }
}
// END_handle_review

// START_CONTRACT_handle_status
// PURPOSE: Execute project_status and return status JSON as MCP text
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_status
pub(crate) async fn handle_status(
    id: Option<serde_json::Value>,
    _args: &serde_json::Value,
) -> serde_json::Value {
    let root = match std::env::current_dir() {
        Ok(r) => r,
        Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
    };
    match crate::grace::status::StatusCollector::collect(&root).await {
        Ok(report) => {
            let text = serde_json::to_string_pretty(&report).unwrap_or_default();
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Status error: {}", e)),
    }
}
// END_handle_status

// START_CONTRACT_handle_gain
// PURPOSE: Execute token_savings and return tracking statistics
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_gain
pub(crate) async fn handle_gain(
    id: Option<serde_json::Value>,
    _args: &serde_json::Value,
) -> serde_json::Value {
    let config = crate::config::Config::load().unwrap_or_default();
    let tracker = crate::tracking::Tracker::new(&config);
    match tracker.get_stats().await {
        Ok(stats) => {
            let mut text = String::new();
            text.push_str(&format!("Commands tracked:  {}\n", stats.total_commands));
            text.push_str(&format!(
                "Input tokens:      {}\n",
                stats.total_input_tokens
            ));
            text.push_str(&format!(
                "Output tokens:     {}\n",
                stats.total_output_tokens
            ));
            text.push_str(&format!(
                "Tokens saved:      {}\n",
                stats.total_saved_tokens
            ));
            text.push_str(&format!(
                "Avg savings:       {:.1}%\n",
                stats.avg_savings_pct
            ));
            if stats.total_commands > 0 {
                let est = stats.total_saved_tokens as f64 * 0.000003;
                text.push_str(&format!("Est. cost saved:   ${:.4}\n", est));
            }
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Gain error: {}", e)),
    }
}
// END_handle_gain

// START_CONTRACT_handle_compress
// PURPOSE: Execute compress_text with selected compression level and return compressed text
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_compress
pub(crate) async fn handle_compress(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let text = match args["text"].as_str() {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => return error(id, -32602, "Missing 'text' parameter"),
    };
    let level = args["level"].as_str().unwrap_or("full");
    let config = crate::config::Config::load().unwrap_or_default();
    let mut cfg = config;
    cfg.compress.output_level = level.to_string();
    let compressor = crate::compress::Compressor::new(&cfg);
    let result_text = compressor.compress_output(&text);
    let saved = text.len().saturating_sub(result_text.len());
    let pct = if text.is_empty() {
        0
    } else {
        saved * 100 / text.len()
    };
    let info = format!(
        "({} → {} chars, {}% saved)\n\n{}",
        text.len(),
        result_text.len(),
        pct,
        result_text
    );
    result(
        id,
        serde_json::json!({
            "content": [{"type": "text", "text": info}],
            "isError": false
        }),
    )
}
// END_handle_compress

// START_CONTRACT_handle_refresh
// PURPOSE: Execute refresh_project in report or fix mode and return drift report JSON
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_refresh
pub(crate) async fn handle_refresh(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let root = match std::env::current_dir() {
        Ok(r) => r,
        Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
    };
    let fix = args["fix"].as_bool().unwrap_or(false);
    let result_report = if fix {
        crate::grace::refresh::Refresher::fix(&root)
    } else {
        crate::grace::refresh::Refresher::refresh(&root)
    };
    match result_report {
        Ok(report) => {
            let text = serde_json::to_string_pretty(&report).unwrap_or_default();
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Refresh error: {}", e)),
    }
}
// END_handle_refresh

// START_CONTRACT_handle_suggest_contract
// PURPOSE: Generate a MODULE_CONTRACT template for the requested module name and language
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_suggest_contract
pub(crate) async fn handle_suggest_contract(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let name = args["module_name"].as_str().unwrap_or("module");
    let purpose = args["purpose"].as_str().unwrap_or("TBD");
    let lang = args["language"].as_str().unwrap_or("rust");
    let comment = comment_prefix_for_language(lang);
    let module_id = module_id_from_name(name);
    let text = format!(
        "{c} MODULE_CONTRACT\n{c} MODULE_ID: {id}\n{c} PURPOSE: {p}\n{c} SCOPE: {n}\n{c} DEPENDS:\n{c} LINKS:\n\n{c} START_MODULE_MAP\n{c} END_MODULE_MAP\n\n{c} START_CHANGE_SUMMARY\n{c} LAST_CHANGE: [v1.0.0 — Initial implementation]\n{c} END_CHANGE_SUMMARY\n\n{c} START_public_api\n{c} END_public_api",
        c = comment, id = module_id, p = purpose, n = name
    );
    result(
        id,
        serde_json::json!({
            "content": [{"type": "text", "text": text}],
            "isError": false
        }),
    )
}
// END_handle_suggest_contract

// START_CONTRACT_parse_grace_profile_arg
// PURPOSE: Parse an optional MCP GRACE profile argument
// INPUTS: { args: &serde_json::Value — tool arguments }
// OUTPUTS: { Result<GraceProfile, String> }
// START_parse_grace_profile_arg
fn parse_grace_profile_arg(args: &serde_json::Value) -> Result<GraceProfile, String> {
    let profile = args["profile"].as_str().unwrap_or("strict");
    GraceProfile::from_name(profile).ok_or_else(|| {
        format!(
            "Unsupported GRACE profile '{}'. Use one of: lite, balanced, strict.",
            profile
        )
    })
}
// END_parse_grace_profile_arg

// START_CONTRACT_comment_prefix_for_language
// PURPOSE: Select the correct line comment prefix for generated contract templates
// INPUTS: { language: &str — language name or extension }
// OUTPUTS: { &'static str }
// START_comment_prefix_for_language
fn comment_prefix_for_language(language: &str) -> &'static str {
    match language.trim().to_ascii_lowercase().as_str() {
        "py" | "python" | "sh" | "bash" | "zsh" | "rb" | "ruby" | "yaml" | "yml" => "#",
        "sql" | "postgres" | "postgresql" | "mysql" | "sqlite" => "--",
        _ => "//",
    }
}
// END_comment_prefix_for_language

// START_CONTRACT_module_id_from_name
// PURPOSE: Build one canonical MODULE_ID from free-form module input without comma aggregation
// INPUTS: { name: &str — requested module name }
// OUTPUTS: { String — M- prefixed id with uppercase letters, digits, and hyphens }
// START_module_id_from_name
fn module_id_from_name(name: &str) -> String {
    let source = name
        .split(',')
        .next()
        .unwrap_or(name)
        .trim()
        .trim_start_matches("M-")
        .trim_start_matches("m-");
    let mut body = String::new();
    let mut last_dash = false;
    for ch in source.chars() {
        if ch.is_ascii_alphanumeric() {
            body.push(ch.to_ascii_uppercase());
            last_dash = false;
        } else if !last_dash && !body.is_empty() {
            body.push('-');
            last_dash = true;
        }
    }
    while body.ends_with('-') {
        body.pop();
    }
    if body.is_empty() {
        "M-MODULE".into()
    } else {
        format!("M-{}", body)
    }
}
// END_module_id_from_name

// START_CONTRACT_handle_grace_skill
// PURPOSE: Execute a registered grace_* skill through SkillEngine and return title/body text
// INPUTS: { skill_engine: &SkillEngine }, { request: SkillRequest }, { id: Option<serde_json::Value> }
// OUTPUTS: { serde_json::Value }
// START_handle_grace_skill
pub(crate) async fn handle_grace_skill(
    skill_engine: &SkillEngine,
    request: SkillRequest,
    id: Option<serde_json::Value>,
) -> serde_json::Value {
    match skill_engine.execute(request).await {
        Ok(result_value) => result(
            id,
            serde_json::json!({
                "content": [{"type": "text", "text": format!("{}\n\n{}", result_value.title, result_value.body)}],
                "isError": false
            }),
        ),
        Err(e) => error(id, -32603, format!("Skill error: {}", e)),
    }
}
// END_handle_grace_skill

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // START_CONTRACT_test_comment_prefix_for_python_and_sql
    // PURPOSE: Verify contract suggestions use executable comment syntax for Python and SQL
    // START_test_comment_prefix_for_python_and_sql
    fn test_comment_prefix_for_python_and_sql() {
        assert_eq!(comment_prefix_for_language("python"), "#");
        assert_eq!(comment_prefix_for_language("sql"), "--");
        assert_eq!(comment_prefix_for_language("rust"), "//");
    }
    // END_test_comment_prefix_for_python_and_sql

    #[test]
    // START_CONTRACT_test_module_id_from_name_removes_commas
    // PURPOSE: Verify generated MODULE_ID values do not aggregate comma-separated module names
    // START_test_module_id_from_name_removes_commas
    fn test_module_id_from_name_removes_commas() {
        assert_eq!(
            module_id_from_name("M-STORAGE, M-BOT, M-SCHEDULER"),
            "M-STORAGE"
        );
        assert_eq!(module_id_from_name("date parser"), "M-DATE-PARSER");
    }
    // END_test_module_id_from_name_removes_commas
}

// END_public_api
