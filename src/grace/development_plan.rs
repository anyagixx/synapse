// MODULE_CONTRACT
// MODULE_ID: M-GRACE-DEVELOPMENT-PLAN
// PURPOSE: DevelopmentPlan parser, validator, and generator for GRACE Stage 3 DataFlows and GenerationOrder artifacts
// SCOPE: DevelopmentPlanReport, DataFlow, GenerationModule, template generation, contract-derived file generation, file validation, generation order topology, contract coverage
// DEPENDS: M-GRACE-CONTRACT
// LINKS:
//   → V-M-GRACE-DEVELOPMENT-PLAN (verified_by) — DevelopmentPlan parser, validator, and generator tests

// START_MODULE_MAP
// DataFlow — One data movement contract between actors/modules
// GenerationModule — One module entry in GenerationOrder
// DevelopmentPlanReport — Completeness report for docs/development-plan.xml
// development_plan_template — Full DevelopmentPlan XML template
// validate_development_plan — Parse and validate docs/development-plan.xml
// generate_development_plan_file — Generate and validate docs/development-plan.xml
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 — Generate DevelopmentPlan from real project contracts]
// END_CHANGE_SUMMARY

use crate::grace::contract::{ContractValidator, GraceProfile};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// START_public_api

// START_DataFlow
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct DataFlow {
    pub id: String,
    pub from: String,
    pub to: String,
    pub data: String,
    pub protocol: String,
    pub contract_module: String,
    pub contract_function: String,
    pub error_handling: String,
}
// END_DataFlow

// START_GenerationModule
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct GenerationModule {
    pub phase: String,
    pub id: String,
    pub order: usize,
    pub depends_on: Vec<String>,
}
// END_GenerationModule

// START_DevelopmentPlanReport
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct DevelopmentPlanReport {
    pub path: String,
    pub exists: bool,
    pub valid: bool,
    pub architecture_modules: Vec<String>,
    pub data_flows: Vec<DataFlow>,
    pub generation_modules: Vec<GenerationModule>,
    pub non_human_patterns: usize,
    pub contract_guidelines: usize,
    pub missing_dataflow_sources: Vec<String>,
    pub missing_dataflow_contracts: Vec<String>,
    pub protocol_issues: Vec<String>,
    pub error_handling_issues: Vec<String>,
    pub generation_order_issues: Vec<String>,
    pub completed_generation_modules: usize,
    pub errors: Vec<String>,
}
// END_DevelopmentPlanReport

impl DevelopmentPlanReport {
    // START_CONTRACT_DevelopmentPlanReport::dataflow_sources_exist
    // PURPOSE: Return true when every DataFlow endpoint resolves to a known module or accepted external actor
    // OUTPUTS: { bool }
    // START_development_plan_report_dataflow_sources_exist
    pub fn dataflow_sources_exist(&self) -> bool {
        self.missing_dataflow_sources.is_empty() && !self.data_flows.is_empty()
    }
    // END_development_plan_report_dataflow_sources_exist

    // START_CONTRACT_DevelopmentPlanReport::dataflow_contracts_exist
    // PURPOSE: Return true when every DataFlow has a matching function contract
    // OUTPUTS: { bool }
    // START_development_plan_report_dataflow_contracts_exist
    pub fn dataflow_contracts_exist(&self) -> bool {
        self.missing_dataflow_contracts.is_empty() && !self.data_flows.is_empty()
    }
    // END_development_plan_report_dataflow_contracts_exist

    // START_CONTRACT_DevelopmentPlanReport::dataflow_protocol_consistent
    // PURPOSE: Return true when every DataFlow declares a protocol
    // OUTPUTS: { bool }
    // START_development_plan_report_dataflow_protocol_consistent
    pub fn dataflow_protocol_consistent(&self) -> bool {
        self.protocol_issues.is_empty() && !self.data_flows.is_empty()
    }
    // END_development_plan_report_dataflow_protocol_consistent

