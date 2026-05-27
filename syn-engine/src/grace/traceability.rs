// MODULE_CONTRACT
// MODULE_ID: M-GRACE-TRACEABILITY
// PURPOSE: End-to-end traceability engine from requirements and use cases through contracts, semantic blocks, verification, and LOG evidence
// SCOPE: TraceabilityChain, TraceabilityReport, project scan, gap detection, trace default integration, and renderer facade exports
// DEPENDS: M-GRACE-CONTRACT, M-GRACE-LAYOUT, M-GRACE-LOG, M-GRACE-REQUIREMENTS, M-GRACE-SEMANTIC, M-INDEXER-WALKER
// LINKS:
//   -> V-M-GRACE-TRACEABILITY (verified_by) - traceability parser, report, index, and gap tests
//   -> UC-002 (implements) - verify and review bounded changes with traceable artifacts
//   -> NFR-002 (traces_to) - verification and review must not panic on malformed project state

// START_MODULE_MAP
// ArtifactType - Stable artifact categories in the trace chain
// TraceLink - One directional traceability edge
// TraceabilityChain - One artifact plus upward and downward trace links
// TraceabilityReport - Project-level coverage, score, and gap list
// scan_project_traceability - Build traceability chains from requirements, plans, contracts, blocks, and LOGs
// write_traceability_index - Re-export persisted docs/traceability-index.xml writer
// format_traceability_report - Re-export scoped MCP-friendly traceability matrix renderer
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.5.0 - Count unique function trace artifacts and skip LOG format templates]
// END_CHANGE_SUMMARY

use super::traceability_defaults::module_trace_defaults;
pub use super::traceability_render::{format_traceability_report, write_traceability_index};
use crate::grace::contract::{ContractValidator, GraceProfile, LinkDirection, TypedLink};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// START_public_api

// START_ArtifactType
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ArtifactType {
    Requirement,
    UseCase,
    Entity,
    Module,
    Contract,
    Function,
    CodeBlock,
    LogEntry,
    Verification,
    Test,
}
// END_ArtifactType

impl ArtifactType {
    // START_CONTRACT_ArtifactType::as_str
    // PURPOSE: Return stable lowercase artifact type labels for reports and XML output
    // OUTPUTS: { &'static str }
    // START_artifact_type_as_str
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Requirement => "requirement",
            Self::UseCase => "use_case",
            Self::Entity => "entity",
            Self::Module => "module",
            Self::Contract => "contract",
            Self::Function => "function",
            Self::CodeBlock => "code_block",
            Self::LogEntry => "log_entry",
            Self::Verification => "verification",
            Self::Test => "test",
        }
    }
    // END_artifact_type_as_str
}

// START_TraceLink
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TraceLink {
    pub target_id: String,
    pub target_type: String,
    pub relationship: String,
}
// END_TraceLink

// START_TraceabilityChain
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceabilityChain {
    pub artifact_id: String,
    pub artifact_type: ArtifactType,
    pub traces_to: Vec<TraceLink>,
    pub traced_by: Vec<TraceLink>,
}
// END_TraceabilityChain

// START_TraceGap
#[derive(Debug, Clone, serde::Serialize)]
pub struct TraceGap {
    pub gap_type: String,
    pub source_id: String,
    pub description: String,
}
// END_TraceGap

// START_TraceabilityReport
#[derive(Debug, Clone, serde::Serialize)]
pub struct TraceabilityReport {
    pub requirements_total: usize,
    pub use_cases_total: usize,
    pub modules_with_traceability: usize,
    pub functions_with_traceability: usize,
    pub logs_with_traceability: usize,
    pub total_functions: usize,
    pub total_logs: usize,
    pub untraced_requirements: Vec<String>,
    pub untraced_use_cases: Vec<String>,
    pub untraced_functions: Vec<String>,
    pub traceability_score: f64,
    pub enforcement_mode: String,
    pub gaps: Vec<TraceGap>,
    pub chains: Vec<TraceabilityChain>,
}
// END_TraceabilityReport

impl TraceabilityReport {
    // START_CONTRACT_TraceabilityReport::requirements_implemented_gate
    // PURPOSE: Return the verify gate outcome for requirement implementation coverage
    // OUTPUTS: { bool }
    // START_traceability_report_requirements_gate
    pub fn requirements_implemented_gate(&self) -> bool {
        self.enforcement_mode != "strict" || self.untraced_requirements.is_empty()
    }
    // END_traceability_report_requirements_gate

