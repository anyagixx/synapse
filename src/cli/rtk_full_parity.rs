// MODULE_CONTRACT
// MODULE_ID: M-RTK-FULL-PARITY
// PURPOSE: Full RTK parity matrix — classifies source-derived rtk-develop command, hook, and command-module coverage in Synapse
// SCOPE: FullRtkParityReport models, rtk-develop enum parsing, command-module inventory, Synapse CLI inventory parsing, coverage classification, and text/JSON rendering
// DEPENDS: M-CLI, M-CLI-RTK-COMMANDS, M-PROXY-FILTER, M-PROXY-ROUTER, M-HOOKS
// LINKS:
//   -> M-CLI (depends) - exposes the --full parity flag through RtkParityCmd
//   -> M-CLI-RTK-COMMANDS (depends) - owns RTK command surfaces being classified
//   -> M-HOOKS (depends) - owns hook processor and install/audit surfaces
//   -> Phase-51 (implements) - full RTK parity matrix
//   -> NFR-003 (traces_to) - parity gates protect token-saving coverage

// START_MODULE_MAP
// FullRtkParityReport - Machine-readable full RTK parity report
// build_full_rtk_parity_report - Builds the source-derived parity matrix
// print_full_rtk_parity_report - Renders compact text output
// source_top_level_commands - Parses rtk-develop Commands enum
// source_hook_processors - Parses rtk-develop HookCommands enum
// source_command_modules - Lists rtk-develop command-module files
// classify_coverage - Splits source items into implemented, equivalent, superseded, and missing groups
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.2.0 - Closed command-module parity with .NET artifact adapters and smart local-llm equivalence]
// END_CHANGE_SUMMARY

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const SYNAPSE_CLI_RS: &str = include_str!("../cli.rs");
const RTK_MAIN_RS: &str = "src/main.rs";
const RTK_CMDS_DIR: &str = "src/cmds";

// START_public_api

// START_FullRtkParityReport
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct FullRtkParityReport {
    pub targeted: bool,
    pub full_standalone_parity: bool,
    pub source_path: Option<String>,
    pub missing_total: usize,
    pub covered_total: usize,
    pub sections: Vec<FullRtkParitySection>,
}
// END_FullRtkParityReport

// START_FullRtkParitySection
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct FullRtkParitySection {
    pub name: String,
    pub status: String,
    pub source_count: usize,
    pub covered_count: usize,
    pub implemented: Vec<String>,
    pub equivalent: Vec<String>,
    pub superseded: Vec<String>,
    pub missing: Vec<String>,
    pub notes: Vec<String>,
}
// END_FullRtkParitySection

// START_CONTRACT_build_full_rtk_parity_report
// PURPOSE: Build a source-derived full RTK parity report for commands, hook processors, and command-module files
// INPUTS: { source: Option<&Path> - resolved rtk-develop source root }
// OUTPUTS: { anyhow::Result<FullRtkParityReport> }
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - command surfaces being classified
//   -> Phase-51 (implements) - full parity matrix
//   -> NFR-003 (traces_to) - token-saving coverage gate
// START_build_full_rtk_parity_report
pub(crate) fn build_full_rtk_parity_report(
    source: Option<&Path>,
) -> anyhow::Result<FullRtkParityReport> {
    let Some(source) = source else {
        return Ok(FullRtkParityReport {
            targeted: false,
            full_standalone_parity: false,
            source_path: None,
            missing_total: 0,
            covered_total: 0,
            sections: vec![unknown_section("source-commands")],
        });
    };

    let synapse_commands = synapse_top_level_commands();
    let synapse_modules = synapse_specialized_modules();
    let source_commands = source_top_level_commands(source)?;
    let source_hooks = source_hook_processors(source)?;
    let source_modules = source_command_modules(source)?;

    let sections = vec![
        command_section(&source_commands, &synapse_commands),
        hook_section(&source_hooks),
        command_module_section(&source_modules, &synapse_modules),
    ];
    let missing_total = sections.iter().map(|section| section.missing.len()).sum();
    let covered_total = sections.iter().map(|section| section.covered_count).sum();
    let full_standalone_parity = !sections.is_empty()
        && sections
            .iter()
            .all(|section| section.status == "pass" || section.status == "not-applicable");

    Ok(FullRtkParityReport {
        targeted: true,
        full_standalone_parity,
        source_path: Some(source.display().to_string()),
        missing_total,
        covered_total,
        sections,
    })
}
// END_build_full_rtk_parity_report

