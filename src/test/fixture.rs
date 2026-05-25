// MODULE_CONTRACT
// MODULE_ID: M-TEST-FIXTURE
// PURPOSE: Fixture factory for isolated temporary Synapse projects.
// SCOPE: TestFixture, FixtureBuilder, project templates, custom overrides, isolated config/data homes, and cleanup.
// DEPENDS: M-CONFIG, M-GRACE-LAYOUT
// LINKS:
//   -> Phase-76 (implements) - fixture foundation
//   <- V-M-TEST-FIXTURE (verified_by) - fixture template verification

use std::path::{Component, Path, PathBuf};
use tempfile::TempDir;

// START_MODULE_MAP
// TestFixture - Temporary project handle with isolated config and data homes
// FixtureBuilder - Builder for template and custom files
// FixtureTemplate - Empty, Minimal, MultiModule, and Broken templates
// FixtureFile - Relative file path and content pair
// safe_join - Rejects absolute paths and parent traversal before writing files
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Implemented UPGRADE_3 fixture factory]
// END_CHANGE_SUMMARY

// START_public_api

// START_FixtureFile
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureFile {
    pub path: PathBuf,
    pub content: String,
}
// END_FixtureFile

impl FixtureFile {
    // START_CONTRACT_FixtureFile::new
    // PURPOSE: Create a fixture file descriptor with a relative path and UTF-8 content
    // INPUTS: { path: impl Into<PathBuf> }, { content: impl Into<String> }
    // OUTPUTS: { FixtureFile }
    // LINKS:
    //   -> NFR-002 (traces_to) - deterministic fixture materialization
    // START_fixture_file_new
    fn new(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
        }
    }
    // END_fixture_file_new
}

// START_TestFixture
#[derive(Debug)]
pub struct TestFixture {
    root: TempDir,
    config_home: TempDir,
    data_home: TempDir,
}
// END_TestFixture

impl TestFixture {
    // START_CONTRACT_TestFixture::builder
    // PURPOSE: Create a new FixtureBuilder
    // OUTPUTS: { FixtureBuilder }
    // LINKS:
    //   -> NFR-002 (traces_to) - reusable isolated verification setup
    // START_test_fixture_builder
    pub fn builder() -> FixtureBuilder {
        FixtureBuilder::new()
    }
    // END_test_fixture_builder

    // START_CONTRACT_TestFixture::root
    // PURPOSE: Return the temporary project root path
    // OUTPUTS: { &Path }
    // LINKS:
    //   -> NFR-002 (traces_to) - fixture root isolation
    // START_test_fixture_root
    pub fn root(&self) -> &Path {
        self.root.path()
    }
    // END_test_fixture_root

    // START_CONTRACT_TestFixture::config_home
    // PURPOSE: Return the isolated config home for commands run inside the fixture
    // OUTPUTS: { &Path }
    // LINKS:
    //   -> NFR-002 (traces_to) - user config isolation
    // START_test_fixture_config_home
    pub fn config_home(&self) -> &Path {
        self.config_home.path()
    }
    // END_test_fixture_config_home

    // START_CONTRACT_TestFixture::data_home
    // PURPOSE: Return the isolated data home for commands run inside the fixture
    // OUTPUTS: { &Path }
    // LINKS:
    //   -> NFR-002 (traces_to) - user data isolation
    // START_test_fixture_data_home
    pub fn data_home(&self) -> &Path {
        self.data_home.path()
    }
    // END_test_fixture_data_home

    // START_CONTRACT_TestFixture::command_env
    // PURPOSE: Return environment overrides that keep Synapse command state inside the fixture
    // OUTPUTS: { Vec<(&'static str, &Path)> }
    // LINKS:
    //   -> NFR-002 (traces_to) - isolated command execution
    // START_test_fixture_command_env
    pub fn command_env(&self) -> Vec<(&'static str, &Path)> {
        vec![
            ("XDG_CONFIG_HOME", self.config_home()),
            ("XDG_DATA_HOME", self.data_home()),
        ]
    }
    // END_test_fixture_command_env
}

// START_FixtureBuilder
#[derive(Debug, Default)]
pub struct FixtureBuilder {
    template: Option<FixtureTemplate>,
    files: Vec<FixtureFile>,
}
// END_FixtureBuilder

