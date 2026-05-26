// MODULE_CONTRACT
// MODULE_ID: M-WORKSPACE
// PURPOSE: CLI command handlers for multi-project Synapse workspaces.
// SCOPE: workspace init/list/verify/status/index/coverage clap schemas, option resolution, report rendering, JSON output, and workspace engine dispatch.
// DEPENDS: M-CLI, M-WORKSPACE, M-CONFIG
// LINKS:
//   -> Phase-91 (implements) - workspace CLI
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - compact cross-project evidence

// START_MODULE_MAP
// WorkspaceCmd - Top-level workspace command schema
// WorkspaceAction - Workspace subcommand enum
// WorkspaceInitCmd - workspace init arguments
// WorkspaceVerifyCmd - workspace verify arguments
// WorkspaceIndexCmd - workspace index arguments
// WorkspaceJsonCmd - JSON flag schema for read-only workspace commands
// WorkspaceCmd::run - Dispatches workspace commands
// render_workspace_report - Renders aggregate report output
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added Phase-91 workspace CLI commands]
// END_CHANGE_SUMMARY

use crate::config::Config;
use crate::workspace::{parse_workspace_profile, Workspace, WorkspaceReport, WorkspaceRunOptions};
use clap::Subcommand;

// START_public_api

// START_WorkspaceCmd
#[derive(clap::Args)]
#[command(about = "Manage multi-project Synapse workspaces")]
pub struct WorkspaceCmd {
    #[command(subcommand)]
    pub action: WorkspaceAction,
}
// END_WorkspaceCmd

// START_WorkspaceAction
#[derive(Subcommand)]
pub enum WorkspaceAction {
    Init(WorkspaceInitCmd),
    List(WorkspaceJsonCmd),
    Verify(WorkspaceVerifyCmd),
    Status(WorkspaceJsonCmd),
    Index(WorkspaceIndexCmd),
    Coverage(WorkspaceJsonCmd),
}
// END_WorkspaceAction

// START_WorkspaceInitCmd
#[derive(clap::Args)]
pub struct WorkspaceInitCmd {
    pub members: Vec<String>,
    #[arg(long)]
    pub name: Option<String>,
}
// END_WorkspaceInitCmd

// START_WorkspaceVerifyCmd
#[derive(clap::Args)]
pub struct WorkspaceVerifyCmd {
    #[arg(long)]
    pub profile: Option<String>,
    #[arg(long)]
    pub sequential: bool,
    #[arg(long = "no-fail-fast")]
    pub no_fail_fast: bool,
    #[arg(long)]
    pub json: bool,
}
// END_WorkspaceVerifyCmd

// START_WorkspaceIndexCmd
#[derive(clap::Args)]
pub struct WorkspaceIndexCmd {
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub json: bool,
}
// END_WorkspaceIndexCmd

// START_WorkspaceJsonCmd
#[derive(clap::Args, Default)]
pub struct WorkspaceJsonCmd {
    #[arg(long)]
    pub json: bool,
}
// END_WorkspaceJsonCmd

impl WorkspaceCmd {
    // START_CONTRACT_WorkspaceCmd::run
    // PURPOSE: Dispatch workspace subcommands to the workspace engine.
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads or writes Synapse.toml, runs verification/status/index/coverage on members, prints reports
    // LINKS:
    //   -> Phase-91 (implements) - workspace CLI
    //   -> M-WORKSPACE (depends) - workspace engine
    //   -> NFR-002 (traces_to) - reliable release verification commands
    //   -> NFR-003 (traces_to) - compact cross-project evidence
    //   <- V-M-WORKSPACE (verified_by) - workspace CLI tests
    // START_workspace_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        match &self.action {
            WorkspaceAction::Init(cmd) => {
                let path = Workspace::init(&root, cmd.members.clone(), cmd.name.clone())?;
                println!("Workspace initialized at {}", path.display());
                Ok(())
            }
            WorkspaceAction::List(cmd) => {
                let workspace = Workspace::load(&root)?;
                if cmd.json {
                    println!("{}", serde_json::to_string_pretty(workspace.members())?);
                } else {
                    print_workspace_members(&workspace);
                }
                Ok(())
            }
            WorkspaceAction::Verify(cmd) => run_verify(&root, cmd).await,
            WorkspaceAction::Status(cmd) => {
                let report = Workspace::load(&root)?.status_all().await?;
                finish_report(report, cmd.json)
            }
            WorkspaceAction::Index(cmd) => {
                let report = Workspace::load(&root)?
                    .index_all(&config, cmd.force)
                    .await?;
                finish_report(report, cmd.json)
            }
            WorkspaceAction::Coverage(cmd) => {
                let report = Workspace::load(&root)?.coverage_all()?;
                finish_report(report, cmd.json)
            }
        }
    }
    // END_workspace_cmd_run
}

