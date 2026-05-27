// MODULE_CONTRACT
// MODULE_ID: M-WORKSPACE
// PURPOSE: Multi-project workspace orchestration for Synapse member projects.
// SCOPE: Synapse.toml config model, workspace init/load validation, member resolution, verify/status/index/coverage aggregation, parallel verify execution, fail-fast policy, and compact reports.
// DEPENDS: M-CONFIG, M-GRACE-VERIFY, M-GRACE-STATUS, M-INDEXER, M-TEST-COVERAGE-MATRIX
// LINKS:
//   -> Phase-91 (implements) - multi-project workspace orchestration
//   -> NFR-002 (traces_to) - reliable release verification commands
//   -> NFR-003 (traces_to) - compact cross-project evidence

// START_MODULE_MAP
// WorkspaceConfig - Synapse.toml root configuration
// WorkspaceSection - Workspace members, excludes, and defaults
// WorkspaceDefaults - Default profile, parallel, and fail-fast policy
// Workspace - Loaded workspace with resolved members
// WorkspaceMember - One resolved workspace member
// WorkspaceRunOptions - Execution options shared by workspace commands
// WorkspaceReport - Aggregated command report
// WorkspaceMemberReport - Per-member command outcome
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added Phase-91 multi-project workspace engine]
// END_CHANGE_SUMMARY

use syn_core::config::Config;
use syn_engine::grace::contract::GraceProfile;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::task::JoinSet;

const WORKSPACE_FILE: &str = "Synapse.toml";

// START_public_api

// START_WorkspaceConfig
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WorkspaceConfig {
    pub workspace: WorkspaceSection,
}
// END_WorkspaceConfig

// START_WorkspaceSection
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WorkspaceSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub members: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub defaults: WorkspaceDefaults,
}
// END_WorkspaceSection

// START_WorkspaceDefaults
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WorkspaceDefaults {
    #[serde(default = "default_profile")]
    pub profile: String,
    #[serde(default = "default_true")]
    pub parallel: bool,
    #[serde(default = "default_true")]
    pub fail_fast: bool,
}
// END_WorkspaceDefaults

impl Default for WorkspaceDefaults {
    // START_CONTRACT_WorkspaceDefaults::default
    // PURPOSE: Provide release-safe workspace command defaults.
    // OUTPUTS: { WorkspaceDefaults }
    // LINKS:
    //   -> Phase-91 (implements) - workspace default policy
    //   -> NFR-002 (traces_to) - reliable release verification commands
    // START_workspace_defaults_default
    fn default() -> Self {
        Self {
            profile: default_profile(),
            parallel: true,
            fail_fast: true,
        }
    }
    // END_workspace_defaults_default
}

// START_WorkspaceMember
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkspaceMember {
    pub spec: String,
    pub path: PathBuf,
}
// END_WorkspaceMember

// START_Workspace
#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
    config: WorkspaceConfig,
    members: Vec<WorkspaceMember>,
}
// END_Workspace

// START_WorkspaceRunOptions
#[derive(Debug, Clone)]
pub struct WorkspaceRunOptions {
    pub profile: GraceProfile,
    pub parallel: bool,
    pub fail_fast: bool,
}
// END_WorkspaceRunOptions

// START_WorkspaceReport
#[derive(Debug, Clone, Default, Serialize)]
pub struct WorkspaceReport {
    pub command: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub members: Vec<WorkspaceMemberReport>,
}
// END_WorkspaceReport

// START_WorkspaceMemberReport
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceMemberReport {
    pub member: String,
    pub path: String,
    pub passed: bool,
    pub summary: String,
}
// END_WorkspaceMemberReport

impl Workspace {
    // START_CONTRACT_Workspace::init
    // PURPOSE: Write a Synapse.toml workspace config at the provided root.
    // INPUTS: { root: &Path }, { members: Vec<String> }, { name: Option<String> }
    // OUTPUTS: { anyhow::Result<PathBuf> }
    // SIDE_EFFECTS: writes Synapse.toml
    // LINKS:
    //   -> Phase-91 (implements) - workspace init
    //   -> NFR-002 (traces_to) - reliable release verification commands
    //   <- V-M-WORKSPACE (verified_by) - workspace init tests
    // START_workspace_init
    pub fn init(
        root: &Path,
        members: Vec<String>,
        name: Option<String>,
    ) -> anyhow::Result<PathBuf> {
        if members.is_empty() {
            anyhow::bail!("workspace init requires at least one member path");
        }
        let config = WorkspaceConfig {
            workspace: WorkspaceSection {
                name,
                members,
                exclude: Vec::new(),
                defaults: WorkspaceDefaults::default(),
            },
        };
        let path = root.join(WORKSPACE_FILE);
        std::fs::write(&path, toml::to_string_pretty(&config)?)?;
        Ok(path)
    }
    // END_workspace_init