impl FixtureBuilder {
    // START_CONTRACT_FixtureBuilder::new
    // PURPOSE: Create an empty fixture builder
    // OUTPUTS: { FixtureBuilder }
    // LINKS:
    //   -> NFR-002 (traces_to) - reusable fixture construction
    // START_fixture_builder_new
    pub fn new() -> Self {
        Self::default()
    }
    // END_fixture_builder_new

    // START_CONTRACT_FixtureBuilder::with_template
    // PURPOSE: Select a preset project template to materialize before custom files
    // INPUTS: { template: FixtureTemplate }
    // OUTPUTS: { FixtureBuilder }
    // LINKS:
    //   -> NFR-002 (traces_to) - deterministic test projects
    // START_fixture_builder_with_template
    pub fn with_template(mut self, template: FixtureTemplate) -> Self {
        self.template = Some(template);
        self
    }
    // END_fixture_builder_with_template

    // START_CONTRACT_FixtureBuilder::with_file
    // PURPOSE: Add or override one file inside the fixture root
    // INPUTS: { path: impl Into<PathBuf> }, { content: impl Into<String> }
    // OUTPUTS: { FixtureBuilder }
    // LINKS:
    //   -> NFR-002 (traces_to) - deterministic override behavior
    // START_fixture_builder_with_file
    pub fn with_file(mut self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        self.files.push(FixtureFile::new(path, content));
        self
    }
    // END_fixture_builder_with_file

    // START_CONTRACT_FixtureBuilder::build
    // PURPOSE: Create the fixture directories and write template plus custom files to disk
    // OUTPUTS: { anyhow::Result<TestFixture> }
    // SIDE_EFFECTS: creates temporary directories and writes fixture files
    // LINKS:
    //   -> NFR-002 (traces_to) - isolated test setup
    //   -> NFR-003 (traces_to) - reusable compact setup instead of duplicated test scaffolding
    // START_fixture_builder_build
    pub fn build(self) -> anyhow::Result<TestFixture> {
        let root = tempfile::tempdir()?;
        let config_home = tempfile::tempdir()?;
        let data_home = tempfile::tempdir()?;

        let mut files = self.template.unwrap_or(FixtureTemplate::Empty).files();
        files.extend(self.files);
        write_files(root.path(), &files)?;

        Ok(TestFixture {
            root,
            config_home,
            data_home,
        })
    }
    // END_fixture_builder_build
}

// START_FixtureTemplate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureTemplate {
    Empty,
    Minimal,
    MultiModule,
    Broken,
}
// END_FixtureTemplate

impl FixtureTemplate {
    // START_CONTRACT_FixtureTemplate::files
    // PURPOSE: Return all files required for a preset fixture template
    // OUTPUTS: { Vec<FixtureFile> }
    // LINKS:
    //   -> NFR-002 (traces_to) - repeatable project structures
    // START_fixture_template_files
    pub fn files(self) -> Vec<FixtureFile> {
        match self {
            Self::Empty => empty_files(),
            Self::Minimal => minimal_files(),
            Self::MultiModule => multi_module_files(),
            Self::Broken => broken_files(),
        }
    }
    // END_fixture_template_files
}

// END_public_api

// START_CONTRACT_write_files
// PURPOSE: Write fixture files under root after validating every path is fixture-local
// INPUTS: { root: &Path }, { files: &[FixtureFile] }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: creates directories and writes files
// LINKS:
//   -> NFR-002 (traces_to) - safe fixture materialization
// START_write_files
fn write_files(root: &Path, files: &[FixtureFile]) -> anyhow::Result<()> {
    for file in files {
        let full_path = safe_join(root, &file.path)?;
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(full_path, &file.content)?;
    }
    Ok(())
}
// END_write_files

// START_CONTRACT_safe_join
// PURPOSE: Join a fixture root and relative path while rejecting absolute paths and parent traversal
// INPUTS: { root: &Path }, { relative: &Path }
// OUTPUTS: { anyhow::Result<PathBuf> }
// LINKS:
//   -> NFR-002 (traces_to) - prevent fixture writes outside tempdir
// START_safe_join
fn safe_join(root: &Path, relative: &Path) -> anyhow::Result<PathBuf> {
    if relative.is_absolute() {
        anyhow::bail!("fixture path must be relative: {}", relative.display());
    }
    for component in relative.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            anyhow::bail!("fixture path must stay inside root: {}", relative.display());
        }
    }
    Ok(root.join(relative))
}
// END_safe_join

