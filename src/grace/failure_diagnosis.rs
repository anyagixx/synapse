// MODULE_CONTRACT
// MODULE_ID: M-GRACE-FAILURE-DIAGNOSIS
// PURPOSE: Structured test failure diagnosis for XML failure reports, exact identifier search, and repair suggestions
// SCOPE: FailureReport parser, FailureCase model, identifier extraction, exact source search, root-cause candidates, and EnhancedFixResult
// DEPENDS: M-INDEXER-WALKER
// LINKS:
//   -> UC-002 (implements) - converts test failures into bounded repair evidence
//   -> NFR-002 (traces_to) - weak failure reports must degrade gracefully
//   <- V-M-GRACE-FAILURE-DIAGNOSIS (verified_by) - failure diagnosis tests

// START_MODULE_MAP
// FailureReport - Parsed tester-agent failure report
// FailureCase - One failed test/assertion extracted from XML or text
// ExactMatch - Exact source match for extracted identifiers
// SuggestedRepairAction - Candidate repair action inferred from failure evidence
// EnhancedFixResult - Diagnosis output with report, exact matches, candidates, and repairs
// diagnose_failure_text - Parse and diagnose a failure report or plain-text failure
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added structured failure diagnosis]
// END_CHANGE_SUMMARY

use crate::indexer::walker::Walker;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

// START_public_api

// START_FailureReport
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailureReport {
    pub raw: String,
    pub tests: Vec<FailureCase>,
    pub log_refs: Vec<String>,
    pub identifiers: Vec<String>,
}
// END_FailureReport

// START_FailureCase
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailureCase {
    pub name: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub log_refs: Vec<String>,
    pub identifiers: Vec<String>,
}
// END_FailureCase

// START_ExactMatch
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExactMatch {
    pub identifier: String,
    pub path: String,
    pub line: usize,
    pub snippet: String,
}
// END_ExactMatch

// START_RootCauseCandidate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootCauseCandidate {
    pub summary: String,
    pub confidence: String,
    pub evidence: Vec<String>,
}
// END_RootCauseCandidate

// START_SuggestedRepairAction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuggestedRepairAction {
    pub kind: String,
    pub file_path: Option<String>,
    pub reason: String,
}
// END_SuggestedRepairAction

// START_EnhancedFixResult
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnhancedFixResult {
    pub description: String,
    pub report: FailureReport,
    pub exact_matches: Vec<ExactMatch>,
    pub root_cause_candidates: Vec<RootCauseCandidate>,
    pub suggested_repairs: Vec<SuggestedRepairAction>,
    pub diagnosis: String,
}
// END_EnhancedFixResult

impl FailureReport {
    // START_CONTRACT_FailureReport::parse
    // PURPOSE: Parse tester-agent XML or plain text failure into structured cases
    // INPUTS: { input: &str }
    // OUTPUTS: { FailureReport }
    // START_failure_report_parse
    pub fn parse(input: &str) -> Self {
        let mut tests = parse_test_cases(input);
        if tests.is_empty() {
            tests.push(FailureCase {
                name: "failure".into(),
                expected: tag_text(input, "Expected"),
                actual: tag_text(input, "Actual").or_else(|| Some(input.trim().to_string())),
                log_refs: collect_log_refs(input),
                identifiers: extract_identifiers(input),
            });
        }
        let mut log_refs = Vec::new();
        let mut identifiers = Vec::new();
        for test in &tests {
            log_refs.extend(test.log_refs.clone());
            identifiers.extend(test.identifiers.clone());
        }
        log_refs.sort();
        log_refs.dedup();
        identifiers.sort();
        identifiers.dedup();
        Self {
            raw: input.to_string(),
            tests,
            log_refs,
            identifiers,
        }
    }
    // END_failure_report_parse

    // START_CONTRACT_FailureReport::summary
    // PURPOSE: Render a compact human-readable summary for diagnostics
    // OUTPUTS: { String }
    // START_failure_report_summary
    pub fn summary(&self) -> String {
        let test_names: Vec<_> = self.tests.iter().map(|test| test.name.as_str()).collect();
        format!(
            "{} failed test(s): {}; identifiers={}; logs={}",
            self.tests.len(),
            test_names.join(", "),
            self.identifiers.len(),
            self.log_refs.len()
        )
    }
    // END_failure_report_summary
}

