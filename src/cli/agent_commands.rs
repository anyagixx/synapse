// MODULE_CONTRACT
// MODULE_ID: M-CLI-AGENT-COMMANDS
// PURPOSE: CLI agent resume/status commands backed by compact run agent context.
// SCOPE: AgentCmd dispatch, resume/status text rendering, JSON output, and run-id selection.
// DEPENDS: M-RUNNER, M-RUNNER-AGENT-CONTEXT
// LINKS:
//   -> M-RUNNER (depends) - loads persisted run context
//   -> M-RUNNER-AGENT-CONTEXT (depends) - builds compact resume payload
//   -> UC-002 (implements) - agents can resume bounded work from CLI
//   -> NFR-003 (traces_to) - CLI resume avoids reading full run state manually

// START_MODULE_MAP
// AgentCmd::run - Dispatches agent resume/status commands
// print_agent_context - Renders compact text output
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 - Allowed agent resume/status to report latest terminal run context]
// END_CHANGE_SUMMARY

use super::{AgentAction, AgentCmd, AgentContextCmd};
use crate::config::Config;
use crate::run::agent_context::AgentContext;
use crate::run::RunManager;

// START_public_api

impl AgentCmd {
    // START_CONTRACT_AgentCmd::run
    // PURPOSE: Run agent resume or status command.
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_agent_cmd_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        match &self.action {
            AgentAction::Resume(cmd) | AgentAction::Status(cmd) => run_agent_context(cmd),
        }
    }
    // END_agent_cmd_run
}

// END_public_api

// START_CONTRACT_run_agent_context
// PURPOSE: Build and print agent context for a selected or latest incomplete run.
// INPUTS: { cmd: &AgentContextCmd }
// OUTPUTS: { anyhow::Result<()> }
// START_run_agent_context
fn run_agent_context(cmd: &AgentContextCmd) -> anyhow::Result<()> {
    let manager = RunManager::new(std::env::current_dir()?);
    let Some(context) = manager.build_agent_context(cmd.run_id.as_deref())? else {
        anyhow::bail!("no run context found; pass --run-id or create a run");
    };
    if cmd.json {
        println!("{}", serde_json::to_string_pretty(&context)?);
    } else {
        print_agent_context(&context);
    }
    Ok(())
}
// END_run_agent_context

// START_CONTRACT_print_agent_context
// PURPOSE: Render compact agent context for terminal resume workflows.
// INPUTS: { context: &AgentContext }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes to stdout
// START_print_agent_context
fn print_agent_context(context: &AgentContext) {
    println!("run_id:     {}", context.run_id);
    println!("status:     {}", context.status);
    println!("phase:      {}", context.phase);
    println!("module:     {}", context.module_id);
    println!("objective:  {}", context.objective);
    if let Some(next_action) = &context.next_action {
        println!("next:       {}", next_action);
    }
    if let Some(belief) = &context.belief_state_ref {
        println!("belief:     {}", belief);
    }
    if let Some(handoff) = &context.latest_handoff {
        println!(
            "handoff:    {} -> {:?}",
            handoff.handoff_id, handoff.to_role
        );
    }
    println!("summary:    {}", context.compact_summary);
}
// END_print_agent_context

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{RunRecord, RunStatus};

    // START_CONTRACT_test_print_agent_context_includes_resume_fields
    // PURPOSE: Verify text rendering includes stable resume fields.
    // START_test_print_agent_context_includes_resume_fields
    #[test]
    fn test_print_agent_context_includes_resume_fields() {
        let context = AgentContext {
            run_id: "run-1".into(),
            goal: "goal".into(),
            phase: "Phase-74".into(),
            module_id: "M-RUNNER".into(),
            objective: "resume".into(),
            status: "Blocked".into(),
            current_step: 1,
            next_action: Some("fix".into()),
            evidence_refs: vec!["verify.log".into()],
            belief_state_ref: Some("docs/belief-states/M-RUNNER.xml".into()),
            latest_handoff: None,
            compact_summary: "run-1 blocked".into(),
        };

        print_agent_context(&context);
    }
    // END_test_print_agent_context_includes_resume_fields

    // START_CONTRACT_test_run_agent_context_json_uses_latest_incomplete_run
    // PURPOSE: Verify agent command can resolve latest incomplete run for JSON output.
    // START_test_run_agent_context_json_uses_latest_incomplete_run
    #[tokio::test]
    async fn test_run_agent_context_json_uses_latest_incomplete_run() {
        let _cwd = crate::utils::test_cwd_lock().lock().await;
        let root = tempfile::tempdir().unwrap();
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(root.path()).unwrap();
        let manager = RunManager::new(root.path());
        let mut record = RunRecord::new(
            "goal".into(),
            "Phase-74".into(),
            "M-RUNNER".into(),
            "resume".into(),
        );
        record.status = RunStatus::Blocked;
        manager.save(&record).unwrap();

        let cmd = AgentContextCmd {
            run_id: None,
            json: true,
        };
        let result = run_agent_context(&cmd);
        std::env::set_current_dir(previous).unwrap();

        assert!(result.is_ok());
    }
    // END_test_run_agent_context_json_uses_latest_incomplete_run
}
