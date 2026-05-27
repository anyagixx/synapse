// MODULE_CONTRACT
// MODULE_ID: M-GRACE-LOG
// PURPOSE: Structured GRACE LOG model, validator, and rule-based analyzer for Log Driven Development
// SCOPE: GraceLog, LogLevel, LogResult, StructuredLogFormatReport, LogAnalysisReport, source scan, log parsing, and analysis helpers
// DEPENDS: M-INDEXER-WALKER
// LINKS:
//   → V-M-GRACE-LOG (verified_by) — structured log parser and analyzer tests

// START_MODULE_MAP
// GraceLog — Runtime structured LOG entry model
// StructuredLogFormatReport — Source marker validation report
// LogAnalysisReport — Rule-based trajectory/anomaly/compare analysis report
// scan_source_logs — Validates XML-like LOG markers in source files
// analyze_log_content — Analyzes structured LOG entries from a log string
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 - Added TRACEABILITY evidence to LOG test fixtures]
// END_CHANGE_SUMMARY

use std::collections::{HashMap, HashSet};
use std::path::Path;

// START_public_api

// START_LogLevel
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}
// END_LogLevel

impl LogLevel {
    // START_CONTRACT_LogLevel::parse
    // PURPOSE: Parse a structured LOG level label
    // INPUTS: { value: &str — DEBUG|INFO|WARN|ERROR|CRITICAL }
    // OUTPUTS: { Option<LogLevel> }
    // START_log_level_parse
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "DEBUG" => Some(Self::Debug),
            "INFO" => Some(Self::Info),
            "WARN" | "WARNING" => Some(Self::Warn),
            "ERROR" => Some(Self::Error),
            "CRITICAL" => Some(Self::Critical),
            _ => None,
        }
    }
    // END_log_level_parse

    // START_CONTRACT_LogLevel::label
    // PURPOSE: Return the canonical uppercase LOG level label
    // OUTPUTS: { &'static str }
    // START_log_level_label
    pub fn label(&self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Critical => "CRITICAL",
        }
    }
    // END_log_level_label
}

// START_LogResult
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum LogResult {
    Success,
    Failure,
    Warning,
    Blocked,
}
// END_LogResult

impl LogResult {
    // START_CONTRACT_LogResult::parse
    // PURPOSE: Parse a structured LOG result label
    // INPUTS: { value: &str — success|failure|warning|blocked }
    // OUTPUTS: { Option<LogResult> }
    // START_log_result_parse
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "success" | "pass" | "passed" => Some(Self::Success),
            "failure" | "fail" | "failed" => Some(Self::Failure),
            "warning" | "warn" => Some(Self::Warning),
            "blocked" | "block" => Some(Self::Blocked),
            _ => None,
        }
    }
    // END_log_result_parse

    // START_CONTRACT_LogResult::label
    // PURPOSE: Return the canonical lowercase LOG result label
    // OUTPUTS: { &'static str }
    // START_log_result_label
    pub fn label(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Warning => "warning",
            Self::Blocked => "blocked",
        }
    }
    // END_log_result_label
}

// START_GraceLog
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraceLog {
    pub id: String,
    pub level: LogLevel,
    pub ref_block: String,
    pub module_id: String,
    pub contract_name: String,
    pub timestamp: String,
    pub event: String,
    pub context: HashMap<String, String>,
    pub state: HashMap<String, String>,
    pub decision: String,
    pub expectation: String,
    pub result: LogResult,
    pub detail: Option<String>,
    pub traceability: Option<HashMap<String, String>>,
}
// END_GraceLog

impl GraceLog {
    // START_CONTRACT_GraceLog::new
    // PURPOSE: Create a structured LOG entry with required identity fields
    // INPUTS: { id: &str }, { module_id: &str }
    // OUTPUTS: { GraceLog }
    // START_grace_log_new
    pub fn new(id: &str, module_id: &str) -> Self {
        Self {
            id: id.to_string(),
            level: LogLevel::Info,
            ref_block: String::new(),
            module_id: module_id.to_string(),
            contract_name: String::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            event: String::new(),
            context: HashMap::new(),
            state: HashMap::new(),
            decision: String::new(),
            expectation: String::new(),
            result: LogResult::Success,
            detail: None,
            traceability: None,
        }
    }
    // END_grace_log_new

