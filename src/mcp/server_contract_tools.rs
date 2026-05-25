// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-GRACE-TOOLS
// PURPOSE: MCP contract-template helper handlers for MyGRACE source scaffolding, failure diagnosis, and safe contract repair
// SCOPE: suggest_contract handler, diagnose_failure handler, repair_contract handler, language comment-prefix detection, and canonical MODULE_ID normalization
// DEPENDS: M-GRACE-CONTRACT-GENERATOR, M-GRACE-FAILURE-DIAGNOSIS, M-MCP-SERVER-RESPONSE
// LINKS:
//   -> V-M-MCP-SERVER-GRACE-TOOLS (verified_by) - MCP GRACE handler tests
//   -> UC-002 (implements) - contract templates support verified bounded changes

// START_MODULE_MAP
// handle_suggest_contract - Generates a language-aware MODULE_CONTRACT template
// handle_diagnose_failure - Parses failure reports and returns EnhancedFixResult
// handle_repair_contract - Performs dry-run or write-mode MODULE_CONTRACT repair
// comment_prefix_for_language - Maps language names to contract comment prefixes
// module_id_from_name - Builds one canonical MODULE_ID from a free-form name
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 - Added diagnose_failure and repair_contract handlers]
// END_CHANGE_SUMMARY

use super::server_response::{error, result};
use crate::grace::contract_generator::{ContractGenerator, ContractRepairRequest};
use std::path::PathBuf;

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

// START_CONTRACT_handle_diagnose_failure
// PURPOSE: Diagnose tester-agent XML or plain text failure and return EnhancedFixResult
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_diagnose_failure
pub(crate) async fn handle_diagnose_failure(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let report = match args["report"]
        .as_str()
        .or_else(|| args["failure_xml"].as_str())
        .or_else(|| args["description"].as_str())
    {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => return error(id, -32602, "Missing 'report' parameter"),
    };
    let root = match args["project_root"].as_str() {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => match std::env::current_dir() {
            Ok(root) => root,
            Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
        },
    };
    match crate::grace::failure_diagnosis::diagnose_failure_text(&root, report) {
        Ok(report) => result(
            id,
            serde_json::json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&report).unwrap_or_default()}],
                "isError": false
            }),
        ),
        Err(e) => error(id, -32603, format!("Diagnosis error: {}", e)),
    }
}
// END_handle_diagnose_failure

// START_CONTRACT_handle_repair_contract
// PURPOSE: Generate or apply a safe MODULE_CONTRACT repair for one source file
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// SIDE_EFFECTS: writes the source file only when dry_run=false
// START_handle_repair_contract
pub(crate) async fn handle_repair_contract(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let file_path = match args["file_path"].as_str().or_else(|| args["file"].as_str()) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return error(id, -32602, "Missing 'file_path' parameter"),
    };
    let root = match args["project_root"].as_str() {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => match std::env::current_dir() {
            Ok(root) => root,
            Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
        },
    };
    let request = ContractRepairRequest {
        file_path,
        module_id: args["module_id"]
            .as_str()
            .map(|value| value.trim().to_string()),
        purpose: args["purpose"]
            .as_str()
            .map(|value| value.trim().to_string()),
        scope: args["scope"].as_str().map(|value| value.trim().to_string()),
        depends: string_array_arg(args, "depends"),
        links: string_array_arg(args, "links"),
        language: args["language"]
            .as_str()
            .map(|value| value.trim().to_string()),
        dry_run: args["dry_run"].as_bool().unwrap_or(true),
    };
    match ContractGenerator::repair_contract(&root, request) {
        Ok(report) => result(
            id,
            serde_json::json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&report).unwrap_or_default()}],
                "isError": false
            }),
        ),
        Err(e) => error(id, -32603, format!("Contract repair error: {}", e)),
    }
}
// END_handle_repair_contract

// END_public_api

// START_CONTRACT_string_array_arg
// PURPOSE: Parse optional string-array MCP argument values
// INPUTS: { args: &serde_json::Value }, { key: &str }
// OUTPUTS: { Vec<String> }
// START_string_array_arg
fn string_array_arg(args: &serde_json::Value, key: &str) -> Vec<String> {
    args[key]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
// END_string_array_arg

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

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_repair_contract
    // PURPOSE: Verify repair_contract returns a dry-run preview without writing source content
    // START_test_handle_repair_contract
    async fn test_handle_repair_contract() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("src")).expect("src");
        std::fs::write(dir.path().join("src/sample.rs"), "pub fn run() {}\n").expect("source");

        let response = handle_repair_contract(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "file_path": "src/sample.rs",
                "module_id": "M-SAMPLE",
                "purpose": "Sample repair",
                "dry_run": true,
                "project_root": dir.path().to_string_lossy()
            }),
        )
        .await;

        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text response");
        assert!(text.contains("\"dry_run\": true"));
        assert!(text.contains("MODULE_ID: M-SAMPLE"));
        assert!(!std::fs::read_to_string(dir.path().join("src/sample.rs"))
            .unwrap()
            .contains("MODULE_CONTRACT"));
    }
    // END_test_handle_repair_contract

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_diagnose_failure
    // PURPOSE: Verify diagnose_failure returns an EnhancedFixResult JSON envelope
    // START_test_handle_diagnose_failure
    async fn test_handle_diagnose_failure() {
        let dir = tempfile::tempdir().expect("tempdir");
        let response = handle_diagnose_failure(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "report": "contract-exists failed: src/missing.rs missing MODULE_CONTRACT",
                "project_root": dir.path().to_string_lossy()
            }),
        )
        .await;

        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text response");
        assert!(text.contains("repair_contract"));
        assert!(text.contains("src/missing.rs"));
    }
    // END_test_handle_diagnose_failure
}
