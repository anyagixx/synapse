// MODULE_CONTRACT
// MODULE_ID: M-CLI-FILTER-COMMANDS
// PURPOSE: CLI filter diagnostics — explains token-saving TOML filter behavior without command execution
// SCOPE: filters dry-run command execution, sample input loading from --input/--sample/stdin, before/after text rendering, JSON metrics rendering
// DEPENDS: M-CLI, M-PROXY-FILTER
// LINKS:
//   → M-CLI (depends) - receives clap command schema
//   → M-PROXY-FILTER (depends) - delegates filter selection and dry-run pipeline reports
//   → Phase-68 (implements) - filter dry-run UX
//   ← V-M-CLI-FILTER-COMMANDS (verified_by) - CLI dry-run rendering

// START_MODULE_MAP
// run_filter_dry_run — Executes the dry-run command without running the shell command
// read_sample_input — Loads inline, file, or stdin sample output
// render_filter_dry_run_report — Renders agent-readable stage decisions
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Initial filters dry-run command]
// END_CHANGE_SUMMARY

use super::FiltersDryRunCmd;
use crate::proxy::toml_filter::{FilterDryRunReport, FilterEngine};
use anyhow::Context;
use std::io::Read;

// START_public_api

// START_CONTRACT_run_filter_dry_run
// PURPOSE: Select a filter and explain how it transforms sample output without executing the command
// INPUTS: { cmd: &FiltersDryRunCmd }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: reads optional sample file or stdin and writes report to stdout
// LINKS:
//   → M-PROXY-FILTER (depends) - dry-run API
//   → NFR-003 (traces_to) - filter diagnostics support token-saving workflows
// START_run_filter_dry_run
pub(super) fn run_filter_dry_run(cmd: &FiltersDryRunCmd) -> anyhow::Result<()> {
    let sample = read_sample_input(cmd)?;
    if sample.is_empty() {
        anyhow::bail!("Input is empty — nothing to filter");
    }
    let engine = FilterEngine::new();
    let command_label = cmd.command.as_deref().unwrap_or("<explicit-filter>");
    let report = match (&cmd.filter, &cmd.command) {
        (Some(filter_name), command) => {
            engine.dry_run_for_command(command.as_deref().unwrap_or(""), Some(filter_name), &sample)
        }
        (None, Some(command)) => engine.dry_run_for_command(command, None, &sample),
        (None, None) => Err("provide --filter <name> or --command <cmd>".into()),
    }
    .map_err(anyhow::Error::msg)?;

    let format = if cmd.json {
        "json"
    } else {
        cmd.format.as_str()
    };
    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&render_filter_dry_run_json(
                &sample,
                &report,
                cmd.show_input
            ))?
        );
    } else if format == "text" {
        print!(
            "{}",
            render_filter_dry_run_report(command_label, &sample, &report, cmd.show_input)
        );
    } else {
        anyhow::bail!("unsupported dry-run format '{format}' (expected text or json)");
    }
    Ok(())
}
// END_run_filter_dry_run

// START_CONTRACT_read_sample_input
// PURPOSE: Load dry-run sample output from --sample, --input path, or stdin when --input is -
// INPUTS: { cmd: &FiltersDryRunCmd }
// OUTPUTS: { anyhow::Result<String> }
// SIDE_EFFECTS: may read filesystem or stdin
// LINKS:
//   → NFR-003 (traces_to) - sample loading enables token-saving filter validation
// START_read_sample_input
fn read_sample_input(cmd: &FiltersDryRunCmd) -> anyhow::Result<String> {
    if let Some(sample) = &cmd.sample {
        return Ok(sample.clone());
    }
    if cmd.input != "-" {
        return std::fs::read_to_string(&cmd.input)
            .with_context(|| format!("failed to read sample input file {}", cmd.input));
    }

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("failed to read sample output from stdin")?;
    Ok(input)
}
// END_read_sample_input

