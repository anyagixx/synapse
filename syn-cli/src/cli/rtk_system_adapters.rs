// MODULE_CONTRACT
// MODULE_ID: M-CLI-RTK-COMMANDS
// PURPOSE: Local RTK-style system adapters for stdin filtering, log compaction, and source summaries
// SCOPE: PipeCmd, LogCmd, SmartCmd execution; bounded stdin/file reads; TOML filter reuse; log deduplication; source structure summaries; adapter-level token tracking
// DEPENDS: M-CONFIG, M-PROXY-FILTER, M-TRACKING, M-UTILS
// LINKS:
//   -> M-CLI (depends) - implements command argument schemas declared by the CLI facade
//   -> M-PROXY-FILTER (depends) - applies trusted Synapse and RTK TOML filters for pipe mode
//   -> M-TRACKING (depends) - records direct adapter token economy
//   -> Phase-46 (implements) - structured system adapter expansion
//   -> NFR-003 (traces_to) - direct adapters reduce context without shell subprocesses

// START_MODULE_MAP
// PipeCmd::run - Filter stdin through trusted TOML filters or auto-detected filter commands
// LogCmd::run - Deduplicate and summarize log output from a file or stdin
// SmartCmd::run - Summarize source file structure without dumping full code
// apply_pipe_filter - Resolve and apply one requested pipe filter
// auto_detect_pipe_filter_command - Infer a filter command from stdin shape
// analyze_logs - Count and group repeated error/warn/info log messages
// summarize_source - Extract compact source metrics, imports, and definitions
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added pipe, log, and smart local RTK system adapters]
// END_CHANGE_SUMMARY

use super::rtk_adapters::record_adapter_savings;
use super::{LogCmd, PipeCmd, SmartCmd};
use syn_core::config::Config;
use syn_proxy::proxy::toml_filter::FilterEngine;
use anyhow::Context;
use regex::Regex;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::Path;
use std::sync::OnceLock;

const SYSTEM_ADAPTER_RAW_CAP: usize = 2_000_000;
const LOG_ERROR_LIMIT: usize = 10;
const LOG_WARNING_LIMIT: usize = 5;
const SMART_IMPORT_LIMIT: usize = 8;
const PIPE_PATH_LIKE_MIN_LINES: usize = 3;
const TIMESTAMP_PATTERN: &str = r"^\d{4}[-/]\d{2}[-/]\d{2}[T ]\d{2}:\d{2}:\d{2}[.,]?\d*\s*";
const UUID_PATTERN: &str =
    r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}";
const HEX_PATTERN: &str = r"0x[0-9a-fA-F]+";
const LONG_NUM_PATTERN: &str = r"\b\d{4,}\b";
const PATH_PATTERN: &str = r"/[\w./\-]+";

static TIMESTAMP_RE: OnceLock<Option<Regex>> = OnceLock::new();
static UUID_RE: OnceLock<Option<Regex>> = OnceLock::new();
static HEX_RE: OnceLock<Option<Regex>> = OnceLock::new();
static LONG_NUM_RE: OnceLock<Option<Regex>> = OnceLock::new();
static PATH_RE: OnceLock<Option<Regex>> = OnceLock::new();

// START_public_api

