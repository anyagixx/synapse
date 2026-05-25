// MODULE_CONTRACT
// MODULE_ID: M-RUNNER-PHASE-ENGINE
// PURPOSE: Autonomous MyGRACE phase gate checking and safe phase advancement for bounded runs.
// SCOPE: Active phase discovery, phase gate report model, dry-run phase advancement, atomic plan-index and phase shard status updates.
// DEPENDS: M-RUNNER, M-GRACE-STATUS
// LINKS:
//   -> M-RUNNER (depends) - exposes phase progression through RunManager
//   -> M-GRACE-STATUS (depends) - aligns with active phase status semantics
//   -> UC-002 (implements) - bounded autonomous work advances only after phase gates pass
//   -> NFR-002 (traces_to) - phase updates fail explicitly and avoid partial writes where possible

// START_MODULE_MAP
// PhaseGateCheck - One phase advancement gate result
// PhaseGateReport - Complete active phase gate and advancement report
// RunManager::check_phase_gates - Inspect active phase readiness
// RunManager::advance_phase - Dry-run or apply safe phase advancement
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added autonomous phase gate engine]
// END_CHANGE_SUMMARY

use super::RunManager;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const PHASE_STATUS_ACTIVE: &str = "active";
const PHASE_STATUS_DONE: &str = "done";
const PHASE_STATUS_PLANNED: &str = "planned";

// START_public_api
// START_PhaseGateCheck
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhaseGateCheck {
    pub name: String,
    pub passed: bool,
    pub reason: Option<String>,
    pub evidence_refs: Vec<String>,
}
// END_PhaseGateCheck

// START_PhaseGateReport
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhaseGateReport {
    pub active_phase: String,
    pub next_phase: Option<String>,
    pub passed: bool,
    pub dry_run: bool,
    pub advanced: bool,
    pub checks: Vec<PhaseGateCheck>,
}
// END_PhaseGateReport

impl RunManager {
    // START_CONTRACT_RunManager::check_phase_gates
    // PURPOSE: Inspect active phase shard and plan-index to decide whether phase advancement is allowed.
    // OUTPUTS: { anyhow::Result<PhaseGateReport> }
    // LINKS:
    //   -> UC-002 (implements) - phase progression requires explicit gate evidence
    //   -> NFR-002 (traces_to) - missing phase artifacts produce explicit blocked checks
    // START_run_manager_check_phase_gates
    pub fn check_phase_gates(&self) -> anyhow::Result<PhaseGateReport> {
        let plan_path = self.root.join("docs/plan-index.xml");
        let plan_index = std::fs::read_to_string(&plan_path)?;
        let active_phase =
            extract_tag_value(&plan_index, "ACTIVE_PHASE").unwrap_or_else(|| "none".to_string());
        let phase_entries = parse_phase_entries(&plan_index);
        let current = phase_entries
            .iter()
            .find(|phase| phase.id == active_phase)
            .cloned();
        let next_phase = next_planned_phase(&phase_entries, &active_phase);
        let mut checks = Vec::new();

        checks.push(PhaseGateCheck {
            name: "active_phase_present".into(),
            passed: active_phase != "none" && current.is_some(),
            reason: current
                .is_none()
                .then(|| format!("active phase '{}' is missing from plan-index", active_phase)),
            evidence_refs: vec![plan_path.display().to_string()],
        });
        checks.push(PhaseGateCheck {
            name: "next_phase_present".into(),
            passed: next_phase.is_some(),
            reason: next_phase
                .is_none()
                .then(|| "no planned next phase is available".to_string()),
            evidence_refs: vec![plan_path.display().to_string()],
        });

        if let Some(current) = current {
            let phase_path = self.root.join(&current.path);
            match std::fs::read_to_string(&phase_path) {
                Ok(phase_content) => {
                    checks.extend(phase_shard_checks(&phase_content, &phase_path));
                }
                Err(err) => checks.push(PhaseGateCheck {
                    name: "active_phase_shard_readable".into(),
                    passed: false,
                    reason: Some(format!("cannot read {}: {}", phase_path.display(), err)),
                    evidence_refs: vec![phase_path.display().to_string()],
                }),
            }
        }

        let passed = checks.iter().all(|check| check.passed);
        Ok(PhaseGateReport {
            active_phase,
            next_phase,
            passed,
            dry_run: true,
            advanced: false,
            checks,
        })
    }
    // END_run_manager_check_phase_gates