// START_CONTRACT_print_full_rtk_parity_report
// PURPOSE: Render the full RTK parity matrix in compact text form
// INPUTS: { report: &FullRtkParityReport }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes stdout
// LINKS:
//   -> M-CLI (depends) - text output for rtk-parity --full
//   -> NFR-003 (traces_to) - developer-visible parity diagnostics
// START_print_full_rtk_parity_report
pub(crate) fn print_full_rtk_parity_report(report: &FullRtkParityReport) {
    println!("Full RTK parity matrix");
    println!(
        "full-targeted: {}",
        if report.targeted { "yes" } else { "no" }
    );
    println!(
        "full-standalone-parity: {}",
        if report.full_standalone_parity {
            "pass"
        } else {
            "partial"
        }
    );
    println!("missing-total: {}", report.missing_total);
    println!("covered-total: {}", report.covered_total);
    for section in &report.sections {
        println!(
            "- {}: {} source={} covered={} missing={}",
            section.name,
            section.status,
            section.source_count,
            section.covered_count,
            section.missing.len()
        );
        print_named_items("implemented", &section.implemented);
        print_named_items("equivalent", &section.equivalent);
        print_named_items("superseded", &section.superseded);
        print_named_items("missing", &section.missing);
        for note in &section.notes {
            println!("  note: {note}");
        }
    }
}
// END_print_full_rtk_parity_report

// END_public_api

// START_CONTRACT_command_section
// PURPOSE: Build full parity coverage for RTK top-level commands
// INPUTS: { source: &BTreeSet<String> }, { synapse: &BTreeSet<String> }
// OUTPUTS: { FullRtkParitySection }
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - command inventory
//   -> Phase-51 (implements) - command parity section
// START_command_section
fn command_section(source: &BTreeSet<String>, synapse: &BTreeSet<String>) -> FullRtkParitySection {
    let coverage = classify_coverage(source, synapse, command_equivalents(), command_superseded());
    coverage.into_section(
        "source-commands",
        source.len(),
        vec![
            "Top-level RTK commands are parsed from rtk-develop src/main.rs Commands enum.".into(),
            "Equivalent means Synapse exposes the behavior under a different command path.".into(),
        ],
    )
}
// END_command_section

// START_CONTRACT_hook_section
// PURPOSE: Build full parity coverage for RTK hook processors
// INPUTS: { source_hooks: &BTreeSet<String> }
// OUTPUTS: { FullRtkParitySection }
// LINKS:
//   -> M-HOOKS (depends) - hook processors and install/audit surfaces
//   -> Phase-55 (traces_to) - future full hook parity work
// START_hook_section
fn hook_section(source_hooks: &BTreeSet<String>) -> FullRtkParitySection {
    let implemented = BTreeSet::from([
        "hook check".to_string(),
        "hook claude".to_string(),
        "hook copilot".to_string(),
        "hook cursor".to_string(),
        "hook gemini".to_string(),
    ]);
    let coverage = classify_coverage(source_hooks, &implemented, BTreeMap::new(), BTreeMap::new());
    coverage.into_section(
        "hook-processors",
        source_hooks.len(),
        vec!["Hook processors are parsed from rtk-develop HookCommands enum.".into()],
    )
}
// END_hook_section

