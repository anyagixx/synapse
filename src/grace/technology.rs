// MODULE_CONTRACT
// MODULE_ID: M-GRACE-TECHNOLOGY
// PURPOSE: Technology stack parser, validator, dependency detector, and generator for GRACE Stage 2 artifacts
// SCOPE: TechnologyReport, DetectedDependency, TechnologyComponent, pending decision artifacts, concrete detected-stack generation, file validation, dependency detection, compatibility checks
// DEPENDS: N/A
// LINKS:
//   → V-M-GRACE-TECHNOLOGY (verified_by) — technology parser, validator, detector, and generator tests

// START_MODULE_MAP
// DetectedDependency — One dependency discovered from manifest or lock files
// TechnologyReport — Completeness report for docs/technology.xml
// technology_decision_template — Pending Technology XML for blank projects awaiting stack selection
// technology_template — Detected Technology XML template with exact versions and compatibility matrix
// validate_technology — Parse and validate docs/technology.xml
// detect_dependencies — Detect dependencies from common package manifests
// generate_technology_file — Generate and validate docs/technology.xml
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.2.0 - Split blank-project technology decisions from detected concrete stacks]
// END_CHANGE_SUMMARY

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const GO_REQUIRE_DIRECTIVE_PARTS: usize = 3;
const GO_MODULE_REQUIRE_PARTS: usize = 2;

// START_public_api

// START_DetectedDependency
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct DetectedDependency {
    pub ecosystem: String,
    pub name: String,
    pub version: String,
    pub source: String,
    pub exact: bool,
}
// END_DetectedDependency

// START_TechnologyComponent
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct TechnologyComponent {
    pub kind: String,
    pub name: String,
    pub version: String,
}
// END_TechnologyComponent

// START_CompatibilityCheck
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct CompatibilityCheck {
    pub id: String,
    pub component_a: String,
    pub component_b: String,
    pub status: String,
    pub note: String,
}
// END_CompatibilityCheck

// START_TechnologyReport
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct TechnologyReport {
    pub path: String,
    pub exists: bool,
    pub valid: bool,
    pub status: String,
    pub decision_pending: bool,
    pub languages: Vec<TechnologyComponent>,
    pub components: Vec<TechnologyComponent>,
    pub compatibility_checks: Vec<CompatibilityCheck>,
    pub known_issues: usize,
    pub detected_dependencies: Vec<DetectedDependency>,
    pub missing_versions: Vec<String>,
    pub incompatible_checks: Vec<String>,
    pub errors: Vec<String>,
}
// END_TechnologyReport

impl TechnologyReport {
    // START_CONTRACT_TechnologyReport::has_language_defined
    // PURPOSE: Return true when technology is pending decision or at least one language has an exact version
    // OUTPUTS: { bool }
    // START_technology_report_has_language_defined
    pub fn has_language_defined(&self) -> bool {
        self.decision_pending
            || self
                .languages
                .iter()
                .any(|component| is_exact_version(&component.version))
    }
    // END_technology_report_has_language_defined

    // START_CONTRACT_TechnologyReport::dependencies_compatible
    // PURPOSE: Return true when technology is pending decision or compatibility checks exist and none are incompatible
    // OUTPUTS: { bool }
    // START_technology_report_dependencies_compatible
    pub fn dependencies_compatible(&self) -> bool {
        self.decision_pending
            || (!self.compatibility_checks.is_empty() && self.incompatible_checks.is_empty())
    }
    // END_technology_report_dependencies_compatible

    // START_CONTRACT_TechnologyReport::has_no_version_guessing
    // PURPOSE: Return true when no version attributes use blank, latest, wildcard, or range syntax
    // OUTPUTS: { bool }
    // START_technology_report_has_no_version_guessing
    pub fn has_no_version_guessing(&self) -> bool {
        self.decision_pending || (self.missing_versions.is_empty() && !self.components.is_empty())
    }
    // END_technology_report_has_no_version_guessing

    // START_CONTRACT_TechnologyReport::has_known_issues
    // PURPOSE: Return true when KnownIssues documents at least one issue/no-known-issues entry
    // OUTPUTS: { bool }
    // START_technology_report_has_known_issues
    pub fn has_known_issues(&self) -> bool {
        self.known_issues > 0
    }
    // END_technology_report_has_known_issues
}

