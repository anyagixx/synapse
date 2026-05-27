// MODULE_CONTRACT
// MODULE_ID: M-GRACE-CONTRACT-GENERATOR
// PURPOSE: Language-aware GRACE contract generator and safe MODULE_CONTRACT repair engine
// SCOPE: Contract repair request/result models, module contract generation, function contract generation, dry-run repair, write-mode repair, and missing-contract failure extraction
// DEPENDS: M-INDEXER-WALKER
// LINKS:
//   -> UC-002 (implements) - repairs contracts required for verified bounded changes
//   -> NFR-002 (traces_to) - repair actions must be explicit, dry-runnable, and non-destructive
//   <- V-M-GRACE-CONTRACT-GENERATOR (verified_by) - contract generator tests

// START_MODULE_MAP
// ContractRepairRequest - Input model for a safe contract repair action
// ContractRepairResult - Result model for dry-run or applied repair
// ContractGenerator - Generates language-aware contracts and applies safe repairs
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added safe contract generation and repair]
// END_CHANGE_SUMMARY

use crate::indexer::walker::detect_language;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// START_public_api

// START_ContractRepairRequest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractRepairRequest {
    pub file_path: String,
    #[serde(default)]
    pub module_id: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub depends: Vec<String>,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}
// END_ContractRepairRequest

// START_ContractRepairResult
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractRepairResult {
    pub file_path: String,
    pub module_id: String,
    pub dry_run: bool,
    pub applied: bool,
    pub changed: bool,
    pub evidence_ref: Option<String>,
    pub message: String,
    pub preview: String,
}
// END_ContractRepairResult

// START_ContractGenerator
pub struct ContractGenerator;
// END_ContractGenerator

impl ContractGenerator {
    // START_CONTRACT_ContractGenerator::generate_module_contract
    // PURPOSE: Generate a language-aware MODULE_CONTRACT, MODULE_MAP, and CHANGE_SUMMARY header
    // INPUTS: { request: &ContractRepairRequest }
    // OUTPUTS: { String }
    // START_contract_generator_generate_module_contract
    pub fn generate_module_contract(request: &ContractRepairRequest) -> String {
        let language = request
            .language
            .clone()
            .unwrap_or_else(|| language_from_path(&request.file_path));
        let comment = comment_prefix_for_language(&language);
        let module_id = request
            .module_id
            .clone()
            .unwrap_or_else(|| module_id_from_path(&request.file_path));
        let purpose = request
            .purpose
            .clone()
            .unwrap_or_else(|| "Generated contract for repaired source file".into());
        let scope = request
            .scope
            .clone()
            .unwrap_or_else(|| "Source file operations covered by generated contract".into());
        let depends = if request.depends.is_empty() {
            "N/A".into()
        } else {
            request.depends.join(", ")
        };
        let links = if request.links.is_empty() {
            vec![
                "  -> UC-002 (implements) - generated contract supports verified bounded changes"
                    .to_string(),
                "  -> NFR-002 (traces_to) - generated contract repairs explicit verification failure"
                    .to_string(),
            ]
        } else {
            request.links.clone()
        };
        let mut text = format!(
            "{c} MODULE_CONTRACT\n{c} MODULE_ID: {module_id}\n{c} PURPOSE: {purpose}\n{c} SCOPE: {scope}\n{c} DEPENDS: {depends}\n{c} LINKS:\n",
            c = comment,
            module_id = module_id,
            purpose = purpose,
            scope = scope,
            depends = depends
        );
        for link in links {
            text.push_str(&format!("{comment} {link}\n"));
        }
        text.push_str(&format!(
            "\n{c} START_MODULE_MAP\n{c} No public exports discovered during automatic repair\n{c} END_MODULE_MAP\n\n{c} START_CHANGE_SUMMARY\n{c} LAST_CHANGE: [v1.0.0 - Generated MODULE_CONTRACT repair]\n{c} END_CHANGE_SUMMARY\n",
            c = comment
        ));
        text
    }
    // END_contract_generator_generate_module_contract

    // START_CONTRACT_ContractGenerator::generate_function_contract
    // PURPOSE: Generate a language-aware function contract block
    // INPUTS: { language: &str }, { name: &str }, { purpose: &str }, { links: &[String] }
    // OUTPUTS: { String }
    // START_contract_generator_generate_function_contract
    pub fn generate_function_contract(
        language: &str,
        name: &str,
        purpose: &str,
        links: &[String],
    ) -> String {
        let comment = comment_prefix_for_language(language);
        let mut text = format!(
            "{c} START_CONTRACT_{name}\n{c} PURPOSE: {purpose}\n{c} INPUTS: N/A\n{c} OUTPUTS: N/A\n",
            c = comment,
            name = sanitize_contract_name(name),
            purpose = purpose
        );
        if !links.is_empty() {
            text.push_str(&format!("{comment} LINKS:\n"));
            for link in links {
                text.push_str(&format!("{comment} {link}\n"));
            }
        }
        text
    }
    // END_contract_generator_generate_function_contract

