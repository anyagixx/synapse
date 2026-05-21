// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-GRACE-TOOLS
// PURPOSE: MCP contract-template helper handlers for MyGRACE source scaffolding
// SCOPE: suggest_contract handler, language comment-prefix detection, and canonical MODULE_ID normalization
// DEPENDS: M-MCP-SERVER-RESPONSE
// LINKS:
//   -> V-M-MCP-SERVER-GRACE-TOOLS (verified_by) - MCP GRACE handler tests
//   -> UC-002 (implements) - contract templates support verified bounded changes

// START_MODULE_MAP
// handle_suggest_contract - Generates a language-aware MODULE_CONTRACT template
// comment_prefix_for_language - Maps language names to contract comment prefixes
// module_id_from_name - Builds one canonical MODULE_ID from a free-form name
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Extracted contract suggestion helpers from server_grace_tools]
// END_CHANGE_SUMMARY

use super::server_response::result;

// START_public_api

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
        "{c} MODULE_CONTRACT\n{c} MODULE_ID: {id}\n{c} PURPOSE: {p}\n{c} SCOPE: {n}\n{c} DEPENDS:\n{c} LINKS:\n\n{c} START_MODULE_MAP\n{c} END_MODULE_MAP\n\n{c} START_CHANGE_SUMMARY\n{c} LAST_CHANGE: [v1.0.0 - Initial implementation]\n{c} END_CHANGE_SUMMARY\n\n{c} START_public_api\n{c} END_public_api",
        c = comment,
        id = module_id,
        p = purpose,
        n = name
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

// END_public_api

// START_CONTRACT_comment_prefix_for_language
// PURPOSE: Select the correct line comment prefix for generated contract templates
// INPUTS: { language: &str - language name or extension }
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
// INPUTS: { name: &str - requested module name }
// OUTPUTS: { String - M- prefixed id with uppercase letters, digits, and hyphens }
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
