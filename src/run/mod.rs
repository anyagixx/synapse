// MODULE_CONTRACT
// MODULE_ID: M-RUNNER
// PURPOSE: Autonomous run runtime — persists bounded agent runs, steps, gates, reviews, replays, handoffs, resumable context, phase gates, pre-commit gates, action queues, self-heal plans, scenarios, and outcomes for controlled execution
// SCOPE: Run state model, task model, step model, gate model, review model, replay model, structured handoffs, agent context, action queue executor, phase gate engine, pre-commit verification, self-heal metadata, scenario harness, outcome model, durable JSON persistence, run lifecycle helpers
// DEPENDS: M-CONFIG, M-GRACE-DEVELOPMENT-PLAN, M-GRACE-MENTAL-TEST, M-GRACE-TRACEABILITY, M-GRACE-STATUS, M-GRACE-VERIFY, M-RUNNER-HANDOFF, M-RUNNER-AGENT-CONTEXT, M-RUNNER-PHASE-ENGINE, M-RUNNER-PRECOMMIT, M-RUNNER-SELF-HEAL, M-TRACKING
// LINKS:
//   → UC-002 (implements) - verify and review bounded autonomous changes
//   → NFR-002 (traces_to) - runtime persistence must not panic on malformed local state
//   → NFR-003 (traces_to) - durable bounded runs reduce context overhead for long agent work
//   ← V-M-RUNNER (verified_by) - runtime verification shard

// START_MODULE_MAP
// RunManager — Durable bounded-run manager for autonomous agent workflows
// RunRecord — Persisted run state
// RunStep — Persisted execution step
// RunGate — Persisted gate requirement
// RunReviewDecision — Persisted human review decision for blocked runs
// RunReplay — Replayable run timeline assembled from persisted state
// phase / pre_commit — Active phase and pre-commit verification gates
// RunActionPlan / SelfHealPlan / RunScenarioResult — End-to-end bounded action, self-heal, and scenario results
// RunOutcome — Persisted run outcome
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v0.7.0 - Added handoff and resumable agent context modules]
// END_CHANGE_SUMMARY

pub mod actions;
pub mod agent_context;
pub mod handoff;
pub mod phase;
pub mod pre_commit;
mod replay;
mod review;
pub mod scenario;
pub mod self_heal;

pub use replay::{RunReplay, RunReplayEvent};
pub use review::{RunReviewDecision, RunReviewStatus};

use crate::config::Config;
use crate::tracking::Tracker;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// START_public_api
// START_RunStatus
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Planned,
    Ready,
    Running,
    Blocked,
    Failed,
    Completed,
    Escalated,
}
// END_RunStatus

// START_RunStepStatus
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStepStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
}
// END_RunStepStatus

// START_RunGateStatus
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunGateStatus {
    Pending,
    Passed,
    Failed,
    Blocked,
}
// END_RunGateStatus

// START_RunOutcomeKind
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcomeKind {
    Success,
    Blocked,
    Failed,
    Escalated,
}
// END_RunOutcomeKind

// START_RunStep
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStep {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: RunStepStatus,
    pub evidence_refs: Vec<String>,
    pub error: Option<String>,
}
// END_RunStep

// START_RunGate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunGate {
    pub id: String,
    pub name: String,
    pub required: bool,
    pub status: RunGateStatus,
    pub reason: Option<String>,
    pub evidence_refs: Vec<String>,
}
// END_RunGate

// START_RunOutcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOutcome {
    pub kind: RunOutcomeKind,
    pub summary: String,
    pub blocked_reason: Option<String>,
    pub escalation_reason: Option<String>,
}
// END_RunOutcome

// START_RunFailureClass
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunFailureClass {
    GateBlocked,
    TraceabilityGap,
    VerificationFailure,
    DriftFailure,
    Unknown,
}
// END_RunFailureClass

// START_RunRecoveryDecision
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunRecoveryDecision {
    pub classification: RunFailureClass,
    pub retry_allowed: bool,
    pub retries_used: u32,
    pub retries_remaining: u32,
    pub next_action: String,
}
// END_RunRecoveryDecision

// START_TraceabilityTask
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceabilityTask {
    pub priority: String,
    pub source_id: String,
    pub action: String,
    pub description: String,
}
// END_TraceabilityTask

// START_RunPlanningHint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunPlanningHint {
    pub category: String,
    pub summary: String,
    pub tasks: Vec<TraceabilityTask>,
}
// END_RunPlanningHint

// START_RunGatePolicy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunGatePolicy {
    pub phase: String,
    pub module_id: String,
    pub required_gates: Vec<RunGate>,
    pub stop_on_block: bool,
    pub retry_budget: u32,
    pub escalation_target: String,
}
// END_RunGatePolicy

// START_RunGateDecision
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunGateDecision {
    pub passed: bool,
    pub blocked: bool,
    pub reason: Option<String>,
    pub next_action: Option<String>,
}
// END_RunGateDecision