impl PipeCmd {
    // START_CONTRACT_PipeCmd::run
    // PURPOSE: Read stdin and print filtered output through a requested or auto-detected Synapse RTK TOML filter
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads stdin, writes stdout, records adapter token economy
    // LINKS:
    //   -> M-PROXY-FILTER (depends) - reuses trusted TOML filter definitions
    //   -> M-TRACKING (depends) - records pipe-adapter savings
    // START_pipe_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        if self.passthrough {
            std::io::copy(&mut std::io::stdin(), &mut std::io::stdout())
                .context("relay stdin to stdout")?;
            return Ok(());
        }

        let raw = read_bounded_stdin("pipe stdin")?;
        let result = if let Some(filter) = self.filter.as_deref() {
            apply_pipe_filter(&raw, filter)?
        } else if let Some(command) = auto_detect_pipe_filter_command(&raw) {
            apply_pipe_filter_command(&raw, command)
                .unwrap_or_else(|| PipeFilterResult::passthrough(raw.clone(), "auto"))
        } else {
            PipeFilterResult::passthrough(raw.clone(), "identity")
        };

        print!("{}", result.output);
        let command = self
            .filter
            .as_deref()
            .map(|filter| format!("syn pipe --filter {filter}"))
            .unwrap_or_else(|| "syn pipe".into());
        record_adapter_savings(
            &config,
            &command,
            &raw,
            &result.output,
            "rtk-pipe",
            &format!("syn pipe {}", result.route_key),
        )
        .await;
        Ok(())
    }
    // END_pipe_cmd_run
}

impl LogCmd {
    // START_CONTRACT_LogCmd::run
    // PURPOSE: Read log text from a file or stdin and print a compact severity/deduplication report
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads file/stdin, writes stdout, records adapter token economy
    // LINKS:
    //   -> M-TRACKING (depends) - records log-adapter savings
    // START_log_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let raw = read_text_source(&self.source, "log input")?;
        let output = analyze_logs(&raw);
        println!("{output}");
        record_adapter_savings(
            &config,
            &format!("syn log {}", self.source),
            &raw,
            &output,
            "rtk-log",
            "syn log",
        )
        .await;
        Ok(())
    }
    // END_log_cmd_run
}

impl SmartCmd {
    // START_CONTRACT_SmartCmd::run
    // PURPOSE: Read one source file and print a compact structural summary for agent context
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads file, writes stdout, records adapter token economy
    // LINKS:
    //   -> M-TRACKING (depends) - records smart-adapter savings
    // START_smart_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let raw = read_text_source(&self.file, "source file")?;
        let output = summarize_source(Path::new(&self.file), &raw, self.items);
        println!("{output}");
        record_adapter_savings(
            &config,
            &format!("syn smart {}", self.file),
            &raw,
            &output,
            "rtk-smart",
            "syn smart",
        )
        .await;
        Ok(())
    }
    // END_smart_cmd_run
}

// END_public_api

// START_PipeFilterResult
struct PipeFilterResult {
    output: String,
    route_key: String,
}

impl PipeFilterResult {
    fn passthrough(output: String, route_key: &str) -> Self {
        Self {
            output,
            route_key: route_key.into(),
        }
    }
}
// END_PipeFilterResult

// START_CONTRACT_read_bounded_stdin
// PURPOSE: Read stdin into memory with a hard byte cap for local system adapters
// INPUTS: { label: &str }
// OUTPUTS: { anyhow::Result<String> }
// START_read_bounded_stdin
fn read_bounded_stdin(label: &str) -> anyhow::Result<String> {
    let mut raw = String::new();
    std::io::stdin()
        .lock()
        .take((SYSTEM_ADAPTER_RAW_CAP + 1) as u64)
        .read_to_string(&mut raw)
        .with_context(|| format!("read {label}"))?;
    ensure_within_cap(&raw, label)?;
    Ok(raw)
}
// END_read_bounded_stdin

// START_CONTRACT_read_text_source
// PURPOSE: Read text from stdin marker or a file path with the local adapter byte cap
// INPUTS: { source: &str }, { label: &str }
// OUTPUTS: { anyhow::Result<String> }
// START_read_text_source
fn read_text_source(source: &str, label: &str) -> anyhow::Result<String> {
    let raw = if source == "-" {
        read_bounded_stdin(label)?
    } else {
        std::fs::read_to_string(source).with_context(|| format!("read {source}"))?
    };
    ensure_within_cap(&raw, label)?;
    Ok(raw)
}
// END_read_text_source