// START_CONTRACT_command_module_section
// PURPOSE: Build full parity coverage for source RTK command-module files
// INPUTS: { source_modules: &BTreeSet<String> }, { synapse_modules: &BTreeSet<String> }
// OUTPUTS: { FullRtkParitySection }
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - specialized adapters
//   -> Phase-53 (traces_to) - core adapter parity
//   -> Phase-54 (traces_to) - ecosystem adapter parity
// START_command_module_section
fn command_module_section(
    source_modules: &BTreeSet<String>,
    synapse_modules: &BTreeSet<String>,
) -> FullRtkParitySection {
    let coverage = classify_coverage(
        source_modules,
        synapse_modules,
        module_equivalents(),
        module_superseded(),
    );
    coverage.into_section(
        "command-modules",
        source_modules.len(),
        vec![
            "RTK command modules are source files below rtk-develop src/cmds, excluding mod.rs.".into(),
            "This section intentionally distinguishes proxy shortcuts from specialized parser modules.".into(),
        ],
    )
}
// END_command_module_section

// START_CoverageBuckets
#[derive(Debug, Default)]
struct CoverageBuckets {
    implemented: Vec<String>,
    equivalent: Vec<String>,
    superseded: Vec<String>,
    missing: Vec<String>,
}
// END_CoverageBuckets

impl CoverageBuckets {
    // START_CONTRACT_CoverageBuckets::covered_count
    // PURPOSE: Count non-missing coverage entries
    // OUTPUTS: { usize }
    // LINKS:
    //   -> M-RTK-FULL-PARITY (depends) - full parity accounting
    //   -> NFR-003 (traces_to) - coverage metric
    // START_coverage_buckets_covered_count
    fn covered_count(&self) -> usize {
        self.implemented.len() + self.equivalent.len() + self.superseded.len()
    }
    // END_coverage_buckets_covered_count

    // START_CONTRACT_CoverageBuckets::into_section
    // PURPOSE: Convert coverage buckets into a serializable full parity section
    // INPUTS: { name: &str }, { source_count: usize }, { notes: Vec<String> }
    // OUTPUTS: { FullRtkParitySection }
    // LINKS:
    //   -> M-RTK-FULL-PARITY (depends) - full parity report rendering
    //   -> NFR-003 (traces_to) - parity section status
    // START_coverage_buckets_into_section
    fn into_section(
        self,
        name: &str,
        source_count: usize,
        notes: Vec<String>,
    ) -> FullRtkParitySection {
        let covered_count = self.covered_count();
        let status = if source_count == 0 {
            "not-applicable"
        } else if self.missing.is_empty() {
            "pass"
        } else {
            "fail"
        }
        .to_string();
        FullRtkParitySection {
            name: name.into(),
            status,
            source_count,
            covered_count,
            implemented: self.implemented,
            equivalent: self.equivalent,
            superseded: self.superseded,
            missing: self.missing,
            notes,
        }
    }
    // END_coverage_buckets_into_section
}

// START_CONTRACT_classify_coverage
// PURPOSE: Classify each source item as implemented, equivalent, superseded, or missing
// INPUTS: { source: &BTreeSet<String> }, { implemented: &BTreeSet<String> }, { equivalents: BTreeMap<String, String> }, { superseded: BTreeMap<String, String> }
// OUTPUTS: { CoverageBuckets }
// LINKS:
//   -> M-RTK-FULL-PARITY (depends) - coverage classification
//   -> Phase-51 (implements) - full parity matrix
// START_classify_coverage
fn classify_coverage(
    source: &BTreeSet<String>,
    implemented: &BTreeSet<String>,
    equivalents: BTreeMap<String, String>,
    superseded: BTreeMap<String, String>,
) -> CoverageBuckets {
    let mut buckets = CoverageBuckets::default();
    for item in source {
        if let Some(target) = equivalents.get(item) {
            buckets.equivalent.push(format!("{item} -> {target}"));
        } else if let Some(reason) = superseded.get(item) {
            buckets.superseded.push(format!("{item} -> {reason}"));
        } else if implemented.contains(item) {
            buckets.implemented.push(item.clone());
        } else {
            buckets.missing.push(item.clone());
        }
    }
    buckets
}
// END_classify_coverage

