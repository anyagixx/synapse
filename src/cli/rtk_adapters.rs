// MODULE_CONTRACT
// MODULE_ID: M-CLI-RTK-COMMANDS
// PURPOSE: Local RTK-style adapters for token-heavy structured data, dependency, environment, and count output
// SCOPE: JsonCmd, DepsCmd, EnvCmd, WcCmd execution; compact renderers; shared adapter-level token tracking for local RTK adapters
// DEPENDS: M-CONFIG, M-TRACKING, M-UTILS
// LINKS:
//   -> M-CLI (depends) - implements command argument schemas declared by the CLI facade
//   -> M-TRACKING (depends) - records direct adapter token economy
//   -> UC-002 (implements) - local adapters produce compact execution evidence
//   -> NFR-003 (traces_to) - direct adapters reduce context without shell subprocesses

// START_MODULE_MAP
// JsonCmd::run - Render compact JSON values or schema-only structure
// DepsCmd::run - Summarize dependency manifests from a project directory
// EnvCmd::run - Render filtered and masked environment variables
// WcCmd::run - Count lines, words, bytes, and chars without invoking wc
// record_adapter_savings - Persist direct adapter token economy
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 - Exposed shared local adapter token tracking helper]
// END_CHANGE_SUMMARY

use super::{DepsCmd, EnvCmd, JsonCmd, WcCmd};
use crate::config::Config;
use crate::tracking::Tracker;
use anyhow::Context;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::{Path, PathBuf};

const JSON_OBJECT_VALUE_KEY_LIMIT: usize = 20;
const JSON_OBJECT_SCHEMA_KEY_LIMIT: usize = 15;
const JSON_ARRAY_INLINE_LIMIT: usize = 5;
const REQUIREMENTS_ITEM_LIMIT: usize = 15;
const TOML_DEPENDENCY_LIMIT: usize = 12;
const TOML_DEV_DEPENDENCY_LIMIT: usize = 8;
const TOML_BUILD_DEPENDENCY_LIMIT: usize = 6;
const PACKAGE_DEPENDENCY_LIMIT: usize = 12;
const PACKAGE_DEV_DEPENDENCY_LIMIT: usize = 8;
const PYPROJECT_DEPENDENCY_LIMIT: usize = 12;
const GO_DEPENDENCY_LIMIT: usize = 12;
const ENV_GROUP_ITEM_LIMIT: usize = 20;
const SHORT_SENSITIVE_VALUE_MAX_CHARS: usize = 4;
const ENV_VALUE_INLINE_LIMIT: usize = 120;
const ENV_VALUE_PREVIEW_CHARS: usize = 80;
const JSON_STRING_PREVIEW_CHARS: usize = 80;

// START_public_api

impl JsonCmd {
    // START_CONTRACT_JsonCmd::run
    // PURPOSE: Read JSON from a file or stdin and print a compact value or keys-only schema
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads file/stdin, writes stdout, records adapter token economy
    // LINKS:
    //   -> M-TRACKING (depends) - records json-adapter savings
    //   -> NFR-003 (traces_to) - JSON compaction avoids dumping full payloads into context
    // START_json_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let (raw, source) = read_text_input(&self.file)?;
        validate_json_source(&source)?;
        let output = render_json(&raw, self.depth, self.keys_only)?;
        println!("{output}");
        record_adapter_savings(
            &config,
            &format!("syn json {}", self.file),
            &raw,
            &output,
            "rtk-json",
            "syn json",
        )
        .await;
        Ok(())
    }
    // END_json_cmd_run
}

impl DepsCmd {
    // START_CONTRACT_DepsCmd::run
    // PURPOSE: Summarize dependency manifests without printing full lockfiles or package metadata
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads dependency manifests, writes stdout, records adapter token economy
    // LINKS:
    //   -> M-TRACKING (depends) - records deps-adapter savings
    //   -> NFR-003 (traces_to) - dependency summaries replace large manifest dumps
    // START_deps_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let dir = dependency_root(Path::new(&self.path));
        let report = summarize_dependencies(dir)?;
        println!("{}", report.output.trim_end());
        record_adapter_savings(
            &config,
            &format!("syn deps {}", self.path),
            &report.raw,
            &report.output,
            "rtk-deps",
            "syn deps",
        )
        .await;
        Ok(())
    }
    // END_deps_cmd_run
}

impl EnvCmd {
    // START_CONTRACT_EnvCmd::run
    // PURPOSE: Print relevant environment variables with secrets masked and long values truncated
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads process environment, writes stdout, records adapter token economy
    // LINKS:
    //   -> M-TRACKING (depends) - records env-adapter savings
    //   -> NFR-002 (traces_to) - sensitive environment values are masked by default
    // START_env_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let vars = std::env::vars().collect::<Vec<_>>();
        let output = render_env(&vars, self.filter.as_deref(), self.show_all);
        println!("{}", output.trim_end());
        let raw = vars
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("\n");
        record_adapter_savings(&config, "syn env", &raw, &output, "rtk-env", "syn env").await;
        Ok(())
    }
    // END_env_cmd_run
}

