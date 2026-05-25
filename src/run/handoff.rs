// MODULE_CONTRACT
// MODULE_ID: M-RUNNER-HANDOFF
// PURPOSE: Structured multi-agent handoff artifacts for bounded autonomous runs.
// SCOPE: AgentHandoff model, deterministic role sequencing, durable handoff storage paths, create/load/latest APIs, and run metadata updates.
// DEPENDS: M-RUNNER
// LINKS:
//   -> M-RUNNER (depends) - stores handoffs under durable run state
//   -> UC-002 (implements) - agents can transfer bounded work with evidence and next actions
//   -> NFR-003 (traces_to) - compact handoff artifacts reduce context reconstruction

// START_MODULE_MAP
// AgentHandoff - Durable structured handoff artifact
// HandoffRole - Known bounded agent roles
// RunManager::create_handoff - Persist one role-to-role handoff
// RunManager::latest_handoff - Load latest handoff for a run
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added structured run handoff artifacts]
// END_CHANGE_SUMMARY

use super::{write_provenance_event, RunManager};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// START_public_api
// START_HandoffRole
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HandoffRole {
    Planner,
    Implementer,
    Verifier,
    Fixer,
    Reviewer,
    Releaser,
}
// END_HandoffRole

impl HandoffRole {
    // START_CONTRACT_HandoffRole::next
    // PURPOSE: Return the default next role in a bounded implementation workflow.
    // OUTPUTS: { Option<HandoffRole> }
    // START_handoff_role_next
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Planner => Some(Self::Implementer),
            Self::Implementer => Some(Self::Verifier),
            Self::Verifier => Some(Self::Fixer),
            Self::Fixer => Some(Self::Reviewer),
            Self::Reviewer => Some(Self::Releaser),
            Self::Releaser => None,
        }
    }
    // END_handoff_role_next
}

// START_AgentHandoff
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentHandoff {
    pub handoff_id: String,
    pub run_id: String,
    pub from_role: HandoffRole,
    pub to_role: HandoffRole,
    pub summary: String,
    pub evidence_refs: Vec<String>,
    pub next_actions: Vec<String>,
    pub created_at: String,
}
// END_AgentHandoff

impl RunManager {
    // START_CONTRACT_RunManager::handoffs_dir
    // PURPOSE: Resolve durable handoff directory for one run.
    // INPUTS: { run_id: &str }
    // OUTPUTS: { PathBuf }
    // START_run_manager_handoffs_dir
    pub fn handoffs_dir(&self, run_id: &str) -> PathBuf {
        self.runs_dir().join("handoffs").join(run_id)
    }
    // END_run_manager_handoffs_dir

    // START_CONTRACT_RunManager::handoff_path
    // PURPOSE: Resolve durable handoff artifact path.
    // INPUTS: { run_id: &str }, { handoff_id: &str }
    // OUTPUTS: { PathBuf }
    // START_run_manager_handoff_path
    pub fn handoff_path(&self, run_id: &str, handoff_id: &str) -> PathBuf {
        self.handoffs_dir(run_id)
            .join(format!("{}.json", handoff_id))
    }
    // END_run_manager_handoff_path

