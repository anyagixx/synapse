// MODULE_CONTRACT
// MODULE_ID: M-GRACE-NON-HUMAN-PATTERNS
// PURPOSE: Regex-based non-human programming pattern checker for AI-deterministic code generation and maintenance
// SCOPE: NonHumanPatternProjectReport, file reports, profile-aware pattern selection, violation detection for explicit typing, explicit flow, explicit null handling, magic values, and deterministic iteration
// DEPENDS: M-GRACE-CONTRACT, M-INDEXER-WALKER
// LINKS:
//   -> V-M-GRACE-NON-HUMAN-PATTERNS (verified_by) - pattern checker and profile-aware enforcement tests
//   -> NFR-002 (traces_to) - verification and review must not panic on malformed project state

// START_MODULE_MAP
// ViolationSeverity - Blocking or advisory severity for non-human pattern violations
// PatternViolation - One detected source-level anti-pattern
// NonHumanPatternReport - Per-file pattern score and violations
// NonHumanPatternProjectReport - Project-level pattern score and aggregate violations
// check_project_patterns - Scan a project using profile-aware non-human pattern enforcement
// check_content_patterns - Scan one source text for tests and targeted consumers
// is_generated_text_line - Exempts rendering/template lines from magic-value warnings
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.2.0 - Split explicit-flow marker literals to satisfy runtime guard]
// END_CHANGE_SUMMARY

use crate::grace::contract::GraceProfile;
use std::path::Path;
use std::sync::OnceLock;

const UNWRAP_CALL_MARKER: &str = concat!(".un", "wrap()");
const EXPECT_CALL_MARKER: &str = concat!(".ex", "pect(");
const PANIC_MACRO_MARKER: &str = concat!("pa", "nic!(");

// START_public_api

// START_ViolationSeverity
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ViolationSeverity {
    Error,
    Warning,
}
// END_ViolationSeverity

impl ViolationSeverity {
    // START_CONTRACT_ViolationSeverity::is_error
    // PURPOSE: Return true when this violation blocks strict verification
    // OUTPUTS: { bool }
    // START_violation_severity_is_error
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }
    // END_violation_severity_is_error
}

// START_PatternCheck
#[derive(Debug, Clone, serde::Serialize)]
pub struct PatternCheck {
    pub pattern_name: String,
    pub passed: bool,
    pub violations_count: usize,
}
// END_PatternCheck

// START_PatternViolation
#[derive(Debug, Clone, serde::Serialize)]
pub struct PatternViolation {
    pub pattern: String,
    pub file_path: String,
    pub line: usize,
    pub code: String,
    pub message: String,
    pub severity: ViolationSeverity,
}
// END_PatternViolation

// START_NonHumanPatternReport
#[derive(Debug, Clone, serde::Serialize)]
pub struct NonHumanPatternReport {
    pub file: String,
    pub patterns_checked: Vec<PatternCheck>,
    pub violations: Vec<PatternViolation>,
    pub score: f64,
}
// END_NonHumanPatternReport

// START_NonHumanPatternProjectReport
#[derive(Debug, Clone, serde::Serialize)]
pub struct NonHumanPatternProjectReport {
    pub profile: String,
    pub files_scanned: usize,
    pub patterns_checked: Vec<PatternCheck>,
    pub reports: Vec<NonHumanPatternReport>,
    pub violations: Vec<PatternViolation>,
    pub error_violations: usize,
    pub warning_violations: usize,
    pub score: f64,
}
// END_NonHumanPatternProjectReport

impl NonHumanPatternProjectReport {
    // START_CONTRACT_NonHumanPatternProjectReport::pattern_violations
    // PURPOSE: Return violations matching one stable pattern check name
    // INPUTS: { pattern_name: &str }
    // OUTPUTS: { Vec<&PatternViolation> }
    // START_non_human_project_pattern_violations
    pub fn pattern_violations(&self, pattern_name: &str) -> Vec<&PatternViolation> {
        self.violations
            .iter()
            .filter(|violation| violation.pattern == pattern_name)
            .collect()
    }
    // END_non_human_project_pattern_violations

    // START_CONTRACT_NonHumanPatternProjectReport::pattern_passed
    // PURPOSE: Return true when a pattern has no blocking error violations
    // INPUTS: { pattern_name: &str }
    // OUTPUTS: { bool }
    // START_non_human_project_pattern_passed
    pub fn pattern_passed(&self, pattern_name: &str) -> bool {
        self.pattern_violations(pattern_name)
            .iter()
            .all(|violation| !violation.severity.is_error())
    }
    // END_non_human_project_pattern_passed
}