impl WcCmd {
    // START_CONTRACT_WcCmd::run
    // PURPOSE: Count text input locally and render compact wc-style output
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads files/stdin, writes stdout, records adapter token economy
    // LINKS:
    //   -> M-TRACKING (depends) - records wc-adapter savings
    //   -> NFR-003 (traces_to) - compact counts avoid native wc padding and path noise
    // START_wc_cmd_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let mode = detect_wc_mode(&self.args);
        let files = wc_files(&self.args);
        let report = render_wc(&files, &mode)?;
        println!("{}", report.output.trim_end());
        record_adapter_savings(
            &config,
            &format!("syn wc {}", self.args.join(" ")),
            &report.raw,
            &report.output,
            "rtk-wc",
            "syn wc",
        )
        .await;
        Ok(())
    }
    // END_wc_cmd_run
}

// END_public_api

// START_CONTRACT_read_text_input
// PURPOSE: Read adapter input from a path or stdin marker
// INPUTS: { source: &str }
// OUTPUTS: { anyhow::Result<(String, String)> }
// START_read_text_input
fn read_text_input(source: &str) -> anyhow::Result<(String, String)> {
    if source == "-" {
        let mut raw = String::new();
        std::io::stdin()
            .lock()
            .read_to_string(&mut raw)
            .context("read JSON from stdin")?;
        return Ok((raw, "stdin".into()));
    }
    let raw = std::fs::read_to_string(source).with_context(|| format!("read {source}"))?;
    Ok((raw, source.into()))
}
// END_read_text_input

// START_CONTRACT_validate_json_source
// PURPOSE: Reject obvious non-JSON manifest formats before parsing to give a useful adapter hint
// INPUTS: { source: &str }
// OUTPUTS: { anyhow::Result<()> }
// START_validate_json_source
fn validate_json_source(source: &str) -> anyhow::Result<()> {
    if source == "stdin" {
        return Ok(());
    }
    let path = Path::new(source);
    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return Ok(());
    };
    let format_name = match ext {
        "toml" => Some("TOML"),
        "yaml" | "yml" => Some("YAML"),
        "xml" => Some("XML"),
        "csv" => Some("CSV"),
        "ini" => Some("INI"),
        "txt" => Some("plain text"),
        _ => None,
    };
    if let Some(format_name) = format_name {
        let mut message = format!("{source} is not a JSON file (detected {format_name})");
        if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml") {
            message.push_str("; use `syn deps` for Cargo.toml");
        }
        anyhow::bail!(message);
    }
    Ok(())
}
// END_validate_json_source

// START_CONTRACT_render_json
// PURPOSE: Render compact JSON values or schema-only structure
// INPUTS: { raw: &str }, { max_depth: usize }, { keys_only: bool }
// OUTPUTS: { anyhow::Result<String> }
// START_render_json
fn render_json(raw: &str, max_depth: usize, keys_only: bool) -> anyhow::Result<String> {
    let value = serde_json::from_str::<Value>(raw).context("parse JSON")?;
    if keys_only {
        Ok(render_json_schema(&value, 0, max_depth))
    } else {
        Ok(render_json_compact(&value, 0, max_depth))
    }
}
// END_render_json

// START_CONTRACT_render_json_compact
// PURPOSE: Render JSON with bounded depth, bounded key counts, and truncated long strings
// INPUTS: { value: &Value }, { depth: usize }, { max_depth: usize }
// OUTPUTS: { String }
// START_render_json_compact
fn render_json_compact(value: &Value, depth: usize, max_depth: usize) -> String {
    let indent = "  ".repeat(depth);
    if depth > max_depth {
        return format!("{indent}...");
    }
    match value {
        Value::Null => format!("{indent}null"),
        Value::Bool(value) => format!("{indent}{value}"),
        Value::Number(value) => format!("{indent}{value}"),
        Value::String(value) => {
            let display = truncate_chars(value, JSON_STRING_PREVIEW_CHARS);
            format!("{indent}\"{display}\"")
        }
        Value::Array(items) => render_json_array(items, depth, max_depth, false),
        Value::Object(map) => {
            if map.is_empty() {
                return format!("{indent}{{}}");
            }
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            let mut lines = vec![format!("{indent}{{")];
            for (index, key) in keys.iter().enumerate() {
                if index >= JSON_OBJECT_VALUE_KEY_LIMIT {
                    lines.push(format!("{indent}  ... +{} more keys", keys.len() - index));
                    break;
                }
                let rendered = render_json_compact(&map[*key], depth + 1, max_depth);
                if is_simple_json(&map[*key]) {
                    lines.push(format!("{indent}  {key}: {}", rendered.trim()));
                } else {
                    lines.push(format!("{indent}  {key}:"));
                    lines.push(rendered);
                }
            }
            lines.push(format!("{indent}}}"));
            lines.join("\n")
        }
    }
}
// END_render_json_compact

