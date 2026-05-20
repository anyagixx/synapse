// MODULE_CONTRACT
// MODULE_ID: M-PROXY-RUNNER
// PURPOSE: Command runner — executes shell commands and captures stdout/stderr with exit status
// SCOPE: CommandRunner struct, CommandOutput status model, execute via std::process::Command
// DEPENDS: N/A
// LINKS: N/A

// START_MODULE_MAP
// CommandOutput — Captured command output plus success and exit status
// CommandRunner — Executes a shell command and returns combined stdout/stderr
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.1.0 — Preserve child process exit status for proxy callers]
// END_CHANGE_SUMMARY

use std::process::{Command, Stdio};

// START_public_api

// START_CommandOutput
pub struct CommandOutput {
    pub text: String,
    pub status_code: i32,
    pub success: bool,
}
// END_CommandOutput

// START_CommandRunner
pub struct CommandRunner {
    cmd: String,
    args: Vec<String>,
}
// END_CommandRunner

impl CommandRunner {
    // START_CONTRACT_CommandRunner::new
    // PURPOSE: Create a CommandRunner from command parts
    // INPUTS: { parts: &[String] — command and arguments }
    // OUTPUTS: { Self }
    // START_runner_new
    pub fn new(parts: &[String]) -> Self {
        let cmd = parts[0].clone();
        let args = if parts.len() > 1 {
            parts[1..].to_vec()
        } else {
            Vec::new()
        };
        Self { cmd, args }
    }
    // END_runner_new

    // START_CONTRACT_CommandRunner::execute
    // PURPOSE: Execute the command and return combined stdout/stderr plus exit status
    // OUTPUTS: { anyhow::Result<CommandOutput> — command output and status }
    // SIDE_EFFECTS: spawns child process
    // START_runner_execute
    pub fn execute(&self) -> anyhow::Result<CommandOutput> {
        let output = Command::new(&self.cmd)
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to execute '{}': {}", self.cmd, e))?;

        let result = output.wait_with_output()?;
        let status_code = result.status.code().unwrap_or(1);
        let success = result.status.success();

        let mut text = String::new();

        // Prefer stdout, but include stderr if non-empty
        if !result.stdout.is_empty() {
            text.push_str(&String::from_utf8_lossy(&result.stdout));
        }
        if !result.stderr.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&String::from_utf8_lossy(&result.stderr));
        }

        if text.is_empty() {
            if success {
                text = "ok".into();
            } else {
                text = format!("Exit code: {}", status_code);
            }
        }

        Ok(CommandOutput {
            text,
            status_code,
            success,
        })
    }
    // END_runner_execute
}
// END_public_api