// START_CONTRACT_source_top_level_commands
// PURPOSE: Parse rtk-develop top-level Commands enum into normalized CLI command names
// INPUTS: { source: &Path }
// OUTPUTS: { anyhow::Result<BTreeSet<String>> }
// LINKS:
//   -> M-RTK-FULL-PARITY (depends) - source-derived command inventory
//   -> NFR-003 (traces_to) - source parity accounting
// START_source_top_level_commands
fn source_top_level_commands(source: &Path) -> anyhow::Result<BTreeSet<String>> {
    let content = std::fs::read_to_string(source.join(RTK_MAIN_RS))?;
    Ok(parse_enum_variants(&content, "Commands")
        .into_iter()
        .collect())
}
// END_source_top_level_commands

// START_CONTRACT_source_hook_processors
// PURPOSE: Parse rtk-develop HookCommands enum into normalized hook processor command names
// INPUTS: { source: &Path }
// OUTPUTS: { anyhow::Result<BTreeSet<String>> }
// LINKS:
//   -> M-HOOKS (depends) - hook processor coverage
//   -> Phase-55 (traces_to) - full hook parity
// START_source_hook_processors
fn source_hook_processors(source: &Path) -> anyhow::Result<BTreeSet<String>> {
    let content = std::fs::read_to_string(source.join(RTK_MAIN_RS))?;
    Ok(parse_enum_variants(&content, "HookCommands")
        .into_iter()
        .map(|name| format!("hook {name}"))
        .collect())
}
// END_source_hook_processors

// START_CONTRACT_source_command_modules
// PURPOSE: List normalized rtk-develop command-module file stems below src/cmds
// INPUTS: { source: &Path }
// OUTPUTS: { anyhow::Result<BTreeSet<String>> }
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - specialized adapter parity
//   -> Phase-53 (traces_to) - core adapter porting
//   -> Phase-54 (traces_to) - ecosystem adapter porting
// START_source_command_modules
fn source_command_modules(source: &Path) -> anyhow::Result<BTreeSet<String>> {
    let root = source.join(RTK_CMDS_DIR);
    if !root.exists() {
        return Ok(BTreeSet::new());
    }
    let mut modules = BTreeSet::new();
    collect_rs_file_stems(&root, &mut modules)?;
    Ok(modules)
}
// END_source_command_modules

// START_CONTRACT_synapse_top_level_commands
// PURPOSE: Parse Synapse top-level Command enum into normalized CLI command names
// OUTPUTS: { BTreeSet<String> }
// LINKS:
//   -> M-CLI (depends) - source-derived Synapse command inventory
//   -> M-RTK-FULL-PARITY (depends) - coverage classification
// START_synapse_top_level_commands
fn synapse_top_level_commands() -> BTreeSet<String> {
    parse_enum_variants(SYNAPSE_CLI_RS, "Command")
        .into_iter()
        .collect()
}
// END_synapse_top_level_commands

