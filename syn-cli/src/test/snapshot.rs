// MODULE_CONTRACT
// MODULE_ID: M-TEST-SNAPSHOT
// PURPOSE: Golden snapshot comparison engine for deterministic command-output regression tests.
// SCOPE: Snapshot creation, update, normalization, bounded diffing, summary reporting, and future E2E runner integration.
// DEPENDS: M-UTILS
// LINKS:
//   -> Phase-77 (implements) - snapshot verification
//   -> NFR-002 (traces_to) - stable regression evidence
//   -> NFR-003 (traces_to) - compact output diffs
//   <- V-M-TEST-SNAPSHOT (verified_by) - snapshot comparison verification

// START_MODULE_MAP
// SnapshotResult - Snapshot comparison outcome and artifact path
// SnapshotSummary - Aggregate matched/mismatched/created/updated counts
// SnapshotOptions - Update and normalization controls
// SnapshotNormalization - Stability filters for volatile output values
// assert_snapshot - Compare output against a stored .snap file with default normalization
// assert_snapshot_with_options - Compare output with explicit snapshot options
// snapshot_summary - Summarize snapshot result collections
// normalize_snapshot_text - Remove configured unstable values from snapshot text
// compute_diff - Build a bounded line-oriented diff for mismatches
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.1 - Derived SnapshotOptions default for clippy gate]
// END_CHANGE_SUMMARY

use syn_core::utils::{strip_ansi, truncate_chars};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const SNAPSHOT_EXTENSION: &str = "snap";
const DEFAULT_SNAPSHOT_NAME: &str = "snapshot";
const DIFF_LINE_LIMIT: usize = 20;
const DIFF_VALUE_LIMIT: usize = 240;

static UUID_RE: OnceLock<Result<Regex, String>> = OnceLock::new();
static VERSION_RE: OnceLock<Result<Regex, String>> = OnceLock::new();
static DURATION_RE: OnceLock<Result<Regex, String>> = OnceLock::new();
static UNIX_TEMP_PATH_RE: OnceLock<Result<Regex, String>> = OnceLock::new();
static WINDOWS_TEMP_PATH_RE: OnceLock<Result<Regex, String>> = OnceLock::new();

const UUID_PATTERN: &str =
    r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b";
const VERSION_PATTERN: &str = r"\bv?\d+\.\d+\.\d+(?:[-+][A-Za-z0-9._-]+)?\b";
const DURATION_PATTERN: &str = r"\b\d+(?:\.\d+)?\s?(?:ms|s|sec|secs|second|seconds|us|µs|ns)\b";
const UNIX_TEMP_PATH_PATTERN: &str =
    r"(?x)(?:/tmp|/var/folders|/private/var/folders)/[A-Za-z0-9._@%+=:,;~/-]+";
const WINDOWS_TEMP_PATH_PATTERN: &str =
    r"(?i)[A-Z]:\\Users\\[^\\\s]+\\AppData\\Local\\Temp\\[^\\\s]+(?:\\[^\\\s]+)*";

// START_public_api

// START_SnapshotResult
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotResult {
    pub name: String,
    pub matched: bool,
    pub created: bool,
    pub updated: bool,
    pub snapshot_path: PathBuf,
    pub diff: Option<String>,
}
// END_SnapshotResult

// START_SnapshotSummary
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotSummary {
    pub total: usize,
    pub matched: usize,
    pub mismatched: usize,
    pub created: usize,
    pub updated: usize,
}
// END_SnapshotSummary

// START_SnapshotOptions
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SnapshotOptions {
    pub update: bool,
    pub normalization: SnapshotNormalization,
}
// END_SnapshotOptions

// START_SnapshotNormalization
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotNormalization {
    pub strip_ansi: bool,
    pub temp_paths: bool,
    pub timings: bool,
    pub versions: bool,
    pub uuids: bool,
}
// END_SnapshotNormalization

impl Default for SnapshotNormalization {
    // START_CONTRACT_SnapshotNormalization::default
    // PURPOSE: Enable all snapshot normalization filters by default
    // OUTPUTS: { SnapshotNormalization }
    // LINKS:
    //   -> NFR-002 (traces_to) - default snapshots must avoid local volatile values
    // START_snapshot_normalization_default
    fn default() -> Self {
        Self {
            strip_ansi: true,
            temp_paths: true,
            timings: true,
            versions: true,
            uuids: true,
        }
    }
    // END_snapshot_normalization_default
}

