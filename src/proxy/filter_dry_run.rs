// MODULE_CONTRACT
// MODULE_ID: M-PROXY-FILTER
// PURPOSE: Filter dry-run engine — explains TOML filter pipeline decisions without executing commands
// SCOPE: dry_run_filter, stage accounting, line slicing, and stable report construction for M-PROXY-FILTER
// DEPENDS: M-UTILS
// LINKS:
//   → M-PROXY-FILTER (depends) - shares validated TOML filter definitions
//   → M-UTILS (depends) - Unicode-safe line truncation and ANSI stripping
//   → Phase-68 (implements) - agent-facing filter dry-run explanations
//   ← V-M-PROXY-FILTER (verified_by) - dry-run stage reporting

// START_MODULE_MAP
// dry_run_filter — Applies the filter pipeline in explain mode
// slice_head_tail — Mirrors head/tail slicing used by the runtime filter pipeline
// push_dry_stage — Records a single stage decision
// finish_dry_run — Builds the immutable report returned to CLI/MCP callers
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Initial filter dry-run implementation]
// END_CHANGE_SUMMARY

use super::toml_filter::{
    filter_display_name, match_output_message, matches_any, merged_patterns, FilterDef,
    FilterDryRunReport, FilterDryRunStage,
};
use regex::Regex;

// START_public_api

// START_CONTRACT_dry_run_filter
// PURPOSE: Apply the TOML filter pipeline in explain mode without command execution or state mutation
// INPUTS: { filter: &FilterDef }, { output: &str }
// OUTPUTS: { FilterDryRunReport }
// START_dry_run_filter
pub(crate) fn dry_run_filter(filter: &FilterDef, output: &str) -> FilterDryRunReport {
    let mut stages = Vec::new();
    let mut result = output.to_string();

    let before = result.clone();
    if filter.strip_ansi.unwrap_or(false) {
        result = crate::utils::strip_ansi(&result);
    }
    push_dry_stage(
        &mut stages,
        "strip_ansi",
        &before,
        &result,
        filter.strip_ansi.unwrap_or(false),
        if filter.strip_ansi.unwrap_or(false) {
            "enabled".into()
        } else {
            "disabled".into()
        },
    );

    let before = result.clone();
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
    push_dry_stage(
        &mut stages,
        "replace",
        &before,
        &result,
        before != result,
        format!("{} rule(s)", filter.replace.len()),
    );

    if let Some(message) = match_output_message(filter, &result) {
        let before = result;
        result = message;
        push_dry_stage(
            &mut stages,
            "match_output",
            &before,
            &result,
            true,
            "matched; short-circuited pipeline".into(),
        );
        return finish_dry_run(filter, output, result, stages);
    }
    push_dry_stage(
        &mut stages,
        "match_output",
        &result,
        &result,
        false,
        if filter.match_output.is_some() {
            "configured; no rule matched".into()
        } else {
            "disabled".into()
        },
    );

    let before = result.clone();
    let lines: Vec<&str> = result.lines().collect();
    let keep_lines = merged_patterns(&filter.keep_lines, &filter.keep_lines_matching);
    let strip_lines = merged_patterns(&filter.strip_lines, &filter.strip_lines_matching);
    let mut current_lines: Vec<String> = if !keep_lines.is_empty() {
        lines
            .iter()
            .filter(|line| matches_any(line, &keep_lines))
            .map(|line| (*line).to_string())
            .collect()
    } else if !strip_lines.is_empty() {
        lines
            .iter()
            .filter(|line| !matches_any(line, &strip_lines))
            .map(|line| (*line).to_string())
            .collect()
    } else {
        lines.iter().map(|line| (*line).to_string()).collect()
    };
    result = current_lines.join("\n");
    push_dry_stage(
        &mut stages,
        "line_filter",
        &before,
        &result,
        before != result,
        if !keep_lines.is_empty() {
            format!("kept lines matching {} pattern(s)", keep_lines.len())
        } else if !strip_lines.is_empty() {
            format!("stripped lines matching {} pattern(s)", strip_lines.len())
        } else {
            "disabled".into()
        },
    );

    let before = result.clone();
    if let Some(max_len) = filter.truncate_lines_at {
        current_lines = current_lines
            .into_iter()
            .map(|line| {
                if line.chars().count() > max_len {
                    crate::utils::truncate_chars(&line, max_len)
                } else {
                    line
                }
            })
            .collect();
    }
    result = current_lines.join("\n");
    push_dry_stage(
        &mut stages,
        "truncate_lines_at",
        &before,
        &result,
        before != result,
        filter
            .truncate_lines_at
            .map(|max| format!("max {max} chars per line"))
            .unwrap_or_else(|| "disabled".into()),
    );

    let before = result.clone();
    current_lines = slice_head_tail(current_lines, filter.head_lines, filter.tail_lines);
    result = current_lines.join("\n");
    push_dry_stage(
        &mut stages,
        "head_tail",
        &before,
        &result,
        before != result,
        match (filter.head_lines, filter.tail_lines) {
            (Some(h), Some(t)) => format!("head {h}, tail {t}"),
            (Some(h), None) => format!("head {h}"),
            (None, Some(t)) => format!("tail {t}"),
            (None, None) => "disabled".into(),
        },
    );

    let before = result.clone();
    if let Some(max) = filter.max_lines {
        let mut next: Vec<String> = current_lines.into_iter().take(max).collect();
        if line_count(&result) > max {
            next.push("...".into());
        }
        current_lines = next;
    }
    result = current_lines.join("\n");
    push_dry_stage(
        &mut stages,
        "max_lines",
        &before,
        &result,
        before != result,
        filter
            .max_lines
            .map(|max| format!("max {max} lines"))
            .unwrap_or_else(|| "disabled".into()),
    );

    let before = result.clone();
    if current_lines.is_empty() || current_lines.iter().all(|line| line.trim().is_empty()) {
        result = filter.on_empty.clone().unwrap_or_default();
    }
    push_dry_stage(
        &mut stages,
        "on_empty",
        &before,
        &result,
        before != result,
        if before.trim().is_empty() {
            "empty output fallback".into()
        } else {
            "non-empty output retained".into()
        },
    );

    finish_dry_run(filter, output, result, stages)
}
// END_dry_run_filter

