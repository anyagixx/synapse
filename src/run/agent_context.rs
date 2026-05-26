// MODULE_CONTRACT
// MODULE_ID: M-RUNNER-AGENT-CONTEXT
// PURPOSE: Compact resumable agent context for long-running bounded work.
// SCOPE: AgentContext and RunStateSignals models, latest incomplete/latest run discovery, action queue summary, belief-state reference, latest handoff inclusion, compact run-state signals, and resume/status builders.
// DEPENDS: M-RUNNER, M-RUNNER-HANDOFF, M-GRACE-BELIEF-STATE
// LINKS:
//   -> M-RUNNER (depends) - reads persisted runs and action queues
//   -> M-RUNNER-HANDOFF (depends) - includes latest handoff summary
//   -> M-GRACE-BELIEF-STATE (depends) - references compact module belief-state artifacts
//   -> UC-002 (implements) - agents can resume bounded work from compact context
//   -> NFR-003 (traces_to) - resume context avoids rereading full project state

// START_MODULE_MAP
// AgentContext - Compact run resume payload
// RunStateSignals - Minimal run-state fields for recommendation and routing
// RunManager::latest_incomplete_run - Locate most recently updated non-terminal run
// RunManager::latest_run - Locate most recently updated run when no incomplete run exists
// RunManager::build_agent_context - Build handoff-aware resume context
// RunManager::run_state_signals - Build compact status/gate/module/next-action signals
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.2.0 - Added compact run-state signals for tool recommendation]
// END_CHANGE_SUMMARY

use super::handoff::AgentHandoff;
use super::{RunGateStatus, RunManager, RunRecord, RunStatus, RunStepStatus};
use serde::{Deserialize, Serialize};

// START_public_api
// START_AgentContext
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub run_id: String,
    pub goal: String,
    pub phase: String,
    pub module_id: String,
    pub objective: String,
    pub status: String,
    pub current_step: usize,
    pub next_action: Option<String>,
    pub evidence_refs: Vec<String>,
    pub belief_state_ref: Option<String>,
    pub latest_handoff: Option<AgentHandoff>,
    pub compact_summary: String,
}
// END_AgentContext

// START_RunStateSignals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunStateSignals {
    pub status: String,
    pub current_gate: Option<String>,
    pub module: String,
    pub next_action: Option<String>,
}
// END_RunStateSignals