// START_CONTRACT_ensure_within_cap
// PURPOSE: Reject oversized local adapter input before expensive processing
// INPUTS: { raw: &str }, { label: &str }
// OUTPUTS: { anyhow::Result<()> }
// START_ensure_within_cap
fn ensure_within_cap(raw: &str, label: &str) -> anyhow::Result<()> {
    if raw.len() > SYSTEM_ADAPTER_RAW_CAP {
        anyhow::bail!("{label} exceeds {SYSTEM_ADAPTER_RAW_CAP} byte local adapter limit");
    }
    Ok(())
}
// END_ensure_within_cap

// START_CONTRACT_apply_pipe_filter
// PURPOSE: Resolve an explicit RTK/Synapse filter alias and apply the matching TOML filter
// INPUTS: { input: &str }, { requested: &str }
// OUTPUTS: { anyhow::Result<PipeFilterResult> }
// START_apply_pipe_filter
fn apply_pipe_filter(input: &str, requested: &str) -> anyhow::Result<PipeFilterResult> {
    for command in pipe_filter_candidates(requested) {
        if let Some(result) = apply_pipe_filter_command(input, &command) {
            return Ok(result);
        }
    }
    anyhow::bail!("unknown pipe filter: {requested}");
}
// END_apply_pipe_filter

// START_CONTRACT_apply_pipe_filter_command
// PURPOSE: Apply the first TOML filter that matches a command surrogate
// INPUTS: { input: &str }, { command: &str }
// OUTPUTS: { Option<PipeFilterResult> }
// START_apply_pipe_filter_command
fn apply_pipe_filter_command(input: &str, command: &str) -> Option<PipeFilterResult> {
    let engine = FilterEngine::new();
    let filter = engine.find_filter(command)?;
    Some(PipeFilterResult {
        output: engine.apply(filter, input),
        route_key: command.into(),
    })
}
// END_apply_pipe_filter_command

// START_CONTRACT_pipe_filter_candidates
// PURPOSE: Expand RTK pipe filter names into command surrogates understood by FilterEngine
// INPUTS: { requested: &str }
// OUTPUTS: { Vec<String> }
// START_pipe_filter_candidates
fn pipe_filter_candidates(requested: &str) -> Vec<String> {
    let requested = requested.trim();
    let mut candidates = vec![requested.to_string()];
    if let Some(mapped) = explicit_pipe_filter_command(requested) {
        candidates.push(mapped.into());
    }
    let dashed = requested.replace('-', " ");
    if dashed != requested {
        candidates.push(dashed);
    }
    candidates.sort();
    candidates.dedup();
    candidates
}
// END_pipe_filter_candidates

// START_CONTRACT_explicit_pipe_filter_command
// PURPOSE: Preserve RTK pipe aliases whose filter names do not map by dash replacement alone
// INPUTS: { requested: &str }
// OUTPUTS: { Option<&'static str> }
// START_explicit_pipe_filter_command
fn explicit_pipe_filter_command(requested: &str) -> Option<&'static str> {
    match requested {
        "cargo" | "cargo-test" => Some("cargo test"),
        "go-test" => Some("go test"),
        "go-build" => Some("go build"),
        "git-log" => Some("git log"),
        "git-diff" => Some("git diff"),
        "git-status" => Some("git status"),
        "ruff-check" => Some("ruff check"),
        "ruff-format" => Some("ruff format"),
        _ => None,
    }
}
// END_explicit_pipe_filter_command

// START_CONTRACT_auto_detect_pipe_filter_command
// PURPOSE: Infer a conservative filter command from stdin shape for no-arg pipe mode
// INPUTS: { input: &str }
// OUTPUTS: { Option<&'static str> }
// START_auto_detect_pipe_filter_command
fn auto_detect_pipe_filter_command(input: &str) -> Option<&'static str> {
    let first = input.get(..input.len().min(4096)).unwrap_or(input);
    if first.contains("test result:") && first.contains("passed;") {
        return Some("cargo test");
    }
    if first.contains("=== test session starts") {
        return Some("pytest");
    }
    if first.contains("Build succeeded.") && first.contains("Warning(s)") {
        return Some("dotnet build");
    }
    if first.lines().take(8).any(is_file_line_match) {
        return Some("rg");
    }
    let nonempty = first
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if nonempty.len() >= PIPE_PATH_LIKE_MIN_LINES
        && nonempty.iter().all(|line| looks_like_path(line))
    {
        return Some("find");
    }
    None
}
// END_auto_detect_pipe_filter_command