    // START_CONTRACT_RunManager::advance_phase
    // PURPOSE: Advance active phase to the next planned phase when gates pass, optionally as dry-run.
    // INPUTS: { dry_run: bool }
    // OUTPUTS: { anyhow::Result<PhaseGateReport> }
    // SIDE_EFFECTS: writes docs/plan-index.xml and phase shards when dry_run=false and gates pass
    // LINKS:
    //   -> UC-002 (implements) - autonomous phase progression remains gate controlled
    //   -> NFR-002 (traces_to) - dry-run mode previews changes without mutating XML
    // START_run_manager_advance_phase
    pub fn advance_phase(&self, dry_run: bool) -> anyhow::Result<PhaseGateReport> {
        let mut report = self.check_phase_gates()?;
        report.dry_run = dry_run;
        if !report.passed || dry_run {
            return Ok(report);
        }

        let next_phase = report
            .next_phase
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no next phase available"))?;
        let plan_path = self.root.join("docs/plan-index.xml");
        let plan_index = std::fs::read_to_string(&plan_path)?;
        let updated_plan = update_plan_index(&plan_index, &report.active_phase, &next_phase);
        atomic_write(&plan_path, &updated_plan)?;

        update_phase_shard_status(&self.root, &report.active_phase, PHASE_STATUS_DONE)?;
        update_phase_shard_status(&self.root, &next_phase, PHASE_STATUS_ACTIVE)?;

        report.advanced = true;
        report.dry_run = false;
        Ok(report)
    }
    // END_run_manager_advance_phase
}
// END_public_api

#[derive(Debug, Clone, PartialEq, Eq)]
struct PhaseEntry {
    id: String,
    path: String,
    status: String,
}

// START_CONTRACT_phase_shard_checks
// PURPOSE: Build gate checks from one active phase shard.
// INPUTS: { phase_content: &str }, { phase_path: &Path }
// OUTPUTS: { Vec<PhaseGateCheck> }
// START_phase_shard_checks
fn phase_shard_checks(phase_content: &str, phase_path: &Path) -> Vec<PhaseGateCheck> {
    let unfinished_steps = phase_content.lines().any(|line| {
        line.contains("<step-")
            && (line.contains("status=\"planned\"")
                || line.contains("status=\"active\"")
                || line.contains("status=\"wip\""))
    });
    let failed_checks = phase_content.lines().any(|line| {
        line.contains("<CHECK")
            && (line.contains("status=\"failed\"") || line.contains("status=\"blocked\""))
    });
    let planned_checks = phase_content
        .lines()
        .any(|line| line.contains("<CHECK") && line.contains("status=\"planned\""));
    vec![
        PhaseGateCheck {
            name: "active_phase_shard_readable".into(),
            passed: true,
            reason: None,
            evidence_refs: vec![phase_path.display().to_string()],
        },
        PhaseGateCheck {
            name: "phase_steps_done".into(),
            passed: !unfinished_steps,
            reason: unfinished_steps.then(|| "active phase still has unfinished steps".into()),
            evidence_refs: vec![phase_path.display().to_string()],
        },
        PhaseGateCheck {
            name: "phase_checks_passed".into(),
            passed: !failed_checks && !planned_checks,
            reason: (failed_checks || planned_checks)
                .then(|| "active phase verification checks are not all passed".into()),
            evidence_refs: vec![phase_path.display().to_string()],
        },
    ]
}
// END_phase_shard_checks

// START_CONTRACT_update_plan_index
// PURPOSE: Rewrite plan-index active/done/planned phase statuses after gates pass.
// INPUTS: { plan_index: &str }, { current_phase: &str }, { next_phase: &str }
// OUTPUTS: { String }
// START_update_plan_index
fn update_plan_index(plan_index: &str, current_phase: &str, next_phase: &str) -> String {
    let with_active = replace_tag_value(plan_index, "ACTIVE_PHASE", next_phase);
    let with_done = replace_phase_entry_status(&with_active, current_phase, PHASE_STATUS_DONE);
    replace_phase_entry_status(&with_done, next_phase, PHASE_STATUS_ACTIVE)
}
// END_update_plan_index