// START_CONTRACT_render_json_schema
// PURPOSE: Render JSON structure and scalar types without exposing values
// INPUTS: { value: &Value }, { depth: usize }, { max_depth: usize }
// OUTPUTS: { String }
// START_render_json_schema
fn render_json_schema(value: &Value, depth: usize, max_depth: usize) -> String {
    let indent = "  ".repeat(depth);
    if depth > max_depth {
        return format!("{indent}...");
    }
    match value {
        Value::Null => format!("{indent}null"),
        Value::Bool(_) => format!("{indent}bool"),
        Value::Number(number) if number.is_i64() || number.is_u64() => format!("{indent}int"),
        Value::Number(_) => format!("{indent}float"),
        Value::String(value) if value.starts_with("http://") || value.starts_with("https://") => {
            format!("{indent}url")
        }
        Value::String(_) => format!("{indent}string"),
        Value::Array(items) => render_json_array(items, depth, max_depth, true),
        Value::Object(map) => {
            if map.is_empty() {
                return format!("{indent}{{}}");
            }
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            let mut lines = vec![format!("{indent}{{")];
            for (index, key) in keys.iter().enumerate() {
                if index >= JSON_OBJECT_SCHEMA_KEY_LIMIT {
                    lines.push(format!("{indent}  ... +{} more keys", keys.len() - index));
                    break;
                }
                let rendered = render_json_schema(&map[*key], depth + 1, max_depth);
                if is_simple_json(&map[*key]) {
                    lines.push(format!("{indent}  {key}: {}", rendered.trim()));
                } else {
                    lines.push(format!("{indent}  {key}:"));
                    lines.push(rendered);
                }
            }
            lines.push(format!("{indent}}}"));
            lines.join("\n")
        }
    }
}
// END_render_json_schema

// START_CONTRACT_render_json_array
// PURPOSE: Render arrays as either short inline values or first-item schema plus cardinality
// INPUTS: { items: &[Value] }, { depth: usize }, { max_depth: usize }, { schema_only: bool }
// OUTPUTS: { String }
// START_render_json_array
fn render_json_array(items: &[Value], depth: usize, max_depth: usize, schema_only: bool) -> String {
    let indent = "  ".repeat(depth);
    if items.is_empty() {
        return format!("{indent}[]");
    }
    if schema_only {
        let first = render_json_schema(&items[0], depth + 1, max_depth);
        return format!("{indent}[{}] ({})", first.trim(), items.len());
    }
    if items.len() > JSON_ARRAY_INLINE_LIMIT {
        let first = render_json_compact(&items[0], depth + 1, max_depth);
        return format!("{indent}[{}, ... +{} more]", first.trim(), items.len() - 1);
    }
    if items.iter().all(is_simple_json) {
        let rendered = items
            .iter()
            .map(|item| render_json_compact(item, 0, max_depth).trim().to_string())
            .collect::<Vec<_>>();
        return format!("{indent}[{}]", rendered.join(", "));
    }
    let mut lines = vec![format!("{indent}[")];
    for item in items {
        lines.push(format!(
            "{},",
            render_json_compact(item, depth + 1, max_depth)
        ));
    }
    lines.push(format!("{indent}]"));
    lines.join("\n")
}
// END_render_json_array

// START_CONTRACT_is_simple_json
// PURPOSE: Return whether a JSON value can be safely rendered inline
// INPUTS: { value: &Value }
// OUTPUTS: { bool }
// START_is_simple_json
fn is_simple_json(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}
// END_is_simple_json

// START_CONTRACT_dependency_root
// PURPOSE: Normalize dependency scan input to a directory
// INPUTS: { path: &Path }
// OUTPUTS: { &Path }
// START_dependency_root
fn dependency_root(path: &Path) -> &Path {
    if path.is_file() {
        path.parent().unwrap_or_else(|| Path::new("."))
    } else {
        path
    }
}
// END_dependency_root

// START_DependencyReport
struct DependencyReport {
    raw: String,
    output: String,
}
// END_DependencyReport

// START_CONTRACT_summarize_dependencies
// PURPOSE: Summarize common language dependency manifests under a directory
// INPUTS: { dir: &Path }
// OUTPUTS: { anyhow::Result<DependencyReport> }
// START_summarize_dependencies
fn summarize_dependencies(dir: &Path) -> anyhow::Result<DependencyReport> {
    let mut raw = String::new();
    let mut output = String::new();
    let mut found = false;

    append_manifest_summary(
        dir,
        "Cargo.toml",
        "Rust (Cargo.toml)",
        summarize_cargo_manifest,
        &mut raw,
        &mut output,
        &mut found,
    )?;
    append_manifest_summary(
        dir,
        "package.json",
        "Node.js (package.json)",
        summarize_package_json,
        &mut raw,
        &mut output,
        &mut found,
    )?;
    append_manifest_summary(
        dir,
        "requirements.txt",
        "Python (requirements.txt)",
        summarize_requirements,
        &mut raw,
        &mut output,
        &mut found,
    )?;
    append_manifest_summary(
        dir,
        "pyproject.toml",
        "Python (pyproject.toml)",
        summarize_pyproject,
        &mut raw,
        &mut output,
        &mut found,
    )?;
    append_manifest_summary(
        dir,
        "go.mod",
        "Go (go.mod)",
        summarize_go_mod,
        &mut raw,
        &mut output,
        &mut found,
    )?;

    if !found {
        writeln!(output, "No dependency manifests found in {}", dir.display())?;
    }

    Ok(DependencyReport { raw, output })
}
// END_summarize_dependencies