    // START_CONTRACT_GraceLog::emit
    // PURPOSE: Render this entry as XML-like GRACE LOG text
    // OUTPUTS: { String }
    // START_grace_log_emit
    pub fn emit(&self) -> String {
        let mut output = format!(
            "<LOG id=\"{}\" level=\"{}\" ref=\"{}\" module=\"{}\" contract=\"{}\" timestamp=\"{}\">\n",
            escape_attr(&self.id),
            self.level.label(),
            escape_attr(&self.ref_block),
            escape_attr(&self.module_id),
            escape_attr(&self.contract_name),
            escape_attr(&self.timestamp)
        );
        output.push_str(&format!("  EVENT: {}\n", self.event));
        output.push_str("  CONTEXT:\n");
        append_map(&mut output, &self.context);
        output.push_str("  STATE:\n");
        append_map(&mut output, &self.state);
        output.push_str(&format!("  DECISION: {}\n", self.decision));
        output.push_str(&format!("  EXPECTATION: {}\n", self.expectation));
        output.push_str(&format!("  RESULT: {}\n", self.result.label()));
        if let Some(detail) = &self.detail {
            output.push_str(&format!("  DETAIL: {}\n", detail));
        }
        if let Some(traceability) = &self.traceability {
            output.push_str("  TRACEABILITY:\n");
            append_map(&mut output, traceability);
        }
        output.push_str("</LOG>");
        output
    }
    // END_grace_log_emit

    // START_CONTRACT_GraceLog::emit_json
    // PURPOSE: Render this entry as compact JSON for machine consumers
    // OUTPUTS: { String }
    // START_grace_log_emit_json
    pub fn emit_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
    // END_grace_log_emit_json
}

// START_StructuredLogFormatReport
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct StructuredLogFormatReport {
    pub total_logs: usize,
    pub valid_logs: usize,
    pub invalid_logs: usize,
    pub duplicate_ids: Vec<String>,
    pub issues: Vec<String>,
}
// END_StructuredLogFormatReport

impl StructuredLogFormatReport {
    // START_CONTRACT_StructuredLogFormatReport::passed
    // PURPOSE: Return true when all discovered source LOG markers are structurally valid
    // OUTPUTS: { bool }
    // START_structured_log_report_passed
    pub fn passed(&self) -> bool {
        self.invalid_logs == 0 && self.duplicate_ids.is_empty()
    }
    // END_structured_log_report_passed
}

// START_LogAnalysisEvent
#[derive(Debug, Clone, serde::Serialize)]
pub struct LogAnalysisEvent {
    pub id: String,
    pub result: String,
    pub actual_matches_expected: bool,
    pub analysis: String,
}
// END_LogAnalysisEvent

// START_LogAnalysisReport
#[derive(Debug, Clone, serde::Serialize)]
pub struct LogAnalysisReport {
    pub module: String,
    pub mode: String,
    pub total_events: usize,
    pub matches: usize,
    pub mismatches: usize,
    pub anomalies: usize,
    pub conclusion: String,
    pub events: Vec<LogAnalysisEvent>,
}
// END_LogAnalysisReport

impl LogAnalysisReport {
    // START_CONTRACT_LogAnalysisReport::to_xml
    // PURPOSE: Render the analysis report as XML-like text for MCP responses
    // OUTPUTS: { String }
    // START_log_analysis_report_to_xml
    pub fn to_xml(&self) -> String {
        let mut output = format!(
            "<LogAnalysisReport module=\"{}\" mode=\"{}\">\n",
            escape_attr(&self.module),
            escape_attr(&self.mode)
        );
        output.push_str("  <Summary>\n");
        output.push_str(&format!(
            "    <TotalEvents>{}</TotalEvents>\n",
            self.total_events
        ));
        output.push_str("    <ExpectedVsActual>\n");
        output.push_str(&format!("      <Match>{}</Match>\n", self.matches));
        output.push_str(&format!("      <Mismatch>{}</Mismatch>\n", self.mismatches));
        output.push_str("    </ExpectedVsActual>\n");
        output.push_str(&format!("    <Anomalies>{}</Anomalies>\n", self.anomalies));
        output.push_str(&format!(
            "    <Conclusion>{}</Conclusion>\n",
            escape_text(&self.conclusion)
        ));
        output.push_str("  </Summary>\n  <DetailedTrace>\n");
        for event in &self.events {
            output.push_str(&format!(
                "    <Event id=\"{}\" result=\"{}\">\n",
                escape_attr(&event.id),
                escape_attr(&event.result)
            ));
            output.push_str(&format!(
                "      <ActualMatchesExpected>{}</ActualMatchesExpected>\n",
                event.actual_matches_expected
            ));
            output.push_str(&format!(
                "      <Analysis>{}</Analysis>\n",
                escape_text(&event.analysis)
            ));
            output.push_str("    </Event>\n");
        }
        output.push_str("  </DetailedTrace>\n</LogAnalysisReport>");
        output
    }
    // END_log_analysis_report_to_xml
}