// START_CONTRACT_render_filter_dry_run_report
// PURPOSE: Render filter dry-run stages in a compact human-readable format
// INPUTS: { command: &str }, { input: &str }, { report: &FilterDryRunReport }, { show_input: bool }
// OUTPUTS: { String }
// LINKS:
//   → NFR-003 (traces_to) - readable stage decisions improve token-saving filter tuning
// START_render_filter_dry_run_report
fn render_filter_dry_run_report(
    command: &str,
    input: &str,
    report: &FilterDryRunReport,
    show_input: bool,
) -> String {
    let mut out = String::new();
    out.push_str("=== Synapse Filter Dry Run ===\n");
    out.push_str(&format!("command: {}\n", command));
    out.push_str(&format!("filter: {}\n", report.filter_name));
    out.push_str(&format!("input_lines: {}\n", report.input_lines));
    out.push_str(&format!("output_lines: {}\n", report.output_lines));
    for stage in &report.stages {
        let status = if stage.changed {
            "changed"
        } else {
            "unchanged"
        };
        out.push_str(&format!(
            "STAGE {} {} before_lines={} after_lines={} decision={}\n",
            stage.name, status, stage.before_lines, stage.after_lines, stage.decision
        ));
    }
    if show_input {
        out.push_str("--- input ---\n");
        out.push_str(input);
        if !input.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str("--- filtered output ---\n");
    out.push_str(&report.output);
    if !report.output.ends_with('\n') {
        out.push('\n');
    }
    out
}
// END_render_filter_dry_run_report

// START_CONTRACT_render_filter_dry_run_json
// PURPOSE: Render dry-run report as machine-readable JSON with token and byte reduction metrics
// INPUTS: { input: &str }, { report: &FilterDryRunReport }, { show_input: bool }
// OUTPUTS: { serde_json::Value }
// LINKS:
//   → NFR-003 (traces_to) - JSON metrics expose token-saving economics to agents
// START_render_filter_dry_run_json
fn render_filter_dry_run_json(
    input: &str,
    report: &FilterDryRunReport,
    show_input: bool,
) -> serde_json::Value {
    let input_tokens = crate::utils::estimate_tokens(input);
    let output_tokens = crate::utils::estimate_tokens(&report.output);
    let reduction_pct = if input.is_empty() {
        0.0
    } else {
        (1.0 - report.output.len() as f64 / input.len() as f64) * 100.0
    };

    serde_json::json!({
        "filter": &report.filter_name,
        "input_bytes": input.len(),
        "output_bytes": report.output.len(),
        "reduction_pct": reduction_pct,
        "input": if show_input { serde_json::Value::String(input.to_string()) } else { serde_json::Value::Null },
        "output": &report.output,
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        "stages": &report.stages,
    })
}
// END_render_filter_dry_run_json

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_sample_input_prefers_inline_sample() {
        let cmd = FiltersDryRunCmd {
            command: Some("unit".into()),
            filter: None,
            input: "-".into(),
            show_input: false,
            format: "text".into(),
            sample: Some("inline sample".into()),
            json: false,
        };

        assert_eq!(read_sample_input(&cmd).unwrap(), "inline sample");
    }

    #[test]
    fn test_render_filter_dry_run_report_includes_stage_decisions() {
        let toml = r#"
schema_version = 1

[filters.unit-dry]
match_command = "unit"
strip_ansi = true
keep_lines_matching = ["error|FAIL"]
max_lines = 1
"#;
        let engine = FilterEngine::from_toml(toml, "unit").expect("parse dry-run filter");
        let report = engine
            .dry_run_for_command(
                "unit run",
                None,
                "\u{1b}[31merror one\u{1b}[0m\nok\nFAIL two",
            )
            .expect("dry run");
        let input = "\u{1b}[31merror one\u{1b}[0m\nok\nFAIL two";
        let rendered = render_filter_dry_run_report("unit run", input, &report, true);

        assert!(rendered.contains("command: unit run"));
        assert!(rendered.contains("filter: unit-dry"));
        assert!(rendered.contains("STAGE strip_ansi changed"));
        assert!(rendered.contains("STAGE line_filter changed"));
        assert!(rendered.contains("STAGE max_lines changed"));
        assert!(rendered.contains("--- input ---"));
        assert!(rendered.contains("--- filtered output ---\nerror one\n..."));
    }

    #[test]
    fn test_render_filter_dry_run_json_reports_token_reduction() {
        let report = FilterDryRunReport {
            filter_name: "unit".into(),
            input_lines: 2,
            output_lines: 1,
            stages: Vec::new(),
            output: "short".into(),
        };
        let value = render_filter_dry_run_json("short\nvery long line", &report, false);

        assert_eq!(value["filter"], "unit");
        assert!(value["reduction_pct"].as_f64().unwrap() > 0.0);
        assert!(value["input"].is_null());
        assert_eq!(value["output"], "short");
    }
}

// END_public_api