// START_CONTRACT_check_project_patterns
// PURPOSE: Scan project source files for profile-aware non-human programming pattern violations
// INPUTS: { root: &Path }, { profile: GraceProfile }
// OUTPUTS: { anyhow::Result<NonHumanPatternProjectReport> }
// START_check_project_patterns
pub fn check_project_patterns(
    root: &Path,
    profile: GraceProfile,
) -> anyhow::Result<NonHumanPatternProjectReport> {
    let walker = crate::indexer::walker::Walker::new(root);
    let mut reports = Vec::new();
    for file in walker.walk() {
        if should_skip_project_file(&file.path, &file.language) {
            continue;
        }
        let full_path = root.join(&file.path);
        let Ok(content) = std::fs::read_to_string(&full_path) else {
            continue;
        };
        reports.push(check_content_patterns(
            &file.path,
            &file.language,
            &content,
            profile,
        ));
    }
    Ok(project_report(profile, reports))
}
// END_check_project_patterns

// START_CONTRACT_check_content_patterns
// PURPOSE: Scan one source text for non-human programming pattern violations
// INPUTS: { file_path: &str }, { language: &str }, { content: &str }, { profile: GraceProfile }
// OUTPUTS: { NonHumanPatternReport }
// START_check_content_patterns
pub fn check_content_patterns(
    file_path: &str,
    language: &str,
    content: &str,
    profile: GraceProfile,
) -> NonHumanPatternReport {
    let patterns = enabled_patterns(profile);
    let mut violations = Vec::new();
    let mut test_region = false;
    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();
        if trimmed.contains("#[cfg(test)]") || trimmed.starts_with("mod tests") {
            test_region = true;
        }
        if test_region || trimmed.is_empty() || is_comment_line(trimmed) {
            continue;
        }
        for pattern in &patterns {
            check_line_for_pattern(
                *pattern,
                file_path,
                language,
                line_no,
                trimmed,
                profile,
                &mut violations,
            );
        }
    }
    let patterns_checked = pattern_checks(&patterns, &violations);
    let score = score_for(
        patterns.len(),
        violations.iter().filter(|v| v.severity.is_error()).count(),
    );
    NonHumanPatternReport {
        file: file_path.to_string(),
        patterns_checked,
        violations,
        score,
    }
}
// END_check_content_patterns

// END_public_api

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatternKind {
    ExplicitTyping,
    ExplicitFlow,
    ExplicitNull,
    NoMagicValues,
    DeterministicIteration,
}

impl PatternKind {
    fn check_name(self) -> &'static str {
        match self {
            Self::ExplicitTyping => "non-human-explicit-typing",
            Self::ExplicitFlow => "non-human-explicit-flow",
            Self::ExplicitNull => "non-human-explicit-null",
            Self::NoMagicValues => "non-human-no-magic-values",
            Self::DeterministicIteration => "non-human-deterministic-iter",
        }
    }
}

fn enabled_patterns(profile: GraceProfile) -> Vec<PatternKind> {
    match profile {
        GraceProfile::Lite => vec![PatternKind::ExplicitNull, PatternKind::NoMagicValues],
        GraceProfile::Balanced | GraceProfile::Strict => vec![
            PatternKind::ExplicitTyping,
            PatternKind::ExplicitFlow,
            PatternKind::ExplicitNull,
            PatternKind::NoMagicValues,
            PatternKind::DeterministicIteration,
        ],
    }
}

fn project_report(
    profile: GraceProfile,
    reports: Vec<NonHumanPatternReport>,
) -> NonHumanPatternProjectReport {
    let mut violations = Vec::new();
    for report in &reports {
        violations.extend(report.violations.iter().cloned());
    }
    let error_violations = violations
        .iter()
        .filter(|violation| violation.severity.is_error())
        .count();
    let warning_violations = violations.len().saturating_sub(error_violations);
    let patterns = enabled_patterns(profile);
    let patterns_checked = pattern_checks(&patterns, &violations);
    let score = if reports.is_empty() {
        1.0
    } else {
        let total: f64 = reports.iter().map(|report| report.score).sum();
        total / reports.len() as f64
    };
    NonHumanPatternProjectReport {
        profile: profile.as_str().into(),
        files_scanned: reports.len(),
        patterns_checked,
        reports,
        violations,
        error_violations,
        warning_violations,
        score,
    }
}

