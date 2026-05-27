// MODULE_CONTRACT
// MODULE_ID: M-GRACE-VERIFY-PHASE
// PURPOSE: Phase-level MyGRACE verification checks for architecture completeness and maintainability limits
// SCOPE: phase sharded artifact gate, TODO/FIXME scan, hard file-size and Phase 2 target reporting
// DEPENDS: M-GRACE-LAYOUT, M-GRACE-VERIFY-TYPES, M-INDEXER-WALKER
// LINKS: docs/plan-index.xml, docs/phases/Phase-2.xml

// START_MODULE_MAP
// verify_phase — Runs phase-level MyGRACE checks
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.8.0 — Extracted phase verification from M-GRACE-VERIFY]
// END_CHANGE_SUMMARY

use crate::grace::layout::DocsLayout;
use crate::grace::verify_types::{CheckResult, VerificationResult};
use std::path::Path;

// START_public_api

// START_CONTRACT_verify_phase
// PURPOSE: Run phase-level regression checks for sharded artifacts, TODO/FIXME markers, and file-size limits
// INPUTS: { root: &Path — project root }
// OUTPUTS: { anyhow::Result<VerificationResult> }
// START_verify_phase
pub async fn verify_phase(root: &Path) -> anyhow::Result<VerificationResult> {
    let mut checks = Vec::new();
    let layout = DocsLayout::new(root);
    let phase0_done = layout.graph_index_path().exists()
        && layout.plan_index_path().exists()
        && layout.verification_index_path().exists()
        && layout.modules_dir().exists()
        && layout.phases_dir().exists()
        && layout.verification_dir().exists();
    checks.push(CheckResult {
        name: "phase-0-sharded".into(),
        passed: phase0_done,
        details: if phase0_done {
            "Primary sharded Phase 0 artifacts complete".into()
        } else {
            "Missing one or more primary sharded Phase 0 artifacts".into()
        },
    });

    let files = crate::indexer::walker::Walker::new(root).walk();
    checks.push(no_todos_check(root, &files));
    checks.push(file_size_check(root, &files));

    let passed = checks.iter().all(|c| c.passed);
    Ok(VerificationResult {
        level: "phase".into(),
        passed,
        checks,
    })
}
// END_verify_phase

// END_public_api

fn no_todos_check(root: &Path, files: &[crate::indexer::walker::IndexFile]) -> CheckResult {
    let mut todos = Vec::new();
    for file in files {
        let full_path = root.join(&file.path);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            for (i, line) in content.lines().enumerate() {
                let l = line.trim().to_lowercase();
                if (l.starts_with("// todo")
                    || l.starts_with("# todo")
                    || l.starts_with("/* todo")
                    || l.contains("TODO:"))
                    || (l.starts_with("// fixme")
                        || l.starts_with("# fixme")
                        || l.starts_with("/* fixme")
                        || l.contains("FIXME:"))
                {
                    todos.push(format!("{}:{}", file.path, i + 1));
                }
            }
        }
    }

    CheckResult {
        name: "no-todos".into(),
        passed: todos.is_empty(),
        details: if todos.is_empty() {
            "No TODO/FIXME found".into()
        } else {
            format!(
                "{} TODO/FIXME found:\n  {}",
                todos.len(),
                todos.join("\n  ")
            )
        },
    }
}

fn file_size_check(root: &Path, files: &[crate::indexer::walker::IndexFile]) -> CheckResult {
    const SOURCE_TARGET_LINES: usize = 500;
    const SOURCE_HARD_LIMIT_LINES: usize = 1300;
    let mut large_files = Vec::new();
    let mut target_overages = Vec::new();

    for file in files {
        let full_path = root.join(&file.path);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            let line_count = content.lines().count();
            if line_count > SOURCE_HARD_LIMIT_LINES {
                large_files.push(format!("{} ({} lines)", file.path, line_count));
            } else if line_count > SOURCE_TARGET_LINES {
                target_overages.push(format!("{} ({} lines)", file.path, line_count));
            }
        }
    }

    CheckResult {
        name: "file-size-limit".into(),
        passed: large_files.is_empty(),
        details: file_size_details(&large_files, &target_overages),
    }
}

fn file_size_details(large_files: &[String], target_overages: &[String]) -> String {
    const SOURCE_TARGET_LINES: usize = 500;
    const SOURCE_HARD_LIMIT_LINES: usize = 1300;

    if !large_files.is_empty() {
        return format!(
            "Files over {}-line hard limit:\n  {}",
            SOURCE_HARD_LIMIT_LINES,
            large_files.join("\n  ")
        );
    }

    if target_overages.is_empty() {
        "All files within 500-line Phase 2 target".into()
    } else {
        format!(
            "All files within {}-line hard limit; {} files exceed {}-line Phase 2 target:\n  {}",
            SOURCE_HARD_LIMIT_LINES,
            target_overages.len(),
            SOURCE_TARGET_LINES,
            target_overages.join("\n  ")
        )
    }
}
