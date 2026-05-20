// MODULE_CONTRACT
// MODULE_ID: M-GRACE-LAYOUT
// PURPOSE: Sharded GRACE artifact layout — resolves index-based docs paths, belief state storage, and bootstraps templates
// SCOPE: DocsLayout struct, path helpers, initialization of graph/plan/verification/belief-state indexes and shard dirs
// DEPENDS: N/A
// LINKS: docs/graph-index.xml, docs/plan-index.xml, docs/verification-index.xml

// START_MODULE_MAP
// DocsLayout — Path resolver for sharded artifact and belief-state model
// ensure_initialized — Create sharded docs skeleton and compatibility docs
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.13.0 — Added docs/belief-states layout support]
// END_CHANGE_SUMMARY

use std::path::{Path, PathBuf};

// START_public_api

// START_DocsLayout
#[derive(Debug, Clone)]
pub struct DocsLayout {
    root: PathBuf,
}
// END_DocsLayout

impl DocsLayout {
    // START_CONTRACT_DocsLayout::new
    // PURPOSE: Create a docs layout resolver for a project root
    // INPUTS: { root: &Path — project root path }
    // OUTPUTS: { DocsLayout }
    // START_docs_layout_new
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }
    // END_docs_layout_new

    // START_CONTRACT_DocsLayout::docs_dir
    // PURPOSE: Return docs directory path
    // OUTPUTS: { PathBuf — docs directory }
    // START_docs_layout_docs_dir
    pub fn docs_dir(&self) -> PathBuf {
        self.root.join("docs")
    }
    // END_docs_layout_docs_dir

    // START_CONTRACT_DocsLayout::modules_dir
    // PURPOSE: Return sharded modules directory path
    // OUTPUTS: { PathBuf — docs/modules directory }
    // START_docs_layout_modules_dir
    pub fn modules_dir(&self) -> PathBuf {
        self.docs_dir().join("modules")
    }
    // END_docs_layout_modules_dir

    // START_CONTRACT_DocsLayout::phases_dir
    // PURPOSE: Return sharded phases directory path
    // OUTPUTS: { PathBuf — docs/phases directory }
    // START_docs_layout_phases_dir
    pub fn phases_dir(&self) -> PathBuf {
        self.docs_dir().join("phases")
    }
    // END_docs_layout_phases_dir

    // START_CONTRACT_DocsLayout::verification_dir
    // PURPOSE: Return sharded verification directory path
    // OUTPUTS: { PathBuf — docs/verification directory }
    // START_docs_layout_verification_dir
    pub fn verification_dir(&self) -> PathBuf {
        self.docs_dir().join("verification")
    }
    // END_docs_layout_verification_dir

    // START_CONTRACT_DocsLayout::belief_states_dir
    // PURPOSE: Return sharded belief states directory path
    // OUTPUTS: { PathBuf — docs/belief-states directory }
    // START_docs_layout_belief_states_dir
    pub fn belief_states_dir(&self) -> PathBuf {
        self.docs_dir().join("belief-states")
    }
    // END_docs_layout_belief_states_dir

    // START_CONTRACT_DocsLayout::graph_index_path
    // PURPOSE: Return graph index path
    // OUTPUTS: { PathBuf — docs/graph-index.xml path }
    // START_docs_layout_graph_index
    pub fn graph_index_path(&self) -> PathBuf {
        self.docs_dir().join("graph-index.xml")
    }
    // END_docs_layout_graph_index

    // START_CONTRACT_DocsLayout::plan_index_path
    // PURPOSE: Return plan index path
    // OUTPUTS: { PathBuf — docs/plan-index.xml path }
    // START_docs_layout_plan_index
    pub fn plan_index_path(&self) -> PathBuf {
        self.docs_dir().join("plan-index.xml")
    }
    // END_docs_layout_plan_index

    // START_CONTRACT_DocsLayout::verification_index_path
    // PURPOSE: Return verification index path
    // OUTPUTS: { PathBuf — docs/verification-index.xml path }
    // START_docs_layout_verification_index
    pub fn verification_index_path(&self) -> PathBuf {
        self.docs_dir().join("verification-index.xml")
    }
    // END_docs_layout_verification_index

    // START_CONTRACT_DocsLayout::ensure_initialized
    // PURPOSE: Create sharded docs directories, indexes, starter shards, and legacy compatibility docs if missing
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: creates docs files and directories on disk
    // START_docs_layout_ensure_initialized
    pub fn ensure_initialized(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.docs_dir())?;
        std::fs::create_dir_all(self.modules_dir())?;
        std::fs::create_dir_all(self.phases_dir())?;
        std::fs::create_dir_all(self.verification_dir())?;
        std::fs::create_dir_all(self.belief_states_dir())?;

        self.write_if_missing(
            &self.graph_index_path(),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<GRAPH_INDEX>
  <META>
    <MODEL>mygrace-sharded</MODEL>
    <PRIMARY>true</PRIMARY>
  </META>
  <MODULES>
    <MODULE id="M-CORE" path="docs/modules/M-CORE.xml" status="planned" />
  </MODULES>
  <RELATIONSHIPS></RELATIONSHIPS>
</GRAPH_INDEX>
"#,
        )?;

        self.write_if_missing(
            &self.plan_index_path(),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<PLAN_INDEX>
  <META>
    <MODEL>mygrace-sharded</MODEL>
    <PRIMARY>true</PRIMARY>
    <ACTIVE_PHASE>Phase-0</ACTIVE_PHASE>
  </META>
  <PHASES>
    <PHASE id="Phase-0" path="docs/phases/Phase-0.xml" status="active" />
    <PHASE id="Phase-1" path="docs/phases/Phase-1.xml" status="planned" />
  </PHASES>
</PLAN_INDEX>
"#,
        )?;

        self.write_if_missing(
            &self.verification_index_path(),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<VERIFICATION_INDEX>
  <META>
    <MODEL>mygrace-sharded</MODEL>
    <PRIMARY>true</PRIMARY>
  </META>
  <VERIFICATIONS>
    <VERIFICATION id="V-M-CORE" module="M-CORE" path="docs/verification/V-M-CORE.xml" priority="critical" status="planned" />
  </VERIFICATIONS>
</VERIFICATION_INDEX>
"#,
        )?;

        self.write_if_missing(
            &self.modules_dir().join("M-CORE.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<MODULE id="M-CORE" type="CORE_LOGIC" status="planned">
  <NAME>Core Module</NAME>
  <PURPOSE>Core application logic</PURPOSE>
  <FILES>
    <FILE>src/core.rs</FILE>
  </FILES>
  <VERIFICATION_REF>V-M-CORE</VERIFICATION_REF>
</MODULE>
"#,
        )?;

        self.write_if_missing(
            &self.phases_dir().join("Phase-0.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<PHASE id="Phase-0" status="active">
  <NAME>Architecture</NAME>
  <GOAL>Create project requirements, stack, plan, graph, verification artifacts before code</GOAL>
  <MODULE_REFS>
    <MODULE_REF id="M-CORE" />
  </MODULE_REFS>
</PHASE>
"#,
        )?;

        self.write_if_missing(
            &self.phases_dir().join("Phase-1.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<PHASE id="Phase-1" status="planned">
  <NAME>Foundation</NAME>
  <GOAL>Implement initial core modules with contracts and verification</GOAL>
  <MODULE_REFS>
    <MODULE_REF id="M-CORE" />
  </MODULE_REFS>
</PHASE>
"#,
        )?;

        self.write_if_missing(
            &self.verification_dir().join("V-M-CORE.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<VERIFICATION id="V-M-CORE" module="M-CORE" priority="critical" status="planned">
  <UNIT_TESTS></UNIT_TESTS>
  <REQUIRED_LOG_MARKERS></REQUIRED_LOG_MARKERS>
  <TRACE_ASSERTIONS></TRACE_ASSERTIONS>
  <PHASE_GATE>Phase-1</PHASE_GATE>
</VERIFICATION>
"#,
        )?;

        self.write_if_missing(
            &self.docs_dir().join("requirements.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<REQUIREMENTS>
  <META><PROJECT>my-project</PROJECT><DESCRIPTION>Describe what this project does</DESCRIPTION><LANGUAGE>rust</LANGUAGE></META>
  <NonGoals></NonGoals>
  <Risks></Risks>
  <OpenQuestions></OpenQuestions>
</REQUIREMENTS>
"#,
        )?;
        self.write_if_missing(
            &self.docs_dir().join("technology.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<TECHNOLOGY>
  <STACK><LANGUAGE>rust</LANGUAGE><FRAMEWORK></FRAMEWORK><DATABASE></DATABASE></STACK>
  <TOOLS><TOOL purpose="build">cargo</TOOL><TOOL purpose="test">cargo test</TOOL></TOOLS>
</TECHNOLOGY>
"#,
        )?;
        self.write_if_missing(
            &self.docs_dir().join("development-plan.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<DEVELOPMENT_PLAN>
  <META><GENERATED_BY>syn init</GENERATED_BY><PRIMARY_MODEL>docs/plan-index.xml</PRIMARY_MODEL></META>
  <PHASES><PHASE ref="docs/phases/Phase-0.xml" /><PHASE ref="docs/phases/Phase-1.xml" /></PHASES>
  <MODULES><MODULE ref="docs/modules/M-CORE.xml" /></MODULES>
</DEVELOPMENT_PLAN>
"#,
        )?;
        self.write_if_missing(
            &self.docs_dir().join("verification-plan.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<VERIFICATION_PLAN>
  <META><PRIMARY_MODEL>docs/verification-index.xml</PRIMARY_MODEL></META>
  <ModuleVerification><VERIFICATION ref="docs/verification/V-M-CORE.xml" /></ModuleVerification>
</VERIFICATION_PLAN>
"#,
        )?;
        self.write_if_missing(
            &self.docs_dir().join("knowledge-graph.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<KNOWLEDGE_GRAPH>
  <META><PRIMARY_MODEL>docs/graph-index.xml</PRIMARY_MODEL></META>
  <NODES><NODE ref="docs/modules/M-CORE.xml" /></NODES>
</KNOWLEDGE_GRAPH>
"#,
        )?;

        Ok(())
    }
    // END_docs_layout_ensure_initialized

    fn write_if_missing(&self, path: &Path, content: &str) -> anyhow::Result<()> {
        if !path.exists() {
            std::fs::write(path, content)?;
        }
        Ok(())
    }
}

// END_public_api
