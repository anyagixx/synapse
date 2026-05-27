// MODULE_CONTRACT
// MODULE_ID: M-CLI-RTK-COMMANDS
// PURPOSE: Core RTK-style adapters — compact command error, test failure, diff, and text summary output
// SCOPE: ErrCmd, legacy TestCmd fallback, DiffCmd, and SummaryCmd execution; compact renderers; adapter-level token tracking for core RTK commands
// DEPENDS: M-CONFIG, M-PROXY-RUNNER, M-TRACKING, M-UTILS
// LINKS:
//   -> M-CLI (depends) - exposes core RTK command schemas
//   -> M-PROXY-RUNNER (depends) - executes err/test command captures with RTK raw caps
//   -> M-TRACKING (depends) - records adapter-level token economy
//   -> Phase-53 (implements) - core RTK command adapter parity
//   -> NFR-003 (traces_to) - token-saving compact output

// START_MODULE_MAP
// ErrCmd::run - Runs a command and keeps only error/warning lines
// run_legacy_test_command - Runs a test command and keeps failure-oriented lines
// DiffCmd::run - Summarizes file-to-file or unified diff input
// SummaryCmd::run - Summarizes file/stdin text without dumping full content
// render_matching_lines - Shared keyword line filter
// render_diff_summary - Compact diff renderer
// render_text_summary - Compact text summary renderer
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.2.0 - Expose legacy test fallback for structured syn test dispatcher]
// END_CHANGE_SUMMARY

use super::{DiffCmd, ErrCmd, SummaryCmd};
use syn_core::config::Config;
use syn_proxy::proxy::runner::CommandRunner;
use std::io::Read;

const MAX_MATCH_LINES: usize = 200;
const MAX_SUMMARY_LINES: usize = 12;
const MAX_FALLBACK_CHARS: usize = 8000;

// START_public_api

impl ErrCmd {
    // START_CONTRACT_ErrCmd::run
    // PURPOSE: Run a command and print only error/warning lines or a compact fallback
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: executes a child command, writes stdout, records token savings, exits with child status on failure
    // LINKS:
    //   -> M-PROXY-RUNNER (depends) - capped command capture
    //   -> M-TRACKING (depends) - adapter savings
    //   -> Phase-53 (implements) - err command parity
    //   -> NFR-003 (traces_to) - compact error output
    // START_err_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        if self.command.is_empty() {
            anyhow::bail!("Usage: syn err -- <command> [args...]");
        }
        let output = CommandRunner::from_config(&self.command, &config)
            .execute()
            .await?;
        let rendered = render_matching_lines(
            &output.text,
            &[
                "error",
                "warning",
                "warn",
                "failed",
                "failure",
                "panic",
                "exception",
            ],
            if output.success { "ok" } else { "" },
        );
        print_and_track_command(CommandAdapterOutput {
            config: &config,
            command: &format!("syn err {}", self.command.join(" ")),
            raw: &output.text,
            rendered: &rendered,
            adapter: "rtk-err",
            route_key: "syn err",
            status_code: output.status_code,
            success: output.success,
        })
        .await
    }
    // END_err_cmd_run
}

impl DiffCmd {
    // START_CONTRACT_DiffCmd::run
    // PURPOSE: Summarize file-to-file or unified diff input without dumping unchanged content
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads files/stdin, writes stdout, records token savings
    // LINKS:
    //   -> M-TRACKING (depends) - adapter savings
    //   -> Phase-53 (implements) - diff command parity
    //   -> NFR-003 (traces_to) - compact diff output
    // START_diff_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let raw = match (&self.file1, &self.file2) {
            (Some(left), Some(right)) => diff_two_files(left, right)?,
            (Some(path), None) => std::fs::read_to_string(path)?,
            (None, None) => read_stdin()?,
            (None, Some(_)) => anyhow::bail!("Usage: syn diff [file1 file2] or stdin unified diff"),
        };
        let rendered = render_diff_summary(&raw);
        println!("{rendered}");
        super::rtk_adapters::record_adapter_savings(
            &config, "syn diff", &raw, &rendered, "rtk-diff", "syn diff",
        )
        .await;
        Ok(())
    }
    // END_diff_cmd_run
}

impl SummaryCmd {
    // START_CONTRACT_SummaryCmd::run
    // PURPOSE: Summarize text from a file or stdin without printing full content
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads file/stdin, writes stdout, records token savings
    // LINKS:
    //   -> M-TRACKING (depends) - adapter savings
    //   -> Phase-53 (implements) - summary command parity
    //   -> NFR-003 (traces_to) - compact summary output
    // START_summary_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let raw = match &self.input {
            Some(path) => std::fs::read_to_string(path)?,
            None => read_stdin()?,
        };
        let rendered = render_text_summary(&raw);
        println!("{rendered}");
        super::rtk_adapters::record_adapter_savings(
            &config,
            "syn summary",
            &raw,
            &rendered,
            "rtk-summary",
            "syn summary",
        )
        .await;
        Ok(())
    }
    // END_summary_cmd_run
}