impl RunManager {
    // START_CONTRACT_RunManager::latest_incomplete_run
    // PURPOSE: Return the most recently updated run that is not terminal.
    // OUTPUTS: { anyhow::Result<Option<RunRecord>> }
    // LINKS:
    //   -> UC-002 (implements) - resume starts from latest incomplete bounded run
    // START_run_manager_latest_incomplete_run
    pub fn latest_incomplete_run(&self) -> anyhow::Result<Option<RunRecord>> {
        let mut runs: Vec<RunRecord> = self
            .list()?
            .into_iter()
            .filter(|run| {
                !matches!(
                    run.status,
                    RunStatus::Completed | RunStatus::Escalated | RunStatus::Failed
                )
            })
            .collect();
        runs.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| right.run_id.cmp(&left.run_id))
        });
        Ok(runs.into_iter().next())
    }
    // END_run_manager_latest_incomplete_run

    // START_CONTRACT_RunManager::latest_run
    // PURPOSE: Return the most recently updated run, including terminal runs.
    // OUTPUTS: { anyhow::Result<Option<RunRecord>> }
    // LINKS:
    //   -> UC-002 (implements) - status context remains available after completion
    // START_run_manager_latest_run
    pub fn latest_run(&self) -> anyhow::Result<Option<RunRecord>> {
        let mut runs = self.list()?;
        runs.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| right.run_id.cmp(&left.run_id))
        });
        Ok(runs.into_iter().next())
    }
    // END_run_manager_latest_run

    // START_CONTRACT_RunManager::build_agent_context
    // PURPOSE: Build compact resume context from one run, the latest incomplete run, or latest terminal run.
    // INPUTS: { run_id: Option<&str> }
    // OUTPUTS: { anyhow::Result<Option<AgentContext>> }
    // LINKS:
    //   -> NFR-003 (traces_to) - context is compact and evidence-linked
    // START_run_manager_build_agent_context
    pub fn build_agent_context(
        &self,
        run_id: Option<&str>,
    ) -> anyhow::Result<Option<AgentContext>> {
        let run = match run_id {
            Some(run_id) => self.load(run_id).map(Some)?,
            None => match self.latest_incomplete_run()? {
                Some(run) => Some(run),
                None => self.latest_run()?,
            },
        };
        let Some(run) = run else {
            return Ok(None);
        };
        let next_action = self.next_action_signal(&run);
        let latest_handoff = self.latest_handoff(&run.run_id)?;
        let belief_state_ref = belief_state_ref(&self.root, &run.module_id);
        let compact_summary =
            compact_summary(&run, next_action.as_deref(), latest_handoff.as_ref());

        Ok(Some(AgentContext {
            run_id: run.run_id,
            goal: run.goal,
            phase: run.phase,
            module_id: run.module_id,
            objective: run.objective,
            status: format!("{:?}", run.status),
            current_step: run.current_step,
            next_action,
            evidence_refs: run.evidence_refs,
            belief_state_ref,
            latest_handoff,
            compact_summary,
        }))
    }
    // END_run_manager_build_agent_context

    // START_CONTRACT_RunManager::run_state_signals
    // PURPOSE: Build compact run-state signals for predictive MCP tool recommendation.
    // INPUTS: { run_id: &str }
    // OUTPUTS: { anyhow::Result<RunStateSignals> }
    // LINKS:
    //   -> NFR-003 (traces_to) - recommendations use compact run state instead of full run history
    // START_run_manager_run_state_signals
    pub fn run_state_signals(&self, run_id: &str) -> anyhow::Result<RunStateSignals> {
        let run = self.load(run_id)?;
        Ok(RunStateSignals {
            status: format!("{:?}", run.status),
            current_gate: current_gate_signal(&run),
            module: run.module_id.clone(),
            next_action: self.next_action_signal(&run),
        })
    }
    // END_run_manager_run_state_signals

    // START_CONTRACT_RunManager::next_action_signal
    // PURPOSE: Return the most compact next action from action planning, blocked reason, or active step.
    // INPUTS: { run: &RunRecord }
    // OUTPUTS: { Option<String> }
    // START_run_manager_next_action_signal
    fn next_action_signal(&self, run: &RunRecord) -> Option<String> {
        self.plan_run_actions(&run.run_id)
            .ok()
            .and_then(|plan| {
                plan.actions
                    .first()
                    .map(|action| action.description.clone())
            })
            .or_else(|| run.blocked_reason.clone())
            .or_else(|| {
                run.steps
                    .iter()
                    .find(|step| {
                        matches!(
                            step.status,
                            RunStepStatus::Pending | RunStepStatus::Running | RunStepStatus::Failed
                        )
                    })
                    .map(|step| step.description.clone())
            })
    }
    // END_run_manager_next_action_signal
}
// END_public_api

// START_CONTRACT_current_gate_signal
// PURPOSE: Return the first non-passed required gate id for compact recommendation context.
// INPUTS: { run: &RunRecord }
// OUTPUTS: { Option<String> }
// START_current_gate_signal
fn current_gate_signal(run: &RunRecord) -> Option<String> {
    run.required_gates
        .iter()
        .find(|gate| gate.required && !matches!(gate.status, RunGateStatus::Passed))
        .map(|gate| gate.id.clone())
}
// END_current_gate_signal

// START_CONTRACT_belief_state_ref
// PURPOSE: Return a project-relative belief-state path when it exists.
// INPUTS: { root: &Path }, { module_id: &str }
// OUTPUTS: { Option<String> }
// START_belief_state_ref
fn belief_state_ref(root: &std::path::Path, module_id: &str) -> Option<String> {
    let path = root
        .join("docs/belief-states")
        .join(format!("{}.xml", module_id));
    path.exists()
        .then(|| format!("docs/belief-states/{}.xml", module_id))
}
// END_belief_state_ref

// START_CONTRACT_compact_summary
// PURPOSE: Build a short resume summary for CLI and agent handoff output.
// INPUTS: { run: &RunRecord }, { next_action: Option<&str> }, { latest_handoff: Option<&AgentHandoff> }
// OUTPUTS: { String }
// START_compact_summary
fn compact_summary(
    run: &RunRecord,
    next_action: Option<&str>,
    latest_handoff: Option<&AgentHandoff>,
) -> String {
    let handoff = latest_handoff
        .map(|handoff| format!("; handoff={} -> {:?}", handoff.handoff_id, handoff.to_role))
        .unwrap_or_default();
    format!(
        "{} {} step={} next={}{}",
        run.run_id,
        format!("{:?}", run.status).to_lowercase(),
        run.current_step,
        next_action.unwrap_or("none"),
        handoff
    )
}
// END_compact_summary

