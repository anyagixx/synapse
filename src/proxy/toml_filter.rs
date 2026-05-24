// MODULE_CONTRACT
// MODULE_ID: M-PROXY-FILTER
// PURPOSE: TOML filter engine — applies regex-based output transformations from trusted TOML filter definitions
// SCOPE: FilterDef, ReplaceRule, MatchOutputRule, FilterFile, FilterEngine with find_filter/apply/verify, project-local trust gating, expanded built-in filter catalogue, 8-stage pipeline
// DEPENDS: M-UTILS
// LINKS:
//   → M-UTILS (depends) - Unicode-safe truncation helpers
//   → Phase-24 (implements) - RTK filter trust and verification parity
//   ← V-M-PROXY-FILTER (verified_by) - filter schema and trust verification

// START_MODULE_MAP
// FilterDef — TOML filter definition with match, replace, strip, truncate rules
// MatchOutputRule — RTK-style short-circuit rule with optional unless guard
// FilterTestDef — Inline test case loaded from TOML tests.<filter-name>
// FilterFile — Backwards-compatible TOML file containing legacy or named filter definitions
// FilterSource — Source of a filter (BuiltIn, User, Project)
// FilterEngine — Loads, applies, and verifies TOML output filters
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v4.0.0 — Added RTK-style schema, inline verification, and trusted project filters]
// END_CHANGE_SUMMARY

use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

static ANSI_RE: OnceLock<Result<Regex, String>> = OnceLock::new();
const ANSI_PATTERN: &str = "\x1b\\[[0-9;]*m";
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

// START_public_api

// START_MatchOutputRule
#[derive(serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MatchOutputRule {
    pub pattern: String,
    pub message: String,
    #[serde(default)]
    pub unless: Option<String>,
}
// END_MatchOutputRule

// START_MatchOutputSpec
#[derive(serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum MatchOutputSpec {
    Legacy(String),
    Rules(Vec<MatchOutputRule>),
}
// END_MatchOutputSpec

// START_FilterDef
#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct FilterDef {
    #[serde(skip)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub match_command: Option<String>,
    #[serde(default)]
    pub match_regex: Option<String>,
    #[serde(default)]
    pub strip_ansi: Option<bool>,
    #[serde(default)]
    pub replace: Vec<ReplaceRule>,
    #[serde(default)]
    pub match_output: Option<MatchOutputSpec>,
    #[serde(default)]
    pub strip_lines: Option<Vec<String>>,
    #[serde(default)]
    pub keep_lines: Option<Vec<String>>,
    #[serde(default)]
    pub strip_lines_matching: Option<Vec<String>>,
    #[serde(default)]
    pub keep_lines_matching: Option<Vec<String>>,
    #[serde(default)]
    pub truncate_lines_at: Option<usize>,
    #[serde(default)]
    pub head_lines: Option<usize>,
    #[serde(default)]
    pub tail_lines: Option<usize>,
    #[serde(default)]
    pub max_lines: Option<usize>,
    #[serde(default)]
    pub on_empty: Option<String>,
    #[serde(default)]
    pub filter_stderr: Option<bool>,
}
// END_FilterDef

// START_ReplaceRule
#[derive(serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReplaceRule {
    pub pattern: String,
    pub replacement: String,
}
// END_ReplaceRule

// START_FilterTestDef
#[derive(serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FilterTestDef {
    pub name: String,
    pub input: String,
    pub expected: String,
}
// END_FilterTestDef

// START_FilterCollection
#[derive(serde::Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum FilterCollection {
    Legacy(Vec<FilterDef>),
    Named(BTreeMap<String, FilterDef>),
}

impl Default for FilterCollection {
    fn default() -> Self {
        Self::Legacy(Vec::new())
    }
}
// END_FilterCollection

// START_FilterFile
#[derive(serde::Deserialize, Clone, Debug, Default)]
pub struct FilterFile {
    #[serde(default)]
    pub schema_version: Option<u32>,
    #[serde(default)]
    pub filters: FilterCollection,
    #[serde(default)]
    pub tests: BTreeMap<String, Vec<FilterTestDef>>,
}
// END_FilterFile

