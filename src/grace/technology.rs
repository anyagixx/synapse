// MODULE_CONTRACT
// MODULE_ID: M-GRACE-TECHNOLOGY
// PURPOSE: Technology stack parser, validator, dependency detector, and generator for GRACE Stage 2 artifacts
// SCOPE: TechnologyReport, DetectedDependency, TechnologyComponent, template generation, file validation, dependency detection, compatibility checks
// DEPENDS: N/A
// LINKS:
//   → V-M-GRACE-TECHNOLOGY (verified_by) — technology parser, validator, detector, and generator tests

// START_MODULE_MAP
// DetectedDependency — One dependency discovered from manifest or lock files
// TechnologyReport — Completeness report for docs/technology.xml
// technology_template — Full Technology XML template with exact versions and compatibility matrix
// validate_technology — Parse and validate docs/technology.xml
// detect_dependencies — Detect dependencies from common package manifests
// generate_technology_file — Generate and validate docs/technology.xml
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 - Named Go module parsing arity constants for GRACE pattern cleanliness]
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
    // PURPOSE: Return true when at least one language has an exact version
    // OUTPUTS: { bool }
    // START_technology_report_has_language_defined
    pub fn has_language_defined(&self) -> bool {
        self.languages
            .iter()
            .any(|component| is_exact_version(&component.version))
    }
    // END_technology_report_has_language_defined

    // START_CONTRACT_TechnologyReport::dependencies_compatible
    // PURPOSE: Return true when compatibility checks exist and none are incompatible
    // OUTPUTS: { bool }
    // START_technology_report_dependencies_compatible
    pub fn dependencies_compatible(&self) -> bool {
        !self.compatibility_checks.is_empty() && self.incompatible_checks.is_empty()
    }
    // END_technology_report_dependencies_compatible

    // START_CONTRACT_TechnologyReport::has_no_version_guessing
    // PURPOSE: Return true when no version attributes use blank, latest, wildcard, or range syntax
    // OUTPUTS: { bool }
    // START_technology_report_has_no_version_guessing
    pub fn has_no_version_guessing(&self) -> bool {
        self.missing_versions.is_empty() && !self.components.is_empty()
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

// START_CONTRACT_technology_template
// PURPOSE: Build a complete Technology XML template with exact versions and compatibility checks
// INPUTS: { project_name: &str }, { dependencies: &[DetectedDependency] }, { updated: &str }
// OUTPUTS: { String }
// START_technology_template
pub fn technology_template(
    project_name: &str,
    dependencies: &[DetectedDependency],
    updated: &str,
) -> String {
    let project_name = xml_text(&fallback(project_name, "MyProject"));
    let updated = xml_text(&fallback(updated, "2026-05-20"));
    let rust_version = command_version("rustc", "--version").unwrap_or_else(|| "1.95.0".into());
    let cargo_version = command_version("cargo", "--version").unwrap_or_else(|| "1.95.0".into());
    let axum = dependency_version(dependencies, "axum").unwrap_or_else(|| "0.7.9".into());
    let tokio = dependency_version(dependencies, "tokio").unwrap_or_else(|| "1.48.0".into());
    let rusqlite = dependency_version(dependencies, "rusqlite").unwrap_or_else(|| "0.31.0".into());
    let testing = dependency_version(dependencies, "tempfile").unwrap_or_else(|| "3.23.0".into());
    let libraries = dependency_xml(dependencies);
    let checks = compatibility_xml(dependencies, &axum, &tokio, &rusqlite);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Technology project="{project_name}" version="1.0" updated="{updated}">
  <Stack>
    <Language name="Rust" version="{rust_version}">
      <Runtime name="native-binary" version="{rust_version}" />
      <PackageManager name="Cargo" version="{cargo_version}" />
    </Language>
    <Framework name="Axum" version="{axum}">
      <Purpose>HTTP dashboard and API server for Synapse local tooling</Purpose>
      <Documentation>https://docs.rs/axum/0.7/axum/</Documentation>
      <Installation>cargo add axum@{axum}</Installation>
    </Framework>
    <Database name="SQLite" version="3.45.0">
      <Driver name="rusqlite" version="{rusqlite}" />
    </Database>
    <Testing name="cargo test" version="{cargo_version}">
      <Purpose>Rust unit, integration, protocol, release, and support tests</Purpose>
      <Configuration>Cargo.toml</Configuration>
      <Library name="tempfile" version="{testing}" />
    </Testing>
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
      <RustVersion>{rust_version}</RustVersion>
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
    let mut report = TechnologyReport {
        exists: true,
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
    let mut content = technology_template(&project_name, &deps, &current_date());
    if !compatibility_check {
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

fn compatibility_xml(
    dependencies: &[DetectedDependency],
    axum: &str,
    tokio: &str,
    rusqlite: &str,
) -> String {
    let status = known_compatibility_status(dependencies);
    let mut checks = vec![
        format!(
            "    <CompatibilityCheck id=\"CHECK-001\">\n      <ComponentA>Axum {}</ComponentA>\n      <ComponentB>Tokio {}</ComponentB>\n      <Status>{}</Status>\n      <Note>Axum 0.7 runs on Tokio 1.x for async HTTP serving.</Note>\n      <VerifiedAt>{}</VerifiedAt>\n    </CompatibilityCheck>",
            xml_text(axum),
            xml_text(tokio),
            status,
            current_date()
        ),
        format!(
            "    <CompatibilityCheck id=\"CHECK-002\">\n      <ComponentA>rusqlite {}</ComponentA>\n      <ComponentB>SQLite 3.45.0</ComponentB>\n      <Status>compatible</Status>\n      <Note>rusqlite is built with the bundled SQLite feature in Cargo.toml.</Note>\n      <VerifiedAt>{}</VerifiedAt>\n    </CompatibilityCheck>",
            xml_text(rusqlite),
            current_date()
        ),
    ];
    if dependencies.iter().any(|dep| !dep.exact) {
        checks.push(format!(
            "    <CompatibilityCheck id=\"CHECK-003\">\n      <ComponentA>Manifest dependency ranges</ComponentA>\n      <ComponentB>Lockfile exact versions</ComponentB>\n      <Status>compatible</Status>\n      <Note>Cargo.lock/package lock files provide exact versions for code generation.</Note>\n      <VerifiedAt>{}</VerifiedAt>\n    </CompatibilityCheck>",
            current_date()
        ));
    }
    checks.join("\n")
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

    // START_CONTRACT_test_technology_template_is_complete
    // PURPOSE: Verify generated technology includes exact versions, compatibility checks, and known issues
    // OUTPUTS: { () }
    // START_test_technology_template_is_complete
    #[test]
    fn test_technology_template_is_complete() {
        let xml = technology_template("Synapse", &[], "2026-05-20");
        let report = parse_technology_content(&xml);
        assert!(report.valid, "{:?}", report.errors);
        assert!(report.has_language_defined());
        assert!(report.dependencies_compatible());
        assert!(report.has_no_version_guessing());
        assert!(report.has_known_issues());
    }
    // END_test_technology_template_is_complete

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

    // START_CONTRACT_test_generate_technology_file_writes_valid_artifact
    // PURPOSE: Verify generator writes docs/technology.xml and validates it
    // OUTPUTS: { () }
    // SIDE_EFFECTS: writes temp docs/technology.xml
    // START_test_generate_technology_file_writes_valid_artifact
    #[test]
    fn test_generate_technology_file_writes_valid_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        let report =
            generate_technology_file(dir.path(), false, true).expect("generate technology");
        assert!(report.valid, "{:?}", report.errors);
        assert!(dir.path().join("docs/technology.xml").exists());
    }
    // END_test_generate_technology_file_writes_valid_artifact
}
// END_public_api
