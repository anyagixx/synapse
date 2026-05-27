// MODULE_CONTRACT
// MODULE_ID: M-CLI-RTK-COMMANDS
// PURPOSE: RTK-style local .NET artifact summarizers for MSBuild binlogs, dotnet format reports, and TRX test results
// SCOPE: BinlogCmd, DotnetFormatReportCmd, and DotnetTrxCmd execution; compact parsers for diagnostics, formatting changes, and test failures; adapter-level token tracking
// DEPENDS: M-CONFIG, M-TRACKING
// LINKS:
//   -> M-CLI (depends) - exposes .NET artifact adapter schemas
//   -> M-TRACKING (depends) - records adapter-level token savings
//   -> Phase-54 (implements) - closes remaining RTK .NET command-module parity gaps
//   -> NFR-003 (traces_to) - compact artifact summaries reduce LLM context cost

// START_MODULE_MAP
// BinlogCmd::run - Summarizes printable MSBuild binlog diagnostics
// DotnetFormatReportCmd::run - Summarizes dotnet format JSON report changes
// DotnetTrxCmd::run - Summarizes TRX test result counters and failures
// render_binlog_summary - Extracts error and warning lines from binary/text artifacts
// render_format_report_summary - Compacts dotnet format JSON report entries
// render_trx_summary - Compacts TRX XML counters and failed test details
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added .NET artifact RTK adapters]
// END_CHANGE_SUMMARY

use super::{BinlogCmd, DotnetFormatReportCmd, DotnetTrxCmd};
use syn_core::config::Config;
use regex::Regex;
use serde::Deserialize;
use std::path::Path;

const MAX_DIAGNOSTICS: usize = 40;
const MAX_FAILED_TESTS: usize = 12;
const MAX_FORMAT_FILES: usize = 30;
const MIN_PRINTABLE_RUN_CHARS: usize = 5;

// START_public_api

impl BinlogCmd {
    // START_CONTRACT_BinlogCmd::run
    // PURPOSE: Summarize MSBuild binlog diagnostics without dumping the full artifact
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads a local artifact, writes stdout, records adapter savings
    // LINKS:
    //   -> Phase-54 (implements) - binlog RTK module parity
    // START_binlog_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let raw = std::fs::read(&self.path)?;
        let rendered = render_binlog_summary(&raw);
        println!("{rendered}");
        record_artifact_savings(
            &config,
            "syn binlog",
            &String::from_utf8_lossy(&raw),
            &rendered,
            "rtk-binlog",
            "syn binlog",
        )
        .await;
        Ok(())
    }
    // END_binlog_cmd_run
}

impl DotnetFormatReportCmd {
    // START_CONTRACT_DotnetFormatReportCmd::run
    // PURPOSE: Summarize a dotnet format JSON report
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads a local JSON file, writes stdout, records adapter savings
    // LINKS:
    //   -> Phase-54 (implements) - dotnet-format-report RTK module parity
    // START_dotnet_format_report_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let raw = std::fs::read_to_string(&self.path)?;
        let rendered = render_format_report_summary(&raw)?;
        println!("{rendered}");
        record_artifact_savings(
            &config,
            "syn dotnet-format-report",
            &raw,
            &rendered,
            "rtk-dotnet-format-report",
            "syn dotnet-format-report",
        )
        .await;
        Ok(())
    }
    // END_dotnet_format_report_cmd_run
}

impl DotnetTrxCmd {
    // START_CONTRACT_DotnetTrxCmd::run
    // PURPOSE: Summarize a TRX file or directory of TRX files
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads local XML files, writes stdout, records adapter savings
    // LINKS:
    //   -> Phase-54 (implements) - dotnet-trx RTK module parity
    // START_dotnet_trx_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let (raw, rendered) = if self.path.is_dir() {
            let raw = read_trx_dir(&self.path)?;
            let rendered = render_trx_summary(&raw);
            (raw, rendered)
        } else {
            let raw = std::fs::read_to_string(&self.path)?;
            let rendered = render_trx_summary(&raw);
            (raw, rendered)
        };
        println!("{rendered}");
        record_artifact_savings(
            &config,
            "syn dotnet-trx",
            &raw,
            &rendered,
            "rtk-dotnet-trx",
            "syn dotnet-trx",
        )
        .await;
        Ok(())
    }
    // END_dotnet_trx_cmd_run
}