#[cfg(test)]
mod tests {
    use super::*;

    fn run(manager: &RunManager, status: RunStatus, module_id: &str) -> String {
        let mut record = RunRecord::new(
            "goal".into(),
            "Phase-74".into(),
            module_id.into(),
            "resume work".into(),
        );
        record.status = status;
        record.steps = super::super::default_steps(module_id);
        let run_id = record.run_id.clone();
        manager.save(&record).unwrap();
        run_id
    }

    // START_CONTRACT_latest_incomplete_run_skips_terminal_runs
    // PURPOSE: Verify latest incomplete run ignores completed runs.
    // START_latest_incomplete_run_skips_terminal_runs
    #[test]
    fn latest_incomplete_run_skips_terminal_runs() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        run(&manager, RunStatus::Completed, "M-RUNNER");
        let active = run(&manager, RunStatus::Blocked, "M-RUNNER");

        let latest = manager.latest_incomplete_run().unwrap().unwrap();

        assert_eq!(latest.run_id, active);
    }
    // END_latest_incomplete_run_skips_terminal_runs

    // START_CONTRACT_build_agent_context_falls_back_to_latest_terminal_run
    // PURPOSE: Verify agent context remains available when only terminal runs exist.
    // START_build_agent_context_falls_back_to_latest_terminal_run
    #[test]
    fn build_agent_context_falls_back_to_latest_terminal_run() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let completed = run(&manager, RunStatus::Completed, "M-RUNNER");

        let context = manager.build_agent_context(None).unwrap().unwrap();

        assert_eq!(context.run_id, completed);
        assert_eq!(context.status, "Completed");
        assert!(context.compact_summary.contains("completed"));
    }
    // END_build_agent_context_falls_back_to_latest_terminal_run

    // START_CONTRACT_build_agent_context_includes_handoff_and_belief_state_ref
    // PURPOSE: Verify agent context includes latest handoff and belief-state path.
    // START_build_agent_context_includes_handoff_and_belief_state_ref
    #[test]
    fn build_agent_context_includes_handoff_and_belief_state_ref() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("docs/belief-states")).unwrap();
        std::fs::write(
            root.path().join("docs/belief-states/M-RUNNER.xml"),
            "<BELIEF_STATE module=\"M-RUNNER\" />",
        )
        .unwrap();
        let manager = RunManager::new(root.path());
        let run_id = run(&manager, RunStatus::Blocked, "M-RUNNER");
        manager
            .create_handoff(
                &run_id,
                crate::run::handoff::HandoffRole::Verifier,
                crate::run::handoff::HandoffRole::Fixer,
                "fix verification",
                vec!["verify.log".into()],
                vec!["repair".into()],
            )
            .unwrap();

        let context = manager.build_agent_context(Some(&run_id)).unwrap().unwrap();

        assert_eq!(
            context.belief_state_ref.as_deref(),
            Some("docs/belief-states/M-RUNNER.xml")
        );
        assert!(context.latest_handoff.is_some());
        assert!(context.compact_summary.contains("handoff="));
    }
    // END_build_agent_context_includes_handoff_and_belief_state_ref

    // START_CONTRACT_run_state_signals_include_gate_module_and_next_action
    // PURPOSE: Verify compact run-state signals expose status, current gate, module, and next action.
    // START_run_state_signals_include_gate_module_and_next_action
    #[test]
    fn run_state_signals_include_gate_module_and_next_action() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let mut record = RunRecord::new(
            "goal".into(),
            "Phase-86".into(),
            "M-AUTH".into(),
            "debug auth failure".into(),
        );
        record.status = RunStatus::Blocked;
        record.blocked_reason = Some("verification gate blocked".into());
        record.required_gates.push(crate::run::RunGate {
            id: "gate-verification".into(),
            name: "Verification".into(),
            required: true,
            status: RunGateStatus::Blocked,
            reason: Some("cargo test failed".into()),
            evidence_refs: vec!["verify.log".into()],
        });
        manager.save(&record).unwrap();

        let signals = manager.run_state_signals(&record.run_id).unwrap();

        assert_eq!(signals.status, "Blocked");
        assert_eq!(signals.current_gate.as_deref(), Some("gate-verification"));
        assert_eq!(signals.module, "M-AUTH");
        assert!(signals
            .next_action
            .as_deref()
            .is_some_and(|action| action.contains("verification")));
    }
    // END_run_state_signals_include_gate_module_and_next_action
}