    // START_CONTRACT_Workspace::load
    // PURPOSE: Load and validate a Synapse.toml workspace from a root directory.
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<Workspace> }
    // LINKS:
    //   -> Phase-91 (implements) - workspace config loading
    //   -> NFR-002 (traces_to) - reliable release verification commands
    //   <- V-M-WORKSPACE (verified_by) - workspace load tests
    // START_workspace_load
    pub fn load(root: &Path) -> anyhow::Result<Self> {
        let config_path = root.join(WORKSPACE_FILE);
        if !config_path.exists() {
            anyhow::bail!(
                "No Synapse.toml found in {}. Run 'syn workspace init' first.",
                root.display()
            );
        }
        let content = std::fs::read_to_string(&config_path)?;
        let config = toml::from_str::<WorkspaceConfig>(&content)?;
        let members = resolve_members(root, &config)?;
        Ok(Self {
            root: root.to_path_buf(),
            config,
            members,
        })
    }
    // END_workspace_load

    // START_CONTRACT_Workspace::root
    // PURPOSE: Return the workspace root directory.
    // OUTPUTS: { &Path }
    // LINKS:
    //   -> Phase-91 (implements) - workspace list/status output
    //   <- V-M-WORKSPACE (verified_by) - workspace load tests
    // START_workspace_root
    pub fn root(&self) -> &Path {
        &self.root
    }
    // END_workspace_root

    // START_CONTRACT_Workspace::members
    // PURPOSE: Return resolved workspace members.
    // OUTPUTS: { &[WorkspaceMember] }
    // LINKS:
    //   -> Phase-91 (implements) - workspace member listing
    //   <- V-M-WORKSPACE (verified_by) - workspace list tests
    // START_workspace_members
    pub fn members(&self) -> &[WorkspaceMember] {
        &self.members
    }
    // END_workspace_members

    // START_CONTRACT_Workspace::defaults
    // PURPOSE: Return workspace default execution policy.
    // OUTPUTS: { &WorkspaceDefaults }
    // LINKS:
    //   -> Phase-91 (implements) - workspace default policy
    //   <- V-M-WORKSPACE (verified_by) - workspace defaults tests
    // START_workspace_defaults
    pub fn defaults(&self) -> &WorkspaceDefaults {
        &self.config.workspace.defaults
    }
    // END_workspace_defaults

    // START_CONTRACT_Workspace::verify_all
    // PURPOSE: Run MyGRACE verification for every member and aggregate results.
    // INPUTS: { options: WorkspaceRunOptions }
    // OUTPUTS: { anyhow::Result<WorkspaceReport> }
    // SIDE_EFFECTS: runs verification against member directories
    // LINKS:
    //   -> Phase-91 (implements) - workspace verify
    //   -> M-GRACE-VERIFY (depends) - member verification
    //   -> NFR-002 (traces_to) - reliable release verification commands
    //   <- V-M-WORKSPACE (verified_by) - workspace verify tests
    // START_workspace_verify_all
    pub async fn verify_all(
        &self,
        options: WorkspaceRunOptions,
    ) -> anyhow::Result<WorkspaceReport> {
        if options.parallel && self.members.len() > 1 {
            self.verify_parallel(options).await
        } else {
            self.verify_sequential(options).await
        }
    }
    // END_workspace_verify_all

    // START_CONTRACT_Workspace::status_all
    // PURPOSE: Collect aggregate health status for all members.
    // OUTPUTS: { anyhow::Result<WorkspaceReport> }
    // LINKS:
    //   -> Phase-91 (implements) - workspace status
    //   -> NFR-003 (traces_to) - compact cross-project evidence
    //   <- V-M-WORKSPACE (verified_by) - workspace status tests
    // START_workspace_status_all
    pub async fn status_all(&self) -> anyhow::Result<WorkspaceReport> {
        let mut report = WorkspaceReport::new("status");
        for member in &self.members {
            let result = status_member(member).await;
            report.push(result);
        }
        Ok(report)
    }
    // END_workspace_status_all

    // START_CONTRACT_Workspace::index_all
    // PURPOSE: Index every workspace member with the existing indexer.
    // INPUTS: { config: &Config }, { force: bool }
    // OUTPUTS: { anyhow::Result<WorkspaceReport> }
    // SIDE_EFFECTS: writes index storage for each member
    // LINKS:
    //   -> Phase-91 (implements) - workspace index
    //   -> M-INDEXER (depends) - member indexing
    //   <- V-M-WORKSPACE (verified_by) - workspace index tests
    // START_workspace_index_all
    pub async fn index_all(&self, config: &Config, force: bool) -> anyhow::Result<WorkspaceReport> {
        let mut report = WorkspaceReport::new("index");
        for member in &self.members {
            report.push(index_member(member, config, force).await);
        }
        Ok(report)
    }
    // END_workspace_index_all

    // START_CONTRACT_Workspace::coverage_all
    // PURPOSE: Build coverage matrices for all members and aggregate member-level outcomes.
    // OUTPUTS: { anyhow::Result<WorkspaceReport> }
    // LINKS:
    //   -> Phase-91 (implements) - workspace coverage
    //   -> M-TEST-COVERAGE-MATRIX (depends) - member coverage matrix
    //   -> NFR-003 (traces_to) - compact cross-project evidence
    // START_workspace_coverage_all
    pub fn coverage_all(&self) -> anyhow::Result<WorkspaceReport> {
        let mut report = WorkspaceReport::new("coverage");
        for member in &self.members {
            report.push(coverage_member(member));
        }
        Ok(report)
    }
    // END_workspace_coverage_all

    // START_CONTRACT_Workspace::verify_sequential
    // PURPOSE: Run member verification sequentially with optional fail-fast.
    // INPUTS: { options: WorkspaceRunOptions }
    // OUTPUTS: { anyhow::Result<WorkspaceReport> }
    // LINKS:
    //   -> Phase-91 (implements) - sequential workspace verify
    //   <- V-M-WORKSPACE (verified_by) - sequential verify tests
    // START_workspace_verify_sequential
    async fn verify_sequential(
        &self,
        options: WorkspaceRunOptions,
    ) -> anyhow::Result<WorkspaceReport> {
        let mut report = WorkspaceReport::new("verify");
        for member in &self.members {
            let result = verify_member(member, options.profile).await;
            let failed = !result.passed;
            report.push(result);
            if failed && options.fail_fast {
                break;
            }
        }
        Ok(report)
    }
    // END_workspace_verify_sequential

    // START_CONTRACT_Workspace::verify_parallel
    // PURPOSE: Run member verification concurrently and abort remaining work on fail-fast failure.
    // INPUTS: { options: WorkspaceRunOptions }
    // OUTPUTS: { anyhow::Result<WorkspaceReport> }
    // SIDE_EFFECTS: spawns Tokio tasks for member verification
    // LINKS:
    //   -> Phase-91 (implements) - parallel workspace verify
    //   <- V-M-WORKSPACE (verified_by) - parallel verify tests
    // START_workspace_verify_parallel
    async fn verify_parallel(
        &self,
        options: WorkspaceRunOptions,
    ) -> anyhow::Result<WorkspaceReport> {
        let mut set = JoinSet::new();
        for member in self.members.clone() {
            let profile = options.profile;
            set.spawn(async move { verify_member(&member, profile).await });
        }

        let mut report = WorkspaceReport::new("verify");
        while let Some(joined) = set.join_next().await {
            let result = joined?;
            let failed = !result.passed;
            report.push(result);
            if failed && options.fail_fast {
                set.abort_all();
                break;
            }
        }
        report
            .members
            .sort_by(|left, right| left.path.cmp(&right.path));
        Ok(report)
    }
    // END_workspace_verify_parallel
}