// START_CONTRACT_diagnose_failure_text
// PURPOSE: Diagnose failure text with parsed XML structure, exact identifier search, and repair suggestions
// INPUTS: { root: &Path }, { input: &str }
// OUTPUTS: { anyhow::Result<EnhancedFixResult> }
// START_diagnose_failure_text
pub fn diagnose_failure_text(root: &Path, input: &str) -> anyhow::Result<EnhancedFixResult> {
    let report = FailureReport::parse(input);
    let exact_matches = exact_search(root, &report.identifiers, 40);
    let suggested_repairs = suggested_repairs(input);
    let root_cause_candidates = root_cause_candidates(&report, &exact_matches, &suggested_repairs);
    let diagnosis = format!(
        "{}; exact_matches={}; repair_suggestions={}",
        report.summary(),
        exact_matches.len(),
        suggested_repairs.len()
    );
    Ok(EnhancedFixResult {
        description: report.summary(),
        report,
        exact_matches,
        root_cause_candidates,
        suggested_repairs,
        diagnosis,
    })
}
// END_diagnose_failure_text

// START_CONTRACT_parse_test_cases
// PURPOSE: Extract Test blocks from tester-agent XML
// INPUTS: { input: &str }
// OUTPUTS: { Vec<FailureCase> }
// START_parse_test_cases
fn parse_test_cases(input: &str) -> Vec<FailureCase> {
    let Ok(test_re) = regex::Regex::new(r#"(?s)<Test\b([^>]*)>(.*?)</Test>"#) else {
        return Vec::new();
    };
    test_re
        .captures_iter(input)
        .map(|cap| {
            let attrs = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
            let body = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
            let name = attr_value(attrs, "name").unwrap_or_else(|| "unnamed-test".into());
            FailureCase {
                name,
                expected: tag_text(body, "Expected"),
                actual: tag_text(body, "Actual"),
                log_refs: collect_log_refs(body),
                identifiers: extract_identifiers(body),
            }
        })
        .collect()
}
// END_parse_test_cases

// START_CONTRACT_attr_value
// PURPOSE: Extract one XML attribute value from a tag attribute string
// INPUTS: { attrs: &str }, { name: &str }
// OUTPUTS: { Option<String> }
// START_attr_value
fn attr_value(attrs: &str, name: &str) -> Option<String> {
    let re = regex::Regex::new(&format!(r#"{}\s*=\s*"([^"]*)""#, regex::escape(name))).ok()?;
    re.captures(attrs).map(|cap| xml_unescape(&cap[1]))
}
// END_attr_value

// START_CONTRACT_tag_text
// PURPOSE: Extract compact text from one XML tag
// INPUTS: { input: &str }, { tag: &str }
// OUTPUTS: { Option<String> }
// START_tag_text
fn tag_text(input: &str, tag: &str) -> Option<String> {
    let re = regex::Regex::new(&format!(
        r#"(?s)<{}\b[^>]*>(.*?)</{}>"#,
        regex::escape(tag),
        regex::escape(tag)
    ))
    .ok()?;
    re.captures(input)
        .and_then(|cap| cap.get(1))
        .map(|m| normalize_ws(&xml_unescape(m.as_str())))
        .filter(|value| !value.is_empty())
}
// END_tag_text

// START_CONTRACT_collect_log_refs
// PURPOSE: Extract LOG evidence refs from XML failure content
// INPUTS: { input: &str }
// OUTPUTS: { Vec<String> }
// START_collect_log_refs
fn collect_log_refs(input: &str) -> Vec<String> {
    let Ok(re) = regex::Regex::new(r#"<LOG\b[^>]*\bref="([^"]+)""#) else {
        return Vec::new();
    };
    let mut refs: Vec<String> = re.captures_iter(input).map(|cap| cap[1].into()).collect();
    refs.sort();
    refs.dedup();
    refs
}
// END_collect_log_refs

// START_CONTRACT_extract_identifiers
// PURPOSE: Extract stable identifiers, module ids, and code-like tokens from failure text
// INPUTS: { input: &str }
// OUTPUTS: { Vec<String> }
// START_extract_identifiers
pub fn extract_identifiers(input: &str) -> Vec<String> {
    let Ok(re) = regex::Regex::new(r#"[A-Za-z_][A-Za-z0-9_:./-]{2,}"#) else {
        return Vec::new();
    };
    let stop = stop_words();
    let mut ids = BTreeSet::new();
    for mat in re.find_iter(input) {
        let token = mat
            .as_str()
            .trim_matches(|ch: char| ch == '.' || ch == ',' || ch == ';');
        let lower = token.to_ascii_lowercase();
        if stop.contains(lower.as_str()) || token.starts_with("http") {
            continue;
        }
        ids.insert(token.to_string());
    }
    ids.into_iter().collect()
}
// END_extract_identifiers

// START_CONTRACT_exact_search
// PURPOSE: Search source files for exact identifier occurrences and return bounded matches
// INPUTS: { root: &Path }, { identifiers: &[String] }, { limit: usize }
// OUTPUTS: { Vec<ExactMatch> }
// START_exact_search
fn exact_search(root: &Path, identifiers: &[String], limit: usize) -> Vec<ExactMatch> {
    if identifiers.is_empty() || limit == 0 {
        return Vec::new();
    }
    let mut matches = Vec::new();
    let walker = Walker::new(root);
    for file in walker.walk() {
        let path = root.join(&file.path);
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        for identifier in identifiers.iter().take(32) {
            if let Some(byte_index) = content.find(identifier) {
                let line = content[..byte_index]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count()
                    + 1;
                let snippet = content
                    .lines()
                    .nth(line.saturating_sub(1))
                    .unwrap_or_default()
                    .trim()
                    .chars()
                    .take(180)
                    .collect();
                matches.push(ExactMatch {
                    identifier: identifier.clone(),
                    path: file.path.clone(),
                    line,
                    snippet,
                });
                if matches.len() >= limit {
                    return matches;
                }
            }
        }
    }
    matches.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.cmp(&b.line))
            .then(a.identifier.cmp(&b.identifier))
    });
    matches
}
// END_exact_search