// START_CONTRACT_update_phase_shard_status
// PURPOSE: Update the root PHASE status attribute for one phase shard.
// INPUTS: { root: &Path }, { phase_id: &str }, { status: &str }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: writes phase shard atomically
// START_update_phase_shard_status
fn update_phase_shard_status(root: &Path, phase_id: &str, status: &str) -> anyhow::Result<()> {
    let path = root.join("docs/phases").join(format!("{}.xml", phase_id));
    let content = std::fs::read_to_string(&path)?;
    let marker = format!("<PHASE id=\"{}\"", phase_id);
    let Some(start) = content.find(&marker) else {
        return Ok(());
    };
    let Some(end) = content[start..].find('>') else {
        return Ok(());
    };
    let head_end = start + end;
    let head = &content[start..head_end];
    let updated_head = replace_status_attr(head, status);
    let updated = format!(
        "{}{}{}",
        &content[..start],
        updated_head,
        &content[head_end..]
    );
    atomic_write(&path, &updated)
}
// END_update_phase_shard_status

// START_CONTRACT_parse_phase_entries
// PURPOSE: Parse PLAN_INDEX phase entries from the canonical one-line shard format.
// INPUTS: { plan_index: &str }
// OUTPUTS: { Vec<PhaseEntry> }
// START_parse_phase_entries
fn parse_phase_entries(plan_index: &str) -> Vec<PhaseEntry> {
    plan_index
        .lines()
        .filter(|line| line.trim_start().starts_with("<PHASE "))
        .filter_map(|line| {
            Some(PhaseEntry {
                id: extract_attr(line, "id")?,
                path: extract_attr(line, "path")?,
                status: extract_attr(line, "status")?,
            })
        })
        .collect()
}
// END_parse_phase_entries

// START_CONTRACT_next_planned_phase
// PURPOSE: Return the first planned phase after the active phase.
// INPUTS: { phases: &[PhaseEntry] }, { active_phase: &str }
// OUTPUTS: { Option<String> }
// START_next_planned_phase
fn next_planned_phase(phases: &[PhaseEntry], active_phase: &str) -> Option<String> {
    let mut seen_active = false;
    for phase in phases {
        if phase.id == active_phase {
            seen_active = true;
            continue;
        }
        if seen_active && phase.status == PHASE_STATUS_PLANNED {
            return Some(phase.id.clone());
        }
    }
    None
}
// END_next_planned_phase

fn extract_tag_value(content: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = content.find(&open)? + open.len();
    let end = content[start..].find(&close)? + start;
    Some(content[start..end].to_string())
}

fn replace_tag_value(content: &str, tag: &str, value: &str) -> String {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let Some(start) = content.find(&open).map(|offset| offset + open.len()) else {
        return content.to_string();
    };
    let Some(end) = content[start..].find(&close).map(|offset| start + offset) else {
        return content.to_string();
    };
    format!("{}{}{}", &content[..start], value, &content[end..])
}