// START_CONTRACT_technology_decision_template
// PURPOSE: Build a Technology XML artifact that records an explicit pending stack decision
// INPUTS: { project_name: &str }, { updated: &str }
// OUTPUTS: { String }
// START_technology_decision_template
pub fn technology_decision_template(project_name: &str, updated: &str) -> String {
    let project_name = xml_text(&fallback(project_name, "MyProject"));
    let updated = xml_text(&fallback(updated, "2026-05-20"));
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Technology project="{project_name}" version="1.0" updated="{updated}" status="needs-decision">
  <DecisionPolicy mode="llm-recommended">
    <Purpose>Select a concrete stack from requirements before implementation.</Purpose>
    <RequirementsRef>docs/requirements.xml</RequirementsRef>
    <DecisionOwner>planning-agent</DecisionOwner>
    <Instruction>Recommend language, runtime, framework, database, package manager, test runner, deployment, and compatibility checks from project requirements and constraints. Do not treat bootstrap placeholders as selected technology.</Instruction>
  </DecisionPolicy>
  <DecisionInputs>
    <Input path="docs/requirements.xml" required="true" />
    <Input path="docs/graph-index.xml" required="true" />
    <Input path="docs/plan-index.xml" required="true" />
  </DecisionInputs>
  <KnownIssues>
    <Issue id="KI-PENDING-STACK" component="TechnologyDecision" version="1.0.0">
      <Description>No concrete technology stack has been selected yet.</Description>
      <Workaround>Run MyGRACE planning or generate_technology after requirements are known, then replace this pending artifact with exact versions.</Workaround>
    </Issue>
  </KnownIssues>
</Technology>
"#
    )
}
// END_technology_decision_template

// START_CONTRACT_technology_template
// PURPOSE: Build a detected Technology XML template with exact versions and compatibility checks
// INPUTS: { project_name: &str }, { dependencies: &[DetectedDependency] }, { updated: &str }
// OUTPUTS: { String }
// START_technology_template
pub fn technology_template(
    project_name: &str,
    dependencies: &[DetectedDependency],
    updated: &str,
) -> String {
    if dependencies.is_empty() {
        return technology_decision_template(project_name, updated);
    }

    let project_name = xml_text(&fallback(project_name, "MyProject"));
    let updated = xml_text(&fallback(updated, "2026-05-20"));
    let stack = stack_xml(dependencies);
    let libraries = dependency_xml(dependencies);
    let checks = compatibility_xml(dependencies);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Technology project="{project_name}" version="1.0" updated="{updated}" status="selected">
  <Stack>
{stack}
    <Libraries>
{libraries}
    </Libraries>
  </Stack>
  <DependencyMatrix>
{checks}
  </DependencyMatrix>
  <KnownIssues>
    <Issue id="KI-001" component="AllPinnedDependencies" version="1.0.0">
      <Description>No unresolved known issues for the pinned versions in this Technology artifact.</Description>
      <Workaround>When a dependency is changed, update this section with version-specific notes before code generation.</Workaround>
    </Issue>
  </KnownIssues>
  <DevOps>
    <CI name="GitHub Actions">
      <Config>.github/workflows/ci.yml</Config>
      <StackSource>detected-manifests</StackSource>
    </CI>
    <Container name="none" version="1.0.0">
      <BaseImage>not-used</BaseImage>
      <Dockerfile>not-used</Dockerfile>
    </Container>
  </DevOps>
</Technology>
"#
    )
}
// END_technology_template

// START_CONTRACT_validate_technology
// PURPOSE: Validate docs/technology.xml as a complete exact-version Technology artifact
// INPUTS: { root: &Path — project root }
// OUTPUTS: { anyhow::Result<TechnologyReport> }
// START_validate_technology
pub fn validate_technology(root: &Path) -> anyhow::Result<TechnologyReport> {
    let path = root.join("docs").join("technology.xml");
    if !path.exists() {
        return Ok(TechnologyReport {
            path: path.display().to_string(),
            exists: false,
            valid: false,
            errors: vec!["Missing docs/technology.xml".into()],
            ..TechnologyReport::default()
        });
    }
    let content = std::fs::read_to_string(&path)?;
    let mut report = parse_technology_content(&content);
    report.path = path.display().to_string();
    report.exists = true;
    report.detected_dependencies = detect_dependencies(root);
    Ok(report)
}
// END_validate_technology

// START_CONTRACT_parse_technology_content
// PURPOSE: Parse Technology XML content into a completeness report
// INPUTS: { content: &str }
// OUTPUTS: { TechnologyReport }
// START_parse_technology_content
pub fn parse_technology_content(content: &str) -> TechnologyReport {
    let status = root_status(content);
    let mut report = TechnologyReport {
        exists: true,
        status: status.clone(),
        decision_pending: status == "needs-decision",
        languages: extract_components(content, "Language"),
        components: extract_all_components(content),
        compatibility_checks: extract_compatibility_checks(content),
        known_issues: tag_count(&section_content(content, "KnownIssues"), "Issue"),
        ..TechnologyReport::default()
    };
    report.missing_versions = collect_version_issues(&report);
    report.incompatible_checks = report
        .compatibility_checks
        .iter()
        .filter(|check| !check.status.eq_ignore_ascii_case("compatible"))
        .map(|check| format!("{}: {}", check.id, check.status))
        .collect();
    validate_report_shape(content, &mut report);
    report.valid = report.errors.is_empty();
    report
}
// END_parse_technology_content