// END_public_api

// START_FormatReportEntry
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct FormatReportEntry {
    file_path: String,
    #[serde(default)]
    file_changes: Vec<FormatFileChange>,
}
// END_FormatReportEntry

// START_FormatFileChange
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct FormatFileChange {
    line_number: u32,
    char_number: u32,
    diagnostic_id: String,
    format_description: String,
}
// END_FormatFileChange

// START_TrxSummary
#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct TrxSummary {
    total: u64,
    passed: u64,
    failed: u64,
    skipped: u64,
    failures: Vec<TrxFailure>,
}
// END_TrxSummary

// START_TrxFailure
#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct TrxFailure {
    name: String,
    message: Option<String>,
}
// END_TrxFailure

// START_CONTRACT_render_binlog_summary
// PURPOSE: Extract compact diagnostics from binary or text MSBuild artifacts
// INPUTS: { bytes: &[u8] }
// OUTPUTS: { String }
// START_render_binlog_summary
fn render_binlog_summary(bytes: &[u8]) -> String {
    let text = printable_text(bytes);
    let diagnostics = diagnostic_lines(&text);
    if diagnostics.is_empty() {
        return "binlog: no printable errors or warnings found".into();
    }

    let error_count = diagnostics
        .iter()
        .filter(|line| line.to_ascii_lowercase().contains("error"))
        .count();
    let warning_count = diagnostics
        .iter()
        .filter(|line| line.to_ascii_lowercase().contains("warning"))
        .count();
    let mut lines = vec![format!(
        "binlog diagnostics: {error_count} errors, {warning_count} warnings"
    )];
    lines.extend(diagnostics.into_iter().take(MAX_DIAGNOSTICS));
    lines.join("\n")
}
// END_render_binlog_summary

// START_CONTRACT_render_format_report_summary
// PURPOSE: Summarize dotnet format JSON report entries
// INPUTS: { raw: &str }
// OUTPUTS: { anyhow::Result<String> }
// START_render_format_report_summary
fn render_format_report_summary(raw: &str) -> anyhow::Result<String> {
    let entries: Vec<FormatReportEntry> = serde_json::from_str(raw)?;
    let total = entries.len();
    let changed = entries
        .iter()
        .filter(|entry| !entry.file_changes.is_empty())
        .count();
    if changed == 0 {
        return Ok(format!("dotnet format: ok ({total} files checked)"));
    }

    let mut lines = vec![format!(
        "dotnet format: {changed}/{total} files need changes"
    )];
    for entry in entries
        .iter()
        .filter(|entry| !entry.file_changes.is_empty())
        .take(MAX_FORMAT_FILES)
    {
        lines.push(format!(
            "- {}: {} changes",
            entry.file_path,
            entry.file_changes.len()
        ));
        for change in entry.file_changes.iter().take(3) {
            lines.push(format!(
                "  {}:{} {} {}",
                change.line_number,
                change.char_number,
                change.diagnostic_id,
                truncate(&change.format_description, 100)
            ));
        }
    }
    Ok(lines.join("\n"))
}
// END_render_format_report_summary

// START_CONTRACT_render_trx_summary
// PURPOSE: Summarize TRX counters and failed tests from one or more concatenated TRX documents
// INPUTS: { raw: &str }
// OUTPUTS: { String }
// START_render_trx_summary
fn render_trx_summary(raw: &str) -> String {
    let summary = parse_trx_summary(raw);
    if summary.total == 0 && summary.failures.is_empty() {
        return "trx: no test results found".into();
    }

    let mut lines = vec![format!(
        "trx: total {} passed {} failed {} skipped {}",
        summary.total, summary.passed, summary.failed, summary.skipped
    )];
    for failure in summary.failures.iter().take(MAX_FAILED_TESTS) {
        lines.push(format!("- failed {}", failure.name));
        if let Some(message) = &failure.message {
            lines.push(format!("  {}", truncate(message, 180)));
        }
    }
    lines.join("\n")
}
// END_render_trx_summary