// START_CONTRACT_assert_snapshot
// PURPOSE: Compare actual output against a stored snapshot with default normalization
// INPUTS: { snapshots_dir: &Path }, { name: &str }, { actual: &str }, { update: bool }
// OUTPUTS: { anyhow::Result<SnapshotResult> }
// SIDE_EFFECTS: creates snapshot directory, creates or updates .snap files
// LINKS:
//   -> M-TEST-SNAPSHOT (depends) - snapshot comparison entry point
//   -> NFR-002 (traces_to) - stable regression evidence
//   -> NFR-003 (traces_to) - compact mismatch evidence
// <LOG id="snapshot_checked" level="INFO" ref="snapshot-check" module="M-TEST-SNAPSHOT" contract="assert_snapshot">
//   EVENT: snapshot_checked
//   EXPECTATION: existing snapshots compare normalized actual output against normalized stored output
//   DECISION: default normalization is applied before compare or write
//   RESULT: success
//   TRACEABILITY: NFR-002
// </LOG>
// START_assert_snapshot
pub fn assert_snapshot(
    snapshots_dir: &Path,
    name: &str,
    actual: &str,
    update: bool,
) -> anyhow::Result<SnapshotResult> {
    assert_snapshot_with_options(
        snapshots_dir,
        name,
        actual,
        SnapshotOptions {
            update,
            ..SnapshotOptions::default()
        },
    )
}
// END_assert_snapshot

// START_CONTRACT_assert_snapshot_with_options
// PURPOSE: Compare actual output against a stored snapshot using explicit update and normalization options
// INPUTS: { snapshots_dir: &Path }, { name: &str }, { actual: &str }, { options: SnapshotOptions }
// OUTPUTS: { anyhow::Result<SnapshotResult> }
// SIDE_EFFECTS: creates snapshot directory, creates or updates .snap files
// LINKS:
//   -> M-TEST-SNAPSHOT (depends) - configurable snapshot assertion
//   -> NFR-002 (traces_to) - configurable deterministic regression checks
// <LOG id="snapshot_updated" level="INFO" ref="snapshot-update" module="M-TEST-SNAPSHOT" contract="assert_snapshot_with_options">
//   EVENT: snapshot_updated
//   EXPECTATION: update mode and first-run baselines write normalized snapshot content
//   DECISION: missing snapshots and explicit update mode share the write path
//   RESULT: success
//   TRACEABILITY: NFR-002
// </LOG>
// START_assert_snapshot_with_options
pub fn assert_snapshot_with_options(
    snapshots_dir: &Path,
    name: &str,
    actual: &str,
    options: SnapshotOptions,
) -> anyhow::Result<SnapshotResult> {
    std::fs::create_dir_all(snapshots_dir)?;
    let snapshot_path = snapshot_path(snapshots_dir, name);
    let normalized_actual = normalize_snapshot_text(actual, &options.normalization);
    let normalized_name = sanitize_name(name);

    if options.update || !snapshot_path.exists() {
        std::fs::write(&snapshot_path, &normalized_actual)?;
        return Ok(SnapshotResult {
            name: normalized_name,
            matched: true,
            created: !options.update,
            updated: options.update,
            snapshot_path,
            diff: None,
        });
    }

    let expected = std::fs::read_to_string(&snapshot_path)?;
    if expected == normalized_actual {
        return Ok(SnapshotResult {
            name: normalized_name,
            matched: true,
            created: false,
            updated: false,
            snapshot_path,
            diff: None,
        });
    }

    Ok(SnapshotResult {
        name: normalized_name,
        matched: false,
        created: false,
        updated: false,
        diff: Some(compute_diff(&expected, &normalized_actual)),
        snapshot_path,
    })
}
// END_assert_snapshot_with_options

// START_CONTRACT_snapshot_summary
// PURPOSE: Summarize snapshot results into matched, mismatched, created, and updated counts
// INPUTS: { results: &[SnapshotResult] }
// OUTPUTS: { SnapshotSummary }
// LINKS:
//   -> M-TEST-SNAPSHOT (depends) - snapshot result aggregation
//   -> NFR-003 (traces_to) - compact snapshot report
// START_snapshot_summary
pub fn snapshot_summary(results: &[SnapshotResult]) -> SnapshotSummary {
    let matched = results.iter().filter(|result| result.matched).count();
    let created = results.iter().filter(|result| result.created).count();
    let updated = results.iter().filter(|result| result.updated).count();
    SnapshotSummary {
        total: results.len(),
        matched,
        mismatched: results.len().saturating_sub(matched),
        created,
        updated,
    }
}
// END_snapshot_summary