// START_CONTRACT_empty_files
// PURPOSE: Return files for an otherwise empty Synapse project
// OUTPUTS: { Vec<FixtureFile> }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic empty fixture
// START_empty_files
fn empty_files() -> Vec<FixtureFile> {
    vec![FixtureFile::new(
        "synapsec.toml",
        r#"[project]
name = "fixture-project"

[proxy]
enabled = true
"#,
    )]
}
// END_empty_files

// START_CONTRACT_minimal_files
// PURPOSE: Return files for a one-module sharded MyGRACE project
// OUTPUTS: { Vec<FixtureFile> }
// LINKS:
//   -> NFR-002 (traces_to) - minimal verification fixture
// START_minimal_files
fn minimal_files() -> Vec<FixtureFile> {
    let mut files = empty_files();
    files.extend([
        FixtureFile::new("docs/graph-index.xml", minimal_graph_index()),
        FixtureFile::new("docs/plan-index.xml", minimal_plan_index()),
        FixtureFile::new("docs/verification-index.xml", minimal_verification_index()),
        FixtureFile::new(
            "docs/modules/M-CORE.xml",
            module_shard("M-CORE", "Core Module", "src/main.rs"),
        ),
        FixtureFile::new("docs/phases/Phase-1.xml", phase_one("M-CORE")),
        FixtureFile::new(
            "docs/verification/V-M-CORE.xml",
            verification_shard("M-CORE"),
        ),
        FixtureFile::new("src/main.rs", minimal_main_rs()),
    ]);
    files
}
// END_minimal_files

// START_CONTRACT_multi_module_files
// PURPOSE: Return files for a three-module project with graph relationships and mental-test evidence
// OUTPUTS: { Vec<FixtureFile> }
// LINKS:
//   -> NFR-002 (traces_to) - integration fixture with dependencies
// START_multi_module_files
fn multi_module_files() -> Vec<FixtureFile> {
    let mut files = empty_files();
    files.extend([
        FixtureFile::new("docs/graph-index.xml", multi_graph_index()),
        FixtureFile::new("docs/plan-index.xml", multi_plan_index()),
        FixtureFile::new("docs/verification-index.xml", multi_verification_index()),
        FixtureFile::new(
            "docs/modules/M-CORE.xml",
            module_shard("M-CORE", "Core Module", "src/main.rs"),
        ),
        FixtureFile::new(
            "docs/modules/M-AUTH.xml",
            module_shard("M-AUTH", "Authentication", "src/auth.rs"),
        ),
        FixtureFile::new(
            "docs/modules/M-STORAGE.xml",
            module_shard("M-STORAGE", "Storage", "src/storage.rs"),
        ),
        FixtureFile::new("docs/phases/Phase-1.xml", phase_one("M-CORE")),
        FixtureFile::new("docs/phases/Phase-2.xml", phase_two()),
        FixtureFile::new(
            "docs/verification/V-M-CORE.xml",
            verification_shard("M-CORE"),
        ),
        FixtureFile::new(
            "docs/verification/V-M-AUTH.xml",
            verification_shard("M-AUTH"),
        ),
        FixtureFile::new(
            "docs/verification/V-M-STORAGE.xml",
            verification_shard("M-STORAGE"),
        ),
        FixtureFile::new("docs/development-plan.xml", multi_development_plan()),
        FixtureFile::new("src/main.rs", minimal_main_rs()),
        FixtureFile::new("src/auth.rs", auth_rs()),
        FixtureFile::new("src/storage.rs", storage_rs()),
    ]);
    files
}
// END_multi_module_files

// START_CONTRACT_broken_files
// PURPOSE: Return files for a deterministic broken project fixture
// OUTPUTS: { Vec<FixtureFile> }
// LINKS:
//   -> NFR-002 (traces_to) - negative-path verification fixture
// START_broken_files
fn broken_files() -> Vec<FixtureFile> {
    let mut files = minimal_files();
    files.retain(|file| file.path != PathBuf::from("docs/verification/V-M-CORE.xml"));
    files.push(FixtureFile::new(
        "docs/graph-index.xml",
        "<GRAPH_INDEX><META><MODEL>broken</MODEL>",
    ));
    files.push(FixtureFile::new(
        "src/main.rs",
        "fn main() { println!(\"missing contract\"); }\n",
    ));
    files
}
// END_broken_files