fn replace_phase_entry_status(content: &str, phase_id: &str, status: &str) -> String {
    content
        .lines()
        .map(|line| {
            if line.contains(&format!("id=\"{}\"", phase_id)) && line.contains("<PHASE ") {
                replace_status_attr(line, status)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn replace_status_attr(line: &str, status: &str) -> String {
    if let Some(current) = extract_attr(line, "status") {
        line.replace(
            &format!("status=\"{}\"", current),
            &format!("status=\"{}\"", status),
        )
    } else {
        format!("{} status=\"{}\"", line.trim_end(), status)
    }
}

fn extract_attr(line: &str, attr: &str) -> Option<String> {
    let needle = format!("{}=\"", attr);
    let start = line.find(&needle)? + needle.len();
    let end = line[start..].find('"')? + start;
    Some(line[start..end].to_string())
}

fn atomic_write(path: &Path, content: &str) -> anyhow::Result<()> {
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, content)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phase_fixture(root: &Path, check_status: &str) {
        std::fs::create_dir_all(root.join("docs/phases")).unwrap();
        std::fs::write(
            root.join("docs/plan-index.xml"),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
                "<PLAN_INDEX>\n",
                "  <META><ACTIVE_PHASE>Phase-1</ACTIVE_PHASE></META>\n",
                "  <PHASES>\n",
                "    <PHASE id=\"Phase-1\" path=\"docs/phases/Phase-1.xml\" status=\"active\" />\n",
                "    <PHASE id=\"Phase-2\" path=\"docs/phases/Phase-2.xml\" status=\"planned\" />\n",
                "  </PHASES>\n",
                "</PLAN_INDEX>\n"
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("docs/phases/Phase-1.xml"),
            format!(
                concat!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
                    "<PHASE id=\"Phase-1\" status=\"active\">\n",
                    "  <STEPS><step-1 status=\"done\">done</step-1></STEPS>\n",
                    "  <VERIFICATION><CHECK status=\"{}\">gate</CHECK></VERIFICATION>\n",
                    "</PHASE>\n"
                ),
                check_status
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("docs/phases/Phase-2.xml"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<PHASE id=\"Phase-2\" status=\"planned\"></PHASE>\n",
        )
        .unwrap();
    }

    // START_CONTRACT_check_phase_gates_blocks_planned_checks
    // PURPOSE: Verify phase gates block advancement while checks are still planned.
    // START_check_phase_gates_blocks_planned_checks
    #[test]
    fn check_phase_gates_blocks_planned_checks() {
        let root = tempfile::tempdir().unwrap();
        phase_fixture(root.path(), PHASE_STATUS_PLANNED);
        let manager = RunManager::new(root.path());

        let report = manager.check_phase_gates().unwrap();

        assert!(!report.passed);
        assert!(report
            .checks
            .iter()
            .any(|check| check.name == "phase_checks_passed" && !check.passed));
    }
    // END_check_phase_gates_blocks_planned_checks

    // START_CONTRACT_advance_phase_dry_run_does_not_mutate_plan
    // PURPOSE: Verify dry-run phase advance reports pass without changing plan-index.
    // START_advance_phase_dry_run_does_not_mutate_plan
    #[test]
    fn advance_phase_dry_run_does_not_mutate_plan() {
        let root = tempfile::tempdir().unwrap();
        phase_fixture(root.path(), "passed");
        let manager = RunManager::new(root.path());
        let before = std::fs::read_to_string(root.path().join("docs/plan-index.xml")).unwrap();

        let report = manager.advance_phase(true).unwrap();
        let after = std::fs::read_to_string(root.path().join("docs/plan-index.xml")).unwrap();

        assert!(report.passed);
        assert!(!report.advanced);
        assert_eq!(before, after);
    }
    // END_advance_phase_dry_run_does_not_mutate_plan

    // START_CONTRACT_advance_phase_updates_plan_and_shards
    // PURPOSE: Verify non-dry-run phase advance moves active phase to the next planned shard.
    // START_advance_phase_updates_plan_and_shards
    #[test]
    fn advance_phase_updates_plan_and_shards() {
        let root = tempfile::tempdir().unwrap();
        phase_fixture(root.path(), "passed");
        let manager = RunManager::new(root.path());

        let report = manager.advance_phase(false).unwrap();
        let plan = std::fs::read_to_string(root.path().join("docs/plan-index.xml")).unwrap();
        let phase_one =
            std::fs::read_to_string(root.path().join("docs/phases/Phase-1.xml")).unwrap();
        let phase_two =
            std::fs::read_to_string(root.path().join("docs/phases/Phase-2.xml")).unwrap();

        assert!(report.advanced);
        assert!(plan.contains("<ACTIVE_PHASE>Phase-2</ACTIVE_PHASE>"));
        assert!(plan.contains("id=\"Phase-1\" path=\"docs/phases/Phase-1.xml\" status=\"done\""));
        assert!(plan.contains("id=\"Phase-2\" path=\"docs/phases/Phase-2.xml\" status=\"active\""));
        assert!(phase_one.contains("<PHASE id=\"Phase-1\" status=\"done\">"));
        assert!(phase_two.contains("<PHASE id=\"Phase-2\" status=\"active\">"));
    }
    // END_advance_phase_updates_plan_and_shards
}