// START_CONTRACT_synapse_specialized_modules
// PURPOSE: Return Synapse command surfaces that are direct specialized RTK adapters rather than thin proxy shortcuts
// OUTPUTS: { BTreeSet<String> }
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - local specialized adapter inventory
//   -> Phase-51 (implements) - command-module parity baseline
// START_synapse_specialized_modules
fn synapse_specialized_modules() -> BTreeSet<String> {
    [
        "binlog",
        "deps",
        "diff-cmd",
        "dotnet-format-report",
        "dotnet-trx",
        "env",
        "err",
        "format-cmd",
        "json",
        "lint-cmd",
        "log",
        "pipe",
        "runner",
        "smart",
        "summary",
        "test",
        "wc",
        "discover",
        "learn",
        "session",
        "cc-economics",
        "rewrite",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}
// END_synapse_specialized_modules

// START_CONTRACT_parse_enum_variants
// PURPOSE: Parse top-level Rust enum variant names with optional clap command name attributes
// INPUTS: { content: &str }, { enum_name: &str }
// OUTPUTS: { Vec<String> }
// LINKS:
//   -> M-RTK-FULL-PARITY (depends) - bounded source parsing
//   -> Phase-51 (implements) - source-derived inventory
// START_parse_enum_variants
fn parse_enum_variants(content: &str, enum_name: &str) -> Vec<String> {
    let mut variants = Vec::new();
    let mut in_enum = false;
    let mut depth = 0_i32;
    let mut pending_command_name: Option<String> = None;
    let enum_marker = format!("enum {enum_name}");

    for line in content.lines() {
        if !in_enum {
            if line.contains(&enum_marker) {
                in_enum = true;
                depth += brace_delta(line);
            }
            continue;
        }

        if let Some(name) = parse_command_name_attr(line) {
            pending_command_name = Some(name);
        }

        if depth == 1 {
            if let Some(variant) = parse_variant_name(line) {
                let name = pending_command_name
                    .take()
                    .unwrap_or_else(|| camel_to_kebab(&variant));
                variants.push(name);
            }
        }

        depth += brace_delta(line);
        if depth <= 0 {
            break;
        }
    }

    variants.sort();
    variants.dedup();
    variants
}
// END_parse_enum_variants

// START_CONTRACT_collect_rs_file_stems
// PURPOSE: Recursively collect normalized Rust file stems below a directory, excluding mod.rs
// INPUTS: { root: &Path }, { modules: &mut BTreeSet<String> }
// OUTPUTS: { anyhow::Result<()> }
// LINKS:
//   -> M-RTK-FULL-PARITY (depends) - source command-module inventory
//   -> Phase-51 (implements) - full parity matrix
// START_collect_rs_file_stems
fn collect_rs_file_stems(root: &Path, modules: &mut BTreeSet<String>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rs_file_stems(&path, modules)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if stem == "mod" {
            continue;
        }
        modules.insert(stem.replace('_', "-"));
    }
    Ok(())
}
// END_collect_rs_file_stems

// START_CONTRACT_command_equivalents
// PURPOSE: Return explicit RTK command names whose behavior is provided by a different Synapse command path
// OUTPUTS: { BTreeMap<String, String> }
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - command equivalence policy
//   -> Phase-51 (implements) - explicit parity classification
// START_command_equivalents
fn command_equivalents() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("cat".into(), "read".into()),
        ("ripgrep".into(), "rg".into()),
        ("github".into(), "gh".into()),
        ("git-hub".into(), "gh".into()),
        ("kubernetes".into(), "kubectl".into()),
        ("verify".into(), "filters verify".into()),
        ("trust".into(), "filters trust".into()),
        ("untrust".into(), "filters untrust".into()),
        ("hook-audit".into(), "hooks audit".into()),
        ("golangci-lint".into(), "golangci".into()),
    ])
}
// END_command_equivalents

// START_CONTRACT_command_superseded
// PURPOSE: Return RTK command names intentionally superseded by Synapse platform policy
// OUTPUTS: { BTreeMap<String, String> }
// LINKS:
//   -> M-RTK-FULL-PARITY (depends) - explicit supersedence classification
//   -> Phase-56 (traces_to) - economics and history parity policy
// START_command_superseded
fn command_superseded() -> BTreeMap<String, String> {
    BTreeMap::from([(
        "telemetry".into(),
        "Synapse keeps local token economics and does not emit telemetry by default".into(),
    )])
}
// END_command_superseded