// START_RunRecord
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub run_id: String,
    pub goal: String,
    pub phase: String,
    pub module_id: String,
    pub objective: String,
    pub status: RunStatus,
    pub current_step: usize,
    pub policy: Option<RunGatePolicy>,
    pub required_gates: Vec<RunGate>,
    pub steps: Vec<RunStep>,
    pub evidence_refs: Vec<String>,
    pub blocked_reason: Option<String>,
    pub escalation_reason: Option<String>,
    pub metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub review_decisions: Vec<RunReviewDecision>,
    pub outcome: Option<RunOutcome>,
    pub created_at: String,
    pub updated_at: String,
}
// END_RunRecord

// START_RunManager
#[derive(Debug, Clone)]
pub struct RunManager {
    root: PathBuf,
}
// END_RunManager

impl RunManager {
    // START_CONTRACT_RunManager::classify_failure
    // PURPOSE: Classify a bounded run failure into a stable recovery category
    // INPUTS: { reason: &str }
    // OUTPUTS: { RunFailureClass }
    // LINKS:
    //   → UC-002 (implements) - bounded runs must choose smallest safe recovery path
    // START_run_manager_classify_failure
    pub fn classify_failure(&self, reason: &str) -> RunFailureClass {
        let lower = reason.to_ascii_lowercase();
        if lower.contains("mental test") || lower.contains("gate") {
            RunFailureClass::GateBlocked
        } else if lower.contains("traceability")
            || lower.contains("requirement")
            || lower.contains("use case")
        {
            RunFailureClass::TraceabilityGap
        } else if lower.contains("verify") || lower.contains("verification") {
            RunFailureClass::VerificationFailure
        } else if lower.contains("drift") {
            RunFailureClass::DriftFailure
        } else {
            RunFailureClass::Unknown
        }
    }
    // END_run_manager_classify_failure

    // START_CONTRACT_RunManager::attempt_recovery
    // PURPOSE: Apply bounded retry policy to a blocked run and return recovery guidance or escalation
    // INPUTS: { run_id: &str }
    // OUTPUTS: { anyhow::Result<RunRecoveryDecision> }
    // LINKS:
    //   → UC-002 (implements) - bounded self-healing should retry only within explicit policy budget
    //   → NFR-002 (traces_to) - repeated failures must escalate instead of looping forever
    // START_run_manager_attempt_recovery
    pub fn attempt_recovery(&self, run_id: &str) -> anyhow::Result<RunRecoveryDecision> {
        let mut record = self.load(run_id)?;
        let reason = record
            .blocked_reason
            .clone()
            .or_else(|| {
                record
                    .outcome
                    .as_ref()
                    .and_then(|o| o.blocked_reason.clone())
            })
            .unwrap_or_else(|| "unknown failure".into());
        let classification = self.classify_failure(&reason);
        let retries_used = record
            .metadata
            .get("retry_count")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let retry_budget = record.policy.as_ref().map(|p| p.retry_budget).unwrap_or(0);
        if retries_used >= retry_budget {
            let escalation_reason = format!(
                "retry budget exhausted for {} after {} attempts",
                record.run_id, retries_used
            );
            let escalated = self.escalate_run(run_id, &escalation_reason)?;
            return Ok(RunRecoveryDecision {
                classification,
                retry_allowed: false,
                retries_used,
                retries_remaining: 0,
                next_action: escalated
                    .escalation_reason
                    .unwrap_or_else(|| "escalate to human review".into()),
            });
        }
        let next_retry = retries_used + 1;
        record
            .metadata
            .insert("retry_count".into(), next_retry.to_string());
        record.status = RunStatus::Ready;
        record.blocked_reason = None;
        if let Some(step) = record.steps.get_mut(record.current_step) {
            step.status = RunStepStatus::Pending;
            step.error = None;
        }
        record.touch();
        self.save(&record)?;
        write_provenance_event(
            &record.run_id,
            &record.module_id,
            &record.phase,
            &record.status,
            "attempt_recovery",
            &format!(
                "classification={:?} retry={} reason={}",
                classification, next_retry, reason
            ),
        );
        Ok(RunRecoveryDecision {
            classification,
            retry_allowed: true,
            retries_used: next_retry,
            retries_remaining: retry_budget.saturating_sub(next_retry),
            next_action: recovery_hint(&reason),
        })
    }
    // END_run_manager_attempt_recovery