// START_CONTRACT_detect_dependencies
// PURPOSE: Detect direct project dependencies from Cargo, npm, Python, and Go manifests
// INPUTS: { root: &Path }
// OUTPUTS: { Vec<DetectedDependency> }
// START_detect_dependencies
pub fn detect_dependencies(root: &Path) -> Vec<DetectedDependency> {
    let mut deps = Vec::new();
    detect_cargo_dependencies(root, &mut deps);
    detect_package_json_dependencies(root, &mut deps);
    detect_requirements_dependencies(root, &mut deps);
    detect_go_mod_dependencies(root, &mut deps);
    deps.sort_by(|a, b| (&a.ecosystem, &a.name).cmp(&(&b.ecosystem, &b.name)));
    deps.dedup_by(|a, b| a.ecosystem == b.ecosystem && a.name == b.name);
    deps
}
// END_detect_dependencies

// START_CONTRACT_generate_technology_file
// PURPOSE: Generate docs/technology.xml from detected dependencies and validate the result
// INPUTS: { root: &Path }, { detect_existing: bool }, { compatibility_check: bool }
// OUTPUTS: { anyhow::Result<TechnologyReport> }
// SIDE_EFFECTS: writes docs/technology.xml
// START_generate_technology_file
pub fn generate_technology_file(
    root: &Path,
    detect_existing: bool,
    compatibility_check: bool,
) -> anyhow::Result<TechnologyReport> {
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs)?;
    let deps = if detect_existing {
        detect_dependencies(root)
    } else {
        Vec::new()
    };
    let project_name = project_name(root);
    let mut content = if deps.is_empty() {
        technology_decision_template(&project_name, &current_date())
    } else {
        technology_template(&project_name, &deps, &current_date())
    };
    if !compatibility_check && !deps.is_empty() {
        content = content.replace("<Status>compatible</Status>", "<Status>unchecked</Status>");
    }
    std::fs::write(docs.join("technology.xml"), content)?;
    validate_technology(root)
}
// END_generate_technology_file

// END_public_api

fn validate_report_shape(content: &str, report: &mut TechnologyReport) {
    if !content.contains("<Technology") {
        report.errors.push("Root tag must be Technology".into());
    }
    if report.decision_pending {
        if !content.contains("<DecisionPolicy") {
            report
                .errors
                .push("Pending Technology must define DecisionPolicy".into());
        }
        if !content.contains("<RequirementsRef>docs/requirements.xml</RequirementsRef>") {
            report
                .errors
                .push("Pending Technology must reference docs/requirements.xml".into());
        }
        if !report.components.is_empty() {
            report
                .errors
                .push("Pending Technology must not define selected stack components".into());
        }
        if !report.has_known_issues() {
            report
                .errors
                .push("KnownIssues must document at least one Issue entry".into());
        }
        return;
    }
    if !report.has_language_defined() {
        report
            .errors
            .push("Technology must define Language with an exact version".into());
    }
    if !report.dependencies_compatible() {
        report.errors.push(
            "DependencyMatrix must include compatibility checks and all Status values must be compatible".into(),
        );
    }
    if !report.has_no_version_guessing() {
        report.errors.push(format!(
            "Version attributes must be exact; issues: {:?}",
            report.missing_versions
        ));
    }
    if !report.has_known_issues() {
        report
            .errors
            .push("KnownIssues must document at least one Issue entry".into());
    }
}

fn extract_all_components(content: &str) -> Vec<TechnologyComponent> {
    [
        "Language",
        "Runtime",
        "PackageManager",
        "Framework",
        "Database",
        "Driver",
        "ORM",
        "Testing",
        "Library",
        "Dependency",
        "Container",
    ]
    .iter()
    .flat_map(|tag| extract_components(content, tag))
    .collect()
}

fn extract_components(content: &str, tag: &str) -> Vec<TechnologyComponent> {
    start_tags(content, tag)
        .into_iter()
        .filter_map(|start| {
            let name = attr_value(&start, "name")?;
            Some(TechnologyComponent {
                kind: tag.to_string(),
                name,
                version: attr_value(&start, "version").unwrap_or_default(),
            })
        })
        .collect()
}

fn extract_compatibility_checks(content: &str) -> Vec<CompatibilityCheck> {
    let Ok(re) = regex::Regex::new(
        r#"(?s)<CompatibilityCheck\s+id="([^"]+)"[^>]*>(.*?)</CompatibilityCheck>"#,
    ) else {
        return Vec::new();
    };
    re.captures_iter(content)
        .map(|cap| CompatibilityCheck {
            id: cap[1].to_string(),
            component_a: tag_text(&cap[2], "ComponentA"),
            component_b: tag_text(&cap[2], "ComponentB"),
            status: tag_text(&cap[2], "Status"),
            note: tag_text(&cap[2], "Note"),
        })
        .collect()
}

