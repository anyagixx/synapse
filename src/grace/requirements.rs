// MODULE_CONTRACT
// MODULE_ID: M-GRACE-REQUIREMENTS
// PURPOSE: RequirementsAnalysis parser, validator, and generator for GRACE Stage 1 AAG artifacts
// SCOPE: RequirementsReport, RequirementsEntity, RequirementsUseCase, template generation, file validation, docs/requirements.xml writer
// DEPENDS: N/A
// LINKS:
//   → V-M-GRACE-REQUIREMENTS (verified_by) — requirements parser, validator, and generator tests

// START_MODULE_MAP
// RequirementsReport — Completeness report for docs/requirements.xml
// requirements_template — Full RequirementsAnalysis XML template with AAG sections
// validate_requirements — Parse and validate docs/requirements.xml
// generate_requirements_file — Generate and validate docs/requirements.xml from MCP inputs
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Added full RequirementsAnalysis parser, validator, and generator]
// END_CHANGE_SUMMARY

use std::path::Path;

// START_public_api

// START_RequirementsEntity
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct RequirementsEntity {
    pub name: String,
    pub attributes: usize,
}
// END_RequirementsEntity

// START_RequirementsUseCase
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct RequirementsUseCase {
    pub id: String,
    pub has_actor: bool,
    pub has_action: bool,
    pub has_goal: bool,
}
// END_RequirementsUseCase

// START_RequirementsReport
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct RequirementsReport {
    pub path: String,
    pub exists: bool,
    pub valid: bool,
    pub goals: usize,
    pub entities: Vec<RequirementsEntity>,
    pub actors: usize,
    pub use_cases: Vec<RequirementsUseCase>,
    pub non_functional_requirements: usize,
    pub constraints: usize,
    pub glossary_terms: Vec<String>,
    pub errors: Vec<String>,
}
// END_RequirementsReport

impl RequirementsReport {
    // START_CONTRACT_RequirementsReport::has_entities_defined
    // PURPOSE: Return true when at least one domain entity has attributes
    // OUTPUTS: { bool }
    // START_requirements_report_has_entities_defined
    pub fn has_entities_defined(&self) -> bool {
        self.entities.iter().any(|entity| entity.attributes > 0)
    }
    // END_requirements_report_has_entities_defined

    // START_CONTRACT_RequirementsReport::has_use_cases
    // PURPOSE: Return true when every discovered use case has Actor, Action, and Goal
    // OUTPUTS: { bool }
    // START_requirements_report_has_use_cases
    pub fn has_use_cases(&self) -> bool {
        !self.use_cases.is_empty()
            && self
                .use_cases
                .iter()
                .all(|uc| uc.has_actor && uc.has_action && uc.has_goal)
    }
    // END_requirements_report_has_use_cases

    // START_CONTRACT_RequirementsReport::has_glossary
    // PURPOSE: Return true when glossary terms exist and include all domain entity names
    // OUTPUTS: { bool }
    // START_requirements_report_has_glossary
    pub fn has_glossary(&self) -> bool {
        !self.glossary_terms.is_empty()
            && self.entities.iter().all(|entity| {
                self.glossary_terms
                    .iter()
                    .any(|term| term.eq_ignore_ascii_case(&entity.name))
            })
    }
    // END_requirements_report_has_glossary

    // START_CONTRACT_RequirementsReport::has_no_empty_sections
    // PURPOSE: Return true when every required RequirementsAnalysis section is populated
    // OUTPUTS: { bool }
    // START_requirements_report_has_no_empty_sections
    pub fn has_no_empty_sections(&self) -> bool {
        self.goals > 0
            && self.has_entities_defined()
            && self.actors > 0
            && self.has_use_cases()
            && self.non_functional_requirements > 0
            && self.constraints > 0
            && self.has_glossary()
    }
    // END_requirements_report_has_no_empty_sections
}