impl WorkspaceReport {
    // START_CONTRACT_WorkspaceReport::new
    // PURPOSE: Create an empty report for one workspace command.
    // INPUTS: { command: &str }
    // OUTPUTS: { WorkspaceReport }
    // LINKS:
    //   -> Phase-91 (implements) - workspace report aggregation
    // START_workspace_report_new
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
            ..Self::default()
        }
    }
    // END_workspace_report_new

    // START_CONTRACT_WorkspaceReport::push
    // PURPOSE: Add one member result and update aggregate counters.
    // INPUTS: { result: WorkspaceMemberReport }
    // OUTPUTS: { () }
    // LINKS:
    //   -> Phase-91 (implements) - workspace report aggregation
    //   -> NFR-003 (traces_to) - compact cross-project evidence
    // START_workspace_report_push
    pub fn push(&mut self, result: WorkspaceMemberReport) {
        self.total += 1;
        if result.passed {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
        self.members.push(result);
    }
    // END_workspace_report_push
}

// END_public_api

// START_CONTRACT_resolve_members
// PURPOSE: Resolve configured members while applying exact exclude paths.
// INPUTS: { root: &Path }, { config: &WorkspaceConfig }
// OUTPUTS: { anyhow::Result<Vec<WorkspaceMember>> }
// LINKS:
//   -> Phase-91 (implements) - workspace member resolution
//   <- V-M-WORKSPACE (verified_by) - member resolution tests
// START_resolve_members
fn resolve_members(root: &Path, config: &WorkspaceConfig) -> anyhow::Result<Vec<WorkspaceMember>> {
    let mut members = Vec::new();
    for spec in &config.workspace.members {
        if config
            .workspace
            .exclude
            .iter()
            .any(|exclude| exclude == spec)
        {
            continue;
        }
        let path = resolve_member_path(root, spec);
        if !path.exists() {
            anyhow::bail!("Workspace member not found: {}", path.display());
        }
        members.push(WorkspaceMember {
            spec: spec.clone(),
            path,
        });
    }
    if members.is_empty() {
        anyhow::bail!("workspace has no resolved members");
    }
    Ok(members)
}
// END_resolve_members