    // START_CONTRACT_RunManager::traceability_plan
    // PURPOSE: Convert traceability gaps into bounded planning hints and next actions
    // INPUTS: { report: &crate::grace::traceability::TraceabilityReport }
    // OUTPUTS: { RunPlanningHint }
    // LINKS:
    //   → UC-002 (implements) - transform traceability debt into actionable autonomous planning input
    //   → NFR-002 (traces_to) - planning hints should be explicit and deterministic
    // START_run_manager_traceability_plan
    pub fn traceability_plan(
        &self,
        report: &crate::grace::traceability::TraceabilityReport,
    ) -> RunPlanningHint {
        let mut tasks = Vec::new();
        for requirement in &report.untraced_requirements {
            tasks.push(TraceabilityTask {
                priority: "high".into(),
                source_id: requirement.clone(),
                action: "trace_requirement".into(),
                description: format!("Add implementing links for requirement {}", requirement),
            });
        }
        for use_case in &report.untraced_use_cases {
            tasks.push(TraceabilityTask {
                priority: "high".into(),
                source_id: use_case.clone(),
                action: "trace_use_case".into(),
                description: format!("Add implementing links for use case {}", use_case),
            });
        }
        for function in &report.untraced_functions {
            tasks.push(TraceabilityTask {
                priority: "medium".into(),
                source_id: function.clone(),
                action: "trace_function".into(),
                description: format!(
                    "Add requirement/use-case trace links for function {}",
                    function
                ),
            });
        }
        for gap in &report.gaps {
            if gap.gap_type == "dangling_traceability_link" {
                tasks.push(TraceabilityTask {
                    priority: "high".into(),
                    source_id: gap.source_id.clone(),
                    action: "repair_dangling_link".into(),
                    description: gap.description.clone(),
                });
            }
        }
        tasks.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then(a.source_id.cmp(&b.source_id))
        });
        RunPlanningHint {
            category: "traceability".into(),
            summary: format!(
                "{} requirement gaps, {} use case gaps, {} function gaps",
                report.untraced_requirements.len(),
                report.untraced_use_cases.len(),
                report.untraced_functions.len()
            ),
            tasks,
        }
    }
    // END_run_manager_traceability_plan

    // START_CONTRACT_RunManager::build_gate_policy
    // PURPOSE: Build machine-readable gate policy for a bounded autonomous run from current project health
    // INPUTS: { phase: &str }, { module_id: &str }, { report: &crate::grace::status::StatusReport }
    // OUTPUTS: { RunGatePolicy }
    // LINKS:
    //   → UC-002 (implements) - convert verification and review expectations into execution gates
    //   → NFR-002 (traces_to) - policy generation must surface blocking conditions explicitly
    // START_run_manager_build_gate_policy
    pub fn build_gate_policy(
        &self,
        phase: &str,
        module_id: &str,
        report: &crate::grace::status::StatusReport,
    ) -> RunGatePolicy {
        let mut gates = Vec::new();
        gates.push(RunGate {
            id: "gate-sharded-artifacts".into(),
            name: "sharded_artifacts".into(),
            required: true,
            status: RunGateStatus::Passed,
            reason: None,
            evidence_refs: vec!["docs/graph-index.xml".into(), "docs/plan-index.xml".into()],
        });
        gates.push(RunGate {
            id: "gate-mental-tests".into(),
            name: "mental_tests".into(),
            required: true,
            status: if report.mental_tests.mental_tests_defined()
                && report.mental_tests.mental_tests_passed()
                && report.mental_tests.mental_test_no_drift()
            {
                RunGateStatus::Passed
            } else {
                RunGateStatus::Blocked
            },
            reason: if report.mental_tests.mental_tests_defined()
                && report.mental_tests.mental_tests_passed()
                && report.mental_tests.mental_test_no_drift()
            {
                None
            } else {
                Some(format!(
                    "mental tests gate blocked: failed={} not_run={} missing_required={} drift={}",
                    report.mental_tests.failed,
                    report.mental_tests.not_run,
                    report.mental_tests.missing_required_targets.len(),
                    report.mental_tests.drift_issues.len()
                ))
            },
            evidence_refs: vec!["docs/development-plan.xml".into()],
        });
        gates.push(RunGate {
            id: "gate-traceability".into(),
            name: "traceability".into(),
            required: true,
            status: if report.traceability.requirements_implemented_gate()
                && report.traceability.code_traced_gate()
                && report.traceability.no_dangling_gate()
            {
                RunGateStatus::Passed
            } else {
                RunGateStatus::Blocked
            },
            reason: if report.traceability.requirements_implemented_gate()
                && report.traceability.code_traced_gate()
                && report.traceability.no_dangling_gate()
            {
                None
            } else {
                Some(format!(
                    "traceability gate blocked: untraced_requirements={} untraced_functions={} dangling_links={}",
                    report.traceability.untraced_requirements.len(),
                    report.traceability.untraced_functions.len(),
                    report
                        .traceability
                        .gaps
                        .iter()
                        .filter(|gap| gap.gap_type == "dangling_traceability_link")
                        .count()
                ))
            },
            evidence_refs: vec!["docs/traceability-index.xml".into()],
        });
        gates.push(RunGate {
            id: "gate-verification".into(),
            name: "verification".into(),
            required: true,
            status: if report.verification.iter().all(|v| v.passed) {
                RunGateStatus::Passed
            } else {
                RunGateStatus::Blocked
            },
            reason: if report.verification.iter().all(|v| v.passed) {
                None
            } else {
                Some(format!(
                    "verification gate blocked: failing_groups={}",
                    report.verification.iter().filter(|v| !v.passed).count()
                ))
            },
            evidence_refs: vec!["syn verify".into()],
        });
        RunGatePolicy {
            phase: phase.to_string(),
            module_id: module_id.to_string(),
            required_gates: gates,
            stop_on_block: true,
            retry_budget: 3,
            escalation_target: "human-review".into(),
        }
    }
    // END_run_manager_build_gate_policy

    // START_CONTRACT_RunManager::evaluate_gate_policy
    // PURPOSE: Evaluate a gate policy and return blocked/pass decision for next autonomous step
    // INPUTS: { policy: &RunGatePolicy }
    // OUTPUTS: { RunGateDecision }
    // LINKS:
    //   → UC-002 (implements) - decide whether bounded execution may continue
    //   → NFR-002 (traces_to) - blocked decisions must explain why execution stopped
    // START_run_manager_evaluate_gate_policy
    pub fn evaluate_gate_policy(&self, policy: &RunGatePolicy) -> RunGateDecision {
        if let Some(blocked_gate) = policy.required_gates.iter().find(|gate| {
            gate.required && matches!(gate.status, RunGateStatus::Blocked | RunGateStatus::Failed)
        }) {
            return RunGateDecision {
                passed: false,
                blocked: true,
                reason: blocked_gate.reason.clone(),
                next_action: Some(format!(
                    "Resolve gate '{}' before execution",
                    blocked_gate.name
                )),
            };
        }
        RunGateDecision {
            passed: true,
            blocked: false,
            reason: None,
            next_action: Some("Proceed to next bounded execution step".into()),
        }
    }
    // END_run_manager_evaluate_gate_policy

    // START_CONTRACT_RunManager::attach_gate_policy
    // PURPOSE: Attach evaluated gate policy snapshot to a run record
    // INPUTS: { record: &mut RunRecord }, { policy: RunGatePolicy }
    // OUTPUTS: { RunGateDecision }
    // LINKS:
    //   → UC-002 (implements) - persist gate snapshot with bounded run state
    // START_run_manager_attach_gate_policy
    pub fn attach_gate_policy(
        &self,
        record: &mut RunRecord,
        policy: RunGatePolicy,
    ) -> RunGateDecision {
        let decision = self.evaluate_gate_policy(&policy);
        record.required_gates = policy.required_gates.clone();
        record.policy = Some(policy);
        record.blocked_reason = decision.reason.clone();
        record.status = if decision.blocked {
            RunStatus::Blocked
        } else {
            RunStatus::Ready
        };
        record.touch();
        decision
    }
    // END_run_manager_attach_gate_policy

    // PURPOSE: Create a run manager bound to current project root
    // INPUTS: { root: impl AsRef<Path> }
    // OUTPUTS: { Self }
    // LINKS:
    //   → UC-002 (implements) - bounded autonomous run setup
    //   → NFR-002 (traces_to) - runtime setup should fail explicitly, not panic
    // START_run_manager_new
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }
    // END_run_manager_new

    // START_CONTRACT_RunManager::runs_dir
    // PURPOSE: Resolve persistent run storage directory
    // OUTPUTS: { PathBuf }
    // LINKS:
    //   → NFR-003 (traces_to) - stable run storage reduces repeated context rebuild work
    // START_run_manager_runs_dir
    pub fn runs_dir(&self) -> PathBuf {
        self.root.join("docs/runs")
    }
    // END_run_manager_runs_dir

    // START_CONTRACT_RunManager::ensure_runs_dir
    // PURPOSE: Ensure persistent run storage directory exists
    // OUTPUTS: { anyhow::Result<PathBuf> }
    // LINKS:
    //   → NFR-002 (traces_to) - directory setup reports filesystem failures explicitly
    // START_run_manager_ensure_runs_dir
    pub fn ensure_runs_dir(&self) -> anyhow::Result<PathBuf> {
        let dir = self.runs_dir();
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }
    // END_run_manager_ensure_runs_dir

    // START_CONTRACT_RunManager::run_path
    // PURPOSE: Resolve JSON file path for one run
    // INPUTS: { run_id: &str }
    // OUTPUTS: { PathBuf }
    // LINKS:
    //   → NFR-003 (traces_to) - deterministic run paths support efficient resume and lookup
    // START_run_manager_run_path
    pub fn run_path(&self, run_id: &str) -> PathBuf {
        self.runs_dir().join(format!("{}.json", run_id))
    }
    // END_run_manager_run_path

    // START_CONTRACT_RunManager::save
    // PURPOSE: Persist run record to durable JSON file
    // INPUTS: { record: &RunRecord }
    // OUTPUTS: { anyhow::Result<()> }
    // LINKS:
    //   → UC-002 (implements) - save bounded autonomous work state
    //   → NFR-002 (traces_to) - persistence errors surface as explicit failures
    // START_run_manager_save
    pub fn save(&self, record: &RunRecord) -> anyhow::Result<()> {
        self.ensure_runs_dir()?;
        let path = self.run_path(&record.run_id);
        let tmp = path.with_extension("json.tmp");
        let data = serde_json::to_vec_pretty(record)?;
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
    // END_run_manager_save

    // START_CONTRACT_RunManager::load
    // PURPOSE: Load run record from durable JSON file
    // INPUTS: { run_id: &str }
    // OUTPUTS: { anyhow::Result<RunRecord> }
    // LINKS:
    //   → UC-002 (implements) - restore bounded autonomous work state
    //   → NFR-002 (traces_to) - malformed or missing run files return explicit errors
    // START_run_manager_load
    pub fn load(&self, run_id: &str) -> anyhow::Result<RunRecord> {
        let path = self.run_path(run_id);
        let data = std::fs::read(&path)?;
        Ok(serde_json::from_slice(&data)?)
    }
    // END_run_manager_load

    // START_CONTRACT_RunManager::list
    // PURPOSE: List persisted runs sorted by file name
    // OUTPUTS: { anyhow::Result<Vec<RunRecord>> }
    // LINKS:
    //   → UC-002 (implements) - inspect bounded autonomous run inventory
    //   → NFR-003 (traces_to) - deterministic listing supports efficient agent resume behavior
    // START_run_manager_list
    pub fn list(&self) -> anyhow::Result<Vec<RunRecord>> {
        let dir = self.ensure_runs_dir()?;
        let mut records = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let data = std::fs::read(&path)?;
            let record: RunRecord = serde_json::from_slice(&data)?;
            records.push(record);
        }
        records.sort_by(|a, b| a.run_id.cmp(&b.run_id));
        Ok(records)
    }
    // END_run_manager_list

    // START_CONTRACT_RunManager::create_run
    // PURPOSE: Create a bounded run record with default step skeleton, attach gate policy, and persist it
    // INPUTS: { goal: &str }, { phase: &str }, { module_id: &str }, { objective: &str }, { report: &crate::grace::status::StatusReport }
    // OUTPUTS: { anyhow::Result<(RunRecord, RunGateDecision)> }
    // LINKS:
    //   → UC-002 (implements) - initialize autonomous bounded change workflow from project health
    //   → NFR-002 (traces_to) - blocked creation returns explicit decision state instead of panicking
    // START_run_manager_create_run
    pub fn create_run(
        &self,
        goal: &str,
        phase: &str,
        module_id: &str,
        objective: &str,
        report: &crate::grace::status::StatusReport,
    ) -> anyhow::Result<(RunRecord, RunGateDecision)> {
        let mut record = RunRecord::new(
            goal.to_string(),
            phase.to_string(),
            module_id.to_string(),
            objective.to_string(),
        );
        record.steps = default_steps(module_id);
        let planning = self.traceability_plan(&report.traceability);
        record
            .metadata
            .insert("traceability_summary".into(), planning.summary.clone());
        if let Some(first_task) = planning.tasks.first() {
            record.metadata.insert(
                "traceability_next_action".into(),
                format!("{}:{}", first_task.action, first_task.source_id),
            );
        }
        let policy = self.build_gate_policy(phase, module_id, report);
        let decision = self.attach_gate_policy(&mut record, policy);
        self.save(&record)?;
        write_provenance_event(
            &record.run_id,
            &record.module_id,
            &record.phase,
            &record.status,
            "create_run",
            &format!(
                "objective={} blocked={}",
                record.objective, decision.blocked
            ),
        );
        Ok((record, decision))
    }
    // END_run_manager_create_run

    // START_CONTRACT_RunManager::start_run
    // PURPOSE: Move a ready run into running state and start its current step
    // INPUTS: { run_id: &str }
    // OUTPUTS: { anyhow::Result<RunRecord> }
    // LINKS:
    //   → UC-002 (implements) - begin bounded autonomous execution only after gates pass
    // START_run_manager_start_run
    pub fn start_run(&self, run_id: &str) -> anyhow::Result<RunRecord> {
        let mut record = self.load(run_id)?;
        if matches!(record.status, RunStatus::Blocked) {
            return Err(anyhow::anyhow!(
                "cannot start blocked run {}: {}",
                run_id,
                record
                    .blocked_reason
                    .clone()
                    .unwrap_or_else(|| "gate policy blocked execution".into())
            ));
        }
        if record.steps.is_empty() {
            record.steps = default_steps(&record.module_id);
        }
        if let Some(step) = record.steps.get_mut(record.current_step) {
            step.status = RunStepStatus::Running;
        }
        record.status = RunStatus::Running;
        record.touch();
        self.save(&record)?;
        write_provenance_event(
            &record.run_id,
            &record.module_id,
            &record.phase,
            &record.status,
            "start_run",
            "run entered running state",
        );
        Ok(record)
    }
    // END_run_manager_start_run

    // START_CONTRACT_RunManager::resume_run
    // PURPOSE: Resume a persisted run from blocked, ready, or running state without losing current-step context
    // INPUTS: { run_id: &str }
    // OUTPUTS: { anyhow::Result<RunRecord> }
    // LINKS:
    //   → UC-002 (implements) - restore bounded execution after interruption or gate fix
    // START_run_manager_resume_run
    pub fn resume_run(&self, run_id: &str) -> anyhow::Result<RunRecord> {
        let mut record = self.load(run_id)?;
        if matches!(
            record.status,
            RunStatus::Completed | RunStatus::Escalated | RunStatus::Failed
        ) {
            return Ok(record);
        }
        if matches!(record.status, RunStatus::Blocked)
            && !matches!(
                record.latest_review_status(),
                Some(RunReviewStatus::Approved)
            )
        {
            anyhow::bail!(
                "blocked run {} requires approved review before resume",
                run_id
            );
        }
        if matches!(record.status, RunStatus::Ready | RunStatus::Blocked) {
            record.status = RunStatus::Running;
            record.blocked_reason = None;
        }
        if let Some(step) = record.steps.get_mut(record.current_step) {
            if matches!(step.status, RunStepStatus::Pending | RunStepStatus::Skipped) {
                step.status = RunStepStatus::Running;
            }
        }
        record.touch();
        self.save(&record)?;
        Ok(record)
    }
    // END_run_manager_resume_run

    // START_CONTRACT_RunManager::complete_current_step
    // PURPOSE: Mark current step passed, advance bounded workflow, and complete run when no steps remain
    // INPUTS: { run_id: &str }, { evidence_ref: Option<&str> }
    // OUTPUTS: { anyhow::Result<RunRecord> }
    // LINKS:
    //   → UC-002 (implements) - capture bounded step progress with explicit evidence
    // START_run_manager_complete_current_step
    pub fn complete_current_step(
        &self,
        run_id: &str,
        evidence_ref: Option<&str>,
    ) -> anyhow::Result<RunRecord> {
        let mut record = self.load(run_id)?;
        let last_index = record.steps.len().saturating_sub(1);
        if let Some(step) = record.steps.get_mut(record.current_step) {
            step.status = RunStepStatus::Passed;
            if let Some(evidence) = evidence_ref {
                step.evidence_refs.push(evidence.to_string());
                record.evidence_refs.push(evidence.to_string());
            }
        }
        if record.current_step >= last_index {
            record.status = RunStatus::Completed;
            record.outcome = Some(RunOutcome {
                kind: RunOutcomeKind::Success,
                summary: format!("run {} completed", record.run_id),
                blocked_reason: None,
                escalation_reason: None,
            });
        } else {
            record.current_step += 1;
            if let Some(step) = record.steps.get_mut(record.current_step) {
                step.status = RunStepStatus::Running;
            }
            record.status = RunStatus::Running;
        }
        record.touch();
        self.save(&record)?;
        write_provenance_event(
            &record.run_id,
            &record.module_id,
            &record.phase,
            &record.status,
            "complete_current_step",
            &format!(
                "step_index={} evidence_count={}",
                record.current_step,
                record.evidence_refs.len()
            ),
        );
        Ok(record)
    }
    // END_run_manager_complete_current_step

    // START_CONTRACT_RunManager::block_run
    // PURPOSE: Mark run as blocked with explicit reason and optional evidence reference
    // INPUTS: { run_id: &str }, { reason: &str }, { evidence_ref: Option<&str> }
    // OUTPUTS: { anyhow::Result<RunRecord> }
    // LINKS:
    //   → NFR-002 (traces_to) - blocked runtime state must preserve exact stop reason
    // START_run_manager_block_run
    pub fn block_run(
        &self,
        run_id: &str,
        reason: &str,
        evidence_ref: Option<&str>,
    ) -> anyhow::Result<RunRecord> {
        let mut record = self.load(run_id)?;
        record.status = RunStatus::Blocked;
        record.blocked_reason = Some(reason.to_string());
        if let Some(step) = record.steps.get_mut(record.current_step) {
            step.status = RunStepStatus::Failed;
            step.error = Some(reason.to_string());
            if let Some(evidence) = evidence_ref {
                step.evidence_refs.push(evidence.to_string());
            }
        }
        if let Some(evidence) = evidence_ref {
            record.evidence_refs.push(evidence.to_string());
        }
        record.outcome = Some(RunOutcome {
            kind: RunOutcomeKind::Blocked,
            summary: format!("run {} blocked", record.run_id),
            blocked_reason: Some(reason.to_string()),
            escalation_reason: None,
        });
        record.touch();
        self.save(&record)?;
        write_provenance_event(
            &record.run_id,
            &record.module_id,
            &record.phase,
            &record.status,
            "block_run",
            reason,
        );
        Ok(record)
    }
    // END_run_manager_block_run

    // START_CONTRACT_RunManager::escalate_run
    // PURPOSE: Escalate a run to human review with explicit escalation reason
    // INPUTS: { run_id: &str }, { reason: &str }
    // OUTPUTS: { anyhow::Result<RunRecord> }
    // LINKS:
    //   → UC-002 (implements) - bounded workflow escalates instead of retrying forever
    // START_run_manager_escalate_run
    pub fn escalate_run(&self, run_id: &str, reason: &str) -> anyhow::Result<RunRecord> {
        let mut record = self.load(run_id)?;
        record.status = RunStatus::Escalated;
        record.escalation_reason = Some(reason.to_string());
        record.outcome = Some(RunOutcome {
            kind: RunOutcomeKind::Escalated,
            summary: format!("run {} escalated", record.run_id),
            blocked_reason: record.blocked_reason.clone(),
            escalation_reason: Some(reason.to_string()),
        });
        record.touch();
        self.save(&record)?;
        write_provenance_event(
            &record.run_id,
            &record.module_id,
            &record.phase,
            &record.status,
            "escalate_run",
            reason,
        );
        Ok(record)
    }
    // END_run_manager_escalate_run
}