// START_CONTRACT_suggested_repairs
// PURPOSE: Infer safe repair actions from missing-contract failure text
// INPUTS: { input: &str }
// OUTPUTS: { Vec<SuggestedRepairAction> }
// START_suggested_repairs
fn suggested_repairs(input: &str) -> Vec<SuggestedRepairAction> {
    let lower = input.to_ascii_lowercase();
    if !(lower.contains("module_contract")
        || lower.contains("contract-exists")
        || lower.contains("missing contract"))
    {
        return Vec::new();
    }
    extract_source_paths(input)
        .into_iter()
        .map(|file_path| SuggestedRepairAction {
            kind: "repair_contract".into(),
            file_path: Some(file_path),
            reason: "failure indicates missing MODULE_CONTRACT".into(),
        })
        .collect()
}
// END_suggested_repairs

// START_CONTRACT_root_cause_candidates
// PURPOSE: Build ranked root-cause candidates from report, exact matches, and repair hints
// INPUTS: { report: &FailureReport }, { exact_matches: &[ExactMatch] }, { repairs: &[SuggestedRepairAction] }
// OUTPUTS: { Vec<RootCauseCandidate> }
// START_root_cause_candidates
fn root_cause_candidates(
    report: &FailureReport,
    exact_matches: &[ExactMatch],
    repairs: &[SuggestedRepairAction],
) -> Vec<RootCauseCandidate> {
    let mut candidates = Vec::new();
    if !repairs.is_empty() {
        candidates.push(RootCauseCandidate {
            summary: "Missing MODULE_CONTRACT can be repaired safely".into(),
            confidence: "high".into(),
            evidence: repairs
                .iter()
                .filter_map(|repair| repair.file_path.clone())
                .collect(),
        });
    }
    if let Some(first) = exact_matches.first() {
        candidates.push(RootCauseCandidate {
            summary: format!(
                "Failure identifier '{}' appears in source",
                first.identifier
            ),
            confidence: "medium".into(),
            evidence: vec![format!("{}:{}", first.path, first.line)],
        });
    }
    if !report.log_refs.is_empty() {
        candidates.push(RootCauseCandidate {
            summary: "Failure report contains LOG evidence refs".into(),
            confidence: "medium".into(),
            evidence: report.log_refs.clone(),
        });
    }
    if candidates.is_empty() {
        candidates.push(RootCauseCandidate {
            summary: "No exact source match found; inspect failure text manually".into(),
            confidence: "low".into(),
            evidence: vec![report.summary()],
        });
    }
    candidates
}
// END_root_cause_candidates