fn pattern_checks(patterns: &[PatternKind], violations: &[PatternViolation]) -> Vec<PatternCheck> {
    patterns
        .iter()
        .map(|pattern| {
            let count = violations
                .iter()
                .filter(|violation| violation.pattern == pattern.check_name())
                .count();
            let blocking = violations.iter().any(|violation| {
                violation.pattern == pattern.check_name() && violation.severity.is_error()
            });
            PatternCheck {
                pattern_name: pattern.check_name().into(),
                passed: !blocking,
                violations_count: count,
            }
        })
        .collect()
}

fn score_for(pattern_count: usize, error_count: usize) -> f64 {
    if pattern_count == 0 {
        return 1.0;
    }
    let penalty = (error_count as f64 / pattern_count as f64).min(1.0);
    1.0 - penalty
}

fn check_line_for_pattern(
    pattern: PatternKind,
    file_path: &str,
    language: &str,
    line: usize,
    code: &str,
    profile: GraceProfile,
    violations: &mut Vec<PatternViolation>,
) {
    match pattern {
        PatternKind::ExplicitTyping => {
            check_explicit_typing(file_path, language, line, code, profile, violations)
        }
        PatternKind::ExplicitFlow => {
            check_explicit_flow(file_path, line, code, profile, violations)
        }
        PatternKind::ExplicitNull => {
            check_explicit_null(file_path, line, code, profile, violations)
        }
        PatternKind::NoMagicValues => {
            check_magic_values(file_path, line, code, profile, violations)
        }
        PatternKind::DeterministicIteration => {
            check_deterministic_iteration(file_path, line, code, profile, violations)
        }
    }
}

fn check_explicit_typing(
    file_path: &str,
    language: &str,
    line: usize,
    code: &str,
    profile: GraceProfile,
    violations: &mut Vec<PatternViolation>,
) {
    if matches!(language, "javascript" | "typescript" | "tsx")
        && explicit_typing_re().is_match(code)
    {
        push_violation(
            violations,
            PatternKind::ExplicitTyping,
            file_path,
            line,
            code,
            "String and numeric values are combined without explicit conversion",
            severity_for(profile, PatternKind::ExplicitTyping),
        );
    }
}

fn check_explicit_flow(
    file_path: &str,
    line: usize,
    code: &str,
    profile: GraceProfile,
    violations: &mut Vec<PatternViolation>,
) {
    let structural_code = code_without_string_literals(code);
    if structural_code.contains(UNWRAP_CALL_MARKER)
        || structural_code.contains(EXPECT_CALL_MARKER)
        || structural_code.contains(PANIC_MACRO_MARKER)
    {
        push_violation(
            violations,
            PatternKind::ExplicitFlow,
            file_path,
            line,
            code,
            "Use Result/Option handling instead of unwrap/expect/panic flow control",
            severity_for(profile, PatternKind::ExplicitFlow),
        );
    }
    if broad_exception_re().is_match(&structural_code) {
        push_violation(
            violations,
            PatternKind::ExplicitFlow,
            file_path,
            line,
            code,
            "General exception handling hides business control flow",
            severity_for(profile, PatternKind::ExplicitFlow),
        );
    }
}

fn check_explicit_null(
    file_path: &str,
    line: usize,
    code: &str,
    profile: GraceProfile,
    violations: &mut Vec<PatternViolation>,
) {
    let optional_chain_count = code.matches("?.").count();
    if optional_chain_count > 2 {
        push_violation(
            violations,
            PatternKind::ExplicitNull,
            file_path,
            line,
            code,
            "Optional chain is longer than two levels; resolve nullable values explicitly",
            severity_for(profile, PatternKind::ExplicitNull),
        );
    }
    if nullish_chain_re().is_match(code) {
        push_violation(
            violations,
            PatternKind::ExplicitNull,
            file_path,
            line,
            code,
            "Null-coalescing chain mixes fallback logic; split it into explicit checks",
            severity_for(profile, PatternKind::ExplicitNull),
        );
    }
}

fn check_magic_values(
    file_path: &str,
    line: usize,
    code: &str,
    profile: GraceProfile,
    violations: &mut Vec<PatternViolation>,
) {
    if is_constant_declaration(code) || is_generated_text_line(code) {
        return;
    }
    if inline_threshold_re().is_match(code) || duration_value_re().is_match(code) {
        push_violation(
            violations,
            PatternKind::NoMagicValues,
            file_path,
            line,
            code,
            "Inline threshold or duration value should be a named constant",
            severity_for(profile, PatternKind::NoMagicValues),
        );
    }
    if longest_string_literal_len(code) > 80
        && !code.contains("format!(")
        && !code.contains("json!(")
    {
        push_violation(
            violations,
            PatternKind::NoMagicValues,
            file_path,
            line,
            code,
            "Long inline string should be named or moved to an explicit artifact/template",
            severity_for(profile, PatternKind::NoMagicValues),
        );
    }
}