// START_CONTRACT_module_equivalents
// PURPOSE: Return RTK command-module filenames covered by differently named Synapse adapters
// OUTPUTS: { BTreeMap<String, String> }
// LINKS:
//   -> M-CLI-RTK-COMMANDS (depends) - specialized adapter equivalence policy
//   -> Phase-51 (implements) - command-module parity classification
// START_module_equivalents
fn module_equivalents() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "aws-cmd".into(),
            "aws proxy shortcut plus cloud-data router".into(),
        ),
        (
            "cargo-cmd".into(),
            "cargo proxy shortcut plus rust-cargo router".into(),
        ),
        ("cat".into(), "read proxy shortcut plus cat filter".into()),
        (
            "container".into(),
            "docker/podman proxy shortcuts plus container filters".into(),
        ),
        (
            "curl-cmd".into(),
            "curl proxy shortcut plus cloud-data router".into(),
        ),
        (
            "dotnet-cmd".into(),
            "dotnet proxy shortcut plus language-tooling router".into(),
        ),
        ("env-cmd".into(), "env".into()),
        (
            "find-cmd".into(),
            "find proxy shortcut plus search router".into(),
        ),
        (
            "gh-cmd".into(),
            "gh proxy shortcut plus vcs-hosting router".into(),
        ),
        (
            "git".into(),
            "git proxy shortcut plus vcs-git router".into(),
        ),
        (
            "glab-cmd".into(),
            "glab proxy shortcut plus vcs-hosting router".into(),
        ),
        (
            "go-cmd".into(),
            "go proxy shortcut plus go-tooling router".into(),
        ),
        (
            "golangci-cmd".into(),
            "golangci proxy shortcut plus go-tooling router".into(),
        ),
        (
            "gradlew-cmd".into(),
            "gradlew proxy shortcut plus build-tool router".into(),
        ),
        (
            "grep-cmd".into(),
            "grep proxy shortcut plus grep filter".into(),
        ),
        (
            "gt-cmd".into(),
            "gt proxy shortcut plus vcs-graphite router".into(),
        ),
        ("json-cmd".into(), "json".into()),
        ("ls".into(), "ls proxy shortcut plus system router".into()),
        ("log-cmd".into(), "log".into()),
        (
            "local-llm".into(),
            "smart heuristic source summarizer".into(),
        ),
        (
            "mypy-cmd".into(),
            "mypy proxy shortcut plus python-tooling router".into(),
        ),
        (
            "next-cmd".into(),
            "next proxy shortcut plus js-tooling router".into(),
        ),
        (
            "npm-cmd".into(),
            "npm proxy shortcut plus js-tooling router".into(),
        ),
        (
            "pip-cmd".into(),
            "pip proxy shortcut plus python-tooling router".into(),
        ),
        ("pipe-cmd".into(), "pipe".into()),
        (
            "playwright-cmd".into(),
            "playwright proxy shortcut plus js-tooling router".into(),
        ),
        (
            "pnpm-cmd".into(),
            "pnpm proxy shortcut plus js-tooling router".into(),
        ),
        (
            "prettier-cmd".into(),
            "prettier proxy shortcut plus js-tooling router".into(),
        ),
        (
            "prisma-cmd".into(),
            "prisma proxy shortcut plus js-tooling router".into(),
        ),
        (
            "psql-cmd".into(),
            "psql proxy shortcut plus cloud-data router".into(),
        ),
        (
            "pytest-cmd".into(),
            "pytest proxy shortcut plus python-pytest router".into(),
        ),
        (
            "rake-cmd".into(),
            "rake proxy shortcut plus language-tooling router".into(),
        ),
        ("read".into(), "read proxy shortcut plus cat filter".into()),
        (
            "rg".into(),
            "rg proxy shortcut plus grep/ripgrep filters".into(),
        ),
        (
            "ripgrep".into(),
            "rg proxy shortcut plus grep/ripgrep filters".into(),
        ),
        (
            "rspec-cmd".into(),
            "rspec proxy shortcut plus language-tooling router".into(),
        ),
        (
            "rubocop-cmd".into(),
            "rubocop proxy shortcut plus language-tooling router".into(),
        ),
        (
            "ruff-cmd".into(),
            "ruff proxy shortcut plus python-tooling router".into(),
        ),
        (
            "tree".into(),
            "tree proxy shortcut plus system router".into(),
        ),
        (
            "tsc-cmd".into(),
            "tsc proxy shortcut plus js-tooling router".into(),
        ),
        ("wc-cmd".into(), "wc".into()),
        (
            "vitest-cmd".into(),
            "vitest proxy shortcut plus js-tooling router".into(),
        ),
        (
            "wget-cmd".into(),
            "wget proxy shortcut plus cloud-data router".into(),
        ),
        ("verify".into(), "filters verify".into()),
        ("trust".into(), "filters trust".into()),
        ("untrust".into(), "filters untrust".into()),
    ])
}
// END_module_equivalents