// START_CONTRACT_resolve_member_path
// PURPOSE: Resolve an absolute or root-relative workspace member path.
// INPUTS: { root: &Path }, { spec: &str }
// OUTPUTS: { PathBuf }
// LINKS:
//   -> Phase-91 (implements) - workspace member resolution
// START_resolve_member_path
fn resolve_member_path(root: &Path, spec: &str) -> PathBuf {
    let path = PathBuf::from(spec);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}
// END_resolve_member_path

// START_CONTRACT_parse_workspace_profile
// PURPOSE: Parse a workspace profile with actionable error text.
// INPUTS: { profile: &str }
// OUTPUTS: { anyhow::Result<GraceProfile> }
// LINKS:
//   -> Phase-91 (implements) - workspace verify profile
//   -> NFR-002 (traces_to) - reliable release verification commands
// START_parse_workspace_profile
pub fn parse_workspace_profile(profile: &str) -> anyhow::Result<GraceProfile> {
    GraceProfile::from_name(profile)
        .ok_or_else(|| anyhow::anyhow!("unsupported workspace profile: {profile}"))
}
// END_parse_workspace_profile

// START_CONTRACT_verify_member
// PURPOSE: Run profile-aware MyGRACE verification for one workspace member.
// INPUTS: { member: &WorkspaceMember }, { profile: GraceProfile }
// OUTPUTS: { WorkspaceMemberReport }
// LINKS:
//   -> Phase-91 (implements) - workspace verify
//   -> M-GRACE-VERIFY (depends) - member verification
// START_verify_member
async fn verify_member(member: &WorkspaceMember, profile: GraceProfile) -> WorkspaceMemberReport {
    match syn_engine::grace::GraceEngine::verify_project_with_profile(&member.path, profile).await {
        Ok(results) => {
            let passed = results.iter().all(|result| result.passed);
            let checks = results
                .iter()
                .map(|result| result.checks.len())
                .sum::<usize>();
            WorkspaceMemberReport::new(
                member,
                passed,
                format!("{} verification levels, {} checks", results.len(), checks),
            )
        }
        Err(error) => WorkspaceMemberReport::new(member, false, error.to_string()),
    }
}
// END_verify_member

// START_CONTRACT_status_member
// PURPOSE: Collect health status for one workspace member.
// INPUTS: { member: &WorkspaceMember }
// OUTPUTS: { WorkspaceMemberReport }
// LINKS:
//   -> Phase-91 (implements) - workspace status
//   -> M-GRACE-VERIFY (depends) - status verification summary
// START_status_member
async fn status_member(member: &WorkspaceMember) -> WorkspaceMemberReport {
    match syn_engine::grace::status::StatusCollector::collect(&member.path).await {
        Ok(status) => WorkspaceMemberReport::new(
            member,
            status.health == "healthy",
            format!("health={} issues={}", status.health, status.mygrace_issues),
        ),
        Err(error) => WorkspaceMemberReport::new(member, false, error.to_string()),
    }
}
// END_status_member

