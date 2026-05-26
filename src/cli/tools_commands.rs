// MODULE_CONTRACT
// MODULE_ID: M-MCP-USER-TOOLS
// PURPOSE: CLI authoring and validation commands for local user-defined MCP tools.
// SCOPE: syn tools list, validate, and new command schemas, text/JSON rendering, template writing, and validation failure exits.
// DEPENDS: M-CLI, M-MCP-USER-TOOLS, M-CONFIG
// LINKS:
//   -> docs/phases/Phase-92.xml (implements) - UPGRADE_5 user tool CLI
//   -> NFR-002 (traces_to) - safe local tool authoring
//   <- V-M-MCP-USER-TOOLS (verified_by) - CLI user tool tests

// START_MODULE_MAP
// ToolsCmd - Top-level tools command schema
// ToolsAction - tools subcommand enum
// ToolsListCmd - list command arguments
// ToolsValidateCmd - validate command arguments
// ToolsNewCmd - new command arguments
// ToolsCmd::run - Dispatches user tool CLI commands
// validate_path - Validates one user tool file or directory
// write_new_tool - Writes a starter JSON tool definition
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added Phase-92 syn tools CLI commands]
// END_CHANGE_SUMMARY

use crate::config::Config;
use crate::mcp::user_tools;
use clap::Subcommand;
use serde::Serialize;
use std::path::{Path, PathBuf};

// START_public_api

// START_ToolsCmd
#[derive(clap::Args)]
#[command(about = "Manage local user-defined MCP tools")]
pub struct ToolsCmd {
    #[command(subcommand)]
    pub action: ToolsAction,
}
// END_ToolsCmd

// START_ToolsAction
#[derive(Subcommand)]
pub enum ToolsAction {
    List(ToolsListCmd),
    Validate(ToolsValidateCmd),
    New(ToolsNewCmd),
}
// END_ToolsAction

// START_ToolsListCmd
#[derive(clap::Args, Default)]
pub struct ToolsListCmd {
    #[arg(long)]
    pub json: bool,
}
// END_ToolsListCmd

// START_ToolsValidateCmd
#[derive(clap::Args)]
pub struct ToolsValidateCmd {
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}
// END_ToolsValidateCmd

// START_ToolsNewCmd
#[derive(clap::Args)]
pub struct ToolsNewCmd {
    pub name: String,
    #[arg(long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub json: bool,
}
// END_ToolsNewCmd

// START_ToolsValidationSummary
#[derive(Debug, Serialize)]
struct ToolsValidationSummary {
    path: PathBuf,
    valid: bool,
    reports: Vec<user_tools::UserToolValidationReport>,
}
// END_ToolsValidationSummary

// START_ToolsListSummary
#[derive(Debug, Serialize)]
struct ToolsListSummary {
    directory: PathBuf,
    tools: Vec<ToolsListItem>,
    warnings: Vec<String>,
}
// END_ToolsListSummary

// START_ToolsListItem
#[derive(Debug, Serialize)]
struct ToolsListItem {
    name: String,
    version: String,
    description: String,
}
// END_ToolsListItem

impl ToolsCmd {
    // START_CONTRACT_ToolsCmd::run
    // PURPOSE: Dispatch user tool management commands.
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads ~/.synapse/tools, writes JSON templates, prints text or JSON
    // START_tools_cmd_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        match &self.action {
            ToolsAction::List(cmd) => run_list(cmd),
            ToolsAction::Validate(cmd) => run_validate(cmd),
            ToolsAction::New(cmd) => run_new(cmd),
        }
    }
    // END_tools_cmd_run
}

// END_public_api

// START_CONTRACT_run_list
// PURPOSE: Print valid local user tools and validation warnings.
// INPUTS: { cmd: &ToolsListCmd }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: reads user tool files and writes stdout/stderr
// START_run_list
fn run_list(cmd: &ToolsListCmd) -> anyhow::Result<()> {
    let directory = user_tools::default_tools_dir();
    let report = user_tools::load_from_dir(&directory, &user_tools::current_builtin_tool_names());
    let summary = ToolsListSummary {
        directory: directory.clone(),
        tools: report
            .tools
            .iter()
            .map(|tool| ToolsListItem {
                name: tool.name.clone(),
                version: tool.version.clone(),
                description: tool.description.clone(),
            })
            .collect(),
        warnings: report.warnings.clone(),
    };
    if cmd.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }
    println!("User tools directory: {}", directory.display());
    if summary.tools.is_empty() {
        println!("No valid user tools found.");
    } else {
        for tool in &summary.tools {
            println!("- {} v{} - {}", tool.name, tool.version, tool.description);
        }
    }
    for warning in &summary.warnings {
        eprintln!("warning: {warning}");
    }
    Ok(())
}
// END_run_list

// START_CONTRACT_run_validate
// PURPOSE: Validate a user tool file or directory and fail on invalid definitions.
// INPUTS: { cmd: &ToolsValidateCmd }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: reads user tool files and writes stdout/stderr
// START_run_validate
fn run_validate(cmd: &ToolsValidateCmd) -> anyhow::Result<()> {
    let path = cmd
        .path
        .clone()
        .unwrap_or_else(user_tools::default_tools_dir);
    let summary = validate_path(&path);
    if cmd.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_validation_summary(&summary);
    }
    if summary.valid {
        Ok(())
    } else {
        anyhow::bail!("user tool validation failed")
    }
}
// END_run_validate