fn write_provenance_event(
    run_id: &str,
    module_id: &str,
    phase: &str,
    status: &RunStatus,
    event_type: &str,
    detail: &str,
) {
    let config = Config::load_or_default();
    if tokio::runtime::Handle::try_current().is_ok() {
        let run_id = run_id.to_string();
        let event_type = event_type.to_string();
        let module_id = module_id.to_string();
        let phase = phase.to_string();
        let status = format!("{:?}", status).to_lowercase();
        let detail = detail.to_string();
        let config = config.clone();
        let handle = std::thread::spawn(move || {
            let tracker = Tracker::new(&config);
            if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                let _ = rt.block_on(tracker.record_run_event(
                    &run_id,
                    &event_type,
                    &module_id,
                    &phase,
                    &status,
                    &detail,
                ));
            }
        });
        let _ = handle.join();
    }
}

fn recovery_hint(reason: &str) -> String {
    let lower = reason.to_ascii_lowercase();
    if lower.contains("mental test") {
        "Run mental_test_run or add missing MentalTests before retry".into()
    } else if lower.contains("traceability") || lower.contains("requirement") {
        "Repair trace links before retry".into()
    } else if lower.contains("verify") || lower.contains("verification") {
        "Fix failing verification checks before retry".into()
    } else if lower.contains("drift") {
        "Run refresh_project and repair drift before retry".into()
    } else {
        "Inspect blocked reason and patch smallest cause before retry".into()
    }
}