    // START_CONTRACT_TraceabilityReport::code_traced_gate
    // PURPOSE: Return the verify gate outcome for function-to-requirement/use-case trace links
    // OUTPUTS: { bool }
    // START_traceability_report_code_gate
    pub fn code_traced_gate(&self) -> bool {
        self.enforcement_mode != "strict" || self.untraced_functions.is_empty()
    }
    // END_traceability_report_code_gate

    // START_CONTRACT_TraceabilityReport::logs_traced_gate
    // PURPOSE: Return the recommended LOG traceability gate outcome without blocking adoption
    // OUTPUTS: { bool }
    // START_traceability_report_logs_gate
    pub fn logs_traced_gate(&self) -> bool {
        true
    }
    // END_traceability_report_logs_gate

    // START_CONTRACT_TraceabilityReport::no_dangling_gate
    // PURPOSE: Return true when no traceability links point to missing known artifacts
    // OUTPUTS: { bool }
    // START_traceability_report_no_dangling_gate
    pub fn no_dangling_gate(&self) -> bool {
        !self
            .gaps
            .iter()
            .any(|gap| gap.gap_type == "dangling_traceability_link")
    }
    // END_traceability_report_no_dangling_gate
}

// START_CONTRACT_traceability_index_template
// PURPOSE: Build an empty traceability index skeleton for new MyGRACE projects
// OUTPUTS: { String }
// START_traceability_index_template
pub fn traceability_index_template() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<TRACEABILITY_INDEX enforcement="advisory">
  <META>
    <MODEL>mygrace-sharded</MODEL>
    <PRIMARY>true</PRIMARY>
    <GENERATED_BY>M-GRACE-TRACEABILITY</GENERATED_BY>
  </META>
  <ENFORCEMENT mode="advisory">Traceability gaps are reported before strict rollout.</ENFORCEMENT>
  <REQUIREMENTS></REQUIREMENTS>
  <USE_CASES></USE_CASES>
  <GAPS></GAPS>
</TRACEABILITY_INDEX>
"#
    .to_string()
}
// END_traceability_index_template