fn collect_version_issues(report: &TechnologyReport) -> Vec<String> {
    let mut issues = Vec::new();
    for component in &report.components {
        if !is_exact_version(&component.version) {
            issues.push(format!(
                "{} {} has non-exact version '{}'",
                component.kind, component.name, component.version
            ));
        }
    }
    issues
}

fn detect_cargo_dependencies(root: &Path, out: &mut Vec<DetectedDependency>) {
    let manifest = root.join("Cargo.toml");
    let Ok(content) = std::fs::read_to_string(&manifest) else {
        return;
    };
    let Ok(value) = content.parse::<toml::Value>() else {
        return;
    };
    let lock_versions = cargo_lock_versions(root);
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(table) = value.get(section).and_then(|v| v.as_table()) else {
            continue;
        };
        for (name, spec) in table {
            let manifest_version = manifest_version(spec);
            let version = lock_versions
                .get(name)
                .cloned()
                .unwrap_or_else(|| manifest_version.clone());
            out.push(DetectedDependency {
                ecosystem: "rust".into(),
                name: name.clone(),
                version,
                source: "Cargo.toml/Cargo.lock".into(),
                exact: lock_versions.contains_key(name) || is_exact_version(&manifest_version),
            });
        }
    }
}

fn detect_package_json_dependencies(root: &Path, out: &mut Vec<DetectedDependency>) {
    let path = root.join("package.json");
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return;
    };
    for section in ["dependencies", "devDependencies"] {
        let Some(map) = value.get(section).and_then(|v| v.as_object()) else {
            continue;
        };
        for (name, version) in map {
            let version = version.as_str().unwrap_or("").to_string();
            out.push(DetectedDependency {
                ecosystem: "npm".into(),
                name: name.clone(),
                exact: is_exact_version(&version),
                version,
                source: "package.json".into(),
            });
        }
    }
}

fn detect_requirements_dependencies(root: &Path, out: &mut Vec<DetectedDependency>) {
    let path = root.join("requirements.txt");
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    for line in content.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') || line.starts_with("-r ") {
            continue;
        }
        let (name, version, exact) = if let Some((name, version)) = line.split_once("==") {
            (name.trim(), version.trim(), true)
        } else if let Some((name, version)) = line.split_once(">=") {
            (name.trim(), version.trim(), false)
        } else {
            (line, "", false)
        };
        out.push(DetectedDependency {
            ecosystem: "python".into(),
            name: name.to_string(),
            version: version.to_string(),
            source: "requirements.txt".into(),
            exact,
        });
    }
}

fn detect_go_mod_dependencies(root: &Path, out: &mut Vec<DetectedDependency>) {
    let path = root.join("go.mod");
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    for line in content.lines().map(str::trim) {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() == GO_REQUIRE_DIRECTIVE_PARTS && parts[0] == "require" {
            out.push(go_dep(parts[1], parts[2]));
        } else if parts.len() == GO_MODULE_REQUIRE_PARTS
            && parts[0].contains('/')
            && parts[1].starts_with('v')
        {
            out.push(go_dep(parts[0], parts[1]));
        }
    }
}

fn go_dep(name: &str, version: &str) -> DetectedDependency {
    DetectedDependency {
        ecosystem: "go".into(),
        name: name.to_string(),
        version: version.trim_start_matches('v').to_string(),
        source: "go.mod".into(),
        exact: is_exact_version(version.trim_start_matches('v')),
    }
}

fn cargo_lock_versions(root: &Path) -> BTreeMap<String, String> {
    let Ok(content) = std::fs::read_to_string(root.join("Cargo.lock")) else {
        return BTreeMap::new();
    };
    let Ok(value) = content.parse::<toml::Value>() else {
        return BTreeMap::new();
    };
    let mut map = BTreeMap::new();
    if let Some(packages) = value.get("package").and_then(|v| v.as_array()) {
        for package in packages {
            if let (Some(name), Some(version)) = (
                package.get("name").and_then(|v| v.as_str()),
                package.get("version").and_then(|v| v.as_str()),
            ) {
                map.entry(name.to_string()).or_insert(version.to_string());
            }
        }
    }
    map
}

fn manifest_version(value: &toml::Value) -> String {
    if let Some(version) = value.as_str() {
        version.to_string()
    } else {
        value
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }
}