// START_CONTRACT_minimal_graph_index
// PURPOSE: Render graph-index.xml for the Minimal fixture
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic fixture graph
// START_minimal_graph_index
fn minimal_graph_index() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<GRAPH_INDEX>
  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY></META>
  <MODULES>
    <MODULE id="M-CORE" path="docs/modules/M-CORE.xml" status="active" />
  </MODULES>
  <RELATIONSHIPS></RELATIONSHIPS>
</GRAPH_INDEX>
"#
    .into()
}
// END_minimal_graph_index

// START_CONTRACT_multi_graph_index
// PURPOSE: Render graph-index.xml for the MultiModule fixture
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic fixture graph
// START_multi_graph_index
fn multi_graph_index() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<GRAPH_INDEX>
  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY></META>
  <MODULES>
    <MODULE id="M-CORE" path="docs/modules/M-CORE.xml" status="active" />
    <MODULE id="M-AUTH" path="docs/modules/M-AUTH.xml" status="active" />
    <MODULE id="M-STORAGE" path="docs/modules/M-STORAGE.xml" status="active" />
  </MODULES>
  <RELATIONSHIPS>
    <REL source="M-AUTH" target="M-CORE" type="DEPENDS" />
    <REL source="M-AUTH" target="M-STORAGE" type="DEPENDS" />
    <REL source="M-STORAGE" target="M-CORE" type="DEPENDS" />
  </RELATIONSHIPS>
</GRAPH_INDEX>
"#
    .into()
}
// END_multi_graph_index

// START_CONTRACT_minimal_plan_index
// PURPOSE: Render plan-index.xml for the Minimal fixture
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic fixture plan
// START_minimal_plan_index
fn minimal_plan_index() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<PLAN_INDEX>
  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY><ACTIVE_PHASE>Phase-1</ACTIVE_PHASE></META>
  <PHASES>
    <PHASE id="Phase-1" path="docs/phases/Phase-1.xml" status="active" />
  </PHASES>
</PLAN_INDEX>
"#
    .into()
}
// END_minimal_plan_index

// START_CONTRACT_multi_plan_index
// PURPOSE: Render plan-index.xml for the MultiModule fixture
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic fixture plan
// START_multi_plan_index
fn multi_plan_index() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<PLAN_INDEX>
  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY><ACTIVE_PHASE>Phase-2</ACTIVE_PHASE></META>
  <PHASES>
    <PHASE id="Phase-1" path="docs/phases/Phase-1.xml" status="done" />
    <PHASE id="Phase-2" path="docs/phases/Phase-2.xml" status="active" />
  </PHASES>
</PLAN_INDEX>
"#
    .into()
}
// END_multi_plan_index

// START_CONTRACT_minimal_verification_index
// PURPOSE: Render verification-index.xml for the Minimal fixture
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic verification fixture
// START_minimal_verification_index
fn minimal_verification_index() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<VERIFICATION_INDEX>
  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY></META>
  <VERIFICATIONS>
    <VERIFICATION id="V-M-CORE" module="M-CORE" path="docs/verification/V-M-CORE.xml" priority="critical" status="active" />
  </VERIFICATIONS>
</VERIFICATION_INDEX>
"#
    .into()
}
// END_minimal_verification_index

// START_CONTRACT_multi_verification_index
// PURPOSE: Render verification-index.xml for the MultiModule fixture
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic verification fixture
// START_multi_verification_index
fn multi_verification_index() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<VERIFICATION_INDEX>
  <META><MODEL>mygrace-sharded</MODEL><PRIMARY>true</PRIMARY></META>
  <VERIFICATIONS>
    <VERIFICATION id="V-M-CORE" module="M-CORE" path="docs/verification/V-M-CORE.xml" priority="critical" status="active" />
    <VERIFICATION id="V-M-AUTH" module="M-AUTH" path="docs/verification/V-M-AUTH.xml" priority="critical" status="active" />
    <VERIFICATION id="V-M-STORAGE" module="M-STORAGE" path="docs/verification/V-M-STORAGE.xml" priority="normal" status="active" />
  </VERIFICATIONS>