// START_CONTRACT_printable_text
// PURPOSE: Convert bytes into printable text runs suitable for diagnostic regex extraction
// INPUTS: { bytes: &[u8] }
// OUTPUTS: { String }
// START_printable_text
fn printable_text(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut current = String::new();
    for byte in bytes {
        let ch = *byte as char;
        if ch.is_ascii_graphic() || matches!(ch, ' ' | '\n' | '\r' | '\t') {
            current.push(ch);
        } else {
            flush_printable_run(&mut current, &mut out);
        }
    }
    flush_printable_run(&mut current, &mut out);
    out
}
// END_printable_text

// START_CONTRACT_flush_printable_run
// PURPOSE: Append meaningful printable runs while dropping binary noise fragments
// INPUTS: { current: &mut String }, { out: &mut String }
// OUTPUTS: { () }
// START_flush_printable_run
fn flush_printable_run(current: &mut String, out: &mut String) {
    if current.trim().chars().count() >= MIN_PRINTABLE_RUN_CHARS {
        out.push_str(current);
        out.push('\n');
    }
    current.clear();
}
// END_flush_printable_run

// START_CONTRACT_diagnostic_lines
// PURPOSE: Select MSBuild diagnostic lines from extracted text
// INPUTS: { text: &str }
// OUTPUTS: { Vec<String> }
// START_diagnostic_lines
fn diagnostic_lines(text: &str) -> Vec<String> {
    let Ok(issue_re) =
        Regex::new(r"(?i)(^|[:\s])(?P<kind>error|warning)\s+([A-Za-z]{2,}\d{3,})?\s*:?")
    else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && issue_re.is_match(line))
        .map(|line| truncate(line, 220))
        .collect()
}
// END_diagnostic_lines

// START_CONTRACT_read_trx_dir
// PURPOSE: Read and concatenate TRX files from a directory
// INPUTS: { path: &Path }
// OUTPUTS: { anyhow::Result<String> }
// START_read_trx_dir
fn read_trx_dir(path: &Path) -> anyhow::Result<String> {
    let mut raw = String::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("trx"))
        {
            raw.push_str(&std::fs::read_to_string(path)?);
            raw.push('\n');
        }
    }
    Ok(raw)
}
// END_read_trx_dir

// START_CONTRACT_parse_trx_summary
// PURPOSE: Extract counters and failure messages from TRX XML text using bounded regex parsing
// INPUTS: { raw: &str }
// OUTPUTS: { TrxSummary }
// START_parse_trx_summary
fn parse_trx_summary(raw: &str) -> TrxSummary {
    let mut summary = TrxSummary::default();
    let Ok(counters_re) = Regex::new(r#"<(?:\w+:)?Counters\b[^>]*>"#) else {
        return summary;
    };
    for mat in counters_re.find_iter(raw) {
        let tag = mat.as_str();
        summary.total += attr_u64(tag, "total");
        summary.passed += attr_u64(tag, "passed");
        summary.failed += attr_u64(tag, "failed");
        summary.skipped += attr_u64(tag, "notExecuted") + attr_u64(tag, "skipped");
    }

    let Ok(result_re) = Regex::new(
        r#"(?s)<(?:\w+:)?UnitTestResult\b(?P<attrs>[^>]*)>(?P<body>.*?)</(?:\w+:)?UnitTestResult>"#,
    ) else {
        return summary;
    };
    for captures in result_re.captures_iter(raw) {
        let attrs = captures.name("attrs").map(|m| m.as_str()).unwrap_or("");
        if attr_value(attrs, "outcome").as_deref() != Some("Failed") {
            continue;
        }
        let body = captures.name("body").map(|m| m.as_str()).unwrap_or("");
        let name = attr_value(attrs, "testName").unwrap_or_else(|| "unknown".into());
        let message = tag_text(body, "Message");
        summary.failures.push(TrxFailure { name, message });
    }
    summary
}
// END_parse_trx_summary

// START_CONTRACT_attr_u64
// PURPOSE: Parse an unsigned integer XML attribute from a tag
// INPUTS: { tag: &str }, { name: &str }
// OUTPUTS: { u64 }
// START_attr_u64
fn attr_u64(tag: &str, name: &str) -> u64 {
    attr_value(tag, name)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}
// END_attr_u64