// START_CONTRACT_is_file_line_match
// PURPOSE: Detect grep/rg style file:line:content records
// INPUTS: { line: &str }
// OUTPUTS: { bool }
// START_is_file_line_match
fn is_file_line_match(line: &str) -> bool {
    let mut parts = line.splitn(3, ':');
    parts.next().is_some()
        && parts
            .next()
            .is_some_and(|line_no| line_no.parse::<usize>().is_ok())
        && parts.next().is_some()
}
// END_is_file_line_match

// START_CONTRACT_looks_like_path
// PURPOSE: Detect simple path-like lines for find/fd output compaction
// INPUTS: { line: &str }
// OUTPUTS: { bool }
// START_looks_like_path
fn looks_like_path(line: &str) -> bool {
    !line.contains(':') && (line.starts_with('.') || line.starts_with('/') || line.contains('/'))
}
// END_looks_like_path

// START_LogEntry
#[derive(Debug, Clone)]
struct LogEntry {
    count: usize,
    sample: String,
}
// END_LogEntry

// START_CONTRACT_analyze_logs
// PURPOSE: Build a compact severity summary with normalized duplicate grouping
// INPUTS: { content: &str }
// OUTPUTS: { String }
// START_analyze_logs
fn analyze_logs(content: &str) -> String {
    let mut errors = BTreeMap::<String, LogEntry>::new();
    let mut warnings = BTreeMap::<String, LogEntry>::new();
    let mut info_count = 0usize;

    for line in content.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("error") || lower.contains("fatal") || lower.contains("panic") {
            push_log_entry(&mut errors, line);
        } else if lower.contains("warn") {
            push_log_entry(&mut warnings, line);
        } else if lower.contains("info") {
            info_count += 1;
        }
    }

    let error_total = total_log_count(&errors);
    let warning_total = total_log_count(&warnings);
    let mut output = String::new();
    let _ = writeln!(output, "Log Summary");
    let _ = writeln!(output, "  errors: {error_total} unique: {}", errors.len());
    let _ = writeln!(
        output,
        "  warnings: {warning_total} unique: {}",
        warnings.len()
    );
    let _ = writeln!(output, "  info: {info_count}");
    append_log_group(&mut output, "ERRORS", &errors, LOG_ERROR_LIMIT);
    append_log_group(&mut output, "WARNINGS", &warnings, LOG_WARNING_LIMIT);
    output.trim_end().to_string()
}
// END_analyze_logs

// START_CONTRACT_push_log_entry
// PURPOSE: Add one log line to a normalized count map
// INPUTS: { entries: &mut BTreeMap<String, LogEntry> }, { line: &str }
// OUTPUTS: { () }
// START_push_log_entry
fn push_log_entry(entries: &mut BTreeMap<String, LogEntry>, line: &str) {
    let key = normalize_log_line(line);
    entries
        .entry(key)
        .and_modify(|entry| entry.count += 1)
        .or_insert_with(|| LogEntry {
            count: 1,
            sample: truncate_chars(line.trim(), 120),
        });
}
// END_push_log_entry

// START_CONTRACT_total_log_count
// PURPOSE: Sum grouped log counts
// INPUTS: { entries: &BTreeMap<String, LogEntry> }
// OUTPUTS: { usize }
// START_total_log_count
fn total_log_count(entries: &BTreeMap<String, LogEntry>) -> usize {
    entries.values().map(|entry| entry.count).sum()
}
// END_total_log_count