fn slice_head_tail(lines: Vec<String>, head: Option<usize>, tail: Option<usize>) -> Vec<String> {
    match (head, tail) {
        (Some(h), Some(t)) => {
            let total = lines.len();
            let mut next = Vec::new();
            next.extend(lines.iter().take(h.min(total)).cloned());
            if h + t < total {
                next.push("...".into());
            }
            next.extend(lines.iter().skip(total.saturating_sub(t)).cloned());
            next
        }
        (Some(h), None) => lines.into_iter().take(h).collect(),
        (None, Some(t)) => {
            let total = lines.len();
            lines.into_iter().skip(total.saturating_sub(t)).collect()
        }
        (None, None) => lines,
    }
}

fn push_dry_stage(
    stages: &mut Vec<FilterDryRunStage>,
    name: &str,
    before: &str,
    after: &str,
    changed: bool,
    decision: String,
) {
    stages.push(FilterDryRunStage {
        name: name.into(),
        changed,
        before_lines: line_count(before),
        after_lines: line_count(after),
        decision,
    });
}

fn finish_dry_run(
    filter: &FilterDef,
    input: &str,
    output: String,
    stages: Vec<FilterDryRunStage>,
) -> FilterDryRunReport {
    FilterDryRunReport {
        filter_name: filter_display_name(filter),
        input_lines: line_count(input),
        output_lines: line_count(&output),
        stages,
        output,
    }
}

fn line_count(value: &str) -> usize {
    if value.is_empty() {
        0
    } else {
        value.lines().count()
    }
}

// END_public_api