// START_CONTRACT_scan_project_traceability
// PURPOSE: Scan project artifacts and source contracts into an end-to-end traceability report
// INPUTS: { root: &Path - project root }
// OUTPUTS: { anyhow::Result<TraceabilityReport> }
// START_scan_project_traceability
pub fn scan_project_traceability(root: &Path) -> anyhow::Result<TraceabilityReport> {
    let enforcement_mode = traceability_enforcement_mode(root);
    let req_artifacts = collect_requirements_artifacts(root)?;
    let mut chains = BTreeMap::<String, TraceabilityChain>::new();
    let mut known = BTreeSet::<String>::new();

    for requirement in &req_artifacts.requirements {
        ensure_chain(&mut chains, requirement, ArtifactType::Requirement);
        known.insert(requirement.clone());
    }
    for use_case in &req_artifacts.use_cases {
        ensure_chain(&mut chains, use_case, ArtifactType::UseCase);
        known.insert(use_case.clone());
    }
    for entity in &req_artifacts.entities {
        ensure_chain(&mut chains, entity, ArtifactType::Entity);
        known.insert(entity.clone());
    }

    collect_plan_trace_links(root, &mut chains, &mut known)?;

    let contract_report =
        ContractValidator::validate_project_with_profile(root, GraceProfile::Lite)?;
    let mut total_function_ids = BTreeSet::new();
    let mut traced_functions = BTreeSet::new();
    let mut traced_modules = BTreeSet::new();

    for contract in &contract_report.contracts {
        let Some(module_id) = contract.module_id.as_deref() else {
            continue;
        };
        if !contract.has_contract {
            continue;
        }
        ensure_chain(&mut chains, module_id, ArtifactType::Module);
        known.insert(module_id.to_string());
        let mut module_business_links = Vec::<(String, String)>::new();
        for link in &contract.links {
            add_typed_link(&mut chains, module_id, ArtifactType::Module, link);
            register_known_target(&mut known, &link.target);
            if is_business_trace_target(&link.target) {
                traced_modules.insert(module_id.to_string());
                module_business_links
                    .push((link.target.clone(), link.link_type.label().to_string()));
            }
        }
        for seed in module_trace_defaults(module_id) {
            add_trace_link(
                &mut chains,
                module_id,
                ArtifactType::Module,
                seed.target_id,
                classify_artifact_id(seed.target_id),
                seed.relationship,
            );
            traced_modules.insert(module_id.to_string());
            module_business_links.push((seed.target_id.to_string(), seed.relationship.to_string()));
        }
        for function in &contract.function_contracts {
            let function_id = format!("{}::{}", module_id, function.name);
            total_function_ids.insert(function_id.clone());
            ensure_chain(&mut chains, &function_id, ArtifactType::Function);
            known.insert(function_id.clone());
            add_trace_link(
                &mut chains,
                module_id,
                ArtifactType::Module,
                &function_id,
                ArtifactType::Function,
                "contains",
            );
            let mut function_has_business_trace = false;
            for link in &function.links {
                add_typed_link(&mut chains, &function_id, ArtifactType::Function, link);
                register_known_target(&mut known, &link.target);
                if is_business_trace_target(&link.target) {
                    traced_functions.insert(function_id.clone());
                    function_has_business_trace = true;
                }
            }
            if !function_has_business_trace {
                for (target, relationship) in &module_business_links {
                    add_trace_link(
                        &mut chains,
                        &function_id,
                        ArtifactType::Function,
                        target,
                        classify_artifact_id(target),
                        relationship,
                    );
                    traced_functions.insert(function_id.clone());
                }
            }
        }
    }

    collect_semantic_blocks(root, &mut chains, &mut known)?;
    let log_scan = collect_log_traceability(root, &mut chains, &mut known)?;
    let mut gaps = Vec::new();
    let untraced_requirements = untraced_artifacts(
        &chains,
        &req_artifacts.requirements,
        "requirement_not_implemented",
        "Requirement has no implementing module, function, verification, or LOG trace",
        &mut gaps,
    );
    let untraced_use_cases = untraced_artifacts(
        &chains,
        &req_artifacts.use_cases,
        "use_case_not_implemented",
        "Use case has no implementing module, function, verification, or LOG trace",
        &mut gaps,
    );

    let mut untraced_functions = Vec::new();
    for chain in chains.values() {
        if chain.artifact_type == ArtifactType::Function
            && !has_business_trace(&chain.traces_to)
            && !chain.artifact_id.contains("::test_")
        {
            untraced_functions.push(chain.artifact_id.clone());
            gaps.push(TraceGap {
                gap_type: "function_not_traced".into(),
                source_id: chain.artifact_id.clone(),
                description: "Function contract has no direct use-case or requirement trace".into(),
            });
        }
    }

    collect_dangling_gaps(&chains, &known, &mut gaps);
    gaps.sort_by(|a, b| {
        a.gap_type
            .cmp(&b.gap_type)
            .then_with(|| a.source_id.cmp(&b.source_id))
    });
    gaps.dedup_by(|a, b| a.gap_type == b.gap_type && a.source_id == b.source_id);

    let implemented_requirements = req_artifacts
        .requirements
        .len()
        .saturating_sub(untraced_requirements.len());
    let implemented_use_cases = req_artifacts
        .use_cases
        .len()
        .saturating_sub(untraced_use_cases.len());
    let total_logs_for_score = if log_scan.total_logs == 0 {
        0
    } else {
        log_scan.total_logs
    };
    let total_functions = total_function_ids.len();
    let score_denominator = req_artifacts.requirements.len()
        + req_artifacts.use_cases.len()
        + total_functions
        + total_logs_for_score;
    let score_numerator = implemented_requirements
        + implemented_use_cases
        + traced_functions.len()
        + log_scan.logs_with_traceability;
    let traceability_score = if score_denominator == 0 {
        1.0
    } else {
        score_numerator as f64 / score_denominator as f64
    };

    let mut chains_vec: Vec<_> = chains.into_values().collect();
    chains_vec.sort_by(|a, b| a.artifact_id.cmp(&b.artifact_id));
    Ok(TraceabilityReport {
        requirements_total: req_artifacts.requirements.len(),
        use_cases_total: req_artifacts.use_cases.len(),
        modules_with_traceability: traced_modules.len(),
        functions_with_traceability: traced_functions.len(),
        logs_with_traceability: log_scan.logs_with_traceability,
        total_functions,
        total_logs: log_scan.total_logs,
        untraced_requirements,
        untraced_use_cases,
        untraced_functions,
        traceability_score,
        enforcement_mode,
        gaps,
        chains: chains_vec,
    })
}
// END_scan_project_traceability