// START_CONTRACT_append_manifest_summary
// PURPOSE: Read one manifest when present and append its compact summary
// INPUTS: { dir: &Path }, { file: &str }, { title: &str }, { summarize: fn(&str) -> anyhow::Result<String> }
// OUTPUTS: { anyhow::Result<()> }
// START_append_manifest_summary
fn append_manifest_summary(
    dir: &Path,
    file: &str,
    title: &str,
    summarize: fn(&str) -> anyhow::Result<String>,
    raw: &mut String,
    output: &mut String,
    found: &mut bool,
) -> anyhow::Result<()> {
    let path = dir.join(file);
    if !path.exists() {
        return Ok(());
    }
    *found = true;
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    raw.push_str(&content);
    if !raw.ends_with('\n') {
        raw.push('\n');
    }
    writeln!(output, "{title}:")?;
    output.push_str(&summarize(&content)?);
    if !output.ends_with('\n') {
        output.push('\n');
    }
    Ok(())
}
// END_append_manifest_summary

// START_CONTRACT_summarize_cargo_manifest
// PURPOSE: Summarize Cargo dependencies and dev-dependencies from TOML
// INPUTS: { content: &str }
// OUTPUTS: { anyhow::Result<String> }
// START_summarize_cargo_manifest
fn summarize_cargo_manifest(content: &str) -> anyhow::Result<String> {
    let value = content.parse::<toml::Value>().context("parse Cargo.toml")?;
    let mut output = String::new();
    append_toml_dependency_table(
        &mut output,
        &value,
        "dependencies",
        "dependencies",
        TOML_DEPENDENCY_LIMIT,
    )?;
    append_toml_dependency_table(
        &mut output,
        &value,
        "dev-dependencies",
        "dev",
        TOML_DEV_DEPENDENCY_LIMIT,
    )?;
    append_toml_dependency_table(
        &mut output,
        &value,
        "build-dependencies",
        "build",
        TOML_BUILD_DEPENDENCY_LIMIT,
    )?;
    if output.is_empty() {
        output.push_str("  dependencies: 0\n");
    }
    Ok(output)
}
// END_summarize_cargo_manifest

// START_CONTRACT_append_toml_dependency_table
// PURPOSE: Append one TOML dependency table with bounded item count
// INPUTS: { output: &mut String }, { value: &toml::Value }, { key: &str }, { label: &str }, { limit: usize }
// OUTPUTS: { anyhow::Result<()> }
// START_append_toml_dependency_table
fn append_toml_dependency_table(
    output: &mut String,
    value: &toml::Value,
    key: &str,
    label: &str,
    limit: usize,
) -> anyhow::Result<()> {
    let Some(table) = value.get(key).and_then(toml::Value::as_table) else {
        return Ok(());
    };
    writeln!(output, "  {label}: {}", table.len())?;
    for (index, (name, spec)) in table.iter().enumerate() {
        if index >= limit {
            writeln!(output, "    ... +{} more", table.len() - index)?;
            break;
        }
        writeln!(output, "    {name} {}", dependency_version(spec))?;
    }
    Ok(())
}
// END_append_toml_dependency_table