    // START_CONTRACT_DevelopmentPlanReport::dataflow_errors_documented
    // PURPOSE: Return true when every DataFlow declares error handling
    // OUTPUTS: { bool }
    // START_development_plan_report_dataflow_errors_documented
    pub fn dataflow_errors_documented(&self) -> bool {
        self.error_handling_issues.is_empty() && !self.data_flows.is_empty()
    }
    // END_development_plan_report_dataflow_errors_documented

    // START_CONTRACT_DevelopmentPlanReport::genorder_topology_correct
    // PURPOSE: Return true when GenerationOrder has no dangling, duplicate, or out-of-order dependency issues
    // OUTPUTS: { bool }
    // START_development_plan_report_genorder_topology_correct
    pub fn genorder_topology_correct(&self) -> bool {
        self.generation_order_issues.is_empty() && !self.generation_modules.is_empty()
    }
    // END_development_plan_report_genorder_topology_correct

    // START_CONTRACT_DevelopmentPlanReport::genorder_all_modules
    // PURPOSE: Return true when every ArchitectureGraph module appears exactly once in GenerationOrder
    // OUTPUTS: { bool }
    // START_development_plan_report_genorder_all_modules
    pub fn genorder_all_modules(&self) -> bool {
        if self.architecture_modules.is_empty() || self.generation_modules.is_empty() {
            return false;
        }
        let planned: BTreeSet<_> = self.architecture_modules.iter().cloned().collect();
        let generated: Vec<_> = self
            .generation_modules
            .iter()
            .map(|module| module.id.clone())
            .collect();
        planned.len() == generated.len() && generated.iter().all(|id| planned.contains(id))
    }
    // END_development_plan_report_genorder_all_modules

    // START_CONTRACT_DevelopmentPlanReport::genorder_no_dangling_deps
    // PURPOSE: Return true when all DependsOn refs point to known earlier GenerationOrder modules
    // OUTPUTS: { bool }
    // START_development_plan_report_genorder_no_dangling_deps
    pub fn genorder_no_dangling_deps(&self) -> bool {
        self.generation_order_issues
            .iter()
            .all(|issue| !issue.contains("dangling"))
    }
    // END_development_plan_report_genorder_no_dangling_deps
}

// START_CONTRACT_development_plan_template
// PURPOSE: Build a complete DevelopmentPlan XML template with DataFlows and GenerationOrder
// INPUTS: { project_name: &str }, { modules: &[String] }
// OUTPUTS: { String }
// START_development_plan_template
pub fn development_plan_template(project_name: &str, modules: &[String]) -> String {
    let modules = selected_modules(modules);
    let project_name = xml_text(&fallback(project_name, "MyProject"));
    let architecture = architecture_xml(&modules);
    let flows = dataflows_xml(&modules);
    let generation_order = generation_order_xml(&modules);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<DevelopmentPlan project="{project_name}" version="1.0">
  <ArchitectureGraph>
{architecture}
  </ArchitectureGraph>
  <DataFlows>
{flows}
  </DataFlows>
  <GenerationOrder>
{generation_order}
  </GenerationOrder>
  <NonHumanPatterns>
    <Pattern name="ExplicitTyping" severity="error">
      <Rule>All type conversions must use explicit cast/parse syntax.</Rule>
      <Languages>rust, typescript, go, java</Languages>
    </Pattern>
    <Pattern name="ExplicitReturns" severity="error">
      <Rule>Business logic returns Result-like values instead of hidden exception paths.</Rule>
      <Languages>rust, typescript, go</Languages>
    </Pattern>
    <Pattern name="DeterministicIteration" severity="warning">
      <Rule>Hash-based collection iteration must be sorted before user-visible output.</Rule>
    </Pattern>
  </NonHumanPatterns>
  <ContractGuidelines>
    <Guideline name="ErrorCoverage">
      <Rule>Every DataFlow error path must be documented in the target function contract.</Rule>
    </Guideline>
    <Guideline name="LogCoverage">
      <Rule>Every DataFlow transition point should have a structured LOG expectation.</Rule>
    </Guideline>
  </ContractGuidelines>
</DevelopmentPlan>
"#
    )
}
// END_development_plan_template

