// MODULE_CONTRACT
// MODULE_ID: M-GRACE-BELIEF-STATE
// PURPOSE: Observable AI belief state parser, validator, coverage scanner, and artifact writer
// SCOPE: BELIEF_STATE data model, XML-like parser, source/artifact scan, validation, and docs/belief-states storage
// DEPENDS: M-GRACE-CONTRACT, M-GRACE-LAYOUT, M-INDEXER-WALKER
// LINKS:
//   → V-M-GRACE-BELIEF-STATE (verified_by) — parser, validator, coverage, and storage tests

// START_MODULE_MAP
// BeliefState — Explicit AI understanding, strategy, expectations, and risks for a module
// BeliefStateReport — Project-level belief state coverage and structural validation report
// parse_belief_states — Parse BELIEF_STATE blocks from source or artifact text
// scan_project_belief_states — Validate discovered belief states and report module coverage
// extract_and_store_belief_state — Build and persist a deterministic belief state scaffold
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Added observable belief state parser, validator, scanner, and storage scaffold]
// END_CHANGE_SUMMARY

use crate::grace::contract::{ContractValidator, GraceProfile};
use crate::grace::layout::DocsLayout;
use std::collections::{HashMap, HashSet};
use std::path::Path;

// START_public_api

// START_BeliefState
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BeliefState {
    pub module_id: String,
    pub version: String,
    pub understanding: BeliefUnderstanding,
    pub implementation_strategy: BeliefStrategy,
    pub verification_intent: Vec<String>,
    pub risks_acknowledged: Vec<String>,
    pub file_path: Option<String>,
    pub valid: bool,
    pub errors: Vec<String>,
}
// END_BeliefState

impl BeliefState {
    // START_CONTRACT_BeliefState::to_xml
    // PURPOSE: Render this belief state as a docs/belief-states XML-like artifact
    // OUTPUTS: { String }
    // START_belief_state_to_xml
    pub fn to_xml(&self) -> String {
        let mut output = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<BELIEF_STATE module=\"{}\" version=\"{}\">\n",
            escape_attr(&self.module_id),
            escape_attr(&self.version)
        );
        output.push_str("  UNDERSTANDING:\n");
        output.push_str(&format!(
            "    Module responsibility: {}\n",
            escape_text(&self.understanding.responsibility)
        ));
        output.push_str("    Key data flows:\n");
        for flow in &self.understanding.key_data_flows {
            output.push_str(&format!(
                "      - {}: {}\n",
                escape_text(&flow.direction),
                escape_text(&flow.description)
            ));
        }
        output.push_str("    Critical invariants:\n");
        for invariant in &self.understanding.critical_invariants {
            output.push_str(&format!("      - {}\n", escape_text(invariant)));
        }
        output.push_str("  IMPLEMENTATION_STRATEGY:\n");
        output.push_str(&format!(
            "    Approach: {}\n",
            escape_text(&self.implementation_strategy.approach)
        ));
        output.push_str(&format!(
            "    Complexity_areas: {}\n",
            join_items(&self.implementation_strategy.complexity_areas)
        ));
        output.push_str(&format!(
            "    Patterns_applied: {}\n",
            join_items(&self.implementation_strategy.patterns_applied)
        ));
        output.push_str("  VERIFICATION_INTENT:\n");
        output.push_str("    I expect the following to be true after my code runs:\n");
        for expectation in &self.verification_intent {
            output.push_str(&format!("    - {}\n", escape_text(expectation)));
        }
        output.push_str("  RISKS_ACKNOWLEDGED:\n");
        for risk in &self.risks_acknowledged {
            output.push_str(&format!("    - {}\n", escape_text(risk)));
        }
        output.push_str("</BELIEF_STATE>\n");
        output
    }
    // END_belief_state_to_xml
}

// START_BeliefUnderstanding
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct BeliefUnderstanding {
    pub responsibility: String,
    pub key_data_flows: Vec<DataFlowBelief>,
    pub critical_invariants: Vec<String>,
}
// END_BeliefUnderstanding

// START_DataFlowBelief
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataFlowBelief {
    pub direction: String,
    pub description: String,
}
// END_DataFlowBelief

