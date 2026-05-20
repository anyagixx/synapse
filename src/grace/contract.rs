// MODULE_CONTRACT
// MODULE_ID: M-GRACE-CONTRACT
// PURPOSE: MODULE_CONTRACT validator — scans source files for language-aware GRACE contract blocks and validates them
// SCOPE: GraceProfile, ModuleContract, FunctionContract, ContractReport models, ContractValidator with scan_file, scan_file_with_profile, validate_project, and validate_project_with_profile
// DEPENDS: M-INDEXER-WALKER
// LINKS: N/A

// START_MODULE_MAP
// GraceProfile — Verification strictness profile for contract-heavy or lightweight projects
// ModuleContract — Parsed MODULE_CONTRACT block from a source file
// FunctionContract — Parsed START_CONTRACT block for a function
// ContractReport — Aggregate contract validation report
// ContractValidator — Scans and validates GRACE contract blocks
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.10.0 — Added language-aware markers, MODULE_ID validation, and GRACE profiles]
// END_CHANGE_SUMMARY

use std::path::Path;

// START_public_api

// START_GraceProfile
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum GraceProfile {
    Lite,
    Balanced,
    Strict,
}
// END_GraceProfile

impl GraceProfile {
    // START_CONTRACT_GraceProfile::from_name
    // PURPOSE: Parse a user-facing GRACE profile name
    // INPUTS: { name: &str — lite|balanced|strict profile name }
    // OUTPUTS: { Option<GraceProfile> }
    // START_grace_profile_from_name
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "lite" | "light" => Some(Self::Lite),
            "balanced" | "standard" | "default" => Some(Self::Balanced),
            "strict" => Some(Self::Strict),
            _ => None,
        }
    }
    // END_grace_profile_from_name

    // START_CONTRACT_GraceProfile::as_str
    // PURPOSE: Return a stable lowercase profile label for CLI and report output
    // OUTPUTS: { &'static str }
    // START_grace_profile_as_str
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lite => "lite",
            Self::Balanced => "balanced",
            Self::Strict => "strict",
        }
    }
    // END_grace_profile_as_str

    // START_CONTRACT_GraceProfile::requires_function_contracts_for_file
    // PURPOSE: Decide whether a file must declare START_CONTRACT markers under this profile
    // INPUTS: { path: &Path — source path }, { content: &str — source text }
    // OUTPUTS: { bool }
    // START_grace_profile_requires_function_contracts_for_file
    pub fn requires_function_contracts_for_file(self, path: &Path, content: &str) -> bool {
        match self {
            Self::Strict => true,
            Self::Lite => false,
            Self::Balanced => {
                let meaningful_lines = content
                    .lines()
                    .filter(|line| {
                        let trimmed = line.trim();
                        !trimmed.is_empty() && normalize_comment_line(trimmed).is_none()
                    })
                    .count();
                let ext = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or_default();
                meaningful_lines >= 120
                    || matches!(ext, "rs" | "ts" | "tsx" | "go" | "java")
                        && content.contains("pub ")
            }
        }
    }
    // END_grace_profile_requires_function_contracts_for_file
}

// START_ModuleContract
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModuleContract {
    pub file_path: String,
    pub module_id: Option<String>,
    pub purpose: Option<String>,
    pub scope: Option<String>,
    pub depends: Vec<String>,
    pub links: Vec<String>,
    pub has_contract: bool,
    pub valid: bool,
    pub has_module_map: bool,
    pub has_change_summary: bool,
    pub function_contracts: Vec<FunctionContract>,
    pub errors: Vec<String>,
}
// END_ModuleContract

// START_FunctionContract
#[derive(Debug, Clone, serde::Serialize)]
pub struct FunctionContract {
    pub name: String,
    pub purpose: Option<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub side_effects: Vec<String>,
    pub links: Vec<String>,
}
// END_FunctionContract

// START_ContractReport
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContractReport {
    pub total_files: usize,
    pub with_contract: usize,
    pub without_contract: usize,
    pub valid: usize,
    pub invalid: usize,
    pub contracts: Vec<ModuleContract>,
}
// END_ContractReport

// START_ContractValidator
pub struct ContractValidator;
// END_ContractValidator