// START_CONTRACT_validate_development_plan
// PURPOSE: Validate docs/development-plan.xml as a complete DevelopmentPlan artifact
// INPUTS: { root: &Path — project root }
// OUTPUTS: { anyhow::Result<DevelopmentPlanReport> }
// START_validate_development_plan
pub fn validate_development_plan(root: &Path) -> anyhow::Result<DevelopmentPlanReport> {
    let path = root.join("docs").join("development-plan.xml");
    if !path.exists() {
        return Ok(DevelopmentPlanReport {
            path: path.display().to_string(),
            exists: false,
            valid: false,
            errors: vec!["Missing docs/development-plan.xml".into()],
            ..DevelopmentPlanReport::default()
        });
    }
    let content = std::fs::read_to_string(&path)?;
    let mut report = parse_development_plan_content(&content);
    report.path = path.display().to_string();
    report.exists = true;
    validate_against_project(root, &mut report)?;
    Ok(report)
}
// END_validate_development_plan

// START_CONTRACT_parse_development_plan_content
// PURPOSE: Parse DevelopmentPlan XML content into a completeness report
// INPUTS: { content: &str }
// OUTPUTS: { DevelopmentPlanReport }
// START_parse_development_plan_content
pub fn parse_development_plan_content(content: &str) -> DevelopmentPlanReport {
    let architecture = section_content(content, "ArchitectureGraph");
    let dataflows = section_content(content, "DataFlows");
    let generation = section_content(content, "GenerationOrder");
    let mut report = DevelopmentPlanReport {
        exists: true,
        architecture_modules: extract_architecture_modules(&architecture),
        data_flows: extract_dataflows(&dataflows),
        generation_modules: extract_generation_modules(&generation),
        non_human_patterns: tag_count(&section_content(content, "NonHumanPatterns"), "Pattern"),
        contract_guidelines: tag_count(
            &section_content(content, "ContractGuidelines"),
            "Guideline",
        ),
        ..DevelopmentPlanReport::default()
    };
    validate_static_shape(content, &mut report);
    report.valid = report.errors.is_empty();
    report
}
// END_parse_development_plan_content

// START_CONTRACT_generate_development_plan_file
// PURPOSE: Generate docs/development-plan.xml from requirements, technology, and graph modules
// INPUTS: { root: &Path }, { from_requirements: bool }, { data_flow_analysis: &str }, { generation_order: &str }
// OUTPUTS: { anyhow::Result<DevelopmentPlanReport> }
// SIDE_EFFECTS: writes docs/development-plan.xml
// START_generate_development_plan_file
pub fn generate_development_plan_file(
    root: &Path,
    _from_requirements: bool,
    _data_flow_analysis: &str,
    _generation_order: &str,
) -> anyhow::Result<DevelopmentPlanReport> {
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs)?;
    let project_name = project_name(root);
    let content = contract_based_development_plan(root, &project_name)?
        .unwrap_or_else(|| development_plan_template(&project_name, &graph_modules(root)));
    std::fs::write(docs.join("development-plan.xml"), content)?;
    validate_development_plan(root)
}
// END_generate_development_plan_file

// END_public_api

fn validate_static_shape(content: &str, report: &mut DevelopmentPlanReport) {
    if !content.contains("<DevelopmentPlan") {
        report
            .errors
            .push("Root tag must be DevelopmentPlan".into());
    }
    if report.architecture_modules.is_empty() {
        report
            .errors
            .push("ArchitectureGraph must define at least one Module".into());
    }
    if report.data_flows.is_empty() {
        report
            .errors
            .push("DataFlows must define at least one DataFlow".into());
    }
    if report.generation_modules.is_empty() {
        report
            .errors
            .push("GenerationOrder must define at least one Module".into());
    }
    if report.non_human_patterns == 0 {
        report
            .errors
            .push("NonHumanPatterns must define at least one Pattern".into());
    }
    if report.contract_guidelines == 0 {
        report
            .errors
            .push("ContractGuidelines must define at least one Guideline".into());
    }
    validate_generation_order(report);
}