// START_CONTRACT_dependency_version
// PURPOSE: Extract a compact dependency version from a TOML dependency spec
// INPUTS: { spec: &toml::Value }
// OUTPUTS: { String }
// START_dependency_version
fn dependency_version(spec: &toml::Value) -> String {
    spec.as_str()
        .map(ToOwned::to_owned)
        .or_else(|| {
            spec.as_table()
                .and_then(|table| table.get("version"))
                .and_then(toml::Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "*".into())
}
// END_dependency_version

// START_CONTRACT_summarize_package_json
// PURPOSE: Summarize package.json dependency sections
// INPUTS: { content: &str }
// OUTPUTS: { anyhow::Result<String> }
// START_summarize_package_json
fn summarize_package_json(content: &str) -> anyhow::Result<String> {
    let value = serde_json::from_str::<Value>(content).context("parse package.json")?;
    let mut output = String::new();
    if let Some(name) = value.get("name").and_then(Value::as_str) {
        let version = value.get("version").and_then(Value::as_str).unwrap_or("?");
        writeln!(output, "  {name} @{version}")?;
    }
    append_json_dependency_object(
        &mut output,
        &value,
        "dependencies",
        "dependencies",
        PACKAGE_DEPENDENCY_LIMIT,
    )?;
    append_json_dependency_object(
        &mut output,
        &value,
        "devDependencies",
        "dev",
        PACKAGE_DEV_DEPENDENCY_LIMIT,
    )?;
    Ok(output)
}
// END_summarize_package_json

// START_CONTRACT_append_json_dependency_object
// PURPOSE: Append one package.json dependency object with bounded item count
// INPUTS: { output: &mut String }, { value: &Value }, { key: &str }, { label: &str }, { limit: usize }
// OUTPUTS: { anyhow::Result<()> }
// START_append_json_dependency_object
fn append_json_dependency_object(
    output: &mut String,
    value: &Value,
    key: &str,
    label: &str,
    limit: usize,
) -> anyhow::Result<()> {
    let Some(map) = value.get(key).and_then(Value::as_object) else {
        return Ok(());
    };
    writeln!(output, "  {label}: {}", map.len())?;
    let ordered = map.iter().collect::<BTreeMap<_, _>>();
    for (index, (name, spec)) in ordered.iter().enumerate() {
        if index >= limit {
            writeln!(output, "    ... +{} more", map.len() - index)?;
            break;
        }
        writeln!(output, "    {name} {}", spec.as_str().unwrap_or("*"))?;
    }
    Ok(())
}
// END_append_json_dependency_object

// START_CONTRACT_summarize_requirements
// PURPOSE: Summarize Python requirements.txt packages
// INPUTS: { content: &str }
// OUTPUTS: { anyhow::Result<String> }
// START_summarize_requirements
fn summarize_requirements(content: &str) -> anyhow::Result<String> {
    let packages = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter(|line| !line.starts_with("-r ") && !line.starts_with("--"))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let mut output = String::new();
    writeln!(output, "  packages: {}", packages.len())?;
    for (index, package) in packages.iter().take(REQUIREMENTS_ITEM_LIMIT).enumerate() {
        let _ = index;
        writeln!(output, "    {package}")?;
    }
    if packages.len() > REQUIREMENTS_ITEM_LIMIT {
        writeln!(
            output,
            "    ... +{} more",
            packages.len() - REQUIREMENTS_ITEM_LIMIT
        )?;
    }
    Ok(output)
}
// END_summarize_requirements

// START_CONTRACT_summarize_pyproject
// PURPOSE: Summarize PEP 621 dependencies from pyproject.toml
// INPUTS: { content: &str }
// OUTPUTS: { anyhow::Result<String> }
// START_summarize_pyproject
fn summarize_pyproject(content: &str) -> anyhow::Result<String> {
    let value = content
        .parse::<toml::Value>()
        .context("parse pyproject.toml")?;
    let deps = value
        .get("project")
        .and_then(|project| project.get("dependencies"))
        .and_then(toml::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(toml::Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut output = String::new();
    writeln!(output, "  dependencies: {}", deps.len())?;
    for dep in deps.iter().take(PYPROJECT_DEPENDENCY_LIMIT) {
        writeln!(output, "    {dep}")?;
    }
    if deps.len() > PYPROJECT_DEPENDENCY_LIMIT {
        writeln!(
            output,
            "    ... +{} more",
            deps.len() - PYPROJECT_DEPENDENCY_LIMIT
        )?;
    }
    Ok(output)
}
// END_summarize_pyproject

// START_CONTRACT_summarize_go_mod
// PURPOSE: Summarize module, go version, and required packages from go.mod
// INPUTS: { content: &str }
// OUTPUTS: { anyhow::Result<String> }
// START_summarize_go_mod
fn summarize_go_mod(content: &str) -> anyhow::Result<String> {
    let mut module_name = "";
    let mut go_version = "";
    let mut in_require = false;
    let mut deps = Vec::new();
    for line in content.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("module ") {
            module_name = rest.trim();
        } else if let Some(rest) = line.strip_prefix("go ") {
            go_version = rest.trim();
        } else if line == "require (" {
            in_require = true;
        } else if line == ")" {
            in_require = false;
        } else if in_require && !line.starts_with("//") && !line.is_empty() {
            deps.push(line.to_string());
        } else if let Some(rest) = line.strip_prefix("require ") {
            deps.push(rest.to_string());
        }
    }
    let mut output = String::new();
    if !module_name.is_empty() {
        writeln!(output, "  module: {module_name}")?;
    }
    if !go_version.is_empty() {
        writeln!(output, "  go: {go_version}")?;
    }
    writeln!(output, "  requires: {}", deps.len())?;
    for dep in deps.iter().take(GO_DEPENDENCY_LIMIT) {
        writeln!(output, "    {dep}")?;
    }
    if deps.len() > GO_DEPENDENCY_LIMIT {
        writeln!(output, "    ... +{} more", deps.len() - GO_DEPENDENCY_LIMIT)?;
    }
    Ok(output)
}
// END_summarize_go_mod

// START_CONTRACT_render_env
// PURPOSE: Render environment variables with filter, masking, categorization, and bounded output
// INPUTS: { vars: &[(String, String)] }, { filter: Option<&str> }, { show_all: bool }
// OUTPUTS: { String }
// START_render_env
fn render_env(vars: &[(String, String)], filter: Option<&str>, show_all: bool) -> String {
    let mut vars = vars.to_vec();
    vars.sort_by(|left, right| left.0.cmp(&right.0));
    let filter_lc = filter.map(|value| value.to_ascii_lowercase());
    let mut groups: BTreeMap<&str, Vec<(String, String)>> = BTreeMap::new();
    for (key, value) in vars.iter() {
        if let Some(filter) = &filter_lc {
            if !key.to_ascii_lowercase().contains(filter) {
                continue;
            }
        } else if !is_interesting_env(key) {
            continue;
        }
        let category = env_category(key);
        let value = if is_sensitive_env(key) && !show_all {
            mask_env_value(value)
        } else {
            truncate_env_value(value)
        };
        groups
            .entry(category)
            .or_default()
            .push((key.clone(), value));
    }

    let mut output = String::new();
    if groups.is_empty() {
        output.push_str("No environment variables matched\n");
    }
    for (category, items) in groups {
        let _ = writeln!(output, "{category}:");
        for (key, value) in items.iter().take(ENV_GROUP_ITEM_LIMIT) {
            let _ = writeln!(output, "  {key}={value}");
        }
        if items.len() > ENV_GROUP_ITEM_LIMIT {
            let _ = writeln!(output, "  ... +{} more", items.len() - ENV_GROUP_ITEM_LIMIT);
        }
    }
    output
}
// END_render_env

// START_CONTRACT_env_category
// PURPOSE: Group environment variables into compact sections
// INPUTS: { key: &str }
// OUTPUTS: { &'static str }
// START_env_category
fn env_category(key: &str) -> &'static str {
    let key = key.to_ascii_uppercase();
    if key.contains("PATH") {
        "paths"
    } else if matches_any(
        &key,
        &["RUST", "CARGO", "PYTHON", "NODE", "NPM", "GO", "JAVA"],
    ) {
        "language"
    } else if matches_any(&key, &["AWS", "AZURE", "GCP", "DOCKER", "KUBE", "HELM"]) {
        "cloud"
    } else if matches_any(
        &key,
        &["SHELL", "TERM", "EDITOR", "GIT", "SSH", "XDG", "SYNAPSE"],
    ) {
        "tools"
    } else {
        "other"
    }
}
// END_env_category

// START_CONTRACT_is_interesting_env
// PURPOSE: Decide whether an unfiltered environment variable is useful enough to show
// INPUTS: { key: &str }
// OUTPUTS: { bool }
// START_is_interesting_env
fn is_interesting_env(key: &str) -> bool {
    let key = key.to_ascii_uppercase();
    matches_any(
        &key,
        &[
            "PATH", "HOME", "USER", "LANG", "LC_", "TZ", "PWD", "RUST", "CARGO", "PYTHON", "NODE",
            "NPM", "GO", "JAVA", "SHELL", "TERM", "EDITOR", "GIT", "SSH", "XDG", "SYNAPSE",
            "OPENCODE", "CLAUDE",
        ],
    )
}
// END_is_interesting_env

// START_CONTRACT_is_sensitive_env
// PURPOSE: Detect environment variable names that should be masked by default
// INPUTS: { key: &str }
// OUTPUTS: { bool }
// START_is_sensitive_env
fn is_sensitive_env(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    matches_any(
        &key,
        &[
            "key",
            "secret",
            "password",
            "token",
            "credential",
            "auth",
            "private",
            "api_key",
            "apikey",
            "access",
            "jwt",
        ],
    )
}
// END_is_sensitive_env

// START_CONTRACT_matches_any
// PURPOSE: Check whether text contains any marker
// INPUTS: { text: &str }, { markers: &[&str] }
// OUTPUTS: { bool }
// START_matches_any
fn matches_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}
// END_matches_any

// START_CONTRACT_mask_env_value
// PURPOSE: Mask sensitive values while preserving tiny debugging hints
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_mask_env_value
fn mask_env_value(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= SHORT_SENSITIVE_VALUE_MAX_CHARS {
        return "****".into();
    }
    let prefix = chars.iter().take(2).collect::<String>();
    let suffix = chars
        .iter()
        .skip(chars.len().saturating_sub(2))
        .collect::<String>();
    format!("{prefix}****{suffix}")
}
// END_mask_env_value

// START_CONTRACT_truncate_env_value
// PURPOSE: Bound long environment values without splitting UTF-8
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_truncate_env_value
fn truncate_env_value(value: &str) -> String {
    if value.chars().count() <= ENV_VALUE_INLINE_LIMIT {
        return value.into();
    }
    format!(
        "{}... ({} chars)",
        truncate_chars(value, ENV_VALUE_PREVIEW_CHARS),
        value.chars().count()
    )
}
// END_truncate_env_value

// START_WcMode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WcMode {
    Full,
    Lines,
    Words,
    Bytes,
    Chars,
    Mixed,
}
// END_WcMode