// START_CONTRACT_module_superseded
// PURPOSE: Return RTK command-module filenames superseded by Synapse platform policy
// OUTPUTS: { BTreeMap<String, String> }
// LINKS:
//   -> M-RTK-FULL-PARITY (depends) - explicit supersedence classification
//   -> Phase-56 (traces_to) - local economics policy
// START_module_superseded
fn module_superseded() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "constants".into(),
            "shared command constants are centralized in Synapse router, filter, and capability registries".into(),
        ),
        (
            "telemetry".into(),
            "local-only Synapse economics policy".into(),
        ),
    ])
}
// END_module_superseded

// START_CONTRACT_unknown_section
// PURPOSE: Build a not-resolved full parity section when no RTK source tree is available
// INPUTS: { name: &str }
// OUTPUTS: { FullRtkParitySection }
// LINKS:
//   -> M-RTK-FULL-PARITY (depends) - no-source diagnostics
//   -> NFR-003 (traces_to) - bounded diagnostic output
// START_unknown_section
fn unknown_section(name: &str) -> FullRtkParitySection {
    FullRtkParitySection {
        name: name.into(),
        status: "unknown".into(),
        source_count: 0,
        covered_count: 0,
        implemented: Vec::new(),
        equivalent: Vec::new(),
        superseded: Vec::new(),
        missing: Vec::new(),
        notes: vec![
            "No rtk-develop source path was resolved; pass --source for full parity.".into(),
        ],
    }
}
// END_unknown_section

// START_CONTRACT_parse_command_name_attr
// PURPOSE: Extract a clap command name override from an attribute line
// INPUTS: { line: &str }
// OUTPUTS: { Option<String> }
// LINKS:
//   -> M-RTK-FULL-PARITY (depends) - Rust enum parser
//   -> Phase-51 (implements) - source-derived command names
// START_parse_command_name_attr
fn parse_command_name_attr(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with("#[command") || !trimmed.contains("name") {
        return None;
    }
    let name_index = trimmed.find("name")?;
    let after_name = &trimmed[name_index..];
    let first_quote = after_name.find('"')?;
    let rest = &after_name[first_quote + 1..];
    let second_quote = rest.find('"')?;
    Some(rest[..second_quote].to_string())
}
// END_parse_command_name_attr

// START_CONTRACT_parse_variant_name
// PURPOSE: Extract a Rust enum variant identifier from a top-level enum line
// INPUTS: { line: &str }
// OUTPUTS: { Option<String> }
// LINKS:
//   -> M-RTK-FULL-PARITY (depends) - Rust enum parser
//   -> Phase-51 (implements) - source-derived command names
// START_parse_variant_name
fn parse_variant_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let first = trimmed.chars().next()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    let name: String = trimmed
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric())
        .collect();
    (!name.is_empty()).then_some(name)
}
// END_parse_variant_name

// START_CONTRACT_brace_delta
// PURPOSE: Return the net brace delta for a Rust source line
// INPUTS: { line: &str }
// OUTPUTS: { i32 }
// LINKS:
//   -> M-RTK-FULL-PARITY (depends) - bounded enum parsing
//   -> Phase-51 (implements) - source-derived inventory
// START_brace_delta
fn brace_delta(line: &str) -> i32 {
    let opens = line.chars().filter(|ch| *ch == '{').count() as i32;
    let closes = line.chars().filter(|ch| *ch == '}').count() as i32;
    opens - closes
}
// END_brace_delta

// START_CONTRACT_camel_to_kebab
// PURPOSE: Convert a Rust enum variant name into clap-style kebab-case
// INPUTS: { name: &str }
// OUTPUTS: { String }
// LINKS:
//   -> M-RTK-FULL-PARITY (depends) - command name normalization
//   -> Phase-51 (implements) - parity inventory
// START_camel_to_kebab
fn camel_to_kebab(name: &str) -> String {
    let mut output = String::new();
    for (idx, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if idx > 0 {
                output.push('-');
            }
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push(ch);
        }
    }
    output
}
// END_camel_to_kebab