// START_FilterSource
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterSource {
    BuiltIn,
    User,
    Project,
}
// END_FilterSource

// START_FilterTestOutcome
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterTestOutcome {
    pub filter_name: String,
    pub test_name: String,
    pub passed: bool,
    pub actual: String,
    pub expected: String,
}
// END_FilterTestOutcome

// START_FilterVerifyResults
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilterVerifyResults {
    pub outcomes: Vec<FilterTestOutcome>,
    pub filters_without_tests: Vec<String>,
    pub warnings: Vec<String>,
}

impl FilterVerifyResults {
    // START_CONTRACT_FilterVerifyResults::passed
    // PURPOSE: Report whether inline filter verification passed under the requested strictness
    // INPUTS: { require_all: bool }
    // OUTPUTS: { bool }
    // START_filter_verify_passed
    pub fn passed(&self, require_all: bool) -> bool {
        self.outcomes.iter().all(|outcome| outcome.passed)
            && (!require_all || self.filters_without_tests.is_empty())
    }
    // END_filter_verify_passed
}
// END_FilterVerifyResults

// START_FilterEngine
pub struct FilterEngine {
    filters: Vec<(FilterDef, FilterSource)>,
    tests: BTreeMap<String, Vec<FilterTestDef>>,
    warnings: Vec<String>,
}
// END_FilterEngine

impl Default for FilterEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterEngine {
    // START_CONTRACT_FilterEngine::new
    // PURPOSE: Create a new FilterEngine, loading project, user, and built-in filters with RTK-style project trust gating
    // OUTPUTS: { Self }
    // SIDE_EFFECTS: reads TOML filter files and trust metadata from disk
    // LINKS:
    //   → Phase-24 (implements) - trusted project filters and schema verification
    // START_fe_new
    pub fn new() -> Self {
        if env_enabled("SYNAPSE_NO_TOML") {
            return Self {
                filters: Vec::new(),
                tests: BTreeMap::new(),
                warnings: Vec::new(),
            };
        }

        let mut engine = Self {
            filters: Vec::new(),
            tests: BTreeMap::new(),
            warnings: Vec::new(),
        };

        engine.load_project_filters();
        engine.load_user_filters();
        engine.load_builtin_filters();
        engine
    }
    // END_fe_new

    // START_CONTRACT_FilterEngine::from_toml
    // PURPOSE: Build an isolated filter engine from a TOML string for verification and tests
    // INPUTS: { content: &str }, { source_label: &str }
    // OUTPUTS: { Result<Self, String> }
    // START_fe_from_toml
    pub fn from_toml(content: &str, source_label: &str) -> Result<Self, String> {
        let parsed = parse_filter_file(content, source_label)?;
        Ok(Self {
            filters: parsed
                .filters
                .into_iter()
                .map(|filter| (filter, FilterSource::User))
                .collect(),
            tests: parsed.tests,
            warnings: parsed.warnings,
        })
    }
    // END_fe_from_toml

    // START_CONTRACT_FilterEngine::warnings
    // PURPOSE: Return non-fatal load and validation warnings
    // OUTPUTS: { &[String] }
    // START_fe_warnings
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
    // END_fe_warnings

    // START_CONTRACT_FilterEngine::find_filter
    // PURPOSE: Find a matching filter for a given command string
    // INPUTS: { cmd: &str - command to match }
    // OUTPUTS: { Option<&FilterDef> }
    // START_fe_find_filter
    pub fn find_filter(&self, cmd: &str) -> Option<&FilterDef> {
        let cmd = cmd.trim();
        let re = Regex::new(r"^(\w+|-+)+").ok()?;
        let cmd_base = re.find(cmd).map(|m| m.as_str()).unwrap_or(cmd);

        for (filter, _source) in &self.filters {
            if filter_matches_command(filter, cmd, cmd_base) {
                if env_enabled("SYNAPSE_TOML_DEBUG") {
                    tracing::debug!(
                        "[ProxyFilter][find_filter][MATCH] filter={} command={}",
                        filter_display_name(filter),
                        cmd
                    );
                }
                return Some(filter);
            }
        }
        None
    }
    // END_fe_find_filter