// START_CONTRACT_scan_source_logs
// PURPOSE: Validate structured LOG markers embedded in source files
// INPUTS: { root: &Path — project root }
// OUTPUTS: { anyhow::Result<StructuredLogFormatReport> }
// START_scan_source_logs
pub fn scan_source_logs(root: &Path) -> anyhow::Result<StructuredLogFormatReport> {
    let walker = crate::indexer::walker::Walker::new(root);
    let mut report = StructuredLogFormatReport::default();
    let mut seen_ids = HashSet::new();
    for file in walker.walk() {
        if file.language == "markdown" {
            continue;
        }
        let full_path = root.join(&file.path);
        let Ok(content) = std::fs::read_to_string(&full_path) else {
            continue;
        };
        let entries = parse_log_entries(&content);
        for entry in entries {
            report.total_logs += 1;
            let context = format!("{}:{}", file.path, entry.start_line);
            let mut issues = validate_marker(&entry, &context);
            if let Some(id) = entry.attrs.get("id") {
                if !seen_ids.insert(id.clone()) {
                    report.duplicate_ids.push(id.clone());
                    issues.push(format!("{} duplicate LOG id '{}'", context, id));
                }
            }
            if issues.is_empty() {
                report.valid_logs += 1;
            } else {
                report.invalid_logs += 1;
                report.issues.extend(issues);
            }
        }
    }
    Ok(report)
}
// END_scan_source_logs

// START_CONTRACT_analyze_log_content
// PURPOSE: Analyze structured LOG content using trajectory, anomaly, or compare mode
// INPUTS: { content: &str }, { mode: &str }, { contract_ref: Option<&str> }
// OUTPUTS: { LogAnalysisReport }
// START_analyze_log_content
pub fn analyze_log_content(
    content: &str,
    mode: &str,
    contract_ref: Option<&str>,
) -> LogAnalysisReport {
    let entries: Vec<ParsedLogEntry> = parse_log_entries(content)
        .into_iter()
        .filter(|entry| {
            contract_ref.is_none_or(|target| {
                entry
                    .attrs
                    .get("module")
                    .is_some_and(|module| module == target)
                    || entry
                        .attrs
                        .get("contract")
                        .is_some_and(|contract| contract == target)
            })
        })
        .collect();
    let mut events = Vec::new();
    let mut matches = 0usize;
    let mut mismatches = 0usize;
    let mut anomalies = 0usize;
    for entry in &entries {
        let result = entry
            .fields
            .get("RESULT")
            .cloned()
            .unwrap_or_else(|| "unknown".into());
        let expectation = entry.fields.get("EXPECTATION").cloned().unwrap_or_default();
        let event_name = entry.fields.get("EVENT").cloned().unwrap_or_default();
        let actual_matches_expected = expectation_matches_result(&expectation, &result);
        if actual_matches_expected {
            matches += 1;
        } else {
            mismatches += 1;
            anomalies += 1;
        }
        if mode == "anomaly" && actual_matches_expected {
            continue;
        }
        let id = entry
            .attrs
            .get("id")
            .cloned()
            .unwrap_or_else(|| format!("line-{}", entry.start_line));
        let analysis = if actual_matches_expected {
            format!(
                "{} matched expectation with RESULT={}",
                empty_as(&event_name, "event"),
                result
            )
        } else {
            format!(
                "{} deviated: EXPECTATION='{}' RESULT={}",
                empty_as(&event_name, "event"),
                expectation,
                result
            )
        };
        events.push(LogAnalysisEvent {
            id,
            result,
            actual_matches_expected,
            analysis,
        });
    }
    let module = contract_ref
        .map(ToOwned::to_owned)
        .or_else(|| {
            entries
                .first()
                .and_then(|entry| entry.attrs.get("module").cloned())
        })
        .unwrap_or_else(|| "unknown".into());
    let conclusion = if entries.is_empty() {
        "No structured LOG entries found for the requested scope.".to_string()
    } else if anomalies == 0 {
        format!(
            "The log trajectory matches declared expectations for {}/{} events.",
            matches,
            entries.len()
        )
    } else {
        format!(
            "{} deviations detected across {} structured LOG events.",
            anomalies,
            entries.len()
        )
    };
    LogAnalysisReport {
        module,
        mode: mode.to_string(),
        total_events: entries.len(),
        matches,
        mismatches,
        anomalies,
        conclusion,
        events,
    }
}
// END_analyze_log_content