// START_WcReport
struct WcReport {
    raw: String,
    output: String,
}
// END_WcReport

// START_WcStats
#[derive(Debug, Clone, PartialEq, Eq)]
struct WcStats {
    path: Option<PathBuf>,
    lines: usize,
    words: usize,
    bytes: usize,
    chars: usize,
}
// END_WcStats

// START_CONTRACT_detect_wc_mode
// PURPOSE: Detect which count columns are requested from wc-compatible flags
// INPUTS: { args: &[String] }
// OUTPUTS: { WcMode }
// START_detect_wc_mode
fn detect_wc_mode(args: &[String]) -> WcMode {
    let mut flags = Vec::new();
    for arg in args
        .iter()
        .filter(|arg| arg.starts_with('-') && *arg != "-")
    {
        for ch in arg.chars().skip(1) {
            match ch {
                'l' | 'w' | 'c' | 'm' => flags.push(ch),
                _ => {}
            }
        }
    }
    flags.sort_unstable();
    flags.dedup();
    match flags.as_slice() {
        [] => WcMode::Full,
        ['l'] => WcMode::Lines,
        ['w'] => WcMode::Words,
        ['c'] => WcMode::Bytes,
        ['m'] => WcMode::Chars,
        _ => WcMode::Mixed,
    }
}
// END_detect_wc_mode