    // START_CONTRACT_FilterEngine::apply
    // PURPOSE: Apply a filter definition to command output via 8-stage pipeline
    // INPUTS: { filter: &FilterDef }, { output: &str - raw command output }
    // OUTPUTS: { String - filtered output }
    // START_fe_apply
    pub fn apply(&self, filter: &FilterDef, output: &str) -> String {
        apply_filter(filter, output)
    }
    // END_fe_apply

    // START_CONTRACT_FilterEngine::verify
    // PURPOSE: Run inline TOML filter tests loaded from tests.<filter-name>
    // INPUTS: { filter_name: Option<&str> }, { require_all: bool }
    // OUTPUTS: { Result<FilterVerifyResults, String> }
    // START_fe_verify
    pub fn verify(
        &self,
        filter_name: Option<&str>,
        require_all: bool,
    ) -> Result<FilterVerifyResults, String> {
        let mut results = FilterVerifyResults {
            warnings: self.warnings.clone(),
            ..Default::default()
        };
        let mut selected = Vec::new();

        for (filter, _source) in &self.filters {
            let name = filter_display_name(filter);
            if filter_name.is_none_or(|wanted| wanted == name) {
                selected.push((name, filter));
            }
        }

        if let Some(wanted) = filter_name {
            if selected.is_empty() {
                return Err(format!("filter not found: {wanted}"));
            }
        }

        for (name, filter) in selected {
            let tests = self.tests.get(&name);
            if tests.is_none_or(Vec::is_empty) {
                if require_all {
                    results.filters_without_tests.push(name);
                }
                continue;
            }
            let Some(tests) = tests else {
                continue;
            };
            for test in tests {
                let actual = self.apply(filter, &test.input);
                results.outcomes.push(FilterTestOutcome {
                    filter_name: name.clone(),
                    test_name: test.name.clone(),
                    passed: actual == test.expected,
                    actual,
                    expected: test.expected.clone(),
                });
            }
        }

        Ok(results)
    }
    // END_fe_verify

    fn load_project_filters(&mut self) {
        let project_path = Path::new(".synapse").join("filters.toml");
        if !project_path.exists() {
            return;
        }

        match super::filter_trust::check_trust(&project_path) {
            Ok(super::filter_trust::TrustStatus::Trusted)
            | Ok(super::filter_trust::TrustStatus::EnvOverride) => {
                self.extend_from_file(&project_path, FilterSource::Project, "project");
            }
            Ok(status) => self.warnings.push(format!(
                "project filters skipped: {} ({})",
                status.label(),
                project_path.display()
            )),
            Err(err) => self.warnings.push(format!(
                "project filters skipped: trust check failed for {}: {}",
                project_path.display(),
                err
            )),
        }
    }

    fn load_user_filters(&mut self) {
        if let Some(dir) = dirs::config_dir() {
            let path = dir.join("synapse").join("filters.toml");
            self.extend_from_file(&path, FilterSource::User, "user");
        }
    }

    fn load_builtin_filters(&mut self) {
        let parsed = parse_filter_file(include_str!("builtin_filters.toml"), "builtin");
        self.extend_from_parsed(parsed, FilterSource::BuiltIn);
    }

    fn extend_from_file(&mut self, path: &Path, source: FilterSource, source_label: &str) {
        let Ok(content) = std::fs::read_to_string(path) else {
            return;
        };
        let parsed = parse_filter_file(&content, source_label);
        self.extend_from_parsed(parsed, source);
    }

    fn extend_from_parsed(
        &mut self,
        parsed: Result<ParsedFilterFile, String>,
        source: FilterSource,
    ) {
        match parsed {
            Ok(parsed) => {
                for filter in parsed.filters {
                    self.filters.push((filter, source));
                }
                merge_tests(&mut self.tests, parsed.tests);
                self.warnings.extend(parsed.warnings);
            }
            Err(err) => self.warnings.push(err),
        }
    }