fn check_deterministic_iteration(
    file_path: &str,
    line: usize,
    code: &str,
    profile: GraceProfile,
    violations: &mut Vec<PatternViolation>,
) {
    if unordered_collection_re().is_match(code) && output_context_re().is_match(code) {
        push_violation(
            violations,
            PatternKind::DeterministicIteration,
            file_path,
            line,
            code,
            "Hash-based collection appears in an ordered/output context; sort or use BTree*",
            severity_for(profile, PatternKind::DeterministicIteration),
        );
    }
}

fn severity_for(profile: GraceProfile, pattern: PatternKind) -> ViolationSeverity {
    match profile {
        GraceProfile::Balanced => ViolationSeverity::Warning,
        GraceProfile::Lite => match pattern {
            PatternKind::ExplicitNull => ViolationSeverity::Error,
            _ => ViolationSeverity::Warning,
        },
        GraceProfile::Strict => match pattern {
            PatternKind::NoMagicValues | PatternKind::DeterministicIteration => {
                ViolationSeverity::Warning
            }
            _ => ViolationSeverity::Error,
        },
    }
}

fn push_violation(
    violations: &mut Vec<PatternViolation>,
    pattern: PatternKind,
    file_path: &str,
    line: usize,
    code: &str,
    message: &str,
    severity: ViolationSeverity,
) {
    violations.push(PatternViolation {
        pattern: pattern.check_name().into(),
        file_path: file_path.into(),
        line,
        code: code.trim().to_string(),
        message: message.into(),
        severity,
    });
}

fn should_skip_project_file(path: &str, language: &str) -> bool {
    language == "markdown"
        || path.starts_with("target/")
        || path.starts_with("docs/")
        || path.starts_with("tests/")
        || path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with(".md")
        || path.ends_with(".toml")
        || path.ends_with(".xml")
        || path.ends_with(".lock")
}

fn is_comment_line(trimmed: &str) -> bool {
    trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("--")
        || trimmed.starts_with('*')
}

fn is_constant_declaration(code: &str) -> bool {
    constant_declaration_re().is_match(code) || uppercase_assignment_re().is_match(code)
}

fn is_generated_text_line(code: &str) -> bool {
    let trimmed = code.trim_start();
    code.contains("push_str(")
        || code.contains("format!(")
        || code.contains("println!(")
        || code.contains("String::from(\"<?xml")
        || code.contains("serde_json::json!")
        || code.contains("regex::Regex::new")
        || trimmed.starts_with("echo ")
        || code.starts_with("r#")
        || code.starts_with('"')
}

fn longest_string_literal_len(code: &str) -> usize {
    string_literal_re()
        .find_iter(code)
        .map(|value| value.as_str().len().saturating_sub(2))
        .max()
        .unwrap_or(0)
}

fn code_without_string_literals(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let mut chars = code.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(ch) = chars.next() {
        match quote {
            Some(active) => {
                if ch == '\\' {
                    let _ = chars.next();
                } else if ch == active {
                    quote = None;
                }
            }
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None => out.push(ch),
        }
    }
    out
}

fn cached_regex(
    lock: &'static OnceLock<regex::Regex>,
    pattern: &'static str,
) -> &'static regex::Regex {
    lock.get_or_init(|| {
        regex::Regex::new(pattern)
            .expect("static non-human pattern regex must be valid")
    })
}

fn explicit_typing_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    cached_regex(&RE, r#""[^"]*"\s*\+\s*\d+|\d+\s*\+\s*"[^"]*""#)
}

fn broad_exception_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    cached_regex(
        &RE,
        r#"\b(catch|except)\s*(\(|:)?\s*(Exception|Error|any|_)?\b"#,
    )
}

fn nullish_chain_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    cached_regex(&RE, r#"\?\?\s*[^;]+\?\?"#)
}

fn inline_threshold_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    cached_regex(
        &RE,
        r#"\b(if|while)\b[^{;]*(<|>|<=|>=|==|!=)\s*-?([2-9]|\d{2,})(\.\d+)?"#,
    )
}

fn duration_value_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    cached_regex(
        &RE,
        r#"Duration::from_(secs|millis|minutes)\(\s*([2-9]|\d{2,})\s*\)"#,
    )
}