// START_CONTRACT_analyze_log_file
// PURPOSE: Read and analyze a structured LOG file from disk
// INPUTS: { path: &Path }, { mode: &str }, { contract_ref: Option<&str> }
// OUTPUTS: { anyhow::Result<LogAnalysisReport> }
// START_analyze_log_file
pub fn analyze_log_file(
    path: &Path,
    mode: &str,
    contract_ref: Option<&str>,
) -> anyhow::Result<LogAnalysisReport> {
    let content = std::fs::read_to_string(path)?;
    Ok(analyze_log_content(&content, mode, contract_ref))
}
// END_analyze_log_file

// END_public_api

#[derive(Debug, Clone)]
struct ParsedLogEntry {
    start_line: usize,
    attrs: HashMap<String, String>,
    fields: HashMap<String, String>,
}

fn parse_log_entries(content: &str) -> Vec<ParsedLogEntry> {
    let lines: Vec<&str> = content.lines().collect();
    let mut entries = Vec::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        let Some(line) = normalize_log_line(lines[idx]) else {
            idx += 1;
            continue;
        };
        if !line.starts_with("<LOG") {
            idx += 1;
            continue;
        }
        let start_line = idx + 1;
        let attrs = parse_attrs(&line);
        let mut body = Vec::new();
        idx += 1;
        while idx < lines.len() {
            let Some(body_line) = normalize_log_line(lines[idx]) else {
                idx += 1;
                continue;
            };
            if body_line.starts_with("</LOG>") {
                break;
            }
            body.push(body_line);
            idx += 1;
        }
        entries.push(ParsedLogEntry {
            start_line,
            attrs,
            fields: parse_fields(&body),
        });
        idx += 1;
    }
    entries
}

fn validate_marker(entry: &ParsedLogEntry, context: &str) -> Vec<String> {
    let mut issues = Vec::new();
    for attr in ["id", "level", "ref"] {
        if entry
            .attrs
            .get(attr)
            .is_none_or(|value| value.trim().is_empty())
        {
            issues.push(format!("{} LOG missing {} attribute", context, attr));
        }
    }
    if entry
        .attrs
        .get("level")
        .is_some_and(|level| LogLevel::parse(level).is_none())
    {
        issues.push(format!("{} LOG has invalid level", context));
    }
    for field in ["EVENT", "DECISION", "EXPECTATION", "RESULT"] {
        if entry
            .fields
            .get(field)
            .is_none_or(|value| value.trim().is_empty())
        {
            issues.push(format!("{} LOG missing {}", context, field));
        }
    }
    if entry
        .fields
        .get("RESULT")
        .is_some_and(|result| LogResult::parse(result).is_none())
    {
        issues.push(format!("{} LOG has invalid RESULT", context));
    }
    issues
}

fn parse_attrs(open_line: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let Ok(re) = regex::Regex::new(r#"([A-Za-z_][A-Za-z0-9_-]*)="([^"]*)""#) else {
        return attrs;
    };
    for cap in re.captures_iter(open_line) {
        attrs.insert(cap[1].to_string(), cap[2].to_string());
    }
    attrs
}

fn parse_fields(lines: &[String]) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    let mut current_key: Option<String> = None;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_field(trimmed) {
            current_key = Some(key.clone());
            fields
                .entry(key)
                .and_modify(|existing: &mut String| {
                    if !value.is_empty() {
                        existing.push('\n');
                        existing.push_str(&value);
                    }
                })
                .or_insert(value);
        } else if let Some(key) = &current_key {
            fields
                .entry(key.clone())
                .and_modify(|existing| {
                    existing.push('\n');
                    existing.push_str(trimmed);
                })
                .or_insert_with(|| trimmed.to_string());
        }
    }
    fields
}