</VERIFICATION_INDEX>
"#
    .into()
}
// END_multi_verification_index

// START_CONTRACT_module_shard
// PURPOSE: Render a module shard for a fixture module
// INPUTS: { id: &str }, { name: &str }, { file: &str }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic module fixture
// START_module_shard
fn module_shard(id: &str, name: &str, file: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MODULE id="{id}" type="CORE_LOGIC" status="active">
  <NAME>{name}</NAME>
  <PURPOSE>Fixture module {id}</PURPOSE>
  <SCOPE>Test-only fixture behavior for Synapse harnesses</SCOPE>
  <FILES><FILE>{file}</FILE></FILES>
  <DEPENDS></DEPENDS>
  <LINKS>docs/phases/Phase-1.xml</LINKS>
  <VERIFICATION_REF>V-{id}</VERIFICATION_REF>
</MODULE>
"#
    )
}
// END_module_shard

// START_CONTRACT_verification_shard
// PURPOSE: Render a verification shard for a fixture module
// INPUTS: { module_id: &str }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic verification fixture
// START_verification_shard
fn verification_shard(module_id: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<VERIFICATION id="V-{module_id}" module="{module_id}" priority="critical" status="active">
  <UNIT_TESTS></UNIT_TESTS>
  <REQUIRED_LOG_MARKERS></REQUIRED_LOG_MARKERS>
  <TRACE_ASSERTIONS></TRACE_ASSERTIONS>
  <PHASE_GATE>Phase-1</PHASE_GATE>
</VERIFICATION>
"#
    )
}
// END_verification_shard

// START_CONTRACT_phase_one
// PURPOSE: Render the first phase shard for a fixture
// INPUTS: { module_id: &str }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic phase fixture
// START_phase_one
fn phase_one(module_id: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<PHASE id="Phase-1" status="active">
  <NAME>Fixture Foundation</NAME>
  <GOAL>Create fixture module {module_id}</GOAL>
  <MODULE_REFS><MODULE_REF id="{module_id}" /></MODULE_REFS>
</PHASE>
"#
    )
}
// END_phase_one

// START_CONTRACT_phase_two
// PURPOSE: Render the second phase shard for the MultiModule fixture
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic phase fixture
// START_phase_two
fn phase_two() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<PHASE id="Phase-2" status="active">
  <NAME>Fixture Integration</NAME>
  <GOAL>Exercise auth and storage fixture modules</GOAL>
  <MODULE_REFS>
    <MODULE_REF id="M-AUTH" />
    <MODULE_REF id="M-STORAGE" />
  </MODULE_REFS>
</PHASE>
"#
    .into()
}
// END_phase_two

// START_CONTRACT_minimal_main_rs
// PURPOSE: Render a minimal Rust source file with module and function contracts
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic source fixture
// START_minimal_main_rs
fn minimal_main_rs() -> String {
    let anchor = |prefix: &str, name: &str| format!("// {prefix}{name}");
    let start_module_map = anchor("START_", "MODULE_MAP");
    let end_module_map = anchor("END_", "MODULE_MAP");
    let start_change_summary = anchor("START_", "CHANGE_SUMMARY");
    let end_change_summary = anchor("END_", "CHANGE_SUMMARY");
    let start_contract_do_work = anchor("START_", "CONTRACT_do_work");
    let start_do_work = anchor("START_", "do_work");
    let end_do_work = anchor("END_", "do_work");
    let start_contract_main = anchor("START_", "CONTRACT_main");
    let start_main = anchor("START_", "main");
    let end_main = anchor("END_", "main");
    r#"// MODULE_CONTRACT
// MODULE_ID: M-CORE
// PURPOSE: Core module for fixture projects.
// SCOPE: Deterministic test behavior.
// DEPENDS:
// LINKS:
//   <- V-M-CORE (verified_by) - core fixture verification
//   -> NFR-002 (traces_to) - reliable fixture behavior

__SMM__
// do_work - Returns a deterministic value
__EMM__

__SCS__
// LAST_CHANGE: [v1.0.0 - Initial fixture source]
__ECS__

__SCDW__
// PURPOSE: Return a deterministic fixture value
// OUTPUTS: { &'static str }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic fixture behavior
__SDW__
fn do_work() -> &'static str {
    "done"
}
__EDW__