// START_BeliefStrategy
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct BeliefStrategy {
    pub approach: String,
    pub complexity_areas: Vec<String>,
    pub patterns_applied: Vec<String>,
}
// END_BeliefStrategy

// START_BeliefStateReport
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct BeliefStateReport {
    pub total_modules: usize,
    pub states_found: usize,
    pub valid_states: usize,
    pub invalid_states: usize,
    pub persisted_states: usize,
    pub coverage_pct: f64,
    pub missing_modules: Vec<String>,
    pub issues: Vec<String>,
}
// END_BeliefStateReport

impl BeliefStateReport {
    // START_CONTRACT_BeliefStateReport::passed
    // PURPOSE: Return true when discovered belief states have no structural errors
    // OUTPUTS: { bool }
    // START_belief_state_report_passed
    pub fn passed(&self) -> bool {
        self.invalid_states == 0
    }
    // END_belief_state_report_passed
}

// START_BeliefStateStorageResult
#[derive(Debug, Clone, serde::Serialize)]
pub struct BeliefStateStorageResult {
    pub module_id: String,
    pub path: String,
    pub valid: bool,
    pub errors: Vec<String>,
}
// END_BeliefStateStorageResult

// START_CONTRACT_parse_belief_states
// PURPOSE: Parse BELIEF_STATE blocks from source or XML artifact text
// INPUTS: { content: &str — source or artifact content }
// OUTPUTS: { Vec<BeliefState> }
// START_parse_belief_states
pub fn parse_belief_states(content: &str) -> Vec<BeliefState> {
    let lines: Vec<&str> = content.lines().collect();
    let mut states = Vec::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        let line = normalize_belief_line(lines[idx]);
        if !line.starts_with("<BELIEF_STATE") {
            idx += 1;
            continue;
        }
        let attrs = parse_attrs(&line);
        let mut body = Vec::new();
        idx += 1;
        while idx < lines.len() {
            let body_line = normalize_belief_line(lines[idx]);
            if body_line.starts_with("</BELIEF_STATE>") {
                break;
            }
            body.push(body_line);
            idx += 1;
        }
        states.push(parse_belief_state_block(&attrs, &body));
        idx += 1;
    }
    states
}
// END_parse_belief_states

// START_CONTRACT_scan_project_belief_states
// PURPOSE: Validate source and docs/belief-states BELIEF_STATE blocks and report project module coverage
// INPUTS: { root: &Path — project root }
// OUTPUTS: { anyhow::Result<BeliefStateReport> }
// START_scan_project_belief_states
pub fn scan_project_belief_states(root: &Path) -> anyhow::Result<BeliefStateReport> {
    let contracts = ContractValidator::validate_project_with_profile(root, GraceProfile::Lite)?;
    let mut module_ids: Vec<String> = contracts
        .contracts
        .iter()
        .filter(|contract| contract.has_contract)
        .filter_map(|contract| contract.module_id.clone())
        .collect();
    module_ids.sort();
    module_ids.dedup();
    let total_modules = module_ids.len();

    let walker = crate::indexer::walker::Walker::new(root);
    let mut discovered = Vec::new();
    for file in walker.walk() {
        if file.language == "markdown" {
            continue;
        }
        let full_path = root.join(&file.path);
        let Ok(content) = std::fs::read_to_string(&full_path) else {
            continue;
        };
        for mut state in parse_belief_states(&content) {
            state.file_path = Some(file.path.clone());
            discovered.push(state);
        }
    }

    let layout = DocsLayout::new(root);
    let mut persisted_states = 0usize;
    if let Ok(entries) = std::fs::read_dir(layout.belief_states_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            for mut state in parse_belief_states(&content) {
                state.file_path = Some(path.display().to_string());
                persisted_states += 1;
                discovered.push(state);
            }
        }
    }

    let mut seen_modules = HashSet::new();
    let mut issues = Vec::new();
    let mut valid_states = 0usize;
    let mut invalid_states = 0usize;
    for state in &discovered {
        if !state.module_id.trim().is_empty() {
            seen_modules.insert(state.module_id.clone());
        }
        if state.valid {
            valid_states += 1;
        } else {
            invalid_states += 1;
            let context = state
                .file_path
                .clone()
                .unwrap_or_else(|| state.module_id.clone());
            for error in &state.errors {
                issues.push(format!("{}: {}", context, error));
            }
        }
    }

    let mut missing_modules: Vec<String> = module_ids
        .into_iter()
        .filter(|module_id| !seen_modules.contains(module_id))
        .collect();
    missing_modules.sort();
    let coverage_pct = if total_modules == 0 {
        100.0
    } else {
        (seen_modules.len().min(total_modules) as f64 / total_modules as f64) * 100.0
    };

    Ok(BeliefStateReport {
        total_modules,
        states_found: seen_modules.len(),
        valid_states,
        invalid_states,
        persisted_states,
        coverage_pct,
        missing_modules,
        issues,
    })
}
// END_scan_project_belief_states