// START_CONTRACT_normalize_snapshot_text
// PURPOSE: Apply configured stable-output normalization filters to snapshot content
// INPUTS: { text: &str }, { normalization: &SnapshotNormalization }
// OUTPUTS: { String }
// LINKS:
//   -> M-UTILS (depends) - ANSI stripping and truncation helpers
//   -> NFR-002 (traces_to) - remove volatile local values from snapshots
// START_normalize_snapshot_text
pub fn normalize_snapshot_text(text: &str, normalization: &SnapshotNormalization) -> String {
    let mut normalized = if normalization.strip_ansi {
        strip_ansi(text)
    } else {
        text.to_string()
    };
    if normalization.temp_paths {
        normalized = replace_cached_regex(
            &UNIX_TEMP_PATH_RE,
            UNIX_TEMP_PATH_PATTERN,
            &normalized,
            "<TEMP_PATH>",
        );
        normalized = replace_cached_regex(
            &WINDOWS_TEMP_PATH_RE,
            WINDOWS_TEMP_PATH_PATTERN,
            &normalized,
            "<TEMP_PATH>",
        );
    }
    if normalization.timings {
        normalized =
            replace_cached_regex(&DURATION_RE, DURATION_PATTERN, &normalized, "<DURATION>");
    }
    if normalization.versions {
        normalized = replace_cached_regex(&VERSION_RE, VERSION_PATTERN, &normalized, "<VERSION>");
    }
    if normalization.uuids {
        normalized = replace_cached_regex(&UUID_RE, UUID_PATTERN, &normalized, "<UUID>");
    }
    normalized
}
// END_normalize_snapshot_text

// END_public_api

// START_CONTRACT_snapshot_path
// PURPOSE: Build the deterministic filesystem path for a snapshot name
// INPUTS: { snapshots_dir: &Path }, { name: &str }
// OUTPUTS: { PathBuf }
// LINKS:
//   -> M-TEST-SNAPSHOT (depends) - safe snapshot artifact paths
//   -> NFR-002 (traces_to) - deterministic snapshot naming
// START_snapshot_path
fn snapshot_path(snapshots_dir: &Path, name: &str) -> PathBuf {
    snapshots_dir.join(format!(
        "{}.{SNAPSHOT_EXTENSION}",
        sanitize_name(name).as_str()
    ))
}
// END_snapshot_path

// START_CONTRACT_sanitize_name
// PURPOSE: Convert arbitrary snapshot names into stable single-file names
// INPUTS: { name: &str }
// OUTPUTS: { String }
// LINKS:
//   -> M-TEST-SNAPSHOT (depends) - snapshot path safety
//   -> NFR-002 (traces_to) - snapshot names cannot escape their directory
// START_sanitize_name
fn sanitize_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        DEFAULT_SNAPSHOT_NAME.to_string()
    } else {
        trimmed.to_string()
    }
}
// END_sanitize_name

// START_CONTRACT_compute_diff
// PURPOSE: Compute a bounded line-oriented diff for mismatched snapshot content
// INPUTS: { expected: &str }, { actual: &str }
// OUTPUTS: { String }
// LINKS:
//   -> M-TEST-SNAPSHOT (depends) - compact mismatch diagnostics
//   -> NFR-003 (traces_to) - bounded output diffing
// START_compute_diff
fn compute_diff(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    let max_lines = expected_lines.len().max(actual_lines.len());
    let mut diff = Vec::new();

    for index in 0..max_lines {
        let expected_line = expected_lines.get(index).copied().unwrap_or("<missing>");
        let actual_line = actual_lines.get(index).copied().unwrap_or("<missing>");
        if expected_line == actual_line {
            continue;
        }
        diff.push(format!(
            "Line {}:\n  expected: {}\n  actual:   {}",
            index + 1,
            truncate_chars(expected_line, DIFF_VALUE_LIMIT),
            truncate_chars(actual_line, DIFF_VALUE_LIMIT)
        ));
        if diff.len() >= DIFF_LINE_LIMIT {
            diff.push(format!(
                "... diff truncated at {DIFF_LINE_LIMIT} changed lines"
            ));
            break;
        }
    }

    if expected_lines.len() != actual_lines.len() {
        diff.push(format!(
            "Line count: expected {}, actual {}",
            expected_lines.len(),
            actual_lines.len()
        ));
    }
    if diff.is_empty() {
        "Snapshot content differs without line-level differences".to_string()
    } else {
        diff.join("\n")
    }
}
// END_compute_diff