// START_CONTRACT_append_log_group
// PURPOSE: Append a bounded sorted log group to output
// INPUTS: { output: &mut String }, { title: &str }, { entries: &BTreeMap<String, LogEntry> }, { limit: usize }
// OUTPUTS: { () }
// START_append_log_group
fn append_log_group(
    output: &mut String,
    title: &str,
    entries: &BTreeMap<String, LogEntry>,
    limit: usize,
) {
    if entries.is_empty() {
        return;
    }
    let _ = writeln!(output);
    let _ = writeln!(output, "{title}:");
    let mut rows = entries.values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then(left.sample.cmp(&right.sample))
    });
    for row in rows.iter().take(limit) {
        if row.count > 1 {
            let _ = writeln!(output, "  [x{}] {}", row.count, row.sample);
        } else {
            let _ = writeln!(output, "  {}", row.sample);
        }
    }
    if rows.len() > limit {
        let _ = writeln!(output, "  ... +{} more unique", rows.len() - limit);
    }
}
// END_append_log_group

// START_CONTRACT_normalize_log_line
// PURPOSE: Normalize volatile log fragments for duplicate grouping
// INPUTS: { line: &str }
// OUTPUTS: { String }
// START_normalize_log_line
fn normalize_log_line(line: &str) -> String {
    let mut normalized = apply_optional_regex(timestamp_re(), line, "");
    normalized = apply_optional_regex(uuid_re(), &normalized, "<UUID>");
    normalized = apply_optional_regex(hex_re(), &normalized, "<HEX>");
    normalized = apply_optional_regex(long_num_re(), &normalized, "<NUM>");
    normalized = apply_optional_regex(path_re(), &normalized, "<PATH>");
    normalized.trim().to_string()
}
// END_normalize_log_line

fn timestamp_re() -> Option<&'static Regex> {
    TIMESTAMP_RE
        .get_or_init(|| Regex::new(TIMESTAMP_PATTERN).ok())
        .as_ref()
}

fn uuid_re() -> Option<&'static Regex> {
    UUID_RE
        .get_or_init(|| Regex::new(UUID_PATTERN).ok())
        .as_ref()
}

fn hex_re() -> Option<&'static Regex> {
    HEX_RE.get_or_init(|| Regex::new(HEX_PATTERN).ok()).as_ref()
}

fn long_num_re() -> Option<&'static Regex> {
    LONG_NUM_RE
        .get_or_init(|| Regex::new(LONG_NUM_PATTERN).ok())
        .as_ref()
}

fn path_re() -> Option<&'static Regex> {
    PATH_RE
        .get_or_init(|| Regex::new(PATH_PATTERN).ok())
        .as_ref()
}

fn apply_optional_regex(regex: Option<&Regex>, input: &str, replacement: &str) -> String {
    regex
        .map(|regex| regex.replace_all(input, replacement).to_string())
        .unwrap_or_else(|| input.to_string())
}

// START_SourceSummary
struct SourceSummary {
    language: &'static str,
    total_lines: usize,
    nonblank_lines: usize,
    imports: Vec<String>,
    definitions: Vec<String>,
}
// END_SourceSummary

// START_CONTRACT_summarize_source
// PURPOSE: Render compact source structure metrics and bounded symbol lists
// INPUTS: { path: &Path }, { content: &str }, { item_limit: usize }
// OUTPUTS: { String }
// START_summarize_source
fn summarize_source(path: &Path, content: &str, item_limit: usize) -> String {
    let summary = collect_source_summary(path, content, item_limit);
    let mut output = String::new();
    let display = path.display();
    let _ = writeln!(output, "Source Summary: {display}");
    let _ = writeln!(output, "  language: {}", summary.language);
    let _ = writeln!(
        output,
        "  lines: {} nonblank: {}",
        summary.total_lines, summary.nonblank_lines
    );
    append_string_list(&mut output, "definitions", &summary.definitions, item_limit);
    append_string_list(&mut output, "imports", &summary.imports, SMART_IMPORT_LIMIT);
    output.trim_end().to_string()
}
// END_summarize_source