// START_CONTRACT_extract_and_store_belief_state
// PURPOSE: Build a deterministic belief state scaffold from module artifacts and store it under docs/belief-states
// INPUTS: { root: &Path }, { module_id: &str }, { context: &str }
// OUTPUTS: { anyhow::Result<BeliefStateStorageResult> }
// SIDE_EFFECTS: creates docs/belief-states/M-XXX.xml
// START_extract_and_store_belief_state
pub fn extract_and_store_belief_state(
    root: &Path,
    module_id: &str,
    context: &str,
) -> anyhow::Result<BeliefStateStorageResult> {
    let module_id = module_id.trim();
    if module_id.is_empty() {
        anyhow::bail!("module_id is required");
    }
    let info = read_module_info(root, module_id)?;
    let state = scaffold_belief_state(module_id, context, &info);
    let layout = DocsLayout::new(root);
    std::fs::create_dir_all(layout.belief_states_dir())?;
    let path = layout
        .belief_states_dir()
        .join(format!("{}.xml", module_id));
    std::fs::write(&path, state.to_xml())?;
    Ok(BeliefStateStorageResult {
        module_id: module_id.to_string(),
        path: path.display().to_string(),
        valid: state.valid,
        errors: state.errors,
    })
}
// END_extract_and_store_belief_state

// END_public_api