    fn remove_ansi(s: &str) -> String {
        match ANSI_RE.get_or_init(|| Regex::new(ANSI_PATTERN).map_err(|e| e.to_string())) {
            Ok(re) => re.replace_all(s, "").to_string(),
            Err(_) => s.to_string(),
        }
    }
}

// START_CONTRACT_apply_filter
// PURPOSE: Apply the TOML filter pipeline as a pure transformation for command output and inline tests
// INPUTS: { filter: &FilterDef }, { output: &str }
// OUTPUTS: { String }
// START_apply_filter
fn apply_filter(filter: &FilterDef, output: &str) -> String {
    let mut result = output.to_string();

    if filter.strip_ansi.unwrap_or(false) {
        result = FilterEngine::remove_ansi(&result);
    }

    if !filter.replace.is_empty() {
        result = result
            .lines()
            .map(|line| {
                let mut line = line.to_string();
                for rule in &filter.replace {
                    if let Ok(re) = Regex::new(&rule.pattern) {
                        line = re.replace_all(&line, &rule.replacement[..]).to_string();
                    }
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    if let Some(message) = match_output_message(filter, &result) {
        return message;
    }

    let lines: Vec<&str> = result.lines().collect();
    let keep_lines = merged_patterns(&filter.keep_lines, &filter.keep_lines_matching);
    let strip_lines = merged_patterns(&filter.strip_lines, &filter.strip_lines_matching);
    let filtered: Vec<&str> = if !keep_lines.is_empty() {
        lines
            .iter()
            .filter(|line| matches_any(line, &keep_lines))
            .copied()
            .collect()
    } else if !strip_lines.is_empty() {
        lines
            .iter()
            .filter(|line| !matches_any(line, &strip_lines))
            .copied()
            .collect()
    } else {
        lines
    };

    let truncated: Vec<String> = if let Some(max_len) = filter.truncate_lines_at {
        filtered
            .iter()
            .map(|line| {
                if line.chars().count() > max_len {
                    crate::utils::truncate_chars(line, max_len)
                } else {
                    line.to_string()
                }
            })
            .collect()
    } else {
        filtered.iter().map(|line| line.to_string()).collect()
    };

    let sliced: Vec<&str> = match (filter.head_lines, filter.tail_lines) {
        (Some(h), Some(t)) => {
            let mut v: Vec<&str> = Vec::new();
            let total = truncated.len();
            for item in truncated.iter().take(h.min(total)) {
                v.push(item);
            }
            if h + t < total {
                v.push("...");
            }
            for item in truncated.iter().skip(total.saturating_sub(t)) {
                v.push(item);
            }
            v
        }
        (Some(h), None) => truncated.iter().take(h).map(|s| s.as_str()).collect(),
        (None, Some(t)) => {
            let total = truncated.len();
            truncated
                .iter()
                .skip(total.saturating_sub(t))
                .map(|s| s.as_str())
                .collect()
        }
        (None, None) => truncated.iter().map(|s| s.as_str()).collect(),
    };

    let final_lines: Vec<&str> = if let Some(max) = filter.max_lines {
        let mut v: Vec<&str> = sliced.iter().take(max).copied().collect();
        if sliced.len() > max {
            v.push("...");
        }
        v
    } else {
        sliced
    };

    if final_lines.is_empty() || final_lines.iter().all(|line| line.trim().is_empty()) {
        return filter.on_empty.clone().unwrap_or_default();
    }

    final_lines.join("\n")
}
// END_apply_filter

// END_public_api

struct ParsedFilterFile {
    filters: Vec<FilterDef>,
    tests: BTreeMap<String, Vec<FilterTestDef>>,
    warnings: Vec<String>,
}

// START_CONTRACT_parse_filter_file
// PURPOSE: Parse legacy or RTK-style TOML filters while preserving valid filters when siblings fail validation
// INPUTS: { content: &str }, { source_label: &str }
// OUTPUTS: { Result<ParsedFilterFile, String> }
// START_parse_filter_file
fn parse_filter_file(content: &str, source_label: &str) -> Result<ParsedFilterFile, String> {
    let file: FilterFile = toml::from_str(content)
        .map_err(|err| format!("TOML parse error in {source_label} filters: {err}"))?;

    if let Some(version) = file.schema_version {
        if version != SUPPORTED_SCHEMA_VERSION {
            return Err(format!(
                "unsupported schema_version {version} in {source_label} filters (expected {SUPPORTED_SCHEMA_VERSION})"
            ));
        }
    }

    let raw_filters = filters_with_names(file.filters, source_label);
    let mut filters = Vec::new();
    let mut warnings = Vec::new();
    for filter in raw_filters {
        match validate_filter(&filter) {
            Ok(()) => filters.push(filter),
            Err(err) => {
                let name = filter_display_name(&filter);
                warnings.push(format!("filter '{name}' in {source_label} skipped: {err}"));
            }
        }
    }

    Ok(ParsedFilterFile {
        filters,
        tests: file.tests,
        warnings,
    })
}
// END_parse_filter_file

fn filters_with_names(collection: FilterCollection, source_label: &str) -> Vec<FilterDef> {
    match collection {
        FilterCollection::Legacy(filters) => filters
            .into_iter()
            .enumerate()
            .map(|(index, mut filter)| {
                if filter.name.is_none() {
                    filter.name = Some(
                        filter
                            .match_command
                            .clone()
                            .or_else(|| filter.match_regex.clone())
                            .unwrap_or_else(|| format!("{source_label}-{index}")),
                    );
                }
                filter
            })
            .collect(),
        FilterCollection::Named(filters) => filters
            .into_iter()
            .map(|(name, mut filter)| {
                filter.name = Some(name);
                filter
            })
            .collect(),
    }
}

fn validate_filter(filter: &FilterDef) -> Result<(), String> {
    if filter.match_command.is_none() && filter.match_regex.is_none() {
        return Err("missing match_command or match_regex".into());
    }
    if let Some(pattern) = &filter.match_command {
        Regex::new(pattern).map_err(|err| format!("invalid match_command regex: {err}"))?;
    }
    if let Some(pattern) = &filter.match_regex {
        Regex::new(pattern).map_err(|err| format!("invalid match_regex regex: {err}"))?;
    }
    for rule in &filter.replace {
        Regex::new(&rule.pattern)
            .map_err(|err| format!("invalid replace pattern '{}': {err}", rule.pattern))?;
    }
    for pattern in merged_patterns(&filter.strip_lines, &filter.strip_lines_matching)
        .into_iter()
        .chain(merged_patterns(
            &filter.keep_lines,
            &filter.keep_lines_matching,
        ))
    {
        Regex::new(&pattern).map_err(|err| format!("invalid line regex '{pattern}': {err}"))?;
    }
    if let Some(match_output) = &filter.match_output {
        validate_match_output(match_output)?;
    }
    Ok(())
}

fn validate_match_output(spec: &MatchOutputSpec) -> Result<(), String> {
    match spec {
        MatchOutputSpec::Legacy(pattern) => {
            Regex::new(pattern).map_err(|err| format!("invalid match_output regex: {err}"))?;
        }
        MatchOutputSpec::Rules(rules) => {
            for rule in rules {
                Regex::new(&rule.pattern).map_err(|err| {
                    format!("invalid match_output pattern '{}': {err}", rule.pattern)
                })?;
                if let Some(unless) = &rule.unless {
                    Regex::new(unless).map_err(|err| {
                        format!("invalid match_output unless pattern '{unless}': {err}")
                    })?;
                }
            }
        }
    }
    Ok(())
}

fn filter_matches_command(filter: &FilterDef, cmd: &str, cmd_base: &str) -> bool {
    if let Some(pattern) = &filter.match_command {
        if cmd.starts_with(pattern) || cmd_base == pattern {
            return true;
        }
        if Regex::new(pattern)
            .map(|re| re.is_match(cmd))
            .unwrap_or(false)
        {
            return true;
        }
    }
    if let Some(pattern) = &filter.match_regex {
        return Regex::new(pattern)
            .map(|re| re.is_match(cmd))
            .unwrap_or(false);
    }
    false
}

fn match_output_message(filter: &FilterDef, output: &str) -> Option<String> {
    match filter.match_output.as_ref()? {
        MatchOutputSpec::Legacy(pattern) => {
            let matches = Regex::new(pattern)
                .map(|re| re.is_match(output))
                .unwrap_or(false);
            matches.then(|| {
                filter
                    .on_empty
                    .clone()
                    .unwrap_or_else(|| output.to_string())
            })
        }
        MatchOutputSpec::Rules(rules) => {
            for rule in rules {
                let matches = Regex::new(&rule.pattern)
                    .map(|re| re.is_match(output))
                    .unwrap_or(false);
                if !matches {
                    continue;
                }
                if let Some(unless) = &rule.unless {
                    if Regex::new(unless)
                        .map(|re| re.is_match(output))
                        .unwrap_or(false)
                    {
                        continue;
                    }
                }
                return Some(rule.message.clone());
            }
            None
        }
    }
}

fn merged_patterns(primary: &Option<Vec<String>>, alias: &Option<Vec<String>>) -> Vec<String> {
    primary
        .iter()
        .chain(alias.iter())
        .flat_map(|items| items.iter().cloned())
        .collect()
}

fn matches_any(line: &str, patterns: &[String]) -> bool {
    patterns
        .iter()
        .any(|pat| Regex::new(pat).map(|re| re.is_match(line)).unwrap_or(false))
}

fn merge_tests(
    target: &mut BTreeMap<String, Vec<FilterTestDef>>,
    source: BTreeMap<String, Vec<FilterTestDef>>,
) {
    for (name, tests) in source {
        target.entry(name).or_default().extend(tests);
    }
}

fn filter_display_name(filter: &FilterDef) -> String {
    filter
        .name
        .clone()
        .or_else(|| filter.match_command.clone())
        .or_else(|| filter.match_regex.clone())
        .unwrap_or_else(|| "unnamed".into())
}

fn env_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_cwd_lock;

    #[test]
    fn test_builtin_filters_load() {
        let engine = FilterEngine::new();
        let filter = engine.find_filter("cargo test");
        assert!(
            filter.is_some(),
            "Should find built-in filter for 'cargo test': {:?}",
            engine.warnings()
        );
    }

    #[test]
    fn test_find_git_status_filter() {
        let engine = FilterEngine::new();
        let filter = engine.find_filter("git status");
        assert!(filter.is_some());
    }

    #[test]
    fn test_passthrough_unknown_command() {
        let engine = FilterEngine::new();
        let filter = engine.find_filter("some_unknown_command_xyz");
        assert!(filter.is_none());
    }

    #[test]
    fn test_apply_strip_lines() {
        let engine = FilterEngine::new();
        let filter = FilterDef {
            match_command: Some("ls".into()),
            strip_lines: Some(vec!["^$".into()]),
            ..Default::default()
        };
        let output = "file1.rs\n\nfile2.rs\n\n";
        let result = engine.apply(&filter, output);
        assert!(!result.contains("\n\n"));
    }

    #[test]
    fn test_apply_max_lines() {
        let engine = FilterEngine::new();
        let filter = FilterDef {
            max_lines: Some(3),
            ..Default::default()
        };
        let output = "line1\nline2\nline3\nline4\nline5\nline6";
        let result = engine.apply(&filter, output);
        assert!(result.lines().count() <= 4);
    }

    #[test]
    fn test_apply_on_empty() {
        let engine = FilterEngine::new();
        let filter = FilterDef {
            on_empty: Some("No output".into()),
            ..Default::default()
        };
        let result = engine.apply(&filter, "");
        assert_eq!(result, "No output");
    }

    #[test]
    fn test_apply_keep_lines() {
        let engine = FilterEngine::new();
        let filter = FilterDef {
            keep_lines: Some(vec!["error".into(), "FAIL".into()]),
            ..Default::default()
        };
        let output =
            "running tests\n  PASS test_a\n  FAIL test_b\n  error: something broke\nfinished";
        let result = engine.apply(&filter, output);
        assert!(result.contains("FAIL"));
        assert!(result.contains("error"));
    }

    #[test]
    fn test_apply_strip_ansi_without_panics() {
        let engine = FilterEngine::new();
        let filter = FilterDef {
            strip_ansi: Some(true),
            ..Default::default()
        };
        let result = engine.apply(&filter, "\x1b[31mred\x1b[0m");

        assert_eq!(result, "red");
    }

    #[test]
    fn test_apply_truncate_lines_at_preserves_utf8_boundaries() {
        let engine = FilterEngine::new();
        let filter = FilterDef {
            truncate_lines_at: Some(1000),
            ..Default::default()
        };
        let output = format!("{}😀x", "я".repeat(999));
        let result = engine.apply(&filter, &output);

        assert!(result.ends_with("..."));
        assert!(result.is_char_boundary(result.len()));
        assert!(result.contains('😀'));
    }

    #[test]
    fn test_parse_named_rtk_filter_with_inline_test() {
        let toml = r#"
schema_version = 1

[filters.clean-build]
match_command = "^build-tool"
strip_ansi = true

[[filters.clean-build.match_output]]
pattern = "Finished"
message = "Build completed"
unless = "error"

[[tests.clean-build]]
name = "success-short-circuit"
input = "\u001b[32mFinished\u001b[0m"
expected = "Build completed"
"#;
        let engine = FilterEngine::from_toml(toml, "unit").expect("parse named filter");
        let filter = engine.find_filter("build-tool --release");
        assert!(filter.is_some());
        let results = engine.verify(Some("clean-build"), true).expect("verify");
        assert!(results.passed(true), "{results:?}");
    }

    #[test]
    fn test_match_output_unless_preserves_error_output() {
        let toml = r#"
schema_version = 1

[filters.clean-build]
match_command = "^build-tool"

[[filters.clean-build.match_output]]
pattern = "Finished"
message = "Build completed"
unless = "error"
"#;
        let engine = FilterEngine::from_toml(toml, "unit").expect("parse named filter");
        let filter = engine.find_filter("build-tool").expect("filter");
        let output = engine.apply(filter, "Finished with error");
        assert_eq!(output, "Finished with error");
    }

    #[test]
    fn test_project_filters_require_trust() {
        let _lock = test_cwd_lock().blocking_lock();
        let dir = tempfile::tempdir().unwrap();
        let data_home = tempfile::tempdir().unwrap();
        let old_data_home = std::env::var_os("XDG_DATA_HOME");
        let old_trust = std::env::var_os("SYNAPSE_TRUST_PROJECT_FILTERS");
        let old_cwd = std::env::current_dir().unwrap();

        std::env::set_var("XDG_DATA_HOME", data_home.path());
        std::env::remove_var("SYNAPSE_TRUST_PROJECT_FILTERS");
        std::env::set_current_dir(dir.path()).unwrap();
        std::fs::create_dir(".synapse").unwrap();
        std::fs::write(
            ".synapse/filters.toml",
            "[[filters]]\nmatch_command = \"unit-only\"\nmax_lines = 1\n",
        )
        .unwrap();

        let engine = FilterEngine::new();
        assert!(engine.find_filter("unit-only").is_none());
        assert!(engine
            .warnings()
            .iter()
            .any(|warning| warning.contains("project filters skipped")));

        super::super::filter_trust::trust_project_filters().unwrap();
        let engine = FilterEngine::new();
        assert!(engine.find_filter("unit-only").is_some());

        std::env::set_current_dir(old_cwd).unwrap();
        restore_env("XDG_DATA_HOME", old_data_home);
        restore_env("SYNAPSE_TRUST_PROJECT_FILTERS", old_trust);
    }

    fn restore_env(name: &str, value: Option<std::ffi::OsString>) {
        if let Some(value) = value {
            std::env::set_var(name, value);
        } else {
            std::env::remove_var(name);
        }
    }
}