fn split_field(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once(':')?;
    let normalized_key = key.trim().to_ascii_uppercase();
    if matches!(
        normalized_key.as_str(),
        "EVENT"
            | "CONTEXT"
            | "STATE"
            | "DECISION"
            | "EXPECTATION"
            | "RESULT"
            | "DETAIL"
            | "TRACEABILITY"
    ) {
        Some((normalized_key, value.trim().to_string()))
    } else {
        None
    }
}

fn normalize_log_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let stripped = if let Some(rest) = trimmed.strip_prefix("//") {
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
        trimmed
    };
    Some(stripped.trim().trim_end_matches("*/").trim().to_string())
}

fn expectation_matches_result(expectation: &str, result: &str) -> bool {
    let result = LogResult::parse(result);
    match result {
        Some(LogResult::Success) => true,
        Some(LogResult::Failure) | Some(LogResult::Blocked) | Some(LogResult::Warning) => {
            let expectation = expectation.to_ascii_lowercase();
            [
                "fail",
                "failure",
                "error",
                "insufficient",
                "reject",
                "cancel",
                "blocked",
                "warning",
                "timeout",
                "not ",
                "do not",
            ]
            .iter()
            .any(|needle| expectation.contains(needle))
        }
        None => false,
    }
}

fn append_map(output: &mut String, values: &HashMap<String, String>) {
    let mut keys: Vec<&String> = values.keys().collect();
    keys.sort();
    for key in keys {
        if let Some(value) = values.get(key) {
            output.push_str(&format!("    {}: {}\n", key, value));
        }
    }
}

fn escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn empty_as(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grace_log_emit_and_parse() {
        let mut log = GraceLog::new("order-001", "M-ORDER");
        log.ref_block = "stock-validation".into();
        log.contract_name = "place_order".into();
        log.event = "stock_validation_started".into();
        log.decision = "Validate stock before payment".into();
        log.expectation = "If stock exists, processing succeeds".into();
        log.result = LogResult::Success;
        log.context.insert("customerId".into(), "cus_1".into());

        let emitted = log.emit();
        let report = analyze_log_content(&emitted, "trajectory", Some("M-ORDER"));
        assert_eq!(report.total_events, 1);
        assert_eq!(report.matches, 1);
        assert!(emitted.contains("EXPECTATION:"));
    }

    #[test]
    fn test_scan_source_logs_reports_missing_expectation() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("bad.rs"),
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-BAD\n",
                "// PURPOSE: Bad log fixture\n",
                "// START_MODULE_MAP\n",
                "// END_MODULE_MAP\n",
                "// START_CHANGE_SUMMARY\n",
                "// END_CHANGE_SUMMARY\n",
                "// START_CONTRACT_run\n",
                "// PURPOSE: Run\n",
                "// <LOG id=\"bad-001\" level=\"INFO\" ref=\"run\" module=\"M-BAD\" contract=\"run\">\n",
                "//   EVENT: run_started\n",
                "//   DECISION: Run the fixture\n",
                "//   RESULT: success\n",
                "//   TRACEABILITY:\n",
                "//     UC-002: malformed log fixtures still point to verification intent\n",
                "// </LOG>\n",
                "// START_run\n",
                "fn run() {}\n",
                "// END_run\n",
            ),
        )
        .expect("write source");
        let report = scan_source_logs(dir.path()).expect("scan");
        assert_eq!(report.total_logs, 1);
        assert_eq!(report.invalid_logs, 1);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.contains("EXPECTATION")));
    }

    #[test]
    fn test_analyze_log_content_detects_anomaly() {
        let content = concat!(
            "<LOG id=\"pay-001\" level=\"INFO\" ref=\"payment\" module=\"M-PAY\" contract=\"pay\">\n",
            "  EVENT: payment_started\n",
            "  DECISION: Authorize payment\n",
            "  EXPECTATION: Payment authorization succeeds\n",
            "  RESULT: failure\n",
            "  TRACEABILITY:\n",
            "    UC-002: anomaly analysis supports verified project changes\n",
            "</LOG>\n"
        );
        let report = analyze_log_content(content, "anomaly", Some("M-PAY"));
        assert_eq!(report.total_events, 1);
        assert_eq!(report.anomalies, 1);
        assert!(report.to_xml().contains("<Mismatch>1</Mismatch>"));
    }
}