// START_CONTRACT_collect_source_summary
// PURPOSE: Extract language, line counts, imports, and definitions from source text
// INPUTS: { path: &Path }, { content: &str }, { item_limit: usize }
// OUTPUTS: { SourceSummary }
// START_collect_source_summary
fn collect_source_summary(path: &Path, content: &str, item_limit: usize) -> SourceSummary {
    let language = detect_source_language(path);
    let total_lines = content.lines().count();
    let nonblank_lines = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    let mut imports = Vec::new();
    let mut definitions = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if imports.len() < SMART_IMPORT_LIMIT && is_import_line(trimmed, language) {
            imports.push(truncate_chars(trimmed, 100));
        }
        if definitions.len() < item_limit.clamp(1, 100) {
            if let Some(definition) = extract_definition(trimmed, language) {
                definitions.push(definition);
            }
        }
    }
    SourceSummary {
        language,
        total_lines,
        nonblank_lines,
        imports,
        definitions,
    }
}
// END_collect_source_summary

// START_CONTRACT_detect_source_language
// PURPOSE: Infer source language from file extension
// INPUTS: { path: &Path }
// OUTPUTS: { &'static str }
// START_detect_source_language
fn detect_source_language(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "rs" => "Rust",
        "py" => "Python",
        "ts" | "tsx" => "TypeScript",
        "js" | "jsx" => "JavaScript",
        "go" => "Go",
        "java" => "Java",
        "cs" => "C#",
        "rb" => "Ruby",
        "toml" => "TOML",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        _ => "text",
    }
}
// END_detect_source_language

// START_CONTRACT_is_import_line
// PURPOSE: Identify source import/use lines for compact context
// INPUTS: { line: &str }, { language: &str }
// OUTPUTS: { bool }
// START_is_import_line
fn is_import_line(line: &str, language: &str) -> bool {
    match language {
        "Rust" => line.starts_with("use "),
        "Python" => line.starts_with("import ") || line.starts_with("from "),
        "TypeScript" | "JavaScript" => line.starts_with("import "),
        "Go" => line.starts_with("import "),
        "Java" => line.starts_with("import "),
        "C#" => line.starts_with("using "),
        "Ruby" => line.starts_with("require "),
        _ => false,
    }
}
// END_is_import_line

// START_CONTRACT_extract_definition
// PURPOSE: Extract a compact definition label from a source line
// INPUTS: { line: &str }, { language: &str }
// OUTPUTS: { Option<String> }
// START_extract_definition
fn extract_definition(line: &str, language: &str) -> Option<String> {
    match language {
        "Rust" => extract_rust_definition(line),
        "Python" => extract_after_keywords(line, &["def ", "class "]),
        "TypeScript" | "JavaScript" => extract_after_keywords(
            line,
            &["export function ", "function ", "export class ", "class "],
        ),
        "Go" => extract_after_keywords(line, &["func ", "type "]),
        "Java" | "C#" => extract_after_keywords(line, &["class ", "interface ", "enum "]),
        "Ruby" => extract_after_keywords(line, &["def ", "class ", "module "]),
        _ => None,
    }
}
// END_extract_definition

// START_CONTRACT_extract_rust_definition
// PURPOSE: Extract Rust fn/type/module definition labels
// INPUTS: { line: &str }
// OUTPUTS: { Option<String> }
// START_extract_rust_definition
fn extract_rust_definition(line: &str) -> Option<String> {
    let line = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub(super) "))
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line);
    extract_after_keywords(
        line,
        &["async fn ", "fn ", "struct ", "enum ", "trait ", "mod "],
    )
}
// END_extract_rust_definition