    // START_CONTRACT_ContractGenerator::repair_contract
    // PURPOSE: Safely prepend a generated MODULE_CONTRACT when a file is missing one
    // INPUTS: { root: &Path }, { request: ContractRepairRequest }
    // OUTPUTS: { anyhow::Result<ContractRepairResult> }
    // START_contract_generator_repair_contract
    pub fn repair_contract(
        root: &Path,
        request: ContractRepairRequest,
    ) -> anyhow::Result<ContractRepairResult> {
        let rel_path = request.file_path.trim().trim_start_matches("./");
        if rel_path.is_empty() {
            anyhow::bail!("file_path is required for contract repair");
        }
        let path = root.join(rel_path);
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        let module_id = request
            .module_id
            .clone()
            .unwrap_or_else(|| module_id_from_path(rel_path));
        if existing.contains("MODULE_CONTRACT") {
            return Ok(ContractRepairResult {
                file_path: rel_path.into(),
                module_id,
                dry_run: request.dry_run,
                applied: false,
                changed: false,
                evidence_ref: Some(format!("contract://{}", rel_path)),
                message: "file already contains MODULE_CONTRACT; no repair applied".into(),
                preview: existing,
            });
        }
        let header = Self::generate_module_contract(&request);
        let repaired = if existing.trim().is_empty() {
            format!("{}\n", header.trim_end())
        } else {
            format!("{}\n\n{}", header.trim_end(), existing)
        };
        if !request.dry_run {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, &repaired)?;
        }
        Ok(ContractRepairResult {
            file_path: rel_path.into(),
            module_id,
            dry_run: request.dry_run,
            applied: !request.dry_run,
            changed: true,
            evidence_ref: Some(if request.dry_run {
                format!("dry-run://{}", rel_path)
            } else {
                format!("contract://{}", rel_path)
            }),
            message: if request.dry_run {
                "dry-run contract repair generated".into()
            } else {
                "MODULE_CONTRACT repair applied".into()
            },
            preview: repaired,
        })
    }
    // END_contract_generator_repair_contract

    // START_CONTRACT_ContractGenerator::repair_actions_from_failure_text
    // PURPOSE: Build dry-run or write-mode repair actions from missing-contract failure text
    // INPUTS: { root: &Path }, { failure_text: &str }, { dry_run: bool }
    // OUTPUTS: { Vec<ContractRepairResult> }
    // START_contract_generator_repair_actions_from_failure_text
    pub fn repair_actions_from_failure_text(
        root: &Path,
        failure_text: &str,
        dry_run: bool,
    ) -> Vec<ContractRepairResult> {
        let lower = failure_text.to_ascii_lowercase();
        if !(lower.contains("module_contract")
            || lower.contains("contract-exists")
            || lower.contains("missing contract"))
        {
            return Vec::new();
        }
        extract_file_paths(failure_text)
            .into_iter()
            .take(8)
            .filter_map(|file_path| {
                let request = ContractRepairRequest {
                    file_path,
                    module_id: None,
                    purpose: Some("Generated contract for missing MODULE_CONTRACT failure".into()),
                    scope: Some("Automatically repaired source contract header".into()),
                    depends: Vec::new(),
                    links: Vec::new(),
                    language: None,
                    dry_run,
                };
                Self::repair_contract(root, request).ok()
            })
            .collect()
    }
    // END_contract_generator_repair_actions_from_failure_text
}

// START_CONTRACT_default_dry_run
// PURPOSE: Default contract repair requests to dry-run mode
// OUTPUTS: { bool }
// START_default_dry_run
fn default_dry_run() -> bool {
    true
}
// END_default_dry_run

// START_CONTRACT_language_from_path
// PURPOSE: Infer a Synapse language id from a file path
// INPUTS: { file_path: &str }
// OUTPUTS: { String }
// START_language_from_path
fn language_from_path(file_path: &str) -> String {
    detect_language(Path::new(file_path)).unwrap_or_else(|| "rust".into())
}
// END_language_from_path

// START_CONTRACT_comment_prefix_for_language
// PURPOSE: Select line comment prefix for a supported language
// INPUTS: { language: &str }
// OUTPUTS: { &'static str }
// START_comment_prefix_for_language
pub fn comment_prefix_for_language(language: &str) -> &'static str {
    match language.trim().to_ascii_lowercase().as_str() {
        "py" | "python" | "sh" | "bash" | "zsh" | "rb" | "ruby" | "yaml" | "yml" => "#",
        "sql" | "postgres" | "postgresql" | "mysql" | "sqlite" => "--",
        _ => "//",
    }
}
// END_comment_prefix_for_language