// START_CONTRACT_requirements_template
// PURPOSE: Build a complete RequirementsAnalysis XML template with AAG notation
// INPUTS: { project_name: &str }, { project_description: &str }, { domain: &str }, { detail_level: &str }
// OUTPUTS: { String }
// START_requirements_template
pub fn requirements_template(
    project_name: &str,
    project_description: &str,
    domain: &str,
    detail_level: &str,
) -> String {
    let project_name = xml_text(&fallback(project_name, "MyProject"));
    let project_description = xml_text(&fallback(
        project_description,
        "Describe the primary product outcome",
    ));
    let domain = xml_text(&fallback(domain, "application domain"));
    let goal_detail = if detail_level == "detailed" {
        "including traceable workflows, explicit constraints, and measurable acceptance criteria"
    } else {
        "with traceable workflows and measurable acceptance criteria"
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<RequirementsAnalysis project="{project_name}" version="1.0">
  <Goals>
    <Goal id="G-001" priority="P0">
      <Description>{project_description} for the {domain} domain</Description>
      <SuccessCriteria>Primary user can complete the core workflow {goal_detail}</SuccessCriteria>
    </Goal>
  </Goals>
  <DomainModel>
    <Entity name="CoreRecord">
      <Description>Primary domain record managed by the system</Description>
      <Attributes>
        <Attribute name="id" type="UUID" required="true">Stable record identifier</Attribute>
        <Attribute name="status" type="String" required="true">Lifecycle status</Attribute>
        <Attribute name="createdAt" type="DateTime" required="true">Creation timestamp</Attribute>
      </Attributes>
      <Relationships>
        <Relationship target="User" type="N:1" direction="owned_by" />
      </Relationships>
    </Entity>
    <Entity name="User">
      <Description>Actor account interacting with the system</Description>
      <Attributes>
        <Attribute name="id" type="UUID" required="true">Stable user identifier</Attribute>
        <Attribute name="role" type="String" required="true">Permission role</Attribute>
      </Attributes>
      <Relationships>
        <Relationship target="CoreRecord" type="1:N" direction="owns" />
      </Relationships>
    </Entity>
  </DomainModel>
  <Actors>
    <Actor name="PrimaryUser">
      <Description>User who performs the main product workflow</Description>
      <Capabilities>Create records, review status, complete core actions</Capabilities>
    </Actor>
    <Actor name="Admin">
      <Description>Operator responsible for configuration and oversight</Description>
      <Capabilities>Manage users, audit records, resolve exceptions</Capabilities>
    </Actor>
  </Actors>
  <UseCases>
    <UseCase id="UC-001" priority="P0">
      <Actor>PrimaryUser</Actor>
      <Action>Complete core workflow</Action>
      <Goal>Achieve the primary project outcome reliably</Goal>
      <Preconditions>
        - PrimaryUser has access
        - Required input data is available
      </Preconditions>
      <Postconditions>
        - CoreRecord status is updated
        - Result is visible to the user
        - Audit trail is recorded
      </Postconditions>
      <MainFlow>
        <Step n="1">PrimaryUser starts the workflow</Step>
        <Step n="2">System validates input data</Step>
        <Step n="3">System applies domain rules</Step>
        <Step n="4">System persists the CoreRecord change</Step>
        <Step n="5">System returns confirmation</Step>
      </MainFlow>
      <AlternativeFlows>
        <Flow id="AF-001" trigger="Invalid input">
          <Step n="1">System rejects the request</Step>
          <Step n="2">System returns actionable validation errors</Step>
        </Flow>
      </AlternativeFlows>
    </UseCase>
  </UseCases>
  <NonFunctionalRequirements>
    <Requirement id="NFR-001" category="performance">
      <Description>Core workflow completes within 2 seconds at p95</Description>
    </Requirement>
    <Requirement id="NFR-002" category="reliability">
      <Description>System preserves data consistency across failed operations</Description>
    </Requirement>
  </NonFunctionalRequirements>
  <Constraints>
    <Constraint id="CON-001" type="business">
      <Description>Only authorized actors may modify CoreRecord state</Description>
    </Constraint>
    <Constraint id="CON-002" type="technical">
      <Description>All persistent state changes must be verifiable by tests or logs</Description>
    </Constraint>
  </Constraints>
  <Glossary>
    <Term name="CoreRecord">Primary domain record managed by the system</Term>
    <Term name="User">Actor account interacting with the system</Term>
    <Term name="PrimaryUser">Actor who completes the main workflow</Term>
  </Glossary>
</RequirementsAnalysis>
"#
    )
}
// END_requirements_template