fn validate_against_project(root: &Path, report: &mut DevelopmentPlanReport) -> anyhow::Result<()> {
    let graph = graph_modules(root);
    let known_modules: BTreeSet<_> = graph
        .iter()
        .chain(report.architecture_modules.iter())
        .cloned()
        .collect();
    for flow in &report.data_flows {
        for endpoint in [&flow.from, &flow.to] {
            if endpoint.starts_with("M-") && !known_modules.contains(endpoint) {
                report.missing_dataflow_sources.push(format!(
                    "{} references missing endpoint {}",
                    flow.id, endpoint
                ));
            }
        }
        if flow.protocol.trim().is_empty() {
            report
                .protocol_issues
                .push(format!("{} has no Protocol", flow.id));
        }
        if flow.error_handling.trim().is_empty() {
            report
                .error_handling_issues
                .push(format!("{} has no ErrorHandling", flow.id));
        }
    }
    validate_contract_refs(root, report)?;
    push_project_errors(report);
    Ok(())
}

fn validate_contract_refs(root: &Path, report: &mut DevelopmentPlanReport) -> anyhow::Result<()> {
    let contracts = ContractValidator::validate_project(root)?;
    let mut functions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for contract in contracts.contracts {
        let Some(module_id) = contract.module_id else {
            continue;
        };
        let entry = functions.entry(module_id).or_default();
        for function in contract.function_contracts {
            entry.insert(function.name);
        }
    }
    report.completed_generation_modules = report
        .generation_modules
        .iter()
        .filter(|module| functions.contains_key(&module.id))
        .count();
    for flow in &report.data_flows {
        let exists = functions
            .get(&flow.contract_module)
            .is_some_and(|names| names.contains(&flow.contract_function));
        if !exists {
            report.missing_dataflow_contracts.push(format!(
                "{} contract {}::{} not found",
                flow.id, flow.contract_module, flow.contract_function
            ));
        }
    }
    Ok(())
}

fn push_project_errors(report: &mut DevelopmentPlanReport) {
    if !report.missing_dataflow_sources.is_empty() {
        report.errors.push(format!(
            "DataFlow source/target issues: {:?}",
            report.missing_dataflow_sources
        ));
    }
    if !report.missing_dataflow_contracts.is_empty() {
        report.errors.push(format!(
            "DataFlow contract issues: {:?}",
            report.missing_dataflow_contracts
        ));
    }
    if !report.protocol_issues.is_empty() {
        report.errors.push(format!(
            "DataFlow protocol issues: {:?}",
            report.protocol_issues
        ));
    }
    if !report.error_handling_issues.is_empty() {
        report.errors.push(format!(
            "DataFlow error handling issues: {:?}",
            report.error_handling_issues
        ));
    }
    if !report.genorder_all_modules() {
        report.errors.push(
            "Every ArchitectureGraph module must appear exactly once in GenerationOrder".into(),
        );
    }
    if !report.generation_order_issues.is_empty() {
        report.errors.push(format!(
            "GenerationOrder issues: {:?}",
            report.generation_order_issues
        ));
    }
    report.valid = report.errors.is_empty();
}

fn validate_generation_order(report: &mut DevelopmentPlanReport) {
    let mut seen = BTreeSet::new();
    let mut order_index = BTreeMap::new();
    for (idx, module) in report.generation_modules.iter().enumerate() {
        if !seen.insert(module.id.clone()) {
            report
                .generation_order_issues
                .push(format!("duplicate module {}", module.id));
        }
        order_index.insert(module.id.clone(), idx);
    }
    for (idx, module) in report.generation_modules.iter().enumerate() {
        for dep in &module.depends_on {
            match order_index.get(dep) {
                Some(dep_idx) if *dep_idx < idx => {}
                Some(_) => report.generation_order_issues.push(format!(
                    "{} depends on {} but appears before or with it",
                    module.id, dep
                )),
                None => report
                    .generation_order_issues
                    .push(format!("{} has dangling dependency {}", module.id, dep)),
            }
        }
    }
}