// END_public_api

// START_CONTRACT_run_legacy_test_command
// PURPOSE: Run a legacy syn test command through the RTK compact failure-oriented adapter
// INPUTS: { command: &[String] }, { config: Config }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: executes a child command, writes stdout, records token savings, exits with child status on failure
// LINKS:
//   -> M-CLI-TEST-COMMANDS (depends) - structured dispatcher delegates legacy commands here
//   -> M-PROXY-RUNNER (depends) - capped command capture
//   -> M-TRACKING (depends) - adapter savings
//   -> Phase-53 (implements) - test command parity
//   -> NFR-003 (traces_to) - compact test output
// <LOG id="legacy_rtk_test_fallback" level="INFO" ref="legacy-test-fallback" module="M-CLI-RTK-COMMANDS" contract="run_legacy_test_command">
//   EVENT: legacy_rtk_test_fallback
//   EXPECTATION: unknown or delimiter-routed syn test commands keep RTK compact output
//   DECISION: structured test dispatcher delegated an external command variant
//   RESULT: success
//   TRACEABILITY: NFR-003
// </LOG>
// START_run_legacy_test_command
pub(super) async fn run_legacy_test_command(
    command: &[String],
    config: Config,
) -> anyhow::Result<()> {
    if command.is_empty() {
        anyhow::bail!("Usage: syn test -- <test-command> [args...]");
    }
    let output = CommandRunner::from_config(command, &config)
        .execute()
        .await?;
    let rendered = render_matching_lines(
        &output.text,
        &[
            "fail",
            "failed",
            "failure",
            "error",
            "panicked",
            "assertion",
            "traceback",
        ],
        if output.success { "tests passed" } else { "" },
    );
    print_and_track_command(CommandAdapterOutput {
        config: &config,
        command: &format!("syn test {}", command.join(" ")),
        raw: &output.text,
        rendered: &rendered,
        adapter: "rtk-test",
        route_key: "syn test",
        status_code: output.status_code,
        success: output.success,
    })
    .await
}
// END_run_legacy_test_command

// START_CommandAdapterOutput
struct CommandAdapterOutput<'a> {
    config: &'a Config,
    command: &'a str,
    raw: &'a str,
    rendered: &'a str,
    adapter: &'a str,
    route_key: &'a str,
    status_code: i32,
    success: bool,
}
// END_CommandAdapterOutput

// START_CONTRACT_print_and_track_command
// PURPOSE: Print rendered adapter output, record token savings, and preserve child exit status
// INPUTS: { output: CommandAdapterOutput }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes stdout, writes tracking database, may exit current process
// LINKS:
//   -> M-TRACKING (depends) - adapter savings
//   -> M-PROXY-RUNNER (depends) - child status propagation
//   -> NFR-003 (traces_to) - token-saving accounting
// START_print_and_track_command
async fn print_and_track_command(output: CommandAdapterOutput<'_>) -> anyhow::Result<()> {
    println!("{}", output.rendered);
    super::rtk_adapters::record_adapter_savings(
        output.config,
        output.command,
        output.raw,
        output.rendered,
        output.adapter,
        output.route_key,
    )
    .await;
    if !output.success {
        std::process::exit(output.status_code);
    }
    Ok(())
}
// END_print_and_track_command

// START_CONTRACT_render_matching_lines
// PURPOSE: Keep lines containing any keyword and return a compact fallback when there are no matches
// INPUTS: { raw: &str }, { keywords: &[&str] }, { success_fallback: &str }
// OUTPUTS: { String }
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - err/test compact renderers
//   -> NFR-003 (traces_to) - token-saving output filter
// START_render_matching_lines
fn render_matching_lines(raw: &str, keywords: &[&str], success_fallback: &str) -> String {
    let mut lines = Vec::new();
    for line in raw.lines() {
        let lower = line.to_ascii_lowercase();
        if keywords.iter().any(|keyword| lower.contains(keyword)) {
            lines.push(line.trim_end().to_string());
            if lines.len() >= MAX_MATCH_LINES {
                lines.push(format!("[truncated at {MAX_MATCH_LINES} matching lines]"));
                break;
            }
        }
    }
    if !lines.is_empty() {
        return lines.join("\n");
    }
    if !success_fallback.is_empty() {
        return success_fallback.into();
    }
    syn_core::utils::truncate_chars(raw, MAX_FALLBACK_CHARS)
}
// END_render_matching_lines