// START_CONTRACT_validate_requirements
// PURPOSE: Validate docs/requirements.xml as a complete RequirementsAnalysis artifact
// INPUTS: { root: &Path — project root }
// OUTPUTS: { anyhow::Result<RequirementsReport> }
// START_validate_requirements
pub fn validate_requirements(root: &Path) -> anyhow::Result<RequirementsReport> {
    let path = root.join("docs").join("requirements.xml");
    if !path.exists() {
        return Ok(RequirementsReport {
            path: path.display().to_string(),
            exists: false,
            valid: false,
            errors: vec!["Missing docs/requirements.xml".into()],
            ..RequirementsReport::default()
        });
    }
    let content = std::fs::read_to_string(&path)?;
    let mut report = parse_requirements_content(&content);
    report.path = path.display().to_string();
    report.exists = true;
    Ok(report)
}
// END_validate_requirements

// START_CONTRACT_parse_requirements_content
// PURPOSE: Parse RequirementsAnalysis XML content into a completeness report
// INPUTS: { content: &str }
// OUTPUTS: { RequirementsReport }
// START_parse_requirements_content
pub fn parse_requirements_content(content: &str) -> RequirementsReport {
    let goals_section = section_content(content, "Goals");
    let domain_section = section_content(content, "DomainModel");
    let actors_section = section_content(content, "Actors");
    let use_cases_section = section_content(content, "UseCases");
    let nfr_section = section_content(content, "NonFunctionalRequirements");
    let constraints_section = section_content(content, "Constraints");
    let glossary_section = section_content(content, "Glossary");
    let mut report = RequirementsReport {
        exists: true,
        goals: tag_count(&goals_section, "Goal"),
        actors: tag_count(&actors_section, "Actor"),
        non_functional_requirements: tag_count(&nfr_section, "Requirement"),
        constraints: tag_count(&constraints_section, "Constraint"),
        entities: extract_entities(&domain_section),
        use_cases: extract_use_cases(&use_cases_section),
        glossary_terms: extract_named_tags(&glossary_section, "Term"),
        ..RequirementsReport::default()
    };
    validate_report_shape(content, &mut report);
    report.valid = report.errors.is_empty();
    report
}
// END_parse_requirements_content

// START_CONTRACT_generate_requirements_file
// PURPOSE: Generate docs/requirements.xml from a project description and validate the result
// INPUTS: { root: &Path }, { project_description: &str }, { domain: &str }, { detail_level: &str }
// OUTPUTS: { anyhow::Result<RequirementsReport> }
// SIDE_EFFECTS: writes docs/requirements.xml
// START_generate_requirements_file
pub fn generate_requirements_file(
    root: &Path,
    project_description: &str,
    domain: &str,
    detail_level: &str,
) -> anyhow::Result<RequirementsReport> {
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs)?;
    let project_name = project_name_from_description(project_description);
    let content = requirements_template(&project_name, project_description, domain, detail_level);
    std::fs::write(docs.join("requirements.xml"), content)?;
    validate_requirements(root)
}
// END_generate_requirements_file

// END_public_api