// START_CONTRACT_index_member
// PURPOSE: Index one workspace member with the existing indexer.
// INPUTS: { member: &WorkspaceMember }, { config: &Config }, { force: bool }
// OUTPUTS: { WorkspaceMemberReport }
// SIDE_EFFECTS: writes index storage
// LINKS:
//   -> Phase-91 (implements) - workspace index
//   -> M-INDEXER (depends) - member indexing
// START_index_member
async fn index_member(
    member: &WorkspaceMember,
    config: &Config,
    force: bool,
) -> WorkspaceMemberReport {
    let indexer = syn_engine::indexer::Indexer::new(config);
    let result = if force {
        indexer
            .index_directory_with_gitignore(&member.path, true)
            .await
    } else {
        indexer.index_directory(&member.path).await
    };
    match result {
        Ok(()) => WorkspaceMemberReport::new(member, true, "indexed".to_string()),
        Err(error) => WorkspaceMemberReport::new(member, false, error.to_string()),
    }
}
// END_index_member

// START_CONTRACT_coverage_member
// PURPOSE: Build one member coverage matrix summary.
// INPUTS: { member: &WorkspaceMember }
// OUTPUTS: { WorkspaceMemberReport }
// LINKS:
//   -> Phase-91 (implements) - workspace coverage
//   -> M-TEST-COVERAGE-MATRIX (depends) - coverage matrix builder
// START_coverage_member
fn coverage_member(member: &WorkspaceMember) -> WorkspaceMemberReport {
    match crate::test::coverage::build_coverage_matrix(&member.path) {
        Ok(matrix) => WorkspaceMemberReport::new(
            member,
            true,
            format!(
                "covered={} partial={} uncovered={}",
                matrix.summary.covered_modules,
                matrix.summary.partial_modules,
                matrix.summary.uncovered_modules
            ),
        ),
        Err(error) => WorkspaceMemberReport::new(member, false, error.to_string()),
    }
}
// END_coverage_member

impl WorkspaceMemberReport {
    // START_CONTRACT_WorkspaceMemberReport::new
    // PURPOSE: Create a member report with stable path and summary fields.
    // INPUTS: { member: &WorkspaceMember }, { passed: bool }, { summary: String }
    // OUTPUTS: { WorkspaceMemberReport }
    // LINKS:
    //   -> Phase-91 (implements) - workspace report aggregation
    // START_workspace_member_report_new
    fn new(member: &WorkspaceMember, passed: bool, summary: String) -> Self {
        Self {
            member: member.spec.clone(),
            path: member.path.display().to_string(),
            passed,
            summary,
        }
    }
    // END_workspace_member_report_new
}

fn default_true() -> bool {
    true
}

fn default_profile() -> String {
    "strict".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_init_and_load_resolve_members() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("proj-a"))?;
        std::fs::create_dir_all(temp.path().join("proj-b"))?;
        Workspace::init(
            temp.path(),
            vec!["proj-a".to_string(), "proj-b".to_string()],
            Some("demo".to_string()),
        )?;
        let workspace = Workspace::load(temp.path())?;
        assert_eq!(workspace.members().len(), 2);
        assert_eq!(workspace.defaults().profile, "strict");
        Ok(())
    }

    #[test]
    fn workspace_excludes_members_by_exact_spec() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("active"))?;
        std::fs::create_dir_all(temp.path().join("old"))?;
        std::fs::write(
            temp.path().join(WORKSPACE_FILE),
            "[workspace]\nmembers=[\"active\",\"old\"]\nexclude=[\"old\"]\n",
        )?;
        let workspace = Workspace::load(temp.path())?;
        assert_eq!(workspace.members().len(), 1);
        assert_eq!(workspace.members()[0].spec, "active");
        Ok(())
    }

    #[tokio::test]
    async fn workspace_verify_reports_missing_member_docs() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("member"))?;
        Workspace::init(temp.path(), vec!["member".to_string()], None)?;
        let workspace = Workspace::load(temp.path())?;
        let report = workspace
            .verify_all(WorkspaceRunOptions {
                profile: GraceProfile::Strict,
                parallel: false,
                fail_fast: true,
            })
            .await?;
        assert_eq!(report.total, 1);
        assert_eq!(report.failed, 1);
        Ok(())
    }
}