// START_CONTRACT_print_named_items
// PURPOSE: Print a labelled comma-separated item list when non-empty
// INPUTS: { label: &str }, { items: &[String] }
// OUTPUTS: { () }
// SIDE_EFFECTS: writes stdout
// LINKS:
//   -> M-CLI (depends) - human-readable parity output
//   -> NFR-003 (traces_to) - compact diagnostics
// START_print_named_items
fn print_named_items(label: &str, items: &[String]) {
    if !items.is_empty() {
        println!("  {label}: {}", items.join(", "));
    }
}
// END_print_named_items

#[cfg(test)]
mod tests {
    use super::*;

    fn write_source(main_rs: &str, command_modules: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        let cmds = src.join("cmds");
        std::fs::create_dir_all(&cmds).expect("cmds dir");
        std::fs::write(src.join("main.rs"), main_rs).expect("main rs");
        for module in command_modules {
            std::fs::write(cmds.join(format!("{module}.rs")), "").expect("module file");
        }
        dir
    }

    #[test]
    fn parse_enum_variants_honors_clap_name_overrides() {
        let content = r#"
            enum Commands {
                Ls { args: Vec<String> },
                #[command(name = "hook-audit")]
                HookAudit { since: u64 },
                CcEconomics { daily: bool },
            }
        "#;

        let variants = parse_enum_variants(content, "Commands");

        assert_eq!(variants, vec!["cc-economics", "hook-audit", "ls"]);
    }

    #[test]
    fn full_report_classifies_equivalent_superseded_and_missing_commands() {
        let source = write_source(
            r#"
            enum Commands {
                Cat { args: Vec<String> },
                Ls { args: Vec<String> },
                FutureGap {},
                Session {},
                Telemetry { command: String },
                Verify { filter: Option<String> },
            }

            enum HookCommands {
                Claude,
                Check { command: Vec<String> },
            }
            "#,
            &["json", "pytest_cmd"],
        );

        let report = build_full_rtk_parity_report(Some(source.path())).expect("report");

        assert!(report.targeted);
        assert!(!report.full_standalone_parity);
        let commands = report
            .sections
            .iter()
            .find(|section| section.name == "source-commands")
            .expect("commands");
        assert!(commands.implemented.contains(&"ls".to_string()));
        assert!(commands.equivalent.contains(&"cat -> read".to_string()));
        assert!(commands
            .equivalent
            .contains(&"verify -> filters verify".to_string()));
        assert!(commands
            .superseded
            .iter()
            .any(|entry| entry.starts_with("telemetry ->")));
        assert!(commands.implemented.contains(&"session".to_string()));
        assert!(commands.missing.contains(&"future-gap".to_string()));

        let hooks = report
            .sections
            .iter()
            .find(|section| section.name == "hook-processors")
            .expect("hooks");
        assert!(hooks.implemented.contains(&"hook check".to_string()));
        assert!(hooks.implemented.contains(&"hook claude".to_string()));

        let modules = report
            .sections
            .iter()
            .find(|section| section.name == "command-modules")
            .expect("modules");
        assert!(modules.implemented.contains(&"json".to_string()));
        assert!(modules.equivalent.contains(
            &"pytest-cmd -> pytest proxy shortcut plus python-pytest router".to_string()
        ));
    }

    #[test]
    fn full_report_passes_when_all_source_surfaces_are_covered() {
        let source = write_source(
            r#"
            enum Commands {
                Cat { args: Vec<String> },
                Ls { args: Vec<String> },
                Telemetry { command: String },
                Verify { filter: Option<String> },
            }

            enum HookCommands {
                Check { command: Vec<String> },
            }
            "#,
            &["json", "telemetry", "verify"],
        );

        let report = build_full_rtk_parity_report(Some(source.path())).expect("report");

        assert!(report.full_standalone_parity);
        assert_eq!(report.missing_total, 0);
    }
}