impl Default for ContractValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractValidator {
    // START_CONTRACT_ContractValidator::new
    // PURPOSE: Create a new ContractValidator
    // OUTPUTS: { Self }
    // START_cv_new
    pub fn new() -> Self {
        Self
    }
    // END_cv_new

    // START_CONTRACT_ContractValidator::scan_file
    // PURPOSE: Scan a single source file for GRACE contract blocks
    // INPUTS: { path: &Path — file path }, { content: &str — file content }
    // OUTPUTS: { ModuleContract — parsed contract (may be invalid) }
    // START_cv_scan_file
    pub fn scan_file(path: &Path, content: &str) -> ModuleContract {
        Self::scan_file_with_profile(path, content, GraceProfile::Strict)
    }
    // END_cv_scan_file

    // START_CONTRACT_ContractValidator::scan_file_with_profile
    // PURPOSE: Scan one source file using the selected GRACE strictness profile
    // INPUTS: { path: &Path — file path }, { content: &str — file content }, { profile: GraceProfile }
    // OUTPUTS: { ModuleContract — parsed contract with profile-aware errors }
    // START_cv_scan_file_with_profile
    pub fn scan_file_with_profile(
        path: &Path,
        content: &str,
        profile: GraceProfile,
    ) -> ModuleContract {
        let file_path = path.to_string_lossy().to_string();
        let mut mc = ModuleContract {
            file_path,
            module_id: None,
            purpose: None,
            scope: None,
            depends: Vec::new(),
            links: Vec::new(),
            has_contract: false,
            valid: false,
            has_module_map: false,
            has_change_summary: false,
            function_contracts: Vec::new(),
            errors: Vec::new(),
        };

        let contract_block = Self::find_contract_block(content);
        let block = match contract_block {
            Some(b) => b,
            None => return mc,
        };

        mc.has_contract = true;

        // Check for MODULE_MAP
        mc.has_module_map = contains_metadata_marker(content, "START_MODULE_MAP")
            || contains_metadata_marker(content, "MODULE_MAP");

        // Check for CHANGE_SUMMARY
        mc.has_change_summary = contains_metadata_marker(content, "START_CHANGE_SUMMARY")
            || contains_metadata_marker(content, "CHANGE_SUMMARY");

        // Extract function contracts
        mc.function_contracts = Self::extract_function_contracts(content);

        let lines: Vec<&str> = block.lines().collect();
        for line in &lines {
            if let Some(val) = normalize_comment_line(line.trim()) {
                Self::parse_field(&mut mc, &val);
            }
        }

        let mut header_valid = true;
        if mc.purpose.is_none() {
            mc.errors.push("Missing PURPOSE in MODULE_CONTRACT".into());
            header_valid = false;
        }
        match mc.module_id.as_deref() {
            Some(module_id) if is_valid_module_id(module_id) => {}
            Some(module_id) if module_id.contains(',') => {
                mc.errors.push(format!(
                    "Invalid MODULE_ID '{}': declare exactly one module id; move related modules to DEPENDS or LINKS",
                    module_id
                ));
                header_valid = false;
            }
            Some(module_id) => {
                mc.errors.push(format!(
                    "Invalid MODULE_ID '{}': expected M- followed by uppercase letters, digits, or hyphens",
                    module_id
                ));
                header_valid = false;
            }
            None => {
                mc.errors
                    .push("Missing MODULE_ID in MODULE_CONTRACT".into());
                header_valid = false;
            }
        }
        mc.valid = header_valid;
        if !mc.has_module_map {
            mc.errors.push("Missing MODULE_MAP".into());
        }
        if !mc.has_change_summary {
            mc.errors.push("Missing CHANGE_SUMMARY".into());
        }
        if mc.function_contracts.is_empty()
            && profile.requires_function_contracts_for_file(path, content)
        {
            mc.errors
                .push("Missing function contracts (START_CONTRACT_name)".into());
        }
        mc
    }
    // END_cv_scan_file_with_profile

    fn extract_function_contracts(content: &str) -> Vec<FunctionContract> {
        let mut contracts = Vec::new();
        extract_contracts_style(content, &mut contracts);
        contracts
    }