// START_CONTRACT_render_diff_summary
// PURPOSE: Summarize unified or synthetic diff text into changed lines and hunks
// INPUTS: { raw: &str }
// OUTPUTS: { String }
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - diff compact renderer
//   -> NFR-003 (traces_to) - compact diff output
// START_render_diff_summary
fn render_diff_summary(raw: &str) -> String {
    let mut additions = 0_usize;
    let mut deletions = 0_usize;
    let mut hunks = Vec::new();
    for line in raw.lines() {
        if line.starts_with("@@") {
            hunks.push(line.to_string());
        } else if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
            if hunks.len() < MAX_MATCH_LINES {
                hunks.push(line.to_string());
            }
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
            if hunks.len() < MAX_MATCH_LINES {
                hunks.push(line.to_string());
            }
        }
    }
    if additions == 0 && deletions == 0 {
        return "No changed lines detected".into();
    }
    let mut output = format!("Diff summary: +{additions} -{deletions}");
    if !hunks.is_empty() {
        output.push('\n');
        output.push_str(&hunks.join("\n"));
    }
    output
}
// END_render_diff_summary

// START_CONTRACT_render_text_summary
// PURPOSE: Summarize text by counts and representative non-empty lines
// INPUTS: { raw: &str }
// OUTPUTS: { String }
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - summary compact renderer
//   -> NFR-003 (traces_to) - compact text output
// START_render_text_summary
fn render_text_summary(raw: &str) -> String {
    let line_count = raw.lines().count();
    let byte_count = raw.len();
    let non_empty: Vec<&str> = raw.lines().filter(|line| !line.trim().is_empty()).collect();
    let mut output = format!(
        "Summary: {line_count} lines, {byte_count} bytes, {} non-empty lines",
        non_empty.len()
    );
    for line in non_empty.into_iter().take(MAX_SUMMARY_LINES) {
        output.push('\n');
        output.push_str("- ");
        output.push_str(&syn_core::utils::truncate_chars(line.trim(), 160));
    }
    output
}
// END_render_text_summary

// START_CONTRACT_diff_two_files
// PURPOSE: Build a compact synthetic line diff for two text files
// INPUTS: { left: &Path }, { right: &Path }
// OUTPUTS: { anyhow::Result<String> }
// SIDE_EFFECTS: reads files
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - diff file mode
//   -> NFR-003 (traces_to) - compact diff input generation
// START_diff_two_files
fn diff_two_files(left: &std::path::Path, right: &std::path::Path) -> anyhow::Result<String> {
    let left_text = std::fs::read_to_string(left)?;
    let right_text = std::fs::read_to_string(right)?;
    let left_lines: Vec<&str> = left_text.lines().collect();
    let right_lines: Vec<&str> = right_text.lines().collect();
    let mut diff = String::new();
    let max = left_lines.len().max(right_lines.len());
    for index in 0..max {
        let left_line = left_lines.get(index).copied();
        let right_line = right_lines.get(index).copied();
        if left_line == right_line {
            continue;
        }
        if let Some(line) = left_line {
            diff.push_str(&format!("-{}: {}\n", index + 1, line));
        }
        if let Some(line) = right_line {
            diff.push_str(&format!("+{}: {}\n", index + 1, line));
        }
    }
    Ok(diff)
}
// END_diff_two_files

// START_CONTRACT_read_stdin
// PURPOSE: Read stdin to string for local core adapters
// OUTPUTS: { anyhow::Result<String> }
// SIDE_EFFECTS: reads stdin
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - stdin adapter mode
//   -> NFR-003 (traces_to) - compact stdin processing
// START_read_stdin
fn read_stdin() -> anyhow::Result<String> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw)?;
    Ok(raw)
}
// END_read_stdin

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn err_renderer_keeps_error_warning_lines() {
        let raw = "ok\nwarning: unused\nmore\nerror: failed\n";
        let rendered = render_matching_lines(raw, &["warning", "error"], "ok");
        assert_eq!(rendered, "warning: unused\nerror: failed");
    }

    #[test]
    fn diff_renderer_summarizes_changed_lines() {
        let rendered = render_diff_summary("--- a\n+++ b\n@@ -1 +1\n-old\n+new\n");
        assert!(rendered.contains("Diff summary: +1 -1"));
        assert!(rendered.contains("-old"));
        assert!(rendered.contains("+new"));
    }

    #[test]
    fn summary_renderer_reports_counts_and_samples() {
        let rendered = render_text_summary("alpha\n\nbeta\n");
        assert!(rendered.contains("3 lines"));
        assert!(rendered.contains("2 non-empty"));
        assert!(rendered.contains("- alpha"));
    }
}