// START_CONTRACT_module_id_from_path
// PURPOSE: Build a stable module id from a source path
// INPUTS: { file_path: &str }
// OUTPUTS: { String }
// START_module_id_from_path
pub fn module_id_from_path(file_path: &str) -> String {
    let path = PathBuf::from(file_path);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("module");
    let mut body = String::new();
    let mut last_dash = false;
    for ch in stem.chars() {
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
// END_module_id_from_path

// START_CONTRACT_sanitize_contract_name
// PURPOSE: Normalize function contract names for generated START_CONTRACT anchors
// INPUTS: { name: &str }
// OUTPUTS: { String }
// START_sanitize_contract_name
fn sanitize_contract_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
// END_sanitize_contract_name

// START_CONTRACT_extract_file_paths
// PURPOSE: Extract source-like file paths from verifier failure text
// INPUTS: { text: &str }
// OUTPUTS: { Vec<String> }
// START_extract_file_paths
fn extract_file_paths(text: &str) -> Vec<String> {
    let Ok(re) = regex::Regex::new(
        r#"([A-Za-z0-9_./-]+\.(rs|py|ts|tsx|js|jsx|go|sql|sh|bash|zsh|rb|java|php|cpp|hpp|c|h))"#,
    ) else {
        return Vec::new();
    };
    let mut paths: Vec<String> = re
        .captures_iter(text)
        .map(|cap| {
            cap[1]
                .trim_matches(|ch: char| ch == '\'' || ch == '"' || ch == ',')
                .into()
        })
        .collect();
    paths.sort();
    paths.dedup();
    paths
}
// END_extract_file_paths

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // START_CONTRACT_test_generate_module_contract_uses_language_prefix
    // PURPOSE: Verify generated contracts use language-specific comment prefixes
    // START_test_generate_module_contract_uses_language_prefix
    fn test_generate_module_contract_uses_language_prefix() {
        let request = ContractRepairRequest {
            file_path: "src/tool.py".into(),
            module_id: Some("M-TOOL".into()),
            purpose: Some("Tool module".into()),
            scope: None,
            depends: Vec::new(),
            links: Vec::new(),
            language: None,
            dry_run: true,
        };

        let contract = ContractGenerator::generate_module_contract(&request);

        assert!(contract.starts_with("# MODULE_CONTRACT"));
        assert!(contract.contains("# MODULE_ID: M-TOOL"));
    }
    // END_test_generate_module_contract_uses_language_prefix

    #[test]
    // START_CONTRACT_test_repair_contract_dry_run_does_not_write
    // PURPOSE: Verify dry-run repair returns a preview without modifying the source file
    // START_test_repair_contract_dry_run_does_not_write
    fn test_repair_contract_dry_run_does_not_write() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("src")).unwrap();
        std::fs::write(root.path().join("src/sample.rs"), "pub fn run() {}\n").unwrap();

        let result = ContractGenerator::repair_contract(
            root.path(),
            ContractRepairRequest {
                file_path: "src/sample.rs".into(),
                module_id: Some("M-SAMPLE".into()),
                purpose: Some("Sample module".into()),
                scope: None,
                depends: Vec::new(),
                links: Vec::new(),
                language: None,
                dry_run: true,
            },
        )
        .unwrap();

        assert!(result.changed);
        assert!(!result.applied);
        assert!(result.preview.contains("MODULE_ID: M-SAMPLE"));
        assert!(!std::fs::read_to_string(root.path().join("src/sample.rs"))
            .unwrap()
            .contains("MODULE_CONTRACT"));
    }
    // END_test_repair_contract_dry_run_does_not_write

    #[test]
    // START_CONTRACT_test_repair_contract_write_mode_prepends_once
    // PURPOSE: Verify write-mode repair prepends one MODULE_CONTRACT and skips duplicates
    // START_test_repair_contract_write_mode_prepends_once
    fn test_repair_contract_write_mode_prepends_once() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("src")).unwrap();
        std::fs::write(root.path().join("src/sample.rs"), "pub fn run() {}\n").unwrap();
        let request = ContractRepairRequest {
            file_path: "src/sample.rs".into(),
            module_id: Some("M-SAMPLE".into()),
            purpose: Some("Sample module".into()),
            scope: None,
            depends: Vec::new(),
            links: Vec::new(),
            language: None,
            dry_run: false,
        };

        let first = ContractGenerator::repair_contract(root.path(), request.clone()).unwrap();
        let second = ContractGenerator::repair_contract(root.path(), request).unwrap();

        assert!(first.applied);
        assert!(!second.changed);
        let content = std::fs::read_to_string(root.path().join("src/sample.rs")).unwrap();
        assert_eq!(content.matches("MODULE_ID: M-SAMPLE").count(), 1);
    }
    // END_test_repair_contract_write_mode_prepends_once

    #[test]
    // START_CONTRACT_test_repair_actions_from_failure_text_extracts_paths
    // PURPOSE: Verify missing-contract failure text becomes dry-run repair actions
    // START_test_repair_actions_from_failure_text_extracts_paths
    fn test_repair_actions_from_failure_text_extracts_paths() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("src")).unwrap();
        std::fs::write(root.path().join("src/missing.rs"), "pub fn run() {}\n").unwrap();
        let actions = ContractGenerator::repair_actions_from_failure_text(
            root.path(),
            "contract-exists failed: src/missing.rs missing MODULE_CONTRACT",
            true,
        );

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].file_path, "src/missing.rs");
        assert!(actions[0].dry_run);
    }
    // END_test_repair_actions_from_failure_text_extracts_paths
}