__SCMAIN__
// PURPOSE: Print deterministic fixture output
// SIDE_EFFECTS: writes stdout
// LINKS:
//   -> NFR-002 (traces_to) - deterministic fixture behavior
__SMAIN__
fn main() {
    println!("{}", do_work());
}
__EMAIN__
"#
    .replace("__SMM__", &start_module_map)
    .replace("__EMM__", &end_module_map)
    .replace("__SCS__", &start_change_summary)
    .replace("__ECS__", &end_change_summary)
    .replace("__SCDW__", &start_contract_do_work)
    .replace("__SDW__", &start_do_work)
    .replace("__EDW__", &end_do_work)
    .replace("__SCMAIN__", &start_contract_main)
    .replace("__SMAIN__", &start_main)
    .replace("__EMAIN__", &end_main)
}
// END_minimal_main_rs

// START_CONTRACT_auth_rs
// PURPOSE: Render an auth fixture source file
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic source fixture
// START_auth_rs
fn auth_rs() -> String {
    let anchor = |prefix: &str, name: &str| format!("// {prefix}{name}");
    let start_module_map = anchor("START_", "MODULE_MAP");
    let end_module_map = anchor("END_", "MODULE_MAP");
    let start_change_summary = anchor("START_", "CHANGE_SUMMARY");
    let end_change_summary = anchor("END_", "CHANGE_SUMMARY");
    let start_contract_login = anchor("START_", "CONTRACT_login");
    let start_login = anchor("START_", "login");
    let end_login = anchor("END_", "login");
    r#"// MODULE_CONTRACT
// MODULE_ID: M-AUTH
// PURPOSE: Authentication fixture module.
// SCOPE: Deterministic login behavior.
// DEPENDS: M-CORE, M-STORAGE
// LINKS:
//   -> M-CORE (depends) - fixture core utilities
//   -> M-STORAGE (depends) - fixture storage
//   <- V-M-AUTH (verified_by) - auth fixture verification
//   -> NFR-002 (traces_to) - reliable fixture behavior

__SMM__
// login - Authenticates fixed fixture credentials
__EMM__

__SCS__
// LAST_CHANGE: [v1.0.0 - Initial auth fixture]
__ECS__

__SCL__
// PURPOSE: Authenticate fixed fixture credentials
// INPUTS: { username: &str }, { password: &str }
// OUTPUTS: { bool }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic fixture behavior
__SL__
pub fn login(username: &str, password: &str) -> bool {
    username == "admin" && password == "secret"
}
__EL__
"#
    .replace("__SMM__", &start_module_map)
    .replace("__EMM__", &end_module_map)
    .replace("__SCS__", &start_change_summary)
    .replace("__ECS__", &end_change_summary)
    .replace("__SCL__", &start_contract_login)
    .replace("__SL__", &start_login)
    .replace("__EL__", &end_login)
}
// END_auth_rs