#[derive(Debug, Clone, Default)]
struct ModuleInfo {
    purpose: String,
    scope: String,
    depends: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BeliefSection {
    None,
    Understanding,
    DataFlows,
    Invariants,
    Strategy,
    Verification,
    Risks,
}

fn parse_belief_state_block(attrs: &HashMap<String, String>, body: &[String]) -> BeliefState {
    let mut understanding = BeliefUnderstanding::default();
    let mut strategy = BeliefStrategy::default();
    let mut verification_intent = Vec::new();
    let mut risks_acknowledged = Vec::new();
    let mut section = BeliefSection::None;

    for raw in body {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        match line {
            "UNDERSTANDING:" => {
                section = BeliefSection::Understanding;
                continue;
            }
            "Key data flows:" => {
                section = BeliefSection::DataFlows;
                continue;
            }
            "Critical invariants:" => {
                section = BeliefSection::Invariants;
                continue;
            }
            "IMPLEMENTATION_STRATEGY:" => {
                section = BeliefSection::Strategy;
                continue;
            }
            "VERIFICATION_INTENT:" => {
                section = BeliefSection::Verification;
                continue;
            }
            "RISKS_ACKNOWLEDGED:" => {
                section = BeliefSection::Risks;
                continue;
            }
            _ => {}
        }

        if let Some(value) = line.strip_prefix("Module responsibility:") {
            understanding.responsibility = unescape_text(value.trim());
            continue;
        }
        if let Some(value) = line.strip_prefix("Approach:") {
            strategy.approach = unescape_text(value.trim());
            continue;
        }
        if let Some(value) = line.strip_prefix("Complexity_areas:") {
            strategy.complexity_areas = split_items(value);
            continue;
        }
        if let Some(value) = line.strip_prefix("Patterns_applied:") {
            strategy.patterns_applied = split_items(value);
            continue;
        }
        if line.starts_with("I expect ") {
            continue;
        }
        if let Some(value) = line.strip_prefix("- ") {
            let value = unescape_text(value.trim());
            match section {
                BeliefSection::DataFlows => {
                    if let Some((direction, description)) = value.split_once(':') {
                        understanding.key_data_flows.push(DataFlowBelief {
                            direction: direction.trim().to_ascii_uppercase(),
                            description: description.trim().to_string(),
                        });
                    }
                }
                BeliefSection::Invariants => understanding.critical_invariants.push(value),
                BeliefSection::Verification => verification_intent.push(value),
                BeliefSection::Risks => risks_acknowledged.push(value),
                BeliefSection::Strategy => strategy.complexity_areas.push(value),
                BeliefSection::None | BeliefSection::Understanding => {}
            }
        }
    }

    let mut state = BeliefState {
        module_id: attrs.get("module").cloned().unwrap_or_default(),
        version: attrs.get("version").cloned().unwrap_or_default(),
        understanding,
        implementation_strategy: strategy,
        verification_intent,
        risks_acknowledged,
        file_path: None,
        valid: true,
        errors: Vec::new(),
    };
    validate_belief_state(&mut state);
    state
}

fn validate_belief_state(state: &mut BeliefState) {
    if state.module_id.trim().is_empty() {
        state
            .errors
            .push("BELIEF_STATE missing module attribute".into());
    }
    if state.version.trim().is_empty() {
        state
            .errors
            .push("BELIEF_STATE missing version attribute".into());
    }
    if state.understanding.responsibility.trim().is_empty() {
        state
            .errors
            .push("UNDERSTANDING missing Module responsibility".into());
    }
    let directions: HashSet<String> = state
        .understanding
        .key_data_flows
        .iter()
        .map(|flow| flow.direction.to_ascii_uppercase())
        .collect();
    for required in ["INPUT", "PROCESSING", "OUTPUT"] {
        if !directions.contains(required) {
            state
                .errors
                .push(format!("Key data flows missing {}", required));
        }
    }
    if state.understanding.critical_invariants.is_empty() {
        state
            .errors
            .push("UNDERSTANDING missing Critical invariants".into());
    }
    if state.implementation_strategy.approach.trim().is_empty() {
        state
            .errors
            .push("IMPLEMENTATION_STRATEGY missing Approach".into());
    }
    if state.implementation_strategy.complexity_areas.is_empty() {
        state
            .errors
            .push("IMPLEMENTATION_STRATEGY missing Complexity_areas".into());
    }
    if state.implementation_strategy.patterns_applied.is_empty() {
        state
            .errors
            .push("IMPLEMENTATION_STRATEGY missing Patterns_applied".into());
    }
    if state.verification_intent.is_empty() {
        state
            .errors
            .push("VERIFICATION_INTENT missing expectations".into());
    }
    if state.risks_acknowledged.is_empty() {
        state.errors.push("RISKS_ACKNOWLEDGED missing risks".into());
    }
    state.valid = state.errors.is_empty();
}

fn read_module_info(root: &Path, module_id: &str) -> anyhow::Result<ModuleInfo> {
    let layout = DocsLayout::new(root);
    let module_path = layout.modules_dir().join(format!("{}.xml", module_id));
    if let Ok(content) = std::fs::read_to_string(&module_path) {
        return Ok(ModuleInfo {
            purpose: tag_text(&content, "PURPOSE").unwrap_or_default(),
            scope: tag_text(&content, "SCOPE").unwrap_or_default(),
            depends: tag_text(&content, "DEPENDS")
                .map(|depends| split_items(&depends))
                .unwrap_or_default(),
        });
    }

    let contracts = ContractValidator::validate_project_with_profile(root, GraceProfile::Lite)?;
    if let Some(contract) = contracts
        .contracts
        .iter()
        .find(|contract| contract.module_id.as_deref() == Some(module_id))
    {
        return Ok(ModuleInfo {
            purpose: contract.purpose.clone().unwrap_or_default(),
            scope: contract.scope.clone().unwrap_or_default(),
            depends: contract.depends.clone(),
        });
    }

    anyhow::bail!(
        "module artifact or source contract not found for {}",
        module_id
    )
}

fn scaffold_belief_state(module_id: &str, context: &str, info: &ModuleInfo) -> BeliefState {
    let purpose = fallback(&info.purpose, "Module purpose is not declared in artifacts");
    let scope = fallback(&info.scope, "Module scope is not declared in artifacts");
    let depends = if info.depends.is_empty() {
        "No declared dependencies".to_string()
    } else {
        info.depends.join(", ")
    };
    let mut state = BeliefState {
        module_id: module_id.to_string(),
        version: "1.0".into(),
        understanding: BeliefUnderstanding {
            responsibility: purpose.clone(),
            key_data_flows: vec![
                DataFlowBelief {
                    direction: "INPUT".into(),
                    description: format!(
                        "Use MODULE_CONTRACT, sharded docs, and execution context: {}",
                        fallback(context, "no extra context provided")
                    ),
                },
                DataFlowBelief {
                    direction: "PROCESSING".into(),
                    description: format!(
                        "Implement only the declared scope while preserving dependencies: {}",
                        depends
                    ),
                },
                DataFlowBelief {
                    direction: "OUTPUT".into(),
                    description: "Updated code/artifacts that verify against MyGRACE gates".into(),
                },
            ],
            critical_invariants: vec![
                format!("Scope remains bounded to: {}", scope),
                "MODULE_CONTRACT, MODULE_MAP, CHANGE_SUMMARY, and semantic anchors stay synchronized"
                    .into(),
            ],
        },
        implementation_strategy: BeliefStrategy {
            approach:
                "Read sharded artifacts first, preserve public contracts, make scoped edits, then run verification gates"
                    .into(),
            complexity_areas: vec![
                "artifact drift".into(),
                "backward compatibility".into(),
                "verification coverage".into(),
            ],
            patterns_applied: vec![
                "explicit data flow".into(),
                "typed semantic links".into(),
                "bounded semantic blocks".into(),
            ],
        },
        verification_intent: vec![
            "belief-state-exists reports no malformed BELIEF_STATE blocks".into(),
            "verify_project, review_code, and relevant unit tests pass after implementation".into(),
        ],
        risks_acknowledged: vec![
            "This scaffold is generated from artifacts and should be refined by the implementing agent before substantial code generation"
                .into(),
            "Incomplete module artifacts can make the first belief state too generic".into(),
        ],
        file_path: None,
        valid: true,
        errors: Vec::new(),
    };
    validate_belief_state(&mut state);
    state
}

fn parse_attrs(open_line: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let Ok(re) = regex::Regex::new(r#"([A-Za-z_][A-Za-z0-9_-]*)="([^"]*)""#) else {
        return attrs;
    };
    for cap in re.captures_iter(open_line) {
        attrs.insert(cap[1].to_string(), unescape_text(&cap[2]));
    }
    attrs
}

fn tag_text(content: &str, tag: &str) -> Option<String> {
    let pattern = format!(
        r"(?s)<{}>(.*?)</{}>",
        regex::escape(tag),
        regex::escape(tag)
    );
    let re = regex::Regex::new(&pattern).ok()?;
    re.captures(content)
        .and_then(|cap| cap.get(1).map(|value| unescape_text(value.as_str().trim())))
}

fn split_items(value: &str) -> Vec<String> {
    value
        .split([',', ';'])
        .map(|item| unescape_text(item.trim()))
        .filter(|item| !item.is_empty())
        .collect()
}

fn join_items(values: &[String]) -> String {
    values
        .iter()
        .map(|value| escape_text(value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn normalize_belief_line(line: &str) -> String {
    let trimmed = line.trim();
    let stripped = if let Some(rest) = trimmed.strip_prefix("//") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix('#') {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("--") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("/*") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix('*') {
        rest
    } else {
        trimmed
    };
    stripped.trim().trim_end_matches("*/").trim().to_string()
}

fn escape_attr(value: &str) -> String {
    escape_text(value).replace('"', "&quot;")
}

fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn unescape_text(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}

fn fallback(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_parse_valid_belief_state
    // PURPOSE: Verify parser extracts all required BELIEF_STATE fields
    // OUTPUTS: { () }
    // START_test_parse_valid_belief_state
    #[test]
    fn test_parse_valid_belief_state() {
        let content = concat!(
            "// <BELIEF_STATE module=\"M-ORDER\" version=\"1.0\">\n",
            "//   UNDERSTANDING:\n",
            "//     Module responsibility: Place orders safely\n",
            "//     Key data flows:\n",
            "//       - INPUT: order request\n",
            "//       - PROCESSING: validate stock and payment\n",
            "//       - OUTPUT: persisted order\n",
            "//     Critical invariants:\n",
            "//       - inventory is not decremented on payment failure\n",
            "//   IMPLEMENTATION_STRATEGY:\n",
            "//     Approach: Use explicit validation before writes\n",
            "//     Complexity_areas: stock race, payment timeout\n",
            "//     Patterns_applied: explicit flow, no magic values\n",
            "//   VERIFICATION_INTENT:\n",
            "//     I expect the following to be true after my code runs:\n",
            "//     - invalid stock cancels order\n",
            "//   RISKS_ACKNOWLEDGED:\n",
            "//     - concurrent orders can race on stock\n",
            "// </BELIEF_STATE>\n",
        );
        let states = parse_belief_states(content);
        assert_eq!(states.len(), 1);
        assert!(states[0].valid, "{:?}", states[0].errors);
        assert_eq!(states[0].module_id, "M-ORDER");
        assert_eq!(states[0].understanding.key_data_flows.len(), 3);
    }
    // END_test_parse_valid_belief_state

    // START_CONTRACT_test_parse_belief_state_missing_required_field
    // PURPOSE: Verify validator reports missing required fields
    // OUTPUTS: { () }
    // START_test_parse_belief_state_missing_required_field
    #[test]
    fn test_parse_belief_state_missing_required_field() {
        let content = concat!(
            "<BELIEF_STATE module=\"M-BAD\" version=\"1.0\">\n",
            "  UNDERSTANDING:\n",
            "    Module responsibility: Missing flows\n",
            "</BELIEF_STATE>\n",
        );
        let states = parse_belief_states(content);
        assert_eq!(states.len(), 1);
        assert!(!states[0].valid);
        assert!(states[0].errors.iter().any(|error| error.contains("INPUT")));
    }
    // END_test_parse_belief_state_missing_required_field

    // START_CONTRACT_test_extract_and_store_belief_state_writes_artifact
    // PURPOSE: Verify deterministic extraction stores a valid docs/belief-states artifact
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temp docs artifacts
    // START_test_extract_and_store_belief_state_writes_artifact
    #[test]
    fn test_extract_and_store_belief_state_writes_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = DocsLayout::new(dir.path());
        std::fs::create_dir_all(layout.modules_dir()).expect("modules dir");
        std::fs::write(
            layout.modules_dir().join("M-SAMPLE.xml"),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
                "<MODULE id=\"M-SAMPLE\" type=\"CORE_LOGIC\" status=\"active\">\n",
                "  <PURPOSE>Sample module purpose</PURPOSE>\n",
                "  <SCOPE>Sample module scope</SCOPE>\n",
                "  <DEPENDS>M-OTHER</DEPENDS>\n",
                "</MODULE>\n",
            ),
        )
        .expect("write module shard");

        let result = extract_and_store_belief_state(
            dir.path(),
            "M-SAMPLE",
            "Read MODULE_CONTRACT + DevelopmentPlan",
        )
        .expect("extract belief state");
        assert!(result.valid, "{:?}", result.errors);
        assert!(dir.path().join("docs/belief-states/M-SAMPLE.xml").exists());
        let saved =
            std::fs::read_to_string(dir.path().join("docs/belief-states/M-SAMPLE.xml")).unwrap();
        let parsed = parse_belief_states(&saved);
        assert_eq!(parsed.len(), 1);
        assert!(parsed[0].valid);
    }
    // END_test_extract_and_store_belief_state_writes_artifact
}