    // START_CONTRACT_RunManager::create_handoff
    // PURPOSE: Persist a structured handoff and attach latest handoff metadata to the run.
    // INPUTS: { run_id: &str }, { from_role: HandoffRole }, { to_role: HandoffRole }, { summary: impl Into<String> }, { evidence_refs: Vec<String> }, { next_actions: Vec<String> }
    // OUTPUTS: { anyhow::Result<AgentHandoff> }
    // LINKS:
    //   -> UC-002 (implements) - preserve evidence and next actions across agent boundaries
    // START_run_manager_create_handoff
    pub fn create_handoff(
        &self,
        run_id: &str,
        from_role: HandoffRole,
        to_role: HandoffRole,
        summary: impl Into<String>,
        evidence_refs: Vec<String>,
        next_actions: Vec<String>,
    ) -> anyhow::Result<AgentHandoff> {
        let created_at = chrono::Utc::now().to_rfc3339();
        let handoff_id = format!(
            "{}-{}-{}",
            created_at.replace([':', '.'], "-"),
            role_slug(&from_role),
            role_slug(&to_role)
        );
        let handoff = AgentHandoff {
            handoff_id: handoff_id.clone(),
            run_id: run_id.to_string(),
            from_role,
            to_role,
            summary: summary.into(),
            evidence_refs,
            next_actions,
            created_at,
        };
        std::fs::create_dir_all(self.handoffs_dir(run_id))?;
        let path = self.handoff_path(run_id, &handoff_id);
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&handoff)?)?;
        std::fs::rename(&tmp, &path)?;

        let mut record = self.load(run_id)?;
        record
            .metadata
            .insert("latest_handoff_id".into(), handoff.handoff_id.clone());
        record.metadata.insert(
            "latest_handoff_path".into(),
            path.strip_prefix(&self.root)
                .unwrap_or(&path)
                .display()
                .to_string(),
        );
        record.touch();
        self.save(&record)?;
        write_provenance_event(
            &record.run_id,
            &record.module_id,
            &record.phase,
            &record.status,
            "create_handoff",
            &format!(
                "{}->{}",
                role_slug(&handoff.from_role),
                role_slug(&handoff.to_role)
            ),
        );
        Ok(handoff)
    }
    // END_run_manager_create_handoff

    // START_CONTRACT_RunManager::load_handoff
    // PURPOSE: Load one structured handoff by id.
    // INPUTS: { run_id: &str }, { handoff_id: &str }
    // OUTPUTS: { anyhow::Result<AgentHandoff> }
    // START_run_manager_load_handoff
    pub fn load_handoff(&self, run_id: &str, handoff_id: &str) -> anyhow::Result<AgentHandoff> {
        let data = std::fs::read(self.handoff_path(run_id, handoff_id))?;
        Ok(serde_json::from_slice(&data)?)
    }
    // END_run_manager_load_handoff

    // START_CONTRACT_RunManager::latest_handoff
    // PURPOSE: Load the latest handoff for a run using run metadata or newest file fallback.
    // INPUTS: { run_id: &str }
    // OUTPUTS: { anyhow::Result<Option<AgentHandoff>> }
    // START_run_manager_latest_handoff
    pub fn latest_handoff(&self, run_id: &str) -> anyhow::Result<Option<AgentHandoff>> {
        if let Ok(record) = self.load(run_id) {
            if let Some(handoff_id) = record.metadata.get("latest_handoff_id") {
                return self.load_handoff(run_id, handoff_id).map(Some);
            }
        }
        let dir = self.handoffs_dir(run_id);
        if !dir.exists() {
            return Ok(None);
        }
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                paths.push(path);
            }
        }
        paths.sort();
        match paths.pop() {
            Some(path) => {
                let data = std::fs::read(path)?;
                Ok(Some(serde_json::from_slice(&data)?))
            }
            None => Ok(None),
        }
    }
    // END_run_manager_latest_handoff
}
// END_public_api

fn role_slug(role: &HandoffRole) -> &'static str {
    match role {
        HandoffRole::Planner => "planner",
        HandoffRole::Implementer => "implementer",
        HandoffRole::Verifier => "verifier",
        HandoffRole::Fixer => "fixer",
        HandoffRole::Reviewer => "reviewer",
        HandoffRole::Releaser => "releaser",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{RunRecord, RunStatus};

    fn saved_run(manager: &RunManager) -> String {
        let mut record = RunRecord::new(
            "goal".into(),
            "Phase-74".into(),
            "M-RUNNER-HANDOFF".into(),
            "create handoff".into(),
        );
        record.status = RunStatus::Blocked;
        let run_id = record.run_id.clone();
        manager.save(&record).unwrap();
        run_id
    }

    // START_CONTRACT_create_handoff_persists_and_updates_run_metadata
    // PURPOSE: Verify handoff artifacts are durable and linked from run metadata.
    // START_create_handoff_persists_and_updates_run_metadata
    #[test]
    fn create_handoff_persists_and_updates_run_metadata() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let run_id = saved_run(&manager);

        let handoff = manager
            .create_handoff(
                &run_id,
                HandoffRole::Verifier,
                HandoffRole::Fixer,
                "verification failed",
                vec!["syn verify".into()],
                vec!["fix failing check".into()],
            )
            .unwrap();
        let loaded = manager.load_handoff(&run_id, &handoff.handoff_id).unwrap();
        let record = manager.load(&run_id).unwrap();

        assert_eq!(loaded.summary, "verification failed");
        assert_eq!(
            record.metadata.get("latest_handoff_id").map(String::as_str),
            Some(handoff.handoff_id.as_str())
        );
    }
    // END_create_handoff_persists_and_updates_run_metadata

    // START_CONTRACT_latest_handoff_returns_newest_handoff
    // PURPOSE: Verify latest_handoff loads the newest metadata-linked handoff.
    // START_latest_handoff_returns_newest_handoff
    #[test]
    fn latest_handoff_returns_newest_handoff() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let run_id = saved_run(&manager);
        manager
            .create_handoff(
                &run_id,
                HandoffRole::Planner,
                HandoffRole::Implementer,
                "start",
                Vec::new(),
                vec!["implement".into()],
            )
            .unwrap();

        let latest = manager.latest_handoff(&run_id).unwrap().unwrap();

        assert_eq!(latest.to_role, HandoffRole::Implementer);
        assert_eq!(HandoffRole::Verifier.next(), Some(HandoffRole::Fixer));
    }
    // END_latest_handoff_returns_newest_handoff
}