// END_public_api

#[derive(Default)]
struct RequirementsArtifacts {
    requirements: Vec<String>,
    use_cases: Vec<String>,
    entities: Vec<String>,
}

#[derive(Default)]
struct LogTraceScan {
    total_logs: usize,
    logs_with_traceability: usize,
}

fn collect_requirements_artifacts(root: &Path) -> anyhow::Result<RequirementsArtifacts> {
    let content =
        std::fs::read_to_string(root.join("docs").join("requirements.xml")).unwrap_or_default();
    Ok(RequirementsArtifacts {
        requirements: collect_ids_from_tag(&content, "Requirement")?,
        use_cases: collect_ids_from_tag(&content, "UseCase")?,
        entities: collect_entity_ids(&content)?,
    })
}

fn collect_ids_from_tag(content: &str, tag: &str) -> anyhow::Result<Vec<String>> {
    let re = regex::Regex::new(&format!(r#"<{}\s+[^>]*id="([^"]+)""#, regex::escape(tag)))?;
    let mut ids = BTreeSet::new();
    for cap in re.captures_iter(content) {
        ids.insert(cap[1].to_string());
    }
    Ok(ids.into_iter().collect())
}

fn collect_entity_ids(content: &str) -> anyhow::Result<Vec<String>> {
    let re = regex::Regex::new(r#"<Entity\s+[^>]*name="([^"]+)""#)?;
    let mut ids = BTreeSet::new();
    for cap in re.captures_iter(content) {
        ids.insert(format!("Entity:{}", &cap[1]));
    }
    Ok(ids.into_iter().collect())
}

fn collect_plan_trace_links(
    root: &Path,
    chains: &mut BTreeMap<String, TraceabilityChain>,
    known: &mut BTreeSet<String>,
) -> anyhow::Result<()> {
    let content =
        std::fs::read_to_string(root.join("docs").join("development-plan.xml")).unwrap_or_default();
    let module_re = regex::Regex::new(r#"(?s)<Module\s+[^>]*id="([^"]+)"[^>]*>(.*?)</Module>"#)?;
    let link_re = regex::Regex::new(r#"<Link\s+[^>]*ref="([^"]+)"[^>]*type="([^"]+)"[^>]*/?>"#)?;
    for cap in module_re.captures_iter(&content) {
        let module_id = cap[1].to_string();
        ensure_chain(chains, &module_id, ArtifactType::Module);
        known.insert(module_id.clone());
        for link_cap in link_re.captures_iter(&cap[2]) {
            let target = link_cap[1].to_string();
            let relationship = link_cap[2].to_string();
            register_known_target(known, &target);
            add_trace_link(
                chains,
                &module_id,
                ArtifactType::Module,
                &target,
                classify_artifact_id(&target),
                &relationship,
            );
        }
    }
    Ok(())
}

fn collect_semantic_blocks(
    root: &Path,
    chains: &mut BTreeMap<String, TraceabilityChain>,
    known: &mut BTreeSet<String>,
) -> anyhow::Result<()> {
    let sem = crate::grace::semantic::SemanticExtractor::scan_project(root)?;
    for block in sem.blocks {
        if !block.is_closed || block.name.starts_with("CONTRACT") {
            continue;
        }
        let block_id = format!("{}#{}", block.file_path, block.name);
        ensure_chain(chains, &block_id, ArtifactType::CodeBlock);
        known.insert(block_id);
    }
    Ok(())
}

fn collect_log_traceability(
    root: &Path,
    chains: &mut BTreeMap<String, TraceabilityChain>,
    known: &mut BTreeSet<String>,
) -> anyhow::Result<LogTraceScan> {
    let walker = crate::indexer::walker::Walker::new(root);
    let log_re = regex::Regex::new(r#"(?s)<LOG\s+([^>]*)>(.*?)</LOG>"#)?;
    let id_re = regex::Regex::new(r#"id="([^"]+)""#)?;
    let target_re = regex::Regex::new(
        r#"(UC-[A-Za-z0-9_-]+|REQ-[A-Za-z0-9_-]+|NFR-[A-Za-z0-9_-]+|CON-[A-Za-z0-9_-]+|G-[A-Za-z0-9_-]+|M-[A-Za-z0-9_-]+|V-M-[A-Za-z0-9_-]+|Entity:[A-Za-z0-9_-]+)"#,
    )?;
    let mut scan = LogTraceScan::default();
    for file in walker.walk() {
        if file.language == "markdown" {
            continue;
        }
        let content = std::fs::read_to_string(root.join(&file.path)).unwrap_or_default();
        for cap in log_re.captures_iter(&content) {
            let attrs = &cap[1];
            if attrs.contains("{}") {
                continue;
            }
            scan.total_logs += 1;
            let id = id_re
                .captures(attrs)
                .map(|id_cap| id_cap[1].to_string())
                .unwrap_or_else(|| format!("{}#log-{}", file.path, scan.total_logs));
            let log_id = format!("LOG:{}", id);
            ensure_chain(chains, &log_id, ArtifactType::LogEntry);
            known.insert(log_id.clone());
            let body = &cap[2];
            let trace_source = traceability_section(body).or_else(|| {
                if attrs.to_ascii_lowercase().contains("traceability") {
                    Some(attrs)
                } else {
                    None
                }
            });
            if let Some(trace_section) = trace_source {
                let mut has_trace_target = false;
                for target_cap in target_re.captures_iter(trace_section) {
                    has_trace_target = true;
                    let target = target_cap[1].to_string();
                    register_known_target(known, &target);
                    add_trace_link(
                        chains,
                        &log_id,
                        ArtifactType::LogEntry,
                        &target,
                        classify_artifact_id(&target),
                        "evidences",
                    );
                }
                if has_trace_target {
                    scan.logs_with_traceability += 1;
                }
            }
        }
    }
    Ok(scan)
}

fn add_typed_link(
    chains: &mut BTreeMap<String, TraceabilityChain>,
    source_id: &str,
    source_type: ArtifactType,
    link: &TypedLink,
) {
    let relationship = link.link_type.label();
    let target_type = classify_artifact_id(&link.target);
    match link.direction {
        LinkDirection::Outgoing => add_trace_link(
            chains,
            source_id,
            source_type,
            &link.target,
            target_type,
            relationship,
        ),
        LinkDirection::Incoming => add_trace_link(
            chains,
            &link.target,
            target_type,
            source_id,
            source_type,
            relationship,
        ),
    }
}

fn add_trace_link(
    chains: &mut BTreeMap<String, TraceabilityChain>,
    source_id: &str,
    source_type: ArtifactType,
    target_id: &str,
    target_type: ArtifactType,
    relationship: &str,
) {
    if target_id.trim().is_empty() || target_id == "N/A" {
        return;
    }
    ensure_chain(chains, source_id, source_type);
    ensure_chain(chains, target_id, target_type.clone());
    let forward = TraceLink {
        target_id: target_id.to_string(),
        target_type: target_type.as_str().to_string(),
        relationship: relationship.to_string(),
    };
    let reverse = TraceLink {
        target_id: source_id.to_string(),
        target_type: chains
            .get(source_id)
            .map(|chain| chain.artifact_type.as_str().to_string())
            .unwrap_or_else(|| "artifact".into()),
        relationship: reverse_relationship(relationship).into(),
    };
    if let Some(source) = chains.get_mut(source_id) {
        push_unique_link(&mut source.traces_to, forward);
    }
    if let Some(target) = chains.get_mut(target_id) {
        push_unique_link(&mut target.traced_by, reverse);
    }
}

fn ensure_chain(
    chains: &mut BTreeMap<String, TraceabilityChain>,
    artifact_id: &str,
    artifact_type: ArtifactType,
) {
    chains
        .entry(artifact_id.to_string())
        .or_insert_with(|| TraceabilityChain {
            artifact_id: artifact_id.to_string(),
            artifact_type,
            traces_to: Vec::new(),
            traced_by: Vec::new(),
        });
}

fn push_unique_link(links: &mut Vec<TraceLink>, link: TraceLink) {
    if !links.iter().any(|existing| existing == &link) {
        links.push(link);
    }
}

fn untraced_artifacts(
    chains: &BTreeMap<String, TraceabilityChain>,
    ids: &[String],
    gap_type: &str,
    description: &str,
    gaps: &mut Vec<TraceGap>,
) -> Vec<String> {
    let mut untraced = Vec::new();
    for id in ids {
        let traced = chains
            .get(id)
            .is_some_and(|chain| !chain.traced_by.is_empty());
        if !traced {
            untraced.push(id.clone());
            gaps.push(TraceGap {
                gap_type: gap_type.into(),
                source_id: id.clone(),
                description: description.into(),
            });
        }
    }
    untraced
}

fn collect_dangling_gaps(
    chains: &BTreeMap<String, TraceabilityChain>,
    known: &BTreeSet<String>,
    gaps: &mut Vec<TraceGap>,
) {
    for chain in chains.values() {
        for link in &chain.traces_to {
            if is_known_id_shape(&link.target_id) && !known.contains(&link.target_id) {
                gaps.push(TraceGap {
                    gap_type: "dangling_traceability_link".into(),
                    source_id: chain.artifact_id.clone(),
                    description: format!(
                        "{} links to missing target {}",
                        chain.artifact_id, link.target_id
                    ),
                });
            }
        }
    }
}

fn has_business_trace(links: &[TraceLink]) -> bool {
    links.iter().any(|link| {
        matches!(
            link.target_type.as_str(),
            "use_case" | "requirement" | "entity"
        ) && matches!(
            link.relationship.as_str(),
            "implements" | "traces_to" | "manages" | "evidences"
        )
    })
}

fn is_business_trace_target(target: &str) -> bool {
    matches!(
        classify_artifact_id(target),
        ArtifactType::UseCase | ArtifactType::Requirement | ArtifactType::Entity
    )
}

fn is_known_id_shape(target: &str) -> bool {
    target.starts_with("UC-")
        || target.starts_with("REQ-")
        || target.starts_with("NFR-")
        || target.starts_with("CON-")
        || target.starts_with("G-")
        || target.starts_with("M-")
        || target.starts_with("V-")
        || target.starts_with("Entity:")
        || target.contains("::")
        || target.starts_with("LOG:")
}

fn register_known_target(known: &mut BTreeSet<String>, target: &str) {
    if target.starts_with("M-")
        || target.starts_with("V-")
        || target.contains("::")
        || target.starts_with("LOG:")
    {
        known.insert(target.to_string());
    }
}

fn classify_artifact_id(target: &str) -> ArtifactType {
    if target.starts_with("UC-") {
        ArtifactType::UseCase
    } else if target.starts_with("REQ-")
        || target.starts_with("NFR-")
        || target.starts_with("CON-")
        || target.starts_with("G-")
    {
        ArtifactType::Requirement
    } else if target.starts_with("Entity:") || target.starts_with("ENT-") {
        ArtifactType::Entity
    } else if target.starts_with("V-") {
        ArtifactType::Verification
    } else if target.starts_with("LOG:") {
        ArtifactType::LogEntry
    } else if target.contains("::") {
        ArtifactType::Function
    } else if target.starts_with("M-") {
        ArtifactType::Module
    } else {
        ArtifactType::Contract
    }
}

fn reverse_relationship(relationship: &str) -> &'static str {
    match relationship {
        "implements" => "implemented_by",
        "traces_to" => "traced_by",
        "verified_by" => "verifies",
        "manages" => "managed_by",
        "uses" => "used_by",
        "depends" => "depended_on_by",
        "contains" => "contained_by",
        "evidences" => "evidenced_by",
        _ => "traced_by",
    }
}

fn traceability_section(body: &str) -> Option<&str> {
    let start = body.find("TRACEABILITY:")?;
    let rest = &body[start + "TRACEABILITY:".len()..];
    let end = rest
        .find("\n  EVENT:")
        .or_else(|| rest.find("\n  CONTEXT:"))
        .or_else(|| rest.find("\n  STATE:"))
        .or_else(|| rest.find("\n  DECISION:"))
        .or_else(|| rest.find("\n  EXPECTATION:"))
        .or_else(|| rest.find("\n  RESULT:"))
        .or_else(|| rest.find("\n  DETAIL:"))
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

fn traceability_enforcement_mode(root: &Path) -> String {
    let path = crate::grace::layout::DocsLayout::new(root).traceability_index_path();
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let mode = regex::Regex::new(r#"<ENFORCEMENT\s+[^>]*mode="([^"]+)""#)
        .ok()
        .and_then(|re| re.captures(&content).map(|cap| cap[1].to_string()))
        .or_else(|| {
            regex::Regex::new(r#"<TRACEABILITY_INDEX\s+[^>]*enforcement="([^"]+)""#)
                .ok()
                .and_then(|re| re.captures(&content).map(|cap| cap[1].to_string()))
        })
        .unwrap_or_else(|| "advisory".into());
    if mode == "strict" {
        "strict".into()
    } else {
        "advisory".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traceability_report_counts_complete_chain() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_requirements(dir.path(), "REQ-001", "UC-001");
        write_traceability_mode(dir.path(), "strict");
        write_module(
            dir.path(),
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-ORDER\n",
                "// PURPOSE: Order workflow\n",
                "// SCOPE: Place orders\n",
                "// DEPENDS: N/A\n",
                "// LINKS:\n",
                "//   -> UC-001 (implements) - place order\n",
                "//   -> REQ-001 (traces_to) - order requirement\n",
                "\n",
                "// START_MODULE_MAP\n",
                "// place_order - Places an order\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 - Initial]\n",
                "// END_CHANGE_SUMMARY\n",
                "\n",
                "// START_CONTRACT_place_order\n",
                "// PURPOSE: Place one order\n",
                "// LINKS:\n",
                "//   -> UC-001 (implements) - use case\n",
                "//   -> REQ-001 (traces_to) - requirement\n",
                "// START_place_order\n",
                "pub fn place_order() {}\n",
                "// END_place_order\n",
            ),
        );

        let report = scan_project_traceability(dir.path()).expect("report");
        assert!(report.requirements_implemented_gate());
        assert!(report.code_traced_gate());
        assert!(report.no_dangling_gate());
        assert_eq!(report.functions_with_traceability, 1);
        assert!(report.untraced_requirements.is_empty());
        assert!(report.untraced_use_cases.is_empty());
        assert!(report.untraced_functions.is_empty());
    }

    #[test]
    fn test_traceability_report_flags_untraced_function() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_requirements(dir.path(), "REQ-001", "UC-001");
        write_traceability_mode(dir.path(), "strict");
        write_module(
            dir.path(),
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-UNTRACED\n",
                "// PURPOSE: Unmapped helper workflow\n",
                "// SCOPE: Exercise missing function trace detection\n",
                "// DEPENDS: N/A\n",
                "// LINKS: N/A\n",
                "\n",
                "// START_MODULE_MAP\n",
                "// helper - Helper\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 - Initial]\n",
                "// END_CHANGE_SUMMARY\n",
                "\n",
                "// START_CONTRACT_helper\n",
                "// PURPOSE: Helper without business trace\n",
                "// START_helper\n",
                "pub fn helper() {}\n",
                "// END_helper\n",
            ),
        );

        let report = scan_project_traceability(dir.path()).expect("report");
        assert!(!report.code_traced_gate());
        assert!(report
            .untraced_functions
            .contains(&"M-UNTRACED::helper".to_string()));
    }

    #[test]
    fn test_module_trace_defaults_cover_grace_verify() {
        let defaults = module_trace_defaults("M-GRACE-VERIFY");
        assert!(defaults
            .iter()
            .any(|seed| seed.target_id == "UC-002" && seed.relationship == "implements"));
        assert!(defaults
            .iter()
            .any(|seed| seed.target_id == "NFR-002" && seed.relationship == "traces_to"));
    }

    #[test]
    fn test_duplicate_function_names_count_as_one_trace_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_requirements(dir.path(), "REQ-001", "UC-001");
        write_module_to(dir.path(), "first.rs", duplicate_function_module("First"));
        write_module_to(dir.path(), "second.rs", duplicate_function_module("Second"));

        let report = scan_project_traceability(dir.path()).expect("report");
        assert_eq!(report.total_functions, 1);
        assert_eq!(report.functions_with_traceability, 1);
        assert!(report.untraced_functions.is_empty());
    }

    #[test]
    fn test_traceability_report_flags_dangling_link() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_requirements(dir.path(), "REQ-001", "UC-001");
        write_module(
            dir.path(),
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-ORDER\n",
                "// PURPOSE: Order workflow\n",
                "// SCOPE: Place orders\n",
                "// DEPENDS: N/A\n",
                "// LINKS:\n",
                "//   -> UC-404 (implements) - missing use case\n",
                "\n",
                "// START_MODULE_MAP\n",
                "// run - Runs\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 - Initial]\n",
                "// END_CHANGE_SUMMARY\n",
                "\n",
                "// START_CONTRACT_run\n",
                "// PURPOSE: Run\n",
                "// START_run\n",
                "pub fn run() {}\n",
                "// END_run\n",
            ),
        );

        let report = scan_project_traceability(dir.path()).expect("report");
        assert!(!report.no_dangling_gate());
        assert!(report
            .gaps
            .iter()
            .any(|gap| gap.gap_type == "dangling_traceability_link"));
    }

    #[test]
    fn test_traceability_index_and_query_render() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_requirements(dir.path(), "REQ-001", "UC-001");
        write_module(
            dir.path(),
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-ORDER\n",
                "// PURPOSE: Order workflow\n",
                "// SCOPE: Place orders\n",
                "// DEPENDS: N/A\n",
                "// LINKS:\n",
                "//   -> UC-001 (implements) - place order\n",
                "\n",
                "// START_MODULE_MAP\n",
                "// place_order - Places an order\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 - Initial]\n",
                "// END_CHANGE_SUMMARY\n",
                "\n",
                "// START_CONTRACT_place_order\n",
                "// PURPOSE: Place one order\n",
                "// LINKS:\n",
                "//   -> UC-001 (implements) - use case\n",
                "// START_place_order\n",
                "pub fn place_order() {}\n",
                "// END_place_order\n",
            ),
        );

        let report = scan_project_traceability(dir.path()).expect("report");
        let path = write_traceability_index(dir.path(), &report).expect("write index");
        let rendered = std::fs::read_to_string(path).expect("index");
        assert!(rendered.contains("<TRACEABILITY_INDEX"));
        let query = format_traceability_report(&report, "module", Some("M-ORDER"), "up");
        assert!(query.contains("M-ORDER::place_order"));
        assert!(query.contains("UC-001"));
    }

    fn write_requirements(root: &Path, requirement: &str, use_case: &str) {
        std::fs::create_dir_all(root.join("docs")).expect("docs");
        std::fs::write(
            root.join("docs/requirements.xml"),
            format!(
                r#"<RequirementsAnalysis>
  <DomainModel><Entity name="Order"><Attributes><Attribute name="id" /></Attributes></Entity></DomainModel>
  <UseCases><UseCase id="{use_case}"><Actor>User</Actor><Action>Order</Action><Goal>Place order</Goal></UseCase></UseCases>
  <NonFunctionalRequirements><Requirement id="{requirement}"><Description>Order must work</Description></Requirement></NonFunctionalRequirements>
</RequirementsAnalysis>"#
            ),
        )
        .expect("requirements");
    }

    fn write_traceability_mode(root: &Path, mode: &str) {
        std::fs::create_dir_all(root.join("docs")).expect("docs");
        std::fs::write(
            root.join("docs/traceability-index.xml"),
            format!(
                "<TRACEABILITY_INDEX enforcement=\"{mode}\"><ENFORCEMENT mode=\"{mode}\" /></TRACEABILITY_INDEX>"
            ),
        )
        .expect("traceability");
    }

    fn write_module(root: &Path, content: &str) {
        write_module_to(root, "order.rs", content.to_string());
    }

    fn write_module_to(root: &Path, file_name: &str, content: String) {
        std::fs::create_dir_all(root.join("src")).expect("src");
        std::fs::write(root.join("src").join(file_name), content).expect("source");
    }

    fn duplicate_function_module(label: &str) -> String {
        format!(
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-DUP\n",
                "// PURPOSE: Duplicate {label} workflow\n",
                "// SCOPE: Exercise duplicate trace artifact counting\n",
                "// DEPENDS: N/A\n",
                "// LINKS:\n",
                "//   -> UC-001 (implements) - duplicate use case\n",
                "\n",
                "// START_MODULE_MAP\n",
                "// read_cargo_version - Reads version\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 - Initial]\n",
                "// END_CHANGE_SUMMARY\n",
                "\n",
                "// START_CONTRACT_read_cargo_version\n",
                "// PURPOSE: Read cargo version\n",
                "// START_read_cargo_version\n",
                "pub fn read_cargo_version() {{}}\n",
                "// END_read_cargo_version\n",
            ),
            label = label
        )
    }
}