// START_CONTRACT_attr_value
// PURPOSE: Extract a double-quoted XML attribute from a tag fragment
// INPUTS: { tag: &str }, { name: &str }
// OUTPUTS: { Option<String> }
// START_attr_value
fn attr_value(tag: &str, name: &str) -> Option<String> {
    let pattern = format!(r#"\b{}\s*=\s*"([^"]*)""#, regex::escape(name));
    let re = Regex::new(&pattern).ok()?;
    re.captures(tag)
        .and_then(|captures| captures.get(1))
        .map(|value| unescape_xml(value.as_str()))
}
// END_attr_value

// START_CONTRACT_tag_text
// PURPOSE: Extract simple XML tag text from a fragment
// INPUTS: { body: &str }, { tag: &str }
// OUTPUTS: { Option<String> }
// START_tag_text
fn tag_text(body: &str, tag: &str) -> Option<String> {
    let pattern = format!(
        r#"(?s)<(?:\w+:)?{}\b[^>]*>(.*?)</(?:\w+:)?{}>"#,
        regex::escape(tag),
        regex::escape(tag)
    );
    let re = Regex::new(&pattern).ok()?;
    re.captures(body)
        .and_then(|captures| captures.get(1))
        .map(|value| unescape_xml(value.as_str().trim()))
        .filter(|value| !value.is_empty())
}
// END_tag_text

// START_CONTRACT_unescape_xml
// PURPOSE: Decode common XML entities used in TRX attributes and messages
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_unescape_xml
fn unescape_xml(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}
// END_unescape_xml

// START_CONTRACT_truncate
// PURPOSE: Truncate text by character count for compact artifact summaries
// INPUTS: { value: &str }, { max: usize }
// OUTPUTS: { String }
// START_truncate
fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        value
            .chars()
            .take(max.saturating_sub(3))
            .collect::<String>()
            + "..."
    }
}
// END_truncate

// START_CONTRACT_record_artifact_savings
// PURPOSE: Persist local adapter token savings for artifact summarizers
// INPUTS: { config: &Config }, { command: &str }, { raw: &str }, { rendered: &str }, { adapter: &str }, { route_key: &str }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes tracking database when tracking is enabled
// START_record_artifact_savings
async fn record_artifact_savings(
    config: &Config,
    command: &str,
    raw: &str,
    rendered: &str,
    adapter: &str,
    route_key: &str,
) {
    super::rtk_adapters::record_adapter_savings(config, command, raw, rendered, adapter, route_key)
        .await;
}
// END_record_artifact_savings

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_report_summary_compacts_changed_files() {
        let raw = r#"[
          {
            "FilePath": "Program.cs",
            "FileChanges": [
              {
                "LineNumber": 42,
                "CharNumber": 9,
                "DiagnosticId": "IDE0055",
                "FormatDescription": "Fix formatting"
              }
            ]
          },
          { "FilePath": "Other.cs", "FileChanges": [] }
        ]"#;

        let rendered = render_format_report_summary(raw).expect("summary");

        assert!(rendered.contains("1/2 files need changes"));
        assert!(rendered.contains("Program.cs"));
        assert!(rendered.contains("IDE0055"));
    }

    #[test]
    fn trx_summary_extracts_counters_and_failures() {
        let raw = r#"
        <TestRun>
          <ResultSummary outcome="Failed">
            <Counters total="3" passed="2" failed="1" notExecuted="0" />
          </ResultSummary>
          <Results>
            <UnitTestResult testName="Tests.Fails" outcome="Failed">
              <Output><ErrorInfo><Message>Expected true but was false</Message></ErrorInfo></Output>
            </UnitTestResult>
          </Results>
        </TestRun>
        "#;

        let rendered = render_trx_summary(raw);

        assert!(rendered.contains("total 3 passed 2 failed 1 skipped 0"));
        assert!(rendered.contains("Tests.Fails"));
        assert!(rendered.contains("Expected true"));
    }

    #[test]
    fn binlog_summary_extracts_printable_diagnostics() {
        let raw =
            b"\0\0src/Program.cs(10,5): error CS1002: ; expected\n\0warning MSB1234: check config";

        let rendered = render_binlog_summary(raw);

        assert!(rendered.contains("1 errors, 1 warnings"));
        assert!(rendered.contains("CS1002"));
        assert!(rendered.contains("MSB1234"));
    }
}