// START_CONTRACT_storage_rs
// PURPOSE: Render a storage fixture source file
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic source fixture
// START_storage_rs
fn storage_rs() -> String {
    let anchor = |prefix: &str, name: &str| format!("// {prefix}{name}");
    let start_module_map = anchor("START_", "MODULE_MAP");
    let end_module_map = anchor("END_", "MODULE_MAP");
    let start_change_summary = anchor("START_", "CHANGE_SUMMARY");
    let end_change_summary = anchor("END_", "CHANGE_SUMMARY");
    let start_contract_save = anchor("START_", "CONTRACT_save");
    let start_save = anchor("START_", "save");
    let end_save = anchor("END_", "save");
    let start_contract_load = anchor("START_", "CONTRACT_load");
    let start_load = anchor("START_", "load");
    let end_load = anchor("END_", "load");
    r#"// MODULE_CONTRACT
// MODULE_ID: M-STORAGE
// PURPOSE: Storage fixture module.
// SCOPE: Deterministic in-memory-like persistence placeholders.
// DEPENDS: M-CORE
// LINKS:
//   -> M-CORE (depends) - fixture core utilities
//   <- V-M-STORAGE (verified_by) - storage fixture verification
//   -> NFR-002 (traces_to) - reliable fixture behavior

__SMM__
// save - Accepts a fixture value
// load - Returns no value
__EMM__

__SCS__
// LAST_CHANGE: [v1.0.0 - Initial storage fixture]
__ECS__

__SCSAVE__
// PURPOSE: Accept a key and value without side effects
// INPUTS: { key: &str }, { value: &str }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic fixture behavior
__SSAVE__
pub fn save(_key: &str, _value: &str) {}
__ESAVE__

__SCLOAD__
// PURPOSE: Return no stored value for deterministic fixture behavior
// INPUTS: { key: &str }
// OUTPUTS: { Option<String> }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic fixture behavior
__SLOAD__
pub fn load(_key: &str) -> Option<String> {
    None
}
__ELOAD__
"#
    .replace("__SMM__", &start_module_map)
    .replace("__EMM__", &end_module_map)
    .replace("__SCS__", &start_change_summary)
    .replace("__ECS__", &end_change_summary)
    .replace("__SCSAVE__", &start_contract_save)
    .replace("__SSAVE__", &start_save)
    .replace("__ESAVE__", &end_save)
    .replace("__SCLOAD__", &start_contract_load)
    .replace("__SLOAD__", &start_load)
    .replace("__ELOAD__", &end_load)
}
// END_storage_rs

// START_CONTRACT_multi_development_plan
// PURPOSE: Render compatibility development-plan.xml with DataFlow and MentalTest evidence
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - deterministic planning fixture
// START_multi_development_plan
fn multi_development_plan() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<DevelopmentPlan>
  <ArchitectureGraph>
    <Module id="M-CORE" critical="true" complexity="medium" />
    <Module id="M-AUTH" critical="true" complexity="complex" />
    <Module id="M-STORAGE" critical="false" complexity="low" />
  </ArchitectureGraph>
  <DataFlows>
    <DataFlow id="DF-001" from="M-AUTH" to="M-STORAGE" contract_module="M-AUTH" direction="bidirectional" protocol="fixture-call">
      <ErrorHandling>Fixture calls are deterministic and cannot fail.</ErrorHandling>
    </DataFlow>
  </DataFlows>
  <GenerationOrder>
    <Phase id="Phase-1"><Module id="M-CORE" /></Phase>
    <Phase id="Phase-2"><Module id="M-AUTH" /><Module id="M-STORAGE" /></Phase>
  </GenerationOrder>
  <MentalTests>
    <MentalTest id="MT-001" target="M-CORE::do_work" status="pass">
      <Description>Pre-code simulation for deterministic fixture work.</Description>
      <Scenario>GIVEN a fixture project. WHEN do_work is called. THEN it returns done.</Scenario>
      <Steps>
        <Step n="1" action="execute" expected="returns done" actual="PASS - returns done">Execute fixture function.</Step>
      </Steps>
      <EdgeCases>
        <Case id="EC-001" description="no input" expectation="handled deterministically" />
      </EdgeCases>
      <Result>pass</Result>
      <Rationale>Deterministic source string has no external dependencies.</Rationale>
    </MentalTest>
  </MentalTests>