// END_public_api

// START_CONTRACT_run_verify
// PURPOSE: Resolve verify options and run workspace verification.
// INPUTS: { root: &Path }, { cmd: &WorkspaceVerifyCmd }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: runs member verification and prints aggregate reports
// LINKS:
//   -> Phase-91 (implements) - workspace verify CLI
//   -> NFR-002 (traces_to) - reliable release verification commands
//   <- V-M-WORKSPACE (verified_by) - workspace verify CLI tests
// START_run_verify
async fn run_verify(root: &std::path::Path, cmd: &WorkspaceVerifyCmd) -> anyhow::Result<()> {
    let workspace = Workspace::load(root)?;
    let profile = cmd
        .profile
        .as_deref()
        .unwrap_or(workspace.defaults().profile.as_str());
    let report = workspace
        .verify_all(WorkspaceRunOptions {
            profile: parse_workspace_profile(profile)?,
            parallel: workspace.defaults().parallel && !cmd.sequential,
            fail_fast: workspace.defaults().fail_fast && !cmd.no_fail_fast,
        })
        .await?;
    finish_report(report, cmd.json)
}
// END_run_verify

// START_CONTRACT_finish_report
// PURPOSE: Render a workspace report and return an error when member failures occurred.
// INPUTS: { report: WorkspaceReport }, { json: bool }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes stdout
// LINKS:
//   -> Phase-91 (implements) - workspace report rendering
//   -> NFR-003 (traces_to) - compact cross-project evidence
// START_finish_report
fn finish_report(report: WorkspaceReport, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render_workspace_report(&report));
    }
    if report.failed > 0 {
        anyhow::bail!(
            "{} workspace member(s) failed {}",
            report.failed,
            report.command
        );
    }
    Ok(())
}
// END_finish_report

// START_CONTRACT_print_workspace_members
// PURPOSE: Print resolved workspace members in a compact human-readable list.
// INPUTS: { workspace: &Workspace }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes stdout
// LINKS:
//   -> Phase-91 (implements) - workspace list CLI
// START_print_workspace_members
fn print_workspace_members(workspace: &Workspace) {
    println!("Workspace: {}", workspace.root().display());
    println!("Members ({}):", workspace.members().len());
    for (index, member) in workspace.members().iter().enumerate() {
        println!(
            "  {}. {} -> {}",
            index + 1,
            member.spec,
            member.path.display()
        );
    }
}
// END_print_workspace_members

// START_CONTRACT_render_workspace_report
// PURPOSE: Render aggregate workspace command outcomes as a stable text table.
// INPUTS: { report: &WorkspaceReport }
// OUTPUTS: { String }
// LINKS:
//   -> Phase-91 (implements) - workspace report rendering
//   -> NFR-003 (traces_to) - compact cross-project evidence
// START_render_workspace_report
pub fn render_workspace_report(report: &WorkspaceReport) -> String {
    let mut lines = vec![format!(
        "Workspace {}: {} passed, {} failed, {} total",
        report.command, report.passed, report.failed, report.total
    )];
    lines.push("Member\tStatus\tSummary".to_string());
    for member in &report.members {
        lines.push(format!(
            "{}\t{}\t{}",
            member.member,
            if member.passed { "PASS" } else { "FAIL" },
            member.summary
        ));
    }
    lines.join("\n")
}
// END_render_workspace_report

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::WorkspaceMemberReport;

    #[test]
    fn workspace_report_renderer_shows_member_results() {
        let mut report = WorkspaceReport::new("verify");
        report.push(WorkspaceMemberReport {
            member: "proj-a".to_string(),
            path: "/tmp/proj-a".to_string(),
            passed: true,
            summary: "3 levels".to_string(),
        });
        let rendered = render_workspace_report(&report);
        assert!(rendered.contains("Workspace verify: 1 passed, 0 failed, 1 total"));
        assert!(rendered.contains("proj-a\tPASS\t3 levels"));
    }

    #[test]
    fn workspace_verify_options_can_disable_fail_fast() {
        let cmd = WorkspaceVerifyCmd {
            profile: Some("standard".to_string()),
            sequential: true,
            no_fail_fast: true,
            json: false,
        };
        assert_eq!(cmd.profile.as_deref(), Some("standard"));
        assert!(cmd.sequential);
        assert!(cmd.no_fail_fast);
    }
}