fn extract_architecture_modules(content: &str) -> Vec<String> {
    start_tags(content, "Module")
        .into_iter()
        .filter_map(|tag| attr_value(&tag, "id"))
        .collect()
}

fn extract_dataflows(content: &str) -> Vec<DataFlow> {
    let Ok(re) = regex::Regex::new(
        r#"(?s)<DataFlow\s+id="([^"]+)"\s+from="([^"]+)"\s+to="([^"]+)"[^>]*>(.*?)</DataFlow>"#,
    ) else {
        return Vec::new();
    };
    re.captures_iter(content)
        .map(|cap| {
            let body = &cap[4];
            let contract = start_tags(&section_content(body, "Contracts"), "Contract")
                .into_iter()
                .next()
                .unwrap_or_default();
            DataFlow {
                id: cap[1].to_string(),
                from: cap[2].to_string(),
                to: cap[3].to_string(),
                data: tag_text(body, "Data"),
                protocol: tag_text(body, "Protocol"),
                contract_module: attr_value(&contract, "module").unwrap_or_default(),
                contract_function: attr_value(&contract, "function").unwrap_or_default(),
                error_handling: tag_text(body, "ErrorHandling"),
            }
        })
        .collect()
}

fn extract_generation_modules(content: &str) -> Vec<GenerationModule> {
    let Ok(phase_re) = regex::Regex::new(r#"(?s)<Phase\s+id="([^"]+)"[^>]*>(.*?)</Phase>"#) else {
        return Vec::new();
    };
    let Ok(module_re) =
        regex::Regex::new(r#"(?s)<Module\s+id="([^"]+)"\s+order="([^"]+)"[^>]*>(.*?)</Module>"#)
    else {
        return Vec::new();
    };
    let mut modules = Vec::new();
    for phase in phase_re.captures_iter(content) {
        for module in module_re.captures_iter(&phase[2]) {
            modules.push(GenerationModule {
                phase: phase[1].to_string(),
                id: module[1].to_string(),
                order: module[2].parse().unwrap_or(0),
                depends_on: split_depends(&tag_text(&module[3], "DependsOn")),
            });
        }
    }
    modules.sort_by(|a, b| (&a.phase, a.order, &a.id).cmp(&(&b.phase, b.order, &b.id)));
    modules
}

fn selected_modules(modules: &[String]) -> Vec<String> {
    let preferred = [
        "M-GRACE-REQUIREMENTS",
        "M-GRACE-TECHNOLOGY",
        "M-GRACE-DEVELOPMENT-PLAN",
        "M-MCP-SERVER-GRACE-TOOLS",
        "M-GRACE-VERIFY",
        "M-GRACE-REVIEW",
        "M-GRACE-STATUS",
        "M-SKILLS-ENGINE",
    ];
    let available: BTreeSet<_> = modules.iter().cloned().collect();
    let mut selected: Vec<String> = preferred
        .iter()
        .filter(|id| available.is_empty() || available.contains(**id))
        .map(|id| (*id).to_string())
        .collect();
    if selected.is_empty() {
        selected.push("M-CORE".into());
    }
    selected
}

fn architecture_xml(modules: &[String]) -> String {
    modules
        .iter()
        .map(|module| {
            format!(
                "    <Module id=\"{}\" type=\"core\">\n      <Responsibility>{} implementation boundary</Responsibility>\n      <Dependencies></Dependencies>\n      <LINKS><Link ref=\"docs/modules/{}.xml\" type=\"traces_to\" /></LINKS>\n    </Module>",
                xml_text(module),
                xml_text(module),
                xml_text(module)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn dataflows_xml(modules: &[String]) -> String {
    let flow_targets = [
        (
            "DF-001",
            "Developer",
            "M-MCP-SERVER-GRACE-TOOLS",
            "handle_generate_requirements",
        ),
        (
            "DF-002",
            "Developer",
            "M-MCP-SERVER-GRACE-TOOLS",
            "handle_generate_technology",
        ),
        (
            "DF-003",
            "M-MCP-SERVER-GRACE-TOOLS",
            "M-GRACE-VERIFY",
            "Verifier::verify_module_local",
        ),
    ];
    let available: BTreeSet<_> = modules.iter().cloned().collect();
    flow_targets
        .iter()
        .filter(|(_, _, to, _)| available.contains(*to))
        .map(|(id, from, to, function)| {
            format!(
                "    <DataFlow id=\"{}\" from=\"{}\" to=\"{}\">\n      <Data>GRACE artifact request and validation result</Data>\n      <Protocol>MCP JSON-RPC or in-process Rust call</Protocol>\n      <Transformation>Validate request, call target contract, return XML/JSON report</Transformation>\n      <Contracts><Contract module=\"{}\" function=\"{}\" /></Contracts>\n      <ErrorHandling>Return structured error details and keep source artifacts unchanged on failure</ErrorHandling>\n    </DataFlow>",
                id, from, to, to, function
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn contract_based_development_plan(
    root: &Path,
    project_name: &str,
) -> anyhow::Result<Option<String>> {
    let contracts = ContractValidator::validate_project_with_profile(root, GraceProfile::Lite)?;
    let mut module_functions: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for contract in contracts.contracts {
        let Some(module_id) = contract.module_id else {
            continue;
        };
        let functions: Vec<String> = contract
            .function_contracts
            .into_iter()
            .map(|function| function.name)
            .collect();
        module_functions
            .entry(module_id)
            .or_default()
            .extend(functions);
    }
    if module_functions.is_empty() {
        return Ok(None);
    }

    let modules: Vec<String> = module_functions.keys().cloned().collect();
    let project_name = xml_text(&fallback(project_name, "MyProject"));
    let architecture = architecture_xml(&modules);
    let flows = contract_dataflows_xml(&module_functions);
    if flows.trim().is_empty() {
        return Ok(None);
    }
    let generation_order = generation_order_xml(&modules);
    Ok(Some(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<DevelopmentPlan project="{project_name}" version="1.0">
  <ArchitectureGraph>
{architecture}
  </ArchitectureGraph>
  <DataFlows>
{flows}
  </DataFlows>
  <GenerationOrder>
{generation_order}
  </GenerationOrder>
  <NonHumanPatterns>
    <Pattern name="ExplicitTyping" severity="error">
      <Rule>All public contracts must declare explicit inputs and outputs.</Rule>
    </Pattern>
    <Pattern name="DeterministicExecution" severity="warning">
      <Rule>Generation order must be stable across repeated refreshes.</Rule>
    </Pattern>
  </NonHumanPatterns>
  <ContractGuidelines>
    <Guideline name="DataFlowContracts">
      <Rule>Every DataFlow must reference an existing target function contract.</Rule>
    </Guideline>
    <Guideline name="ErrorCoverage">
      <Rule>Every DataFlow must document error handling behavior.</Rule>
    </Guideline>
  </ContractGuidelines>
</DevelopmentPlan>
"#
    )))
}

fn contract_dataflows_xml(module_functions: &BTreeMap<String, Vec<String>>) -> String {
    module_functions
        .iter()
        .filter_map(|(module, functions)| {
            functions.first().map(|function| {
                let order = module_functions
                    .keys()
                    .position(|id| id == module)
                    .unwrap_or(0)
                    + 1;
                format!(
                    "    <DataFlow id=\"DF-{order:03}\" from=\"Developer\" to=\"{}\">\n      <Data>Implementation request and verification feedback for {}</Data>\n      <Protocol>Contract-guided source edit plus MyGRACE verification</Protocol>\n      <Transformation>Read contract, implement bounded code, run module verification</Transformation>\n      <Contracts><Contract module=\"{}\" function=\"{}\" /></Contracts>\n      <ErrorHandling>Return verification errors without mutating unrelated modules</ErrorHandling>\n    </DataFlow>",
                    xml_text(module),
                    xml_text(module),
                    xml_text(module),
                    xml_text(function)
                )
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn generation_order_xml(modules: &[String]) -> String {
    modules
        .iter()
        .enumerate()
        .map(|(idx, module)| {
            let deps = if idx == 0 {
                String::new()
            } else {
                modules[..idx.min(2)].join(", ")
            };
            format!(
                "        <Module id=\"{}\" order=\"{}\">\n          <Rationale>Generate after required artifact primitives are available</Rationale>\n          <DependsOn>{}</DependsOn>\n        </Module>",
                xml_text(module),
                idx + 1,
                xml_text(&deps)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        .pipe(|modules_xml| {
            format!(
                "    <Phase id=\"Phase-1\" name=\"Artifact Foundation\">\n      <Description>Generate artifact parsers before dependent MCP, verification, review, and status flows</Description>\n      <Generates>\n{}\n      </Generates>\n    </Phase>",
                modules_xml
            )
        })
}

fn graph_modules(root: &Path) -> Vec<String> {
    let content =
        std::fs::read_to_string(root.join("docs").join("graph-index.xml")).unwrap_or_default();
    start_tags(&content, "MODULE")
        .into_iter()
        .filter_map(|tag| attr_value(&tag, "id"))
        .collect()
}

fn project_name(root: &Path) -> String {
    let requirements =
        std::fs::read_to_string(root.join("docs").join("requirements.xml")).unwrap_or_default();
    start_tags(&requirements, "RequirementsAnalysis")
        .into_iter()
        .next()
        .and_then(|tag| attr_value(&tag, "project"))
        .unwrap_or_else(|| "GeneratedProject".into())
}

fn split_depends(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn section_content(content: &str, tag: &str) -> String {
    let pattern = format!(
        r"(?s)<{}[^>]*>(.*?)</{}>",
        regex::escape(tag),
        regex::escape(tag)
    );
    regex::Regex::new(&pattern)
        .ok()
        .and_then(|re| re.captures(content))
        .and_then(|cap| cap.get(1))
        .map(|value| value.as_str().to_string())
        .unwrap_or_default()
}

fn tag_text(content: &str, tag: &str) -> String {
    let pattern = format!(
        r"(?s)<{}[^>]*>(.*?)</{}>",
        regex::escape(tag),
        regex::escape(tag)
    );
    regex::Regex::new(&pattern)
        .ok()
        .and_then(|re| re.captures(content))
        .and_then(|cap| cap.get(1))
        .map(|value| value.as_str().trim().to_string())
        .unwrap_or_default()
}

fn tag_count(content: &str, tag: &str) -> usize {
    let pattern = format!(r#"<{}\b"#, regex::escape(tag));
    regex::Regex::new(&pattern)
        .map(|re| re.find_iter(content).count())
        .unwrap_or(0)
}

fn start_tags(content: &str, tag: &str) -> Vec<String> {
    let pattern = format!(r#"<{}\b[^>]*>"#, regex::escape(tag));
    regex::Regex::new(&pattern)
        .map(|re| {
            re.find_iter(content)
                .map(|m| m.as_str().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn attr_value(start_tag: &str, attr: &str) -> Option<String> {
    let pattern = format!(r#"{}\s*=\s*"([^"]*)""#, regex::escape(attr));
    regex::Regex::new(&pattern)
        .ok()
        .and_then(|re| re.captures(start_tag).map(|cap| cap[1].to_string()))
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}

fn fallback(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.trim().to_string()
    }
}

fn xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_development_plan_template_is_complete
    // PURPOSE: Verify generated DevelopmentPlan includes all required Stage 3 sections
    // START_test_development_plan_template_is_complete
    #[test]
    fn test_development_plan_template_is_complete() {
        let modules = vec!["M-MCP-SERVER-GRACE-TOOLS".into(), "M-GRACE-VERIFY".into()];
        let xml = development_plan_template("Synapse", &modules);
        let report = parse_development_plan_content(&xml);
        assert!(report.valid, "{:?}", report.errors);
        assert!(report.genorder_all_modules());
        assert!(report.genorder_topology_correct());
    }
    // END_test_development_plan_template_is_complete

    // START_CONTRACT_test_incomplete_development_plan_reports_errors
    // PURPOSE: Verify legacy DEVELOPMENT_PLAN stubs fail completeness validation
    // START_test_incomplete_development_plan_reports_errors
    #[test]
    fn test_incomplete_development_plan_reports_errors() {
        let report = parse_development_plan_content(
            "<DEVELOPMENT_PLAN><MODULES></MODULES></DEVELOPMENT_PLAN>",
        );
        assert!(!report.valid);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("DevelopmentPlan")));
    }
    // END_test_incomplete_development_plan_reports_errors

    // START_CONTRACT_test_generation_order_detects_out_of_order_dependency
    // PURPOSE: Verify GenerationOrder rejects modules generated before dependencies
    // START_test_generation_order_detects_out_of_order_dependency
    #[test]
    fn test_generation_order_detects_out_of_order_dependency() {
        let xml = r#"<DevelopmentPlan><ArchitectureGraph><Module id="M-A" /><Module id="M-B" /></ArchitectureGraph><DataFlows><DataFlow id="DF-1" from="M-A" to="M-B"><Data>x</Data><Protocol>Rust</Protocol><Contracts><Contract module="M-B" function="run" /></Contracts><ErrorHandling>err</ErrorHandling></DataFlow></DataFlows><GenerationOrder><Phase id="Phase-1"><Generates><Module id="M-A" order="1"><DependsOn>M-B</DependsOn></Module><Module id="M-B" order="2"><DependsOn></DependsOn></Module></Generates></Phase></GenerationOrder><NonHumanPatterns><Pattern name="ExplicitTyping" /></NonHumanPatterns><ContractGuidelines><Guideline name="ErrorCoverage" /></ContractGuidelines></DevelopmentPlan>"#;
        let report = parse_development_plan_content(xml);
        assert!(!report.genorder_topology_correct());
    }
    // END_test_generation_order_detects_out_of_order_dependency

    // START_CONTRACT_test_generate_development_plan_file_writes_valid_artifact
    // PURPOSE: Verify generator writes docs/development-plan.xml and validates static shape
    // START_test_generate_development_plan_file_writes_valid_artifact
    #[test]
    fn test_generate_development_plan_file_writes_valid_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("docs")).expect("docs");
        std::fs::create_dir_all(dir.path().join("src")).expect("src");
        std::fs::write(
            dir.path().join("docs/graph-index.xml"),
            r#"<GRAPH_INDEX><MODULES><MODULE id="M-TEST" path="docs/modules/M-TEST.xml" status="active" /></MODULES></GRAPH_INDEX>"#,
        )
        .expect("graph");
        std::fs::write(
            dir.path().join("src/main.rs"),
            "// MODULE_CONTRACT\n// MODULE_ID: M-TEST\n// PURPOSE: Test module\n// START_MODULE_MAP\n// main — entry\n// END_MODULE_MAP\n// START_CHANGE_SUMMARY\n// LAST_CHANGE: [v1.0 — test]\n// END_CHANGE_SUMMARY\n// START_CONTRACT_main\n// PURPOSE: Entry\n// START_main\nfn main() {}\n// END_main\n",
        )
        .expect("source");
        let report =
            generate_development_plan_file(dir.path(), false, "auto", "topological").expect("plan");
        assert!(dir.path().join("docs/development-plan.xml").exists());
        assert!(report.exists);
        assert!(report.valid, "{:?}", report.errors);
        assert!(report.dataflow_contracts_exist());
    }
    // END_test_generate_development_plan_file_writes_valid_artifact
}
// END_public_api