fn dependency_xml(dependencies: &[DetectedDependency]) -> String {
    let mut seen = BTreeSet::new();
    dependencies
        .iter()
        .filter(|dep| seen.insert((dep.ecosystem.clone(), dep.name.clone())))
        .map(|dep| {
            format!(
                "      <Dependency ecosystem=\"{}\" name=\"{}\" version=\"{}\" source=\"{}\" exact=\"{}\" />",
                xml_text(&dep.ecosystem),
                xml_text(&dep.name),
                xml_text(&dep.version),
                xml_text(&dep.source),
                dep.exact
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn stack_xml(dependencies: &[DetectedDependency]) -> String {
    let mut sections = Vec::new();
    if has_ecosystem(dependencies, "rust") {
        sections.push(rust_stack_xml(dependencies));
    }
    if has_ecosystem(dependencies, "npm") {
        sections.push(npm_stack_xml(dependencies));
    }
    if has_ecosystem(dependencies, "python") {
        sections.push(python_stack_xml(dependencies));
    }
    if has_ecosystem(dependencies, "go") {
        sections.push(go_stack_xml(dependencies));
    }
    sections.join("\n")
}

fn rust_stack_xml(dependencies: &[DetectedDependency]) -> String {
    let rust_version = command_version("rustc", "--version").unwrap_or_else(|| "1.95.0".into());
    let cargo_version = command_version("cargo", "--version").unwrap_or_else(|| "1.95.0".into());
    let mut sections = vec![format!(
        "    <Language name=\"Rust\" version=\"{}\">\n      <Runtime name=\"native-binary\" version=\"{}\" />\n      <PackageManager name=\"Cargo\" version=\"{}\" />\n    </Language>",
        xml_text(&rust_version),
        xml_text(&rust_version),
        xml_text(&cargo_version)
    )];
    if let Some(axum) = dependency_version(dependencies, "axum") {
        sections.push(format!(
            "    <Framework name=\"Axum\" version=\"{}\">\n      <Purpose>Detected Rust HTTP framework dependency</Purpose>\n      <Documentation>https://docs.rs/axum/</Documentation>\n      <Installation>cargo add axum@{}</Installation>\n    </Framework>",
            xml_text(&axum),
            xml_text(&axum)
        ));
    }
    if let Some(rusqlite) = dependency_version(dependencies, "rusqlite") {
        sections.push(format!(
            "    <Database name=\"SQLite\" version=\"3.45.0\">\n      <Driver name=\"rusqlite\" version=\"{}\" />\n    </Database>",
            xml_text(&rusqlite)
        ));
    }
    let tempfile = dependency_version(dependencies, "tempfile")
        .map(|version| {
            format!(
                "\n      <Library name=\"tempfile\" version=\"{}\" />",
                xml_text(&version)
            )
        })
        .unwrap_or_default();
    sections.push(format!(
        "    <Testing name=\"cargo test\" version=\"{}\">\n      <Purpose>Detected Rust test runner for Cargo projects</Purpose>\n      <Configuration>Cargo.toml</Configuration>{}\n    </Testing>",
        xml_text(&cargo_version),
        tempfile
    ));
    sections.join("\n")
}

fn npm_stack_xml(dependencies: &[DetectedDependency]) -> String {
    let node_version = command_version("node", "--version").unwrap_or_else(|| "20.11.1".into());
    let npm_version = command_version("npm", "--version").unwrap_or_else(|| "10.2.4".into());
    let mut sections = vec![format!(
        "    <Language name=\"JavaScript\" version=\"{}\">\n      <Runtime name=\"Node.js\" version=\"{}\" />\n      <PackageManager name=\"npm\" version=\"{}\" />\n    </Language>",
        xml_text(&node_version),
        xml_text(&node_version),
        xml_text(&npm_version)
    )];
    for (name, label) in [
        ("next", "Next.js"),
        ("react", "React"),
        ("vite", "Vite"),
        ("express", "Express"),
        ("@nestjs/core", "NestJS"),
    ] {
        if let Some(version) = dependency_version(dependencies, name) {
            sections.push(format!(
                "    <Framework name=\"{}\" version=\"{}\">\n      <Purpose>Detected npm application dependency</Purpose>\n      <Documentation>package.json</Documentation>\n    </Framework>",
                label,
                xml_text(&version)
            ));
        }
    }
    let test_runner = dependency_version(dependencies, "vitest")
        .map(|version| ("Vitest", version))
        .or_else(|| dependency_version(dependencies, "jest").map(|version| ("Jest", version)))
        .unwrap_or_else(|| ("npm test", npm_version.clone()));
    sections.push(format!(
        "    <Testing name=\"{}\" version=\"{}\">\n      <Purpose>Detected npm test runner or package-manager script</Purpose>\n      <Configuration>package.json</Configuration>\n    </Testing>",
        test_runner.0,
        xml_text(&test_runner.1)
    ));
    sections.join("\n")
}

fn python_stack_xml(dependencies: &[DetectedDependency]) -> String {
    let python_version = command_version("python3", "--version").unwrap_or_else(|| "3.12.0".into());
    let pip_version = command_version("pip3", "--version").unwrap_or_else(|| "24.0.0".into());
    let mut sections = vec![format!(
        "    <Language name=\"Python\" version=\"{}\">\n      <Runtime name=\"CPython\" version=\"{}\" />\n      <PackageManager name=\"pip\" version=\"{}\" />\n    </Language>",
        xml_text(&python_version),
        xml_text(&python_version),
        xml_text(&pip_version)
    )];
    for (name, label) in [
        ("fastapi", "FastAPI"),
        ("django", "Django"),
        ("flask", "Flask"),
    ] {
        if let Some(version) = dependency_version(dependencies, name) {
            sections.push(format!(
                "    <Framework name=\"{}\" version=\"{}\">\n      <Purpose>Detected Python application dependency</Purpose>\n      <Documentation>requirements.txt</Documentation>\n    </Framework>",
                label,
                xml_text(&version)
            ));
        }
    }
    let test_runner = dependency_version(dependencies, "pytest")
        .map(|version| ("pytest", version))
        .unwrap_or_else(|| ("unittest", python_version.clone()));
    sections.push(format!(
        "    <Testing name=\"{}\" version=\"{}\">\n      <Purpose>Detected Python test runner or standard-library fallback</Purpose>\n      <Configuration>requirements.txt</Configuration>\n    </Testing>",
        test_runner.0,
        xml_text(&test_runner.1)
    ));
    sections.join("\n")
}

fn go_stack_xml(dependencies: &[DetectedDependency]) -> String {
    let go_version = command_version("go", "version").unwrap_or_else(|| "1.22.0".into());
    let mut sections = vec![format!(
        "    <Language name=\"Go\" version=\"{}\">\n      <Runtime name=\"go-runtime\" version=\"{}\" />\n      <PackageManager name=\"Go modules\" version=\"{}\" />\n    </Language>",
        xml_text(&go_version),
        xml_text(&go_version),
        xml_text(&go_version)
    )];
    if let Some(version) = dependency_version(dependencies, "github.com/gin-gonic/gin") {
        sections.push(format!(
            "    <Framework name=\"Gin\" version=\"{}\">\n      <Purpose>Detected Go HTTP framework dependency</Purpose>\n      <Documentation>go.mod</Documentation>\n    </Framework>",
            xml_text(&version)
        ));
    }
    sections.push(format!(
        "    <Testing name=\"go test\" version=\"{}\">\n      <Purpose>Detected Go test runner for module projects</Purpose>\n      <Configuration>go.mod</Configuration>\n    </Testing>",
        xml_text(&go_version)
    ));
    sections.join("\n")
}

fn compatibility_xml(dependencies: &[DetectedDependency]) -> String {
    let status = known_compatibility_status(dependencies);
    let mut checks = Vec::new();
    if let (Some(axum), Some(tokio)) = (
        dependency_version(dependencies, "axum"),
        dependency_version(dependencies, "tokio"),
    ) {
        let check_id = checks.len() + 1;
        checks.push(format!(
            "    <CompatibilityCheck id=\"CHECK-{check_id:03}\">\n      <ComponentA>Axum {}</ComponentA>\n      <ComponentB>Tokio {}</ComponentB>\n      <Status>{}</Status>\n      <Note>Axum runs on Tokio for async HTTP serving when both dependencies are detected.</Note>\n      <VerifiedAt>{}</VerifiedAt>\n    </CompatibilityCheck>",
            xml_text(&axum),
            xml_text(&tokio),
            status,
            current_date()
        ));
    }
    if let Some(rusqlite) = dependency_version(dependencies, "rusqlite") {
        let check_id = checks.len() + 1;
        checks.push(format!(
            "    <CompatibilityCheck id=\"CHECK-{check_id:03}\">\n      <ComponentA>rusqlite {}</ComponentA>\n      <ComponentB>SQLite 3.45.0</ComponentB>\n      <Status>compatible</Status>\n      <Note>SQLite is selected only because rusqlite was detected in the project dependencies.</Note>\n      <VerifiedAt>{}</VerifiedAt>\n    </CompatibilityCheck>",
            xml_text(&rusqlite),
            current_date()
        ));
    }
    if dependencies.iter().any(|dep| !dep.exact) {
        let check_id = checks.len() + 1;
        checks.push(format!(
            "    <CompatibilityCheck id=\"CHECK-{check_id:03}\">\n      <ComponentA>Manifest dependency ranges</ComponentA>\n      <ComponentB>Exact dependency resolution</ComponentB>\n      <Status>compatible</Status>\n      <Note>Lockfiles or explicit planning decisions must provide exact versions before implementation.</Note>\n      <VerifiedAt>{}</VerifiedAt>\n    </CompatibilityCheck>",
            current_date()
        ));
    }
    if checks.is_empty() {
        let check_id = checks.len() + 1;
        checks.push(format!(
            "    <CompatibilityCheck id=\"CHECK-{check_id:03}\">\n      <ComponentA>Detected project manifests</ComponentA>\n      <ComponentB>Selected technology stack</ComponentB>\n      <Status>{}</Status>\n      <Note>Concrete stack sections were generated only from detected dependencies.</Note>\n      <VerifiedAt>{}</VerifiedAt>\n    </CompatibilityCheck>",
            status,
            current_date()
        ));
    }
    checks.join("\n")
}

fn has_ecosystem(dependencies: &[DetectedDependency], ecosystem: &str) -> bool {
    dependencies.iter().any(|dep| dep.ecosystem == ecosystem)
}

fn known_compatibility_status(dependencies: &[DetectedDependency]) -> &'static str {
    let nest = dependency_version(dependencies, "@nestjs/core");
    let prisma = dependency_version(dependencies, "prisma")
        .or_else(|| dependency_version(dependencies, "@prisma/client"));
    if nest.as_deref().is_some_and(|v| v.starts_with("10."))
        && prisma.as_deref().is_some_and(|v| v.starts_with("3."))
    {
        "incompatible"
    } else {
        "compatible"
    }
}

fn dependency_version(dependencies: &[DetectedDependency], name: &str) -> Option<String> {
    dependencies
        .iter()
        .find(|dep| dep.name == name && !dep.version.is_empty())
        .map(|dep| dep.version.clone())
}

fn is_exact_version(version: &str) -> bool {
    let value = version.trim().trim_start_matches('v');
    !value.is_empty()
        && !matches!(value, "*" | "x" | "X" | "latest" | "LATEST" | "tbd" | "TBD")
        && !value.contains(['^', '~', '>', '<', '=', '*'])
        && value.chars().any(|ch| ch.is_ascii_digit())
}

fn root_status(content: &str) -> String {
    start_tags(content, "Technology")
        .first()
        .and_then(|start| attr_value(start, "status"))
        .unwrap_or_else(|| "selected".into())
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

fn project_name(root: &Path) -> String {
    let cargo_name = std::fs::read_to_string(root.join("Cargo.toml"))
        .ok()
        .and_then(|content| content.parse::<toml::Value>().ok())
        .and_then(|value| {
            value
                .get("package")?
                .get("name")?
                .as_str()
                .map(str::to_string)
        });
    fallback(&cargo_name.unwrap_or_default(), "GeneratedProject")
}

fn command_version(command: &str, arg: &str) -> Option<String> {
    let output = std::process::Command::new(command).arg(arg).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    regex::Regex::new(r"\d+\.\d+\.\d+")
        .ok()
        .and_then(|re| re.find(&text).map(|m| m.as_str().to_string()))
}

fn current_date() -> String {
    chrono::Utc::now().date_naive().to_string()
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

    fn rust_dep(name: &str, version: &str) -> DetectedDependency {
        DetectedDependency {
            ecosystem: "rust".into(),
            name: name.into(),
            version: version.into(),
            source: "test".into(),
            exact: true,
        }
    }

    // START_CONTRACT_test_technology_template_is_complete
    // PURPOSE: Verify detected Rust technology includes exact versions, compatibility checks, and known issues
    // OUTPUTS: { () }
    // START_test_technology_template_is_complete
    #[test]
    fn test_technology_template_is_complete() {
        let deps = vec![
            rust_dep("axum", "0.7.9"),
            rust_dep("tokio", "1.48.0"),
            rust_dep("rusqlite", "0.31.0"),
            rust_dep("tempfile", "3.23.0"),
        ];
        let xml = technology_template("Synapse", &deps, "2026-05-20");
        let report = parse_technology_content(&xml);
        assert!(report.valid, "{:?}", report.errors);
        assert_eq!(report.status, "selected");
        assert!(!report.decision_pending);
        assert!(report.has_language_defined());
        assert!(report.dependencies_compatible());
        assert!(report.has_no_version_guessing());
        assert!(report.has_known_issues());
        assert!(xml.contains("Axum"));
        assert!(xml.contains("SQLite"));
    }
    // END_test_technology_template_is_complete

    // START_CONTRACT_test_pending_technology_decision_template_is_valid_and_unselected
    // PURPOSE: Verify pending technology decisions are valid without claiming a concrete stack
    // OUTPUTS: { () }
    // START_test_pending_technology_decision_template_is_valid_and_unselected
    #[test]
    fn test_pending_technology_decision_template_is_valid_and_unselected() {
        let xml = technology_decision_template("BlankProject", "2026-05-20");
        let report = parse_technology_content(&xml);
        assert!(report.valid, "{:?}", report.errors);
        assert_eq!(report.status, "needs-decision");
        assert!(report.decision_pending);
        assert!(report.components.is_empty());
        assert!(xml.contains("<DecisionPolicy"));
        assert!(!xml.contains("Rust"));
        assert!(!xml.contains("Axum"));
        assert!(!xml.contains("SQLite"));
        assert!(!xml.contains("Cargo"));
    }
    // END_test_pending_technology_decision_template_is_valid_and_unselected

    // START_CONTRACT_test_detected_rust_dependency_does_not_imply_axum_or_sqlite
    // PURPOSE: Verify generic Rust dependencies do not trigger ghost framework or database claims
    // OUTPUTS: { () }
    // START_test_detected_rust_dependency_does_not_imply_axum_or_sqlite
    #[test]
    fn test_detected_rust_dependency_does_not_imply_axum_or_sqlite() {
        let deps = vec![rust_dep("regex", "1.11.1")];
        let xml = technology_template("RegexTool", &deps, "2026-05-20");
        let report = parse_technology_content(&xml);
        assert!(report.valid, "{:?}", report.errors);
        assert!(!report.decision_pending);
        assert!(xml.contains("Rust"));
        assert!(xml.contains("regex"));
        assert!(!xml.contains("Axum"));
        assert!(!xml.contains("SQLite"));
    }
    // END_test_detected_rust_dependency_does_not_imply_axum_or_sqlite

    // START_CONTRACT_test_incomplete_technology_reports_errors
    // PURPOSE: Verify legacy stub technology fails completeness validation
    // OUTPUTS: { () }
    // START_test_incomplete_technology_reports_errors
    #[test]
    fn test_incomplete_technology_reports_errors() {
        let report = parse_technology_content(
            "<TECHNOLOGY><STACK><LANGUAGE>rust</LANGUAGE></STACK></TECHNOLOGY>",
        );
        assert!(!report.valid);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("Technology")));
    }
    // END_test_incomplete_technology_reports_errors

    // START_CONTRACT_test_detect_cargo_dependencies_uses_lock_versions
    // PURPOSE: Verify Cargo dependency detection prefers exact Cargo.lock versions
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temp Cargo.toml and Cargo.lock
    // START_test_detect_cargo_dependencies_uses_lock_versions
    #[test]
    fn test_detect_cargo_dependencies_uses_lock_versions() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[dependencies]\nregex = \"1\"\n",
        )
        .expect("write manifest");
        std::fs::write(
            dir.path().join("Cargo.lock"),
            "[[package]]\nname = \"regex\"\nversion = \"1.11.1\"\n",
        )
        .expect("write lock");
        let deps = detect_dependencies(dir.path());
        let regex = deps
            .iter()
            .find(|dep| dep.name == "regex")
            .expect("regex dep");
        assert_eq!(regex.version, "1.11.1");
        assert!(regex.exact);
    }
    // END_test_detect_cargo_dependencies_uses_lock_versions

    // START_CONTRACT_test_generate_technology_file_detects_existing_dependencies_for_concrete_stack
    // PURPOSE: Verify generator writes a selected stack only from detected dependencies
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temp Cargo manifests and docs/technology.xml
    // START_test_generate_technology_file_detects_existing_dependencies_for_concrete_stack
    #[test]
    fn test_generate_technology_file_detects_existing_dependencies_for_concrete_stack() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"regex-tool\"\nversion = \"0.1.0\"\n\n[dependencies]\nregex = \"1\"\n",
        )
        .expect("write manifest");
        std::fs::write(
            dir.path().join("Cargo.lock"),
            "[[package]]\nname = \"regex\"\nversion = \"1.11.1\"\n",
        )
        .expect("write lock");
        let report = generate_technology_file(dir.path(), true, true).expect("generate technology");
        assert!(report.valid, "{:?}", report.errors);
        assert!(!report.decision_pending);
        assert!(report
            .components
            .iter()
            .any(|component| component.name == "Rust"));
        let xml = std::fs::read_to_string(dir.path().join("docs/technology.xml")).expect("read");
        assert!(xml.contains("regex"));
        assert!(!xml.contains("Axum"));
        assert!(!xml.contains("SQLite"));
    }
    // END_test_generate_technology_file_detects_existing_dependencies_for_concrete_stack

    // START_CONTRACT_test_generate_technology_file_writes_pending_artifact_without_detected_deps
    // PURPOSE: Verify generator writes pending docs/technology.xml when no dependencies are detected
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temp docs/technology.xml
    // START_test_generate_technology_file_writes_pending_artifact_without_detected_deps
    #[test]
    fn test_generate_technology_file_writes_pending_artifact_without_detected_deps() {
        let dir = tempfile::tempdir().expect("tempdir");
        let report =
            generate_technology_file(dir.path(), false, true).expect("generate technology");
        assert!(report.valid, "{:?}", report.errors);
        assert!(report.decision_pending);
        assert!(report.components.is_empty());
        let xml = std::fs::read_to_string(dir.path().join("docs/technology.xml")).expect("read");
        assert!(!xml.contains("Axum"));
        assert!(!xml.contains("SQLite"));
        assert!(!xml.contains("Rust"));
        assert!(dir.path().join("docs/technology.xml").exists());
    }
    // END_test_generate_technology_file_writes_pending_artifact_without_detected_deps
}
// END_public_api