</DevelopmentPlan>
"#
    .into()
}
// END_multi_development_plan

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_minimal_fixture_has_required_files
    // PURPOSE: Verify the Minimal template creates required sharded MyGRACE and source files
    // LINKS:
    //   -> V-M-TEST-FIXTURE (verified_by) - minimal fixture acceptance
    // START_test_minimal_fixture_has_required_files
    #[test]
    fn test_minimal_fixture_has_required_files() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .build()
            .expect("fixture");

        assert!(fixture.root().join("synapsec.toml").exists());
        assert!(fixture.root().join("docs/graph-index.xml").exists());
        assert!(fixture.root().join("docs/plan-index.xml").exists());
        assert!(fixture.root().join("docs/verification-index.xml").exists());
        assert!(fixture.root().join("docs/modules/M-CORE.xml").exists());
        assert!(fixture.root().join("docs/phases/Phase-1.xml").exists());
        assert!(fixture
            .root()
            .join("docs/verification/V-M-CORE.xml")
            .exists());
        assert!(fixture.root().join("src/main.rs").exists());
    }
    // END_test_minimal_fixture_has_required_files

    // START_CONTRACT_test_multi_module_fixture_has_graph_and_mental_test_evidence
    // PURPOSE: Verify the MultiModule template contains three modules plus DataFlow and MentalTest evidence
    // LINKS:
    //   -> V-M-TEST-FIXTURE (verified_by) - multimodule fixture acceptance
    // START_test_multi_module_fixture_has_graph_and_mental_test_evidence
    #[test]
    fn test_multi_module_fixture_has_graph_and_mental_test_evidence() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::MultiModule)
            .build()
            .expect("fixture");

        let graph = std::fs::read_to_string(fixture.root().join("docs/graph-index.xml")).unwrap();
        assert!(graph.contains("M-AUTH"));
        assert!(graph.contains("M-STORAGE"));
        assert!(graph.contains("REL source=\"M-AUTH\" target=\"M-STORAGE\""));

        let plan =
            std::fs::read_to_string(fixture.root().join("docs/development-plan.xml")).unwrap();
        assert!(plan.contains("<DataFlow"));
        assert!(plan.contains("<MentalTest"));
    }
    // END_test_multi_module_fixture_has_graph_and_mental_test_evidence

    // START_CONTRACT_test_broken_fixture_contains_intentional_errors
    // PURPOSE: Verify the Broken template contains deterministic contract and XML failures
    // LINKS:
    //   -> V-M-TEST-FIXTURE (verified_by) - broken fixture acceptance
    // START_test_broken_fixture_contains_intentional_errors
    #[test]
    fn test_broken_fixture_contains_intentional_errors() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Broken)
            .build()
            .expect("fixture");

        let source = std::fs::read_to_string(fixture.root().join("src/main.rs")).unwrap();
        assert!(!source.contains("MODULE_CONTRACT"));
        assert!(!fixture
            .root()
            .join("docs/verification/V-M-CORE.xml")
            .exists());

        let graph = std::fs::read_to_string(fixture.root().join("docs/graph-index.xml")).unwrap();
        assert!(graph.contains("<GRAPH_INDEX>"));
        assert!(!graph.contains("</GRAPH_INDEX>"));
    }
    // END_test_broken_fixture_contains_intentional_errors

    // START_CONTRACT_test_custom_file_overrides_template_file
    // PURPOSE: Verify custom files overwrite template files with deterministic content
    // LINKS:
    //   -> V-M-TEST-FIXTURE (verified_by) - override behavior acceptance
    // START_test_custom_file_overrides_template_file
    #[test]
    fn test_custom_file_overrides_template_file() {
        let fixture = TestFixture::builder()
            .with_template(FixtureTemplate::Minimal)
            .with_file("src/main.rs", "// custom\nfn main() {}\n")
            .build()
            .expect("fixture");

        let source = std::fs::read_to_string(fixture.root().join("src/main.rs")).unwrap();
        assert!(source.contains("// custom"));
        assert!(!source.contains("MODULE_CONTRACT"));
    }
    // END_test_custom_file_overrides_template_file

    // START_CONTRACT_test_fixture_rejects_parent_path_override
    // PURPOSE: Verify fixture writes cannot escape the temporary project root
    // LINKS:
    //   -> V-M-TEST-FIXTURE (verified_by) - fixture path safety acceptance
    // START_test_fixture_rejects_parent_path_override
    #[test]
    fn test_fixture_rejects_parent_path_override() {
        let result = TestFixture::builder()
            .with_file("../escape.txt", "bad")
            .build();
        assert!(result.is_err());
    }
    // END_test_fixture_rejects_parent_path_override

    // START_CONTRACT_test_fixture_exposes_isolated_command_env
    // PURPOSE: Verify fixture command environment points to isolated config and data homes
    // LINKS:
    //   -> V-M-TEST-FIXTURE (verified_by) - isolated env acceptance
    // START_test_fixture_exposes_isolated_command_env
    #[test]
    fn test_fixture_exposes_isolated_command_env() {
        let fixture = TestFixture::builder().build().expect("fixture");
        let env = fixture.command_env();
        assert_eq!(env.len(), 2);
        assert_ne!(fixture.root(), fixture.config_home());
        assert_ne!(fixture.root(), fixture.data_home());
    }
    // END_test_fixture_exposes_isolated_command_env
}