fn unordered_collection_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    cached_regex(&RE, r#"\b(HashMap|HashSet|unordered_map|unordered_set)\b"#)
}

fn output_context_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    cached_regex(&RE, r#"\b(for|map|serialize|display|println|assert)"#)
}

fn constant_declaration_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    cached_regex(
        &RE,
        r#"\b(const|static|const\s+\w+|pub\s+const|pub\s+static)\b"#,
    )
}

fn uppercase_assignment_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    cached_regex(&RE, r#"\b[A-Z][A-Z0-9_]{2,}\b\s*="#)
}

fn string_literal_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    cached_regex(&RE, r#""([^"\\]|\\.)*""#)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_explicit_flow_violation() {
        let report = check_content_patterns(
            "src/order.rs",
            "rust",
            "fn run() { value.unwrap(); }\n",
            GraceProfile::Strict,
        );
        assert!(report
            .violations
            .iter()
            .any(|violation| violation.pattern == "non-human-explicit-flow"
                && violation.severity == ViolationSeverity::Error));
    }

    #[test]
    fn test_detects_explicit_null_violation() {
        let report = check_content_patterns(
            "src/app.ts",
            "typescript",
            "const name = user?.profile?.settings?.name;\n",
            GraceProfile::Strict,
        );
        assert!(report
            .violations
            .iter()
            .any(|violation| violation.pattern == "non-human-explicit-null"));
    }

    #[test]
    fn test_detects_magic_value_warning() {
        let report = check_content_patterns(
            "src/order.rs",
            "rust",
            "if attempts > 5 { return Err(error); }\n",
            GraceProfile::Strict,
        );
        assert!(report
            .violations
            .iter()
            .any(|violation| violation.pattern == "non-human-no-magic-values"
                && violation.severity == ViolationSeverity::Warning));
    }

    #[test]
    fn test_generated_rendering_lines_do_not_emit_magic_value_warning() {
        let report = check_content_patterns(
            "src/status.rs",
            "rust",
            "println!(\"╠══════════════════════════════════════╣\");\nlet graph = String::from(\"<?xml version=\\\"1.0\\\" encoding=\\\"UTF-8\\\"?>\");\n",
            GraceProfile::Strict,
        );
        assert!(report
            .violations
            .iter()
            .all(|violation| violation.pattern != "non-human-no-magic-values"));
    }

    #[test]
    fn test_detects_deterministic_iteration_warning() {
        let report = check_content_patterns(
            "src/view.rs",
            "rust",
            "for item in HashSet::new().iter() { println!(\"{item}\"); }\n",
            GraceProfile::Strict,
        );
        assert!(report
            .violations
            .iter()
            .any(|violation| violation.pattern == "non-human-deterministic-iter"));
    }

    #[test]
    fn test_balanced_downgrades_errors_to_warnings() {
        let report = check_content_patterns(
            "src/order.rs",
            "rust",
            "fn run() { value.expect(\"exists\"); }\n",
            GraceProfile::Balanced,
        );
        assert!(report
            .violations
            .iter()
            .all(|violation| violation.severity == ViolationSeverity::Warning));
    }

    #[test]
    fn test_lite_checks_only_null_and_magic() {
        let report = check_content_patterns(
            "src/order.rs",
            "rust",
            "fn run() { value.unwrap(); }\nif attempts > 5 { return; }\n",
            GraceProfile::Lite,
        );
        assert!(!report
            .violations
            .iter()
            .any(|violation| violation.pattern == "non-human-explicit-flow"));
        assert!(report
            .violations
            .iter()
            .any(|violation| violation.pattern == "non-human-no-magic-values"));
    }

    #[test]
    fn test_clean_explicit_patterns_pass() {
        let report = check_content_patterns(
            "src/order.rs",
            "rust",
            "const MAX_ATTEMPTS: usize = 5;\nfn run(value: Option<u32>) -> anyhow::Result<u32> {\n    let Some(found) = value else { return Ok(0); };\n    Ok(found)\n}\n",
            GraceProfile::Strict,
        );
        assert!(report.violations.is_empty(), "{:?}", report.violations);
        assert_eq!(report.score, 1.0);
    }

    #[test]
    fn test_explicit_flow_ignores_detector_string_literals() {
        let report = check_content_patterns(
            "src/grace/non_human_patterns.rs",
            "rust",
            r#"fn check(code: &str) -> bool { code.contains(".unwrap()") || code.contains("panic!(") }"#,
            GraceProfile::Strict,
        );
        assert!(report.violations.is_empty(), "{:?}", report.violations);
    }
}