    fn find_contract_block(content: &str) -> Option<String> {
        let mut found = false;
        let mut block = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if !found {
                if normalize_comment_line(trimmed)
                    .as_deref()
                    .map(is_module_contract_marker)
                    .unwrap_or(false)
                {
                    found = true;
                    block.push(line);
                }
                continue;
            }
            if is_metadata_boundary(trimmed) {
                break;
            }
            if normalize_comment_line(trimmed).is_some() || trimmed.is_empty() {
                block.push(line);
            } else {
                break;
            }
        }
        if found {
            Some(block.join("\n"))
        } else {
            None
        }
    }

    fn parse_field(mc: &mut ModuleContract, val: &str) {
        if let Some(p) = val.strip_prefix("PURPOSE:") {
            mc.purpose = Some(p.trim().to_string());
        } else if let Some(s) = val.strip_prefix("SCOPE:") {
            mc.scope = Some(s.trim().to_string());
        } else if let Some(d) = val.strip_prefix("DEPENDS:") {
            mc.depends = d
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if let Some(l) = val.strip_prefix("LINKS:") {
            mc.links = l
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if let Some(id) = val.strip_prefix("MODULE_ID:") {
            mc.module_id = Some(id.trim().to_string());
        }
    }

    // START_CONTRACT_ContractValidator::validate_project
    // PURPOSE: Validate all source files in a project for GRACE contract compliance
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<ContractReport> }
    // START_cv_validate_project
    pub fn validate_project(root: &Path) -> anyhow::Result<ContractReport> {
        Self::validate_project_with_profile(root, GraceProfile::Strict)
    }
    // END_cv_validate_project

    // START_CONTRACT_ContractValidator::validate_project_with_profile
    // PURPOSE: Validate all source files using a profile-aware contract strictness policy
    // INPUTS: { root: &Path — project root }, { profile: GraceProfile }
    // OUTPUTS: { anyhow::Result<ContractReport> }
    // START_cv_validate_project_with_profile
    pub fn validate_project_with_profile(
        root: &Path,
        profile: GraceProfile,
    ) -> anyhow::Result<ContractReport> {
        let mut contracts = Vec::new();
        let walker = crate::indexer::walker::Walker::new(root);
        let files = walker.walk();

        let source_extensions = [
            "rs", "py", "ts", "tsx", "js", "jsx", "go", "rb", "java", "php", "cpp", "hpp", "c",
            "h", "css", "scss", "lua", "sh", "bash", "zsh", "svelte", "sql",
        ];
        for file in &files {
            let ext = std::path::Path::new(&file.path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            if !source_extensions.contains(&ext) {
                continue;
            }
            let full_path = root.join(&file.path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let mc = Self::scan_file_with_profile(&full_path, &content, profile);
                contracts.push(mc);
            }
        }

        let total = contracts.len();
        let with = contracts.iter().filter(|c| c.has_contract).count();
        let valid = contracts.iter().filter(|c| c.valid).count();

        Ok(ContractReport {
            total_files: total,
            with_contract: with,
            without_contract: total - with,
            valid,
            invalid: with - valid,
            contracts,
        })
    }
    // END_cv_validate_project_with_profile
}

// START_CONTRACT_is_metadata_boundary
// PURPOSE: Detect comment markers that begin metadata sections outside MODULE_CONTRACT
// INPUTS: { trimmed: &str — trimmed source line }
// OUTPUTS: { bool }
// START_is_metadata_boundary
fn is_metadata_boundary(trimmed: &str) -> bool {
    let Some(marker) = normalize_comment_line(trimmed) else {
        return false;
    };
    marker.starts_with("START_MODULE_MAP")
        || marker == "MODULE_MAP"
        || marker.starts_with("START_CHANGE_SUMMARY")
        || marker == "CHANGE_SUMMARY"
        || marker.starts_with("START_CONTRACT_")
        || marker.starts_with("START_CONTRACT:")
}
// END_is_metadata_boundary

fn extract_contracts_style(content: &str, contracts: &mut Vec<FunctionContract>) {
    let lines: Vec<&str> = content.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        let Some(name) = extract_contract_marker_name(line) else {
            continue;
        };
        if contracts.iter().any(|c| c.name == name) {
            continue;
        }

        let mut fc = FunctionContract {
            name,
            purpose: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            side_effects: Vec::new(),
            links: Vec::new(),
        };

        for body_line in lines.iter().skip(idx + 1) {
            let trimmed = body_line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Some(t) = normalize_comment_line(trimmed) else {
                break;
            };
            if t.starts_with("START_")
                && !t.starts_with("START_CONTRACT_")
                && !t.starts_with("START_CONTRACT:")
            {
                break;
            }
            if let Some(p) = t.strip_prefix("PURPOSE:") {
                fc.purpose = Some(p.trim().to_string());
            } else if let Some(i) = t.strip_prefix("INPUTS:") {
                fc.inputs = i
                    .split("}, {")
                    .map(|s| s.trim().trim_matches('{').trim_matches('}').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            } else if let Some(o) = t.strip_prefix("OUTPUTS:") {
                fc.outputs = o
                    .split("}, {")
                    .map(|s| s.trim().trim_matches('{').trim_matches('}').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            } else if let Some(s) = t.strip_prefix("SIDE_EFFECTS:") {
                fc.side_effects = s
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            } else if let Some(l) = t.strip_prefix("LINKS:") {
                fc.links = l
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
        contracts.push(fc);
    }
}

// START_CONTRACT_extract_contract_marker_name
// PURPOSE: Extract a function contract name from a START_CONTRACT marker without regex construction
// INPUTS: { line: &str — source line }
// OUTPUTS: { Option<String> — parsed marker name when present and valid }
// START_extract_contract_marker_name
fn extract_contract_marker_name(line: &str) -> Option<String> {
    let marker = normalize_comment_line(line.trim())?;
    let rest = marker
        .strip_prefix("START_CONTRACT_")
        .or_else(|| marker.strip_prefix("START_CONTRACT:"))?
        .trim_start();
    let name: String = rest
        .chars()
        .take_while(|ch| {
            ch.is_ascii_alphanumeric() || *ch == '_' || *ch == ':' || *ch == '-' || *ch == '.'
        })
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}
// END_extract_contract_marker_name

// START_CONTRACT_normalize_comment_line
// PURPOSE: Strip a supported language comment prefix from one metadata line
// INPUTS: { trimmed: &str — source line without surrounding whitespace }
// OUTPUTS: { Option<String> — marker text without comment syntax }
// START_normalize_comment_line
fn normalize_comment_line(trimmed: &str) -> Option<String> {
    let value = if let Some(rest) = trimmed.strip_prefix("//") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix('#') {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("--") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("/*") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix('*') {
        rest
    } else {
        trimmed.strip_prefix("<!--")?
    };
    let marker = value
        .trim()
        .trim_end_matches("*/")
        .trim_end_matches("-->")
        .trim();
    Some(canonical_marker_text(marker))
}
// END_normalize_comment_line

// START_CONTRACT_canonical_marker_text
// PURPOSE: Normalize decorative GRACE markers such as === MODULE_CONTRACT ===
// INPUTS: { marker: &str — raw marker text after comment prefix stripping }
// OUTPUTS: { String }
// START_canonical_marker_text
fn canonical_marker_text(marker: &str) -> String {
    marker.trim().trim_matches('=').trim().to_string()
}
// END_canonical_marker_text

// START_CONTRACT_contains_metadata_marker
// PURPOSE: Check whether source content contains a language-aware metadata marker
// INPUTS: { content: &str }, { marker: &str }
// OUTPUTS: { bool }
// START_contains_metadata_marker
fn contains_metadata_marker(content: &str, marker: &str) -> bool {
    content.lines().any(|line| {
        normalize_comment_line(line.trim())
            .as_deref()
            .map(|candidate| candidate == marker)
            .unwrap_or(false)
    })
}
// END_contains_metadata_marker

// START_CONTRACT_is_module_contract_marker
// PURPOSE: Detect supported MODULE_CONTRACT opening markers
// INPUTS: { marker: &str — normalized marker text }
// OUTPUTS: { bool }
// START_is_module_contract_marker
fn is_module_contract_marker(marker: &str) -> bool {
    marker == "MODULE_CONTRACT"
}
// END_is_module_contract_marker

// START_CONTRACT_is_valid_module_id
// PURPOSE: Validate a canonical single MyGRACE module id
// INPUTS: { module_id: &str — parsed MODULE_ID value }
// OUTPUTS: { bool }
// START_is_valid_module_id
fn is_valid_module_id(module_id: &str) -> bool {
    let Some(rest) = module_id.strip_prefix("M-") else {
        return false;
    };
    !rest.is_empty()
        && !module_id.contains(',')
        && rest
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-')
}
// END_is_valid_module_id

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_scan_file_with_contract() {
        let code = "// MODULE_CONTRACT\n// MODULE_ID: M-TEST\n// PURPOSE: Test module\n// SCOPE: Testing\n// DEPENDS: M-DB\n// LINKS: docs/kg.xml\n\n// START_MODULE_MAP\n// test_fn — does testing\n// END_MODULE_MAP\n\n// START_CHANGE_SUMMARY\n// LAST_CHANGE: [v1.0.0 — Initial]\n// END_CHANGE_SUMMARY\n\n// START_CONTRACT_test_fn\n// PURPOSE: Run test\n// INPUTS: { name: String — test name }\n// OUTPUTS: { bool — success }\n// SIDE_EFFECTS: prints to stdout\n// START_test_fn\nfn test_fn(name: &str) -> bool { true }\n// END_test_fn";
        let mc = ContractValidator::scan_file(Path::new("src/test.rs"), code);
        assert!(mc.has_contract);
        assert_eq!(mc.module_id.as_deref(), Some("M-TEST"));
        assert!(mc.purpose.is_some());
        assert_eq!(mc.depends.len(), 1);
        assert!(mc.has_module_map);
        assert!(mc.has_change_summary);
        assert_eq!(mc.function_contracts.len(), 1);
        assert_eq!(mc.function_contracts[0].name, "test_fn");
        assert!(mc.function_contracts[0].purpose.is_some());
        assert_eq!(mc.function_contracts[0].inputs.len(), 1);
        assert_eq!(mc.function_contracts[0].outputs.len(), 1);
        assert_eq!(mc.function_contracts[0].side_effects.len(), 1);
        assert!(mc.valid);
    }

    #[test]
    fn test_scan_file_without_contract() {
        let code = "fn main() {}\nfn helper() {}";
        let mc = ContractValidator::scan_file(Path::new("src/main.rs"), code);
        assert!(!mc.has_contract);
        assert!(!mc.has_module_map);
        assert!(!mc.has_change_summary);
        assert!(mc.function_contracts.is_empty());
        assert!(!mc.valid);
    }

    #[test]
    fn test_scan_file_missing_purpose() {
        let code = "// MODULE_CONTRACT\n// MODULE_ID: M-BAD\nfn foo() {}";
        let mc = ContractValidator::scan_file(Path::new("src/bad.rs"), code);
        assert!(mc.has_contract);
        assert!(!mc.valid);
        assert!(!mc.errors.is_empty());
    }

    #[test]
    fn test_extract_function_contracts_python() {
        let code = "# MODULE_CONTRACT\n# MODULE_ID: M-PY\n# PURPOSE: Python test\n\n# START_CONTRACT_greet\n# PURPOSE: Say hello\n# INPUTS: { name: str — user name }\n# OUTPUTS: { str — greeting }\n# START_greet\ndef greet(name): return f'Hello {name}'\n# END_greet";
        let mc = ContractValidator::scan_file(Path::new("src/mod.py"), code);
        assert!(mc.has_contract);
        assert_eq!(mc.function_contracts.len(), 1);
        assert_eq!(mc.function_contracts[0].name, "greet");
        assert_eq!(mc.function_contracts[0].inputs.len(), 1);
    }

    #[test]
    fn test_module_contract_stops_before_function_contract() {
        let code = "# MODULE_CONTRACT\n# MODULE_ID: M-SH\n# PURPOSE: Shell module\n# SCOPE: Test shell parsing\n# DEPENDS: M-ONE\n# LINKS: docs/modules/M-SH.xml\n\n# START_MODULE_MAP\n# run — executes\n# END_MODULE_MAP\n\n# START_CHANGE_SUMMARY\n# LAST_CHANGE: [v1.0.0 — Initial]\n# END_CHANGE_SUMMARY\n\n# START_CONTRACT_run\n# PURPOSE: Function purpose must not replace module purpose\n# START_run\nset -euo pipefail\n# END_run";
        let mc = ContractValidator::scan_file(Path::new("scripts/test.sh"), code);
        assert_eq!(mc.module_id.as_deref(), Some("M-SH"));
        assert_eq!(mc.purpose.as_deref(), Some("Shell module"));
        assert_eq!(mc.scope.as_deref(), Some("Test shell parsing"));
        assert_eq!(mc.function_contracts.len(), 1);
        assert_eq!(
            mc.function_contracts[0].purpose.as_deref(),
            Some("Function purpose must not replace module purpose")
        );
    }

    #[test]
    fn test_extract_function_contract_name_stops_at_marker_suffix() {
        let code = "// MODULE_CONTRACT\n// MODULE_ID: M-TEST\n// PURPOSE: Test module\n\n// START_MODULE_MAP\n// alpha — does work\n// END_MODULE_MAP\n\n// START_CHANGE_SUMMARY\n// LAST_CHANGE: [v1.0.0 — Initial]\n// END_CHANGE_SUMMARY\n\n// START_CONTRACT_alpha trailing words\n// PURPOSE: Alpha work\n// START_alpha\nfn alpha() {}\n// END_alpha";
        let mc = ContractValidator::scan_file(Path::new("src/test.rs"), code);

        assert_eq!(mc.function_contracts.len(), 1);
        assert_eq!(mc.function_contracts[0].name, "alpha");
    }

    #[test]
    fn test_extract_contracts_sql_comment_style() {
        let code = "-- MODULE_CONTRACT\n-- MODULE_ID: M-SQL\n-- PURPOSE: SQL schema\n-- SCOPE: Tables\n-- DEPENDS: M-STORAGE\n-- LINKS: docs/modules/M-SQL.xml\n\n-- START_MODULE_MAP\n-- users — table\n-- END_MODULE_MAP\n\n-- START_CHANGE_SUMMARY\n-- LAST_CHANGE: [v1.0.0 — Initial]\n-- END_CHANGE_SUMMARY\n\n-- START_CONTRACT_create_users\n-- PURPOSE: Create users table\n-- OUTPUTS: { table — users }\n-- START_create_users\nCREATE TABLE users(id INTEGER PRIMARY KEY);\n-- END_create_users";
        let mc = ContractValidator::scan_file(Path::new("schema.sql"), code);

        assert!(mc.has_contract);
        assert_eq!(mc.module_id.as_deref(), Some("M-SQL"));
        assert_eq!(mc.function_contracts.len(), 1);
        assert_eq!(mc.function_contracts[0].name, "create_users");
        assert!(mc.valid);
    }

    #[test]
    fn test_module_id_with_commas_is_invalid() {
        let code = "# MODULE_CONTRACT\n# MODULE_ID: M-STORAGE, M-BOT, M-SCHEDULER\n# PURPOSE: Bot module\n# START_MODULE_MAP\n# END_MODULE_MAP\n# START_CHANGE_SUMMARY\n# END_CHANGE_SUMMARY";
        let mc = ContractValidator::scan_file_with_profile(
            Path::new("bot.py"),
            code,
            GraceProfile::Lite,
        );

        assert!(mc.has_contract);
        assert!(!mc.valid);
        assert!(mc.errors.iter().any(|error| error.contains("exactly one")));
    }

    #[test]
    fn test_lite_profile_allows_small_helper_without_function_contracts() {
        let code = "# MODULE_CONTRACT\n# MODULE_ID: M-HELPER\n# PURPOSE: Small helper\n# START_MODULE_MAP\n# parse_date — parse\n# END_MODULE_MAP\n# START_CHANGE_SUMMARY\n# LAST_CHANGE: [v1.0.0 — Initial]\n# END_CHANGE_SUMMARY\n\ndef parse_date(value):\n    return value";
        let mc = ContractValidator::scan_file_with_profile(
            Path::new("date_parser.py"),
            code,
            GraceProfile::Lite,
        );

        assert!(mc.valid);
        assert!(mc
            .errors
            .iter()
            .all(|error| !error.contains("Missing function contracts")));
    }
}
// END_public_api