// START_CONTRACT_wc_files
// PURPOSE: Extract file paths from wc-compatible trailing args
// INPUTS: { args: &[String] }
// OUTPUTS: { Vec<String> }
// START_wc_files
fn wc_files(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|arg| !arg.starts_with('-') || *arg == "-")
        .cloned()
        .collect()
}
// END_wc_files

// START_CONTRACT_render_wc
// PURPOSE: Count one or more files or stdin and render compact wc output
// INPUTS: { files: &[String] }, { mode: &WcMode }
// OUTPUTS: { anyhow::Result<WcReport> }
// START_render_wc
fn render_wc(files: &[String], mode: &WcMode) -> anyhow::Result<WcReport> {
    let mut raw = String::new();
    let mut stats = Vec::new();
    if files.is_empty() {
        let mut content = String::new();
        std::io::stdin()
            .lock()
            .read_to_string(&mut content)
            .context("read stdin")?;
        raw.push_str(&content);
        stats.push(count_text(None, &content));
    } else {
        for file in files {
            let content = if file == "-" {
                let mut content = String::new();
                std::io::stdin()
                    .lock()
                    .read_to_string(&mut content)
                    .context("read stdin")?;
                content
            } else {
                std::fs::read_to_string(file).with_context(|| format!("read {file}"))?
            };
            raw.push_str(&content);
            if !raw.ends_with('\n') {
                raw.push('\n');
            }
            stats.push(count_text(
                if file == "-" {
                    None
                } else {
                    Some(PathBuf::from(file))
                },
                &content,
            ));
        }
    }
    let output = format_wc_stats(&stats, mode);
    Ok(WcReport { raw, output })
}
// END_render_wc

// START_CONTRACT_count_text
// PURPOSE: Count lines, words, bytes, and chars in text using wc-compatible newline line counts
// INPUTS: { path: Option<PathBuf> }, { content: &str }
// OUTPUTS: { WcStats }
// START_count_text
fn count_text(path: Option<PathBuf>, content: &str) -> WcStats {
    WcStats {
        path,
        lines: content
            .as_bytes()
            .iter()
            .filter(|byte| **byte == b'\n')
            .count(),
        words: content.split_whitespace().count(),
        bytes: content.len(),
        chars: content.chars().count(),
    }
}
// END_count_text

// START_CONTRACT_format_wc_stats
// PURPOSE: Render compact single-file, multi-file, and total wc stats
// INPUTS: { stats: &[WcStats] }, { mode: &WcMode }
// OUTPUTS: { String }
// START_format_wc_stats
fn format_wc_stats(stats: &[WcStats], mode: &WcMode) -> String {
    if stats.is_empty() {
        return String::new();
    }
    if stats.len() == 1 {
        return format_wc_line(&stats[0], mode, false);
    }
    let mut lines = stats
        .iter()
        .map(|item| format_wc_line(item, mode, true))
        .collect::<Vec<_>>();
    let total = stats.iter().fold(
        WcStats {
            path: Some(PathBuf::from("total")),
            lines: 0,
            words: 0,
            bytes: 0,
            chars: 0,
        },
        |mut total, item| {
            total.lines += item.lines;
            total.words += item.words;
            total.bytes += item.bytes;
            total.chars += item.chars;
            total
        },
    );
    lines.push(format_wc_line(&total, mode, true));
    lines.join("\n")
}
// END_format_wc_stats