// START_CONTRACT_replace_cached_regex
// PURPOSE: Apply a cached regex replacement and degrade to unchanged input if regex initialization fails
// INPUTS: { cache: &'static OnceLock<Result<Regex, String>> }, { pattern: &'static str }, { input: &str }, { replacement: &str }
// OUTPUTS: { String }
// LINKS:
//   -> M-TEST-SNAPSHOT (depends) - stable normalization filters
//   -> NFR-002 (traces_to) - regex failures must degrade deterministically
// START_replace_cached_regex
fn replace_cached_regex(
    cache: &'static OnceLock<Result<Regex, String>>,
    pattern: &'static str,
    input: &str,
    replacement: &str,
) -> String {
    match cache.get_or_init(|| Regex::new(pattern).map_err(|error| error.to_string())) {
        Ok(regex) => regex.replace_all(input, replacement).to_string(),
        Err(_) => input.to_string(),
    }
}
// END_replace_cached_regex

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn snapshot_test_missing_snapshot_creates_baseline() {
        let dir = TempDir::new().unwrap();
        let result = assert_snapshot(dir.path(), "first run", "hello\n", false).unwrap();

        assert!(result.matched);
        assert!(result.created);
        assert!(result.diff.is_none());
        assert_eq!(
            std::fs::read_to_string(result.snapshot_path).unwrap(),
            "hello\n"
        );
    }

    #[test]
    fn snapshot_test_existing_matching_snapshot_returns_matched() {
        let dir = TempDir::new().unwrap();
        assert_snapshot(dir.path(), "same", "stable\n", false).unwrap();

        let result = assert_snapshot(dir.path(), "same", "stable\n", false).unwrap();

        assert!(result.matched);
        assert!(!result.created);
        assert!(result.diff.is_none());
    }

    #[test]
    fn snapshot_test_mismatch_returns_bounded_diff() {
        let dir = TempDir::new().unwrap();
        assert_snapshot(dir.path(), "diff", "a\nb\nc\n", false).unwrap();

        let result = assert_snapshot(dir.path(), "diff", "a\nchanged\nc\n", false).unwrap();

        assert!(!result.matched);
        let diff = result.diff.unwrap();
        assert!(diff.contains("Line 2"));
        assert!(diff.contains("expected: b"));
        assert!(diff.contains("actual:   changed"));
    }

    #[test]
    fn snapshot_test_update_overwrites_snapshot() {
        let dir = TempDir::new().unwrap();
        assert_snapshot(dir.path(), "update", "old\n", false).unwrap();

        let result = assert_snapshot(dir.path(), "update", "new\n", true).unwrap();

        assert!(result.matched);
        assert!(result.updated);
        assert_eq!(
            std::fs::read_to_string(result.snapshot_path).unwrap(),
            "new\n"
        );
    }

    #[test]
    fn snapshot_test_normalization_removes_unstable_values() {
        let input = "\x1b[31merror\x1b[0m /tmp/synapse-abc/run 12.5ms v2.6.4 550e8400-e29b-41d4-a716-446655440000";

        let normalized = normalize_snapshot_text(input, &SnapshotNormalization::default());

        assert_eq!(normalized, "error <TEMP_PATH> <DURATION> <VERSION> <UUID>");
    }

    #[test]
    fn snapshot_test_summary_counts_results() {
        let dir = TempDir::new().unwrap();
        let created = assert_snapshot(dir.path(), "created", "a", false).unwrap();
        let updated = assert_snapshot(dir.path(), "created", "b", true).unwrap();
        let mismatch = assert_snapshot(dir.path(), "created", "c", false).unwrap();

        let summary = snapshot_summary(&[created, updated, mismatch]);

        assert_eq!(summary.total, 3);
        assert_eq!(summary.matched, 2);
        assert_eq!(summary.mismatched, 1);
        assert_eq!(summary.created, 1);
        assert_eq!(summary.updated, 1);
    }

    #[test]
    fn snapshot_test_sanitize_name_prevents_path_escape() {
        let dir = TempDir::new().unwrap();
        let result = assert_snapshot(dir.path(), "../unsafe/name", "ok", false).unwrap();

        assert_eq!(result.name, "unsafe_name");
        assert_eq!(result.snapshot_path.parent().unwrap(), dir.path());
    }
}
