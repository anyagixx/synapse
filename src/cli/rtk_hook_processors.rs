// MODULE_CONTRACT
// MODULE_ID: M-HOOKS
// PURPOSE: RTK-style hook processors — consume agent hook JSON and return Synapse rewrite decisions
// SCOPE: HookCmd execution, Claude/Cursor/Gemini/Copilot processor aliases, dry-run check mode, JSON command extraction, rewrite decision rendering
// DEPENDS: M-CLI, M-CLI-RTK-COMMANDS, M-HOOK-OPENCODE-REWRITE
// LINKS:
//   -> M-CLI (depends) - exposes syn hook subcommands
//   -> M-CLI-RTK-COMMANDS (depends) - reuses rewrite_command source of truth
//   -> M-HOOKS (depends) - hook processing surface
//   -> Phase-55 (implements) - full agent hook parity
//   -> NFR-003 (traces_to) - auto-proxy token-saving adoption

// START_MODULE_MAP
// HookCmd::run - Dispatches hook processor subcommands
// process_hook_stdin - Reads hook JSON from stdin and emits rewrite/pass JSON
// hook_check - Prints a rewrite for a dry-run command
// extract_command - Finds a command string inside common agent hook payload shapes
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added Claude/Cursor/Gemini/Copilot hook processors]
// END_CHANGE_SUMMARY

use super::rtk_commands::rewrite_command;
use super::{HookCmd, HookProcessorAction};
use crate::config::Config;
use serde::Serialize;
use serde_json::Value;
use std::io::Read;

// START_public_api

impl HookCmd {
    // START_CONTRACT_HookCmd::run
    // PURPOSE: Dispatch RTK-style hook processors or dry-run rewrite checks
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads stdin for processor modes, writes stdout JSON or dry-run rewrite
    // LINKS:
    //   -> M-HOOKS (depends) - hook processor surface
    //   -> M-CLI-RTK-COMMANDS (depends) - rewrite source of truth
    //   -> Phase-55 (implements) - RTK hook processor parity
    //   -> NFR-003 (traces_to) - automatic token-saving rewrites
    // START_hook_cmd_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        match &self.command {
            HookProcessorAction::Claude => process_hook_stdin("claude"),
            HookProcessorAction::Cursor => process_hook_stdin("cursor"),
            HookProcessorAction::Gemini => process_hook_stdin("gemini"),
            HookProcessorAction::Copilot => process_hook_stdin("copilot"),
            HookProcessorAction::Check { agent, command } => hook_check(agent, command),
        }
    }
    // END_hook_cmd_run
}

// END_public_api

// START_HookResponse
#[derive(Serialize)]
struct HookResponse<'a> {
    agent: &'a str,
    decision: &'a str,
    command: Option<String>,
}
// END_HookResponse

// START_CONTRACT_process_hook_stdin
// PURPOSE: Read one JSON hook payload from stdin and emit a rewrite/pass response
// INPUTS: { agent: &str }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: reads stdin, writes stdout JSON
// LINKS:
//   -> M-HOOKS (depends) - hook processor runtime
//   -> Phase-55 (implements) - multi-agent hook parity
// START_process_hook_stdin
fn process_hook_stdin(agent: &str) -> anyhow::Result<()> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw)?;
    let command = serde_json::from_str::<Value>(&raw)
        .ok()
        .and_then(|value| extract_command(&value));
    let rewritten = command
        .as_deref()
        .and_then(|command| rewrite_command(&[command.to_string()]));
    let response = HookResponse {
        agent,
        decision: if rewritten.is_some() {
            "rewrite"
        } else {
            "pass"
        },
        command: rewritten,
    };
    println!("{}", serde_json::to_string(&response)?);
    Ok(())
}
// END_process_hook_stdin

// START_CONTRACT_hook_check
// PURPOSE: Print the rewrite for a dry-run hook command or fail when no rewrite is available
// INPUTS: { agent: &str }, { command: &[String] }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes stdout or returns failure
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - rewrite source of truth
//   -> Phase-55 (implements) - hook check parity
// START_hook_check
fn hook_check(agent: &str, command: &[String]) -> anyhow::Result<()> {
    if command.is_empty() {
        anyhow::bail!("Usage: syn hook check [--agent <agent>] <command> [args...]");
    }
    let Some(rewritten) = rewrite_command(command) else {
        anyhow::bail!("no rewrite available for {agent} hook command");
    };
    println!("{rewritten}");
    Ok(())
}
// END_hook_check

// START_CONTRACT_extract_command
// PURPOSE: Extract a shell command from common agent hook JSON shapes
// INPUTS: { value: &Value }
// OUTPUTS: { Option<String> }
// LINKS:
//   -> M-HOOKS (depends) - hook payload compatibility
//   -> Phase-55 (implements) - Claude/Cursor/Gemini/Copilot processors
// START_extract_command
fn extract_command(value: &Value) -> Option<String> {
    for path in [
        &["tool_input", "command"][..],
        &["tool_input", "cmd"][..],
        &["input", "command"][..],
        &["args", "command"][..],
        &["command"][..],
        &["cmd"][..],
    ] {
        if let Some(command) = value_at_path(value, path).and_then(Value::as_str) {
            if !command.trim().is_empty() {
                return Some(command.trim().to_string());
            }
        }
    }
    None
}
// END_extract_command

// START_CONTRACT_value_at_path
// PURPOSE: Return a nested JSON value by object path
// INPUTS: { value: &Value }, { path: &[&str] }
// OUTPUTS: { Option<&Value> }
// LINKS:
//   -> M-HOOKS (depends) - hook JSON extraction
//   -> Phase-55 (implements) - hook processor compatibility
// START_value_at_path
fn value_at_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}
// END_value_at_path

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_command_reads_common_hook_shapes() {
        let value: Value = serde_json::json!({
            "tool_input": {
                "command": "git status"
            }
        });

        assert_eq!(extract_command(&value), Some("git status".to_string()));
    }

    #[test]
    fn hook_check_rewrites_routeable_commands() {
        let command = vec!["git".to_string(), "status".to_string()];
        let rewritten = rewrite_command(&command).expect("rewrite");

        assert_eq!(rewritten, "syn proxy -- git status");
    }
}