// START_CONTRACT_format_wc_line
// PURPOSE: Render one compact wc stat row
// INPUTS: { stats: &WcStats }, { mode: &WcMode }, { include_name: bool }
// OUTPUTS: { String }
// START_format_wc_line
fn format_wc_line(stats: &WcStats, mode: &WcMode, include_name: bool) -> String {
    let body = match mode {
        WcMode::Full => format!("{}L {}W {}B", stats.lines, stats.words, stats.bytes),
        WcMode::Lines => stats.lines.to_string(),
        WcMode::Words => stats.words.to_string(),
        WcMode::Bytes => stats.bytes.to_string(),
        WcMode::Chars => stats.chars.to_string(),
        WcMode::Mixed => format!(
            "{}L {}W {}B {}C",
            stats.lines, stats.words, stats.bytes, stats.chars
        ),
    };
    if include_name {
        format!("{body} {}", wc_display_name(stats))
    } else {
        body
    }
}
// END_format_wc_line

// START_CONTRACT_wc_display_name
// PURPOSE: Render a compact file display name for wc multi-file output
// INPUTS: { stats: &WcStats }
// OUTPUTS: { String }
// START_wc_display_name
fn wc_display_name(stats: &WcStats) -> String {
    stats
        .path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("stdin")
        .to_string()
}
// END_wc_display_name

// START_CONTRACT_record_adapter_savings
// PURPOSE: Persist route-aware savings for direct RTK adapters without failing user output on tracking degradation
// INPUTS: { config: &Config }, { command: &str }, { raw: &str }, { output: &str }, { adapter: &str }, { route_key: &str }
// OUTPUTS: { () }
// SIDE_EFFECTS: may write tracking database
// START_record_adapter_savings
pub(super) async fn record_adapter_savings(
    config: &Config,
    command: &str,
    raw: &str,
    output: &str,
    adapter: &str,
    route_key: &str,
) {
    let input_tokens = crate::utils::estimate_tokens(raw);
    let output_tokens = crate::utils::estimate_tokens(output);
    if let Err(error) = Tracker::new(config)
        .record_routed(command, input_tokens, output_tokens, adapter, route_key)
        .await
    {
        tracing::warn!(
            "[RtkAdapters][record_adapter_savings][TRACKING] token tracking degraded: {}",
            error
        );
    }
}
// END_record_adapter_savings

// START_CONTRACT_truncate_chars
// PURPOSE: Truncate a string by Unicode scalar count and append an ASCII omission marker
// INPUTS: { value: &str }, { max_chars: usize }
// OUTPUTS: { String }
// START_truncate_chars
fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.into();
    }
    let mut output = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    output.push_str("...");
    output
}
// END_truncate_chars

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_schema_hides_values_and_reports_array_count() {
        let raw = r#"{"name":"demo","items":[{"url":"https://example.test","count":3},{"url":"x","count":4}]}"#;
        let rendered = render_json(raw, 4, true).unwrap();

        assert!(rendered.contains("name: string"));
        assert!(rendered.contains("items:"));
        assert!(rendered.contains("(2)"));
        assert!(!rendered.contains("demo"));
    }

    #[test]
    fn deps_summary_reads_cargo_dependencies() {
        let rendered = summarize_cargo_manifest(
            r#"
[dependencies]
serde = "1"
tokio = { version = "1", features = ["full"] }
[dev-dependencies]
tempfile = "3"
"#,
        )
        .unwrap();

        assert!(rendered.contains("dependencies: 2"));
        assert!(rendered.contains("serde 1"));
        assert!(rendered.contains("tempfile 3"));
    }

    #[test]
    fn env_masks_sensitive_values() {
        let marker = format!("{}{}", "sec", "ret");
        let key = format!("SYNAPSE_{}", marker.to_ascii_uppercase());
        let value = "sample-fixture-value";
        let vars = vec![(key.clone(), value.to_string())];
        let rendered = render_env(&vars, Some(&marker), false);

        assert!(rendered.contains(&format!("{key}=sa****ue")));
        assert!(!rendered.contains(value));
    }

    #[test]
    fn wc_counts_and_formats_single_file() {
        let stats = count_text(Some(PathBuf::from("sample.txt")), "one two\nthree\n");

        assert_eq!(stats.lines, 2);
        assert_eq!(stats.words, 3);
        assert_eq!(format_wc_stats(&[stats], &WcMode::Full), "2L 3W 14B");
    }

    #[test]
    fn wc_detects_mixed_flags() {
        let args = vec!["-lw".to_string(), "file.txt".to_string()];

        assert_eq!(detect_wc_mode(&args), WcMode::Mixed);
        assert_eq!(wc_files(&args), vec!["file.txt"]);
    }
}