// START_CONTRACT_extract_source_paths
// PURPOSE: Extract source-like file paths from failure text
// INPUTS: { input: &str }
// OUTPUTS: { Vec<String> }
// START_extract_source_paths
fn extract_source_paths(input: &str) -> Vec<String> {
    let Ok(re) = regex::Regex::new(
        r#"([A-Za-z0-9_./-]+\.(rs|py|ts|tsx|js|jsx|go|sql|sh|bash|zsh|rb|java|php|cpp|hpp|c|h))"#,
    ) else {
        return Vec::new();
    };
    let mut paths: Vec<String> = re
        .captures_iter(input)
        .map(|cap| cap[1].trim_start_matches("./").to_string())
        .collect();
    paths.sort();
    paths.dedup();
    paths
}
// END_extract_source_paths

// START_CONTRACT_stop_words
// PURPOSE: Return common XML/prose tokens that should not become identifiers
// OUTPUTS: { BTreeSet<&'static str> }
// START_stop_words
fn stop_words() -> BTreeSet<&'static str> {
    [
        "actual",
        "assertion",
        "expected",
        "failure",
        "false",
        "log",
        "observed",
        "result",
        "test",
        "true",
    ]
    .into_iter()
    .collect()
}
// END_stop_words

// START_CONTRACT_normalize_ws
// PURPOSE: Normalize XML text whitespace to a compact single-line value
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_normalize_ws
fn normalize_ws(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
// END_normalize_ws

// START_CONTRACT_xml_unescape
// PURPOSE: Decode common XML entities used in failure reports
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_xml_unescape
fn xml_unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}
// END_xml_unescape

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // START_CONTRACT_test_failure_report_parser_extracts_log_refs_and_identifiers
    // PURPOSE: Verify XML failure reports produce structured tests, LOG refs, and identifiers
    // START_test_failure_report_parser_extracts_log_refs_and_identifiers
    fn test_failure_report_parser_extracts_log_refs_and_identifiers() {
        let report = FailureReport::parse(
            r#"<TestFailureReport><Test name="Contract"><Expected>MODULE_CONTRACT</Expected><Actual>src/new.rs missing MODULE_CONTRACT</Actual><LogEvidence><LOG ref="log-1">M-NEW</LOG></LogEvidence></Test></TestFailureReport>"#,
        );

        assert_eq!(report.tests.len(), 1);
        assert_eq!(report.log_refs, vec!["log-1"]);
        assert!(report.identifiers.iter().any(|id| id == "src/new.rs"));
    }
    // END_test_failure_report_parser_extracts_log_refs_and_identifiers

    #[test]
    // START_CONTRACT_test_diagnose_failure_text_finds_exact_source_match
    // PURPOSE: Verify diagnosis returns exact source matches for extracted identifiers
    // START_test_diagnose_failure_text_finds_exact_source_match
    fn test_diagnose_failure_text_finds_exact_source_match() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("src")).unwrap();
        std::fs::write(
            root.path().join("src/sample.rs"),
            "pub fn target_symbol() {}\n",
        )
        .unwrap();

        let result = diagnose_failure_text(
            root.path(),
            "<Test name=\"Symbol\"><Actual>target_symbol failed</Actual></Test>",
        )
        .unwrap();

        assert!(result
            .exact_matches
            .iter()
            .any(|item| item.identifier == "target_symbol"));
    }
    // END_test_diagnose_failure_text_finds_exact_source_match

    #[test]
    // START_CONTRACT_test_diagnose_failure_text_suggests_contract_repair
    // PURPOSE: Verify missing MODULE_CONTRACT failures produce repair suggestions
    // START_test_diagnose_failure_text_suggests_contract_repair
    fn test_diagnose_failure_text_suggests_contract_repair() {
        let root = tempfile::tempdir().unwrap();
        let result = diagnose_failure_text(
            root.path(),
            "contract-exists failed: src/missing.rs missing MODULE_CONTRACT",
        )
        .unwrap();

        assert_eq!(result.suggested_repairs.len(), 1);
        assert_eq!(
            result.suggested_repairs[0].file_path.as_deref(),
            Some("src/missing.rs")
        );
    }
    // END_test_diagnose_failure_text_suggests_contract_repair
}