fn default_steps(module_id: &str) -> Vec<RunStep> {
    vec![
        RunStep {
            id: format!("{}-step-1", module_id.to_lowercase()),
            name: "read-artifacts".into(),
            description: format!("Read bounded artifact context for {}", module_id),
            status: RunStepStatus::Pending,
            evidence_refs: Vec::new(),
            error: None,
        },
        RunStep {
            id: format!("{}-step-2", module_id.to_lowercase()),
            name: "check-gates".into(),
            description: format!("Evaluate autonomous execution gates for {}", module_id),
            status: RunStepStatus::Pending,
            evidence_refs: Vec::new(),
            error: None,
        },
        RunStep {
            id: format!("{}-step-3", module_id.to_lowercase()),
            name: "apply-bounded-change".into(),
            description: format!("Apply smallest safe bounded change for {}", module_id),
            status: RunStepStatus::Pending,
            evidence_refs: Vec::new(),
            error: None,
        },
        RunStep {
            id: format!("{}-step-4", module_id.to_lowercase()),
            name: "verify-and-review".into(),
            description: format!(
                "Run verification and review after bounded change for {}",
                module_id
            ),
            status: RunStepStatus::Pending,
            evidence_refs: Vec::new(),
            error: None,
        },
    ]
}

// START_run_record_constructors
impl RunRecord {
    // START_CONTRACT_RunRecord::new
    // PURPOSE: Create a new bounded run record with default planned state
    // INPUTS: { goal: String }, { phase: String }, { module_id: String }, { objective: String }
    // OUTPUTS: { Self }
    // LINKS:
    //   → UC-002 (implements) - initialize bounded autonomous change workflow
    //   → NFR-003 (traces_to) - explicit run metadata reduces repeated context reconstruction
    // START_run_record_new
    pub fn new(goal: String, phase: String, module_id: String, objective: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            run_id: format!("run-{}", uuid::Uuid::new_v4()),
            goal,
            phase,
            module_id,
            objective,
            status: RunStatus::Planned,
            current_step: 0,
            policy: None,
            required_gates: Vec::new(),
            steps: Vec::new(),
            evidence_refs: Vec::new(),
            blocked_reason: None,
            escalation_reason: None,
            metadata: BTreeMap::new(),
            review_decisions: Vec::new(),
            outcome: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
    // END_run_record_new

    // START_CONTRACT_RunRecord::touch
    // PURPOSE: Update run record timestamp after a state transition
    // OUTPUTS: { () }
    // LINKS:
    //   → NFR-003 (traces_to) - timestamp refresh supports efficient resume and stale-run detection
    // START_run_record_touch
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
    // END_run_record_touch
}
// END_run_record_constructors

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_record_round_trips_through_json() {
        let mut record = RunRecord::new(
            "Improve autonomous runtime".into(),
            "Phase-17".into(),
            "M-RUNNER".into(),
            "Persist bounded run state".into(),
        );
        record.status = RunStatus::Ready;
        record.required_gates.push(RunGate {
            id: "gate-mental-test".into(),
            name: "mental_test".into(),
            required: true,
            status: RunGateStatus::Pending,
            reason: None,
            evidence_refs: vec!["docs/development-plan.xml#MT-001".into()],
        });
        record.steps.push(RunStep {
            id: "step-1".into(),
            name: "create-run".into(),
            description: "create first durable run".into(),
            status: RunStepStatus::Pending,
            evidence_refs: Vec::new(),
            error: None,
        });

        let json = serde_json::to_string_pretty(&record).unwrap();
        let restored: RunRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.goal, record.goal);
        assert_eq!(restored.phase, "Phase-17");
        assert_eq!(restored.required_gates.len(), 1);
        assert_eq!(restored.steps.len(), 1);
        assert_eq!(restored.status, RunStatus::Ready);
    }

    #[test]
    fn run_manager_persists_and_lists_records() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let first = RunRecord::new(
            "goal-a".into(),
            "Phase-17".into(),
            "M-RUNNER".into(),
            "objective-a".into(),
        );
        let second = RunRecord::new(
            "goal-b".into(),
            "Phase-17".into(),
            "M-RUNNER".into(),
            "objective-b".into(),
        );
        manager.save(&first).unwrap();
        manager.save(&second).unwrap();

        let loaded = manager.load(&first.run_id).unwrap();
        assert_eq!(loaded.goal, "goal-a");

        let listed = manager.list().unwrap();
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().any(|r| r.run_id == first.run_id));
        assert!(listed.iter().any(|r| r.run_id == second.run_id));
    }

    #[test]
    fn run_manager_blocks_when_mental_tests_missing() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let report = rt.block_on(crate::grace::status::StatusCollector::collect(root.path()));
        let policy = manager.build_gate_policy("Phase-17", "M-RUNNER", &report.unwrap());
        let decision = manager.evaluate_gate_policy(&policy);
        assert!(decision.blocked);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn traceability_plan_turns_gaps_into_tasks() {
        let manager = RunManager::new(".");
        let report = crate::grace::traceability::TraceabilityReport {
            requirements_total: 1,
            use_cases_total: 1,
            modules_with_traceability: 0,
            functions_with_traceability: 0,
            logs_with_traceability: 0,
            total_functions: 1,
            total_logs: 0,
            untraced_requirements: vec!["REQ-001".into()],
            untraced_use_cases: vec!["UC-001".into()],
            untraced_functions: vec!["M-RUNNER::start_run".into()],
            traceability_score: 0.0,
            enforcement_mode: "strict".into(),
            gaps: vec![crate::grace::traceability::TraceGap {
                gap_type: "dangling_traceability_link".into(),
                source_id: "M-RUNNER::start_run".into(),
                description: "dangling link to missing artifact".into(),
            }],
            chains: Vec::new(),
        };
        let plan = manager.traceability_plan(&report);
        assert_eq!(plan.category, "traceability");
        assert_eq!(plan.tasks.len(), 4);
        assert!(plan
            .tasks
            .iter()
            .any(|task| task.action == "trace_requirement"));
        assert!(plan
            .tasks
            .iter()
            .any(|task| task.action == "repair_dangling_link"));
    }

    #[test]
    fn attempt_recovery_retries_then_escalates() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let mut record = RunRecord::new(
            "goal-d".into(),
            "Phase-17".into(),
            "M-RUNNER".into(),
            "objective-d".into(),
        );
        record.steps = default_steps("M-RUNNER");
        record.status = RunStatus::Blocked;
        record.blocked_reason = Some("verification gate blocked".into());
        record.policy = Some(RunGatePolicy {
            phase: "Phase-17".into(),
            module_id: "M-RUNNER".into(),
            required_gates: Vec::new(),
            stop_on_block: true,
            retry_budget: 1,
            escalation_target: "human-review".into(),
        });
        manager.save(&record).unwrap();

        let first = manager.attempt_recovery(&record.run_id).unwrap();
        assert!(first.retry_allowed);
        assert_eq!(first.retries_used, 1);

        let second = manager.attempt_recovery(&record.run_id).unwrap();
        assert!(!second.retry_allowed);
        assert_eq!(second.retries_remaining, 0);
    }

    #[test]
    fn run_manager_advances_bounded_workflow() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());

        let mut record = RunRecord::new(
            "goal-c".into(),
            "Phase-17".into(),
            "M-RUNNER".into(),
            "objective-c".into(),
        );
        record.steps = default_steps("M-RUNNER");
        record.status = RunStatus::Ready;
        manager.save(&record).unwrap();

        let started = manager.start_run(&record.run_id).unwrap();
        assert_eq!(started.status, RunStatus::Running);
        assert_eq!(started.steps[0].status, RunStepStatus::Running);

        let progressed = manager
            .complete_current_step(&record.run_id, Some("evidence://step-1"))
            .unwrap();
        assert_eq!(progressed.current_step, 1);
        assert_eq!(progressed.steps[0].status, RunStepStatus::Passed);
        assert_eq!(progressed.steps[1].status, RunStepStatus::Running);

        let blocked = manager
            .block_run(&record.run_id, "verification pending", Some("syn verify"))
            .unwrap();
        assert_eq!(blocked.status, RunStatus::Blocked);
        assert_eq!(blocked.outcome.unwrap().kind, RunOutcomeKind::Blocked);
    }

    #[test]
    fn run_review_approval_allows_blocked_resume_and_replay() {
        let root = tempfile::tempdir().unwrap();
        let manager = RunManager::new(root.path());
        let mut record = RunRecord::new(
            "goal-e".into(),
            "Phase-19".into(),
            "M-RUNNER".into(),
            "objective-e".into(),
        );
        record.steps = default_steps("M-RUNNER");
        record.status = RunStatus::Ready;
        manager.save(&record).unwrap();
        manager.start_run(&record.run_id).unwrap();
        let blocked = manager
            .block_run(&record.run_id, "review required", Some("verify.log"))
            .unwrap();
        assert!(manager.resume_run(&blocked.run_id).is_err());

        let reviewed = manager
            .approve_run(
                &blocked.run_id,
                "maintainer",
                "bounded retry approved",
                None,
            )
            .unwrap();
        assert_eq!(
            reviewed.latest_review_status(),
            Some(RunReviewStatus::Approved)
        );
        let resumed = manager.resume_run(&blocked.run_id).unwrap();
        assert_eq!(resumed.status, RunStatus::Running);
        let replay = resumed.replay();
        assert!(replay.events.iter().any(|event| event.kind == "review"));
    }
}
// END_public_api