// START_CONTRACT_run_new
// PURPOSE: Write a starter user tool JSON file.
// INPUTS: { cmd: &ToolsNewCmd }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: creates parent directories and writes one JSON file
// START_run_new
fn run_new(cmd: &ToolsNewCmd) -> anyhow::Result<()> {
    let path = write_new_tool(cmd)?;
    if cmd.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "created": path,
                "name": cmd.name
            }))?
        );
    } else {
        println!("Created user tool template: {}", path.display());
    }
    Ok(())
}
// END_run_new

// START_CONTRACT_validate_path
// PURPOSE: Validate one JSON file, all JSON files in a directory, or a missing target.
// INPUTS: { path: &Path }
// OUTPUTS: { ToolsValidationSummary }
// SIDE_EFFECTS: reads user tool files from disk
// START_validate_path
fn validate_path(path: &Path) -> ToolsValidationSummary {
    let builtin_names = user_tools::current_builtin_tool_names();
    if path.is_dir() {
        let reports = match std::fs::read_dir(path) {
            Ok(entries) => {
                let mut paths = entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.extension().and_then(|value| value.to_str()) == Some("json")
                    })
                    .collect::<Vec<_>>();
                paths.sort();
                paths
                    .iter()
                    .map(|path| user_tools::validate_file(path, &builtin_names))
                    .collect()
            }
            Err(error) => vec![user_tools::UserToolValidationReport {
                path: path.to_path_buf(),
                valid: false,
                tool_name: None,
                errors: vec![format!("cannot read directory: {error}")],
            }],
        };
        return ToolsValidationSummary {
            path: path.to_path_buf(),
            valid: reports.iter().all(|report| report.valid),
            reports,
        };
    }
    if !path.exists() {
        return ToolsValidationSummary {
            path: path.to_path_buf(),
            valid: false,
            reports: vec![user_tools::UserToolValidationReport {
                path: path.to_path_buf(),
                valid: false,
                tool_name: None,
                errors: vec!["path does not exist".into()],
            }],
        };
    }
    let report = user_tools::validate_file(path, &builtin_names);
    ToolsValidationSummary {
        path: path.to_path_buf(),
        valid: report.valid,
        reports: vec![report],
    }
}
// END_validate_path

// START_CONTRACT_write_new_tool
// PURPOSE: Validate and write a starter user tool JSON template.
// INPUTS: { cmd: &ToolsNewCmd }
// OUTPUTS: { anyhow::Result<PathBuf> }
// SIDE_EFFECTS: creates directories and writes a JSON file
// START_write_new_tool
fn write_new_tool(cmd: &ToolsNewCmd) -> anyhow::Result<PathBuf> {
    let value = user_tools::new_tool_template(&cmd.name);
    let parsed: user_tools::UserToolDefinition = serde_json::from_value(value.clone())?;
    let errors =
        user_tools::validate_definition(&parsed, &user_tools::current_builtin_tool_names());
    if !errors.is_empty() {
        anyhow::bail!("invalid user tool template: {}", errors.join("; "));
    }
    let path = cmd
        .output
        .clone()
        .unwrap_or_else(|| user_tools::default_tools_dir().join(format!("{}.json", cmd.name)));
    if path.exists() && !cmd.force {
        anyhow::bail!(
            "{} already exists; pass --force to overwrite",
            path.display()
        );
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&value)? + "\n")?;
    Ok(path)
}
// END_write_new_tool

// START_CONTRACT_print_validation_summary
// PURPOSE: Render validation results in compact human-readable form.
// INPUTS: { summary: &ToolsValidationSummary }
// SIDE_EFFECTS: writes stdout
// START_print_validation_summary
fn print_validation_summary(summary: &ToolsValidationSummary) {
    for report in &summary.reports {
        let label = report
            .tool_name
            .as_deref()
            .unwrap_or_else(|| report.path.to_str().unwrap_or("<tool>"));
        if report.valid {
            println!("ok: {label}");
        } else {
            println!("invalid: {label}");
            for error in &report.errors {
                println!("  - {error}");
            }
        }
    }
}
// END_print_validation_summary

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // START_CONTRACT_validate_path_rejects_builtin_override
    // PURPOSE: Verify CLI validation rejects user tools that override built-in tool names.
    // START_validate_path_rejects_builtin_override
    #[test]
    fn validate_path_rejects_builtin_override() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&serde_json::json!({
                "name": "semantic_search",
                "description": "Bad override",
                "version": "0.1.0",
                "inputSchema": {"type": "object"},
                "handler": {"type": "command", "command": "echo ok"}
            }))
            .unwrap(),
        )
        .unwrap();

        let summary = validate_path(&path);

        assert!(!summary.valid);
        assert!(summary.reports[0]
            .errors
            .iter()
            .any(|error| error.contains("overrides a built-in")));
    }
    // END_validate_path_rejects_builtin_override

    // START_CONTRACT_write_new_tool_creates_valid_template
    // PURPOSE: Verify syn tools new writes a valid starter definition.
    // START_write_new_tool_creates_valid_template
    #[test]
    fn write_new_tool_creates_valid_template() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local_echo.json");
        let cmd = ToolsNewCmd {
            name: "local_echo".into(),
            output: Some(path.clone()),
            force: false,
            json: false,
        };

        let written = write_new_tool(&cmd).expect("write template");
        let summary = validate_path(&written);

        assert_eq!(written, path);
        assert!(summary.valid, "{:?}", summary.reports);
    }
    // END_write_new_tool_creates_valid_template
}