// START_CONTRACT_extract_after_keywords
// PURPOSE: Extract the first identifier-like token following any matching keyword
// INPUTS: { line: &str }, { keywords: &[&str] }
// OUTPUTS: { Option<String> }
// START_extract_after_keywords
fn extract_after_keywords(line: &str, keywords: &[&str]) -> Option<String> {
    for keyword in keywords {
        if let Some(rest) = line.strip_prefix(keyword) {
            let name = rest
                .trim_start()
                .split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
                .next()
                .unwrap_or("");
            if !name.is_empty() {
                return Some(format!("{} {name}", keyword.trim()));
            }
        }
    }
    None
}
// END_extract_after_keywords

// START_CONTRACT_append_string_list
// PURPOSE: Append a bounded list section to source summary output
// INPUTS: { output: &mut String }, { label: &str }, { items: &[String] }, { limit: usize }
// OUTPUTS: { () }
// START_append_string_list
fn append_string_list(output: &mut String, label: &str, items: &[String], limit: usize) {
    let _ = writeln!(output, "  {label}: {}", items.len());
    for item in items.iter().take(limit) {
        let _ = writeln!(output, "    {item}");
    }
    if items.len() > limit {
        let _ = writeln!(output, "    ... +{} more", items.len() - limit);
    }
}
// END_append_string_list

// START_CONTRACT_truncate_chars
// PURPOSE: Truncate a string by Unicode scalar count and append an ASCII omission marker
// INPUTS: { value: &str }, { max_chars: usize }
// OUTPUTS: { String }
// START_truncate_chars
fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.into();
    }
    let mut output = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    output.push_str("...");
    output
}
// END_truncate_chars

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipe_filter_applies_toml_filter_by_name() {
        let input = "make[1]: Entering directory '/tmp'\nmake[1]: Leaving directory '/tmp'\n";
        let result = apply_pipe_filter(input, "make").unwrap();

        assert_eq!(result.output, "make: ok");
        assert_eq!(result.route_key, "make");
    }

    #[test]
    fn pipe_filter_maps_dash_alias_to_command() {
        let input =
            "Microsoft (R) Build Engine\nBuild succeeded.\n    0 Warning(s)\n    0 Error(s)\n";
        let result = apply_pipe_filter(input, "dotnet-build").unwrap();

        assert_eq!(result.output, "ok (build succeeded)");
        assert_eq!(result.route_key, "dotnet build");
    }

    #[test]
    fn pipe_auto_detects_cargo_test_output() {
        assert_eq!(
            auto_detect_pipe_filter_command("running 1 test\ntest result: ok. 1 passed; 0 failed"),
            Some("cargo test")
        );
    }

    #[test]
    fn log_analysis_deduplicates_volatile_lines() {
        let logs = "\
2026-01-01 10:00:00 ERROR failed /tmp/a/one id=12345\n\
2026-01-01 10:00:01 ERROR failed /tmp/b/two id=67890\n\
2026-01-01 10:00:02 WARN retry 0xabc\n\
2026-01-01 10:00:03 INFO connected\n";
        let output = analyze_logs(logs);

        assert!(output.contains("errors: 2 unique: 1"), "{output}");
        assert!(output.contains("[x2]"), "{output}");
        assert!(output.contains("warnings: 1 unique: 1"), "{output}");
    }

    #[test]
    fn smart_summary_extracts_rust_definitions_without_body() {
        let source = "\
use std::fs;\n\
pub struct Config { value: String }\n\
pub async fn run() -> anyhow::Result<()> { Ok(()) }\n\
fn helper() {}\n";
        let output = summarize_source(Path::new("src/lib.rs"), source, 12);

        assert!(output.contains("language: Rust"), "{output}");
        assert!(output.contains("struct Config"), "{output}");
        assert!(output.contains("async fn run"), "{output}");
        assert!(output.contains("use std::fs;"), "{output}");
        assert!(!output.contains("value: String"), "{output}");
    }
}