fn validate_report_shape(content: &str, report: &mut RequirementsReport) {
    if !content.contains("<RequirementsAnalysis") {
        report
            .errors
            .push("Root tag must be RequirementsAnalysis".into());
    }
    if report.goals == 0 {
        report
            .errors
            .push("Goals section has no Goal entries".into());
    }
    if !report.has_entities_defined() {
        report
            .errors
            .push("DomainModel must define at least one Entity with Attribute entries".into());
    }
    if report.actors == 0 {
        report
            .errors
            .push("Actors section has no Actor entries".into());
    }
    if !report.has_use_cases() {
        report
            .errors
            .push("UseCases must include Actor, Action, and Goal for each use case".into());
    }
    if report.non_functional_requirements == 0 {
        report
            .errors
            .push("NonFunctionalRequirements section has no Requirement entries".into());
    }
    if report.constraints == 0 {
        report
            .errors
            .push("Constraints section has no Constraint entries".into());
    }
    if !report.has_glossary() {
        report
            .errors
            .push("Glossary must include terms matching every domain Entity name".into());
    }
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

fn extract_entities(content: &str) -> Vec<RequirementsEntity> {
    let Ok(re) = regex::Regex::new(r#"(?s)<Entity\s+name="([^"]+)"[^>]*>(.*?)</Entity>"#) else {
        return Vec::new();
    };
    re.captures_iter(content)
        .map(|cap| RequirementsEntity {
            name: cap[1].to_string(),
            attributes: tag_count(&cap[2], "Attribute"),
        })
        .collect()
}

fn extract_use_cases(content: &str) -> Vec<RequirementsUseCase> {
    let Ok(re) = regex::Regex::new(r#"(?s)<UseCase\s+id="([^"]+)"[^>]*>(.*?)</UseCase>"#) else {
        return Vec::new();
    };
    re.captures_iter(content)
        .map(|cap| {
            let body = &cap[2];
            RequirementsUseCase {
                id: cap[1].to_string(),
                has_actor: has_non_empty_tag(body, "Actor"),
                has_action: has_non_empty_tag(body, "Action"),
                has_goal: has_non_empty_tag(body, "Goal"),
            }
        })
        .collect()
}

fn extract_named_tags(content: &str, tag: &str) -> Vec<String> {
    let pattern = format!(r#"<{}\s+name="([^"]+)""#, regex::escape(tag));
    let Ok(re) = regex::Regex::new(&pattern) else {
        return Vec::new();
    };
    re.captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect()
}

fn tag_count(content: &str, tag: &str) -> usize {
    let pattern = format!(r#"<{}\b"#, regex::escape(tag));
    regex::Regex::new(&pattern)
        .map(|re| re.find_iter(content).count())
        .unwrap_or(0)
}

fn has_non_empty_tag(content: &str, tag: &str) -> bool {
    let pattern = format!(
        r"(?s)<{}>(.*?)</{}>",
        regex::escape(tag),
        regex::escape(tag)
    );
    regex::Regex::new(&pattern)
        .ok()
        .and_then(|re| re.captures(content))
        .and_then(|cap| cap.get(1))
        .is_some_and(|value| !value.as_str().trim().is_empty())
}

fn project_name_from_description(description: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in description.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize {
                out.push(ch.to_ascii_uppercase());
                capitalize = false;
            } else {
                out.push(ch);
            }
        } else {
            capitalize = true;
        }
        if out.len() >= 32 {
            break;
        }
    }
    fallback(&out, "GeneratedProject")
}

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

    // START_CONTRACT_test_requirements_template_is_complete
    // PURPOSE: Verify generated requirements include every required Stage 1 section
    // OUTPUTS: { () }
    // START_test_requirements_template_is_complete
    #[test]
    fn test_requirements_template_is_complete() {
        let xml = requirements_template("Shop", "Retail checkout", "retail", "standard");
        let report = parse_requirements_content(&xml);
        assert!(report.valid, "{:?}", report.errors);
        assert!(report.has_entities_defined());
        assert!(report.has_use_cases());
        assert!(report.has_glossary());
        assert!(report.has_no_empty_sections());
    }
    // END_test_requirements_template_is_complete

    // START_CONTRACT_test_incomplete_requirements_reports_errors
    // PURPOSE: Verify legacy stub requirements fail completeness validation
    // OUTPUTS: { () }
    // START_test_incomplete_requirements_reports_errors
    #[test]
    fn test_incomplete_requirements_reports_errors() {
        let report =
            parse_requirements_content("<REQUIREMENTS><NonGoals></NonGoals></REQUIREMENTS>");
        assert!(!report.valid);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("RequirementsAnalysis")));
    }
    // END_test_incomplete_requirements_reports_errors

    // START_CONTRACT_test_generate_requirements_file_writes_valid_artifact
    // PURPOSE: Verify generator writes docs/requirements.xml and validates it
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temp docs/requirements.xml
    // START_test_generate_requirements_file_writes_valid_artifact
    #[test]
    fn test_generate_requirements_file_writes_valid_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        let report = generate_requirements_file(
            dir.path(),
            "Developer automation platform",
            "developer tooling",
            "standard",
        )
        .expect("generate requirements");
        assert!(report.valid, "{:?}", report.errors);
        assert!(dir.path().join("docs/requirements.xml").exists());
    }
    // END_test_generate_requirements_file_writes_valid_artifact
}
// END_public_api
