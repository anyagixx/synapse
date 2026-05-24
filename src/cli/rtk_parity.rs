// MODULE_CONTRACT
// MODULE_ID: M-CLI-RTK-COMMANDS
// PURPOSE: RTK parity inventory gate — compares Synapse RTK coverage against source and planned parity domains
// SCOPE: RtkParityCmd execution, source filter inventory, Synapse filter inventory, router family gate, expanded first-class proxy shortcut inventory, compact/JSON report rendering
// DEPENDS: M-CLI, M-PROXY-FILTER, M-PROXY-ROUTER
// LINKS:
//   -> M-CLI (depends) - exposes the rtk-parity command schema
//   -> M-PROXY-FILTER (depends) - reads built-in RTK filter inventory
//   -> M-PROXY-ROUTER (depends) - reads routed adapter family catalogue
//   -> Phase-44 (implements) - machine-checkable RTK parity inventory gate
//   -> Phase-45 (implements) - expanded first-class RTK proxy shortcut parity
//   -> Phase-46 (implements) - local RTK system adapter inventory
//   -> NFR-003 (traces_to) - parity gates protect token-saving coverage

// START_MODULE_MAP
// RtkParityCmd::run - Builds and prints the parity report, failing CI on critical gaps
// build_rtk_parity_report - Produces machine-readable parity sections
// source_filter_names - Reads filter names from a rtk-develop source tree
// synapse_filter_names - Reads Synapse built-in RTK filter names
// router_family_section - Validates routed family coverage
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.2.0 - Added local RTK system adapter inventory]
// END_CHANGE_SUMMARY

use super::RtkParityCmd;
use crate::config::Config;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const SYNAPSE_RTK_FILTERS_TOML: &str = include_str!("../proxy/rtk_builtin_filters.toml");
const REQUIRED_ROUTER_FAMILIES: &[&str] = &[
    "build",
    "cloud",
    "go",
    "infrastructure",
    "javascript",
    "language",
    "logs",
    "python",
    "rust",
    "search",
    "system",
    "vcs",
];
const PROXY_SHORTCUTS: &[&str] = &[
    "read",
    "ls",
    "tree",
    "find",
    "rg",
    "grep",
    "git",
    "cargo",
    "npm",
    "pnpm",
    "npx",
    "pytest",
    "gh",
    "glab",
    "aws",
    "psql",
    "curl",
    "wget",
    "jq",
    "go",
    "golangci",
    "dotnet",
    "rake",
    "rspec",
    "rubocop",
    "gradle",
    "gradlew",
    "make",
    "just",
    "helm",
    "kubectl",
    "ruff",
    "mypy",
    "basedpyright",
    "pip",
    "uv",
    "next",
    "playwright",
    "prettier",
    "prisma",
    "tsc",
    "vitest",
];
const LOCAL_ADAPTERS: &[&str] = &["json", "deps", "env", "wc", "pipe", "log", "smart"];
const HOOKS: &[&str] = &["opencode-rewrite"];
const ANALYTICS: &[&str] = &["gain", "gain --graph", "gain --sessions", "gain --adapters"];
const PROXY_SHORTCUTS_NOTE: &str =
    "Phase-45 adds common proxy shortcuts; Phase-47 adds language ecosystem shortcuts.";
const LOCAL_ADAPTERS_NOTE: &str =
    "Phase-46 adds pipe, log, and smart local system adapters; Phase-47 evaluates additional specialized adapters.";

// START_public_api

impl RtkParityCmd {
    // START_CONTRACT_RtkParityCmd::run
    // PURPOSE: Print a compact or JSON RTK parity inventory and fail CI on critical gaps
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: reads optional rtk-develop source tree, writes stdout, may return CI failure
    // LINKS:
    //   -> M-PROXY-FILTER (depends) - validates RTK filter coverage
    //   -> M-PROXY-ROUTER (depends) - validates routed family coverage
    // START_rtk_parity_cmd_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let explicit_source = self.source.as_deref().map(Path::new);
        let report = build_rtk_parity_report(explicit_source)?;
        if self.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print_rtk_parity_report(&report);
        }
        if self.ci && !report.critical_passed {
            anyhow::bail!("RTK parity critical gate failed");
        }
        Ok(())
    }
    // END_rtk_parity_cmd_run
}

// END_public_api

// START_RtkParityReport
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RtkParityReport {
    critical_passed: bool,
    full_standalone_parity: bool,
    source_path: Option<String>,
    sections: Vec<RtkParitySection>,
}
// END_RtkParityReport

// START_RtkParitySection
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RtkParitySection {
    name: String,
    status: String,
    critical: bool,
    synapse_count: usize,
    source_count: Option<usize>,
    missing: Vec<String>,
    extra: Vec<String>,
    items: Vec<String>,
    notes: Vec<String>,
}
// END_RtkParitySection

// START_CONTRACT_build_rtk_parity_report
// PURPOSE: Build the RTK parity report from Synapse inventories and optional source-derived RTK data
// INPUTS: { explicit_source: Option<&Path> }
// OUTPUTS: { anyhow::Result<RtkParityReport> }
// START_build_rtk_parity_report
fn build_rtk_parity_report(explicit_source: Option<&Path>) -> anyhow::Result<RtkParityReport> {
    let source_path = resolve_rtk_source(explicit_source);
    let source_filters = match source_path.as_deref() {
        Some(path) => Some(source_filter_names(path)?),
        None => None,
    };
    let mut sections = vec![
        filter_parity_section(source_filters.as_ref()),
        router_family_section(),
        static_inventory_section(
            "proxy-shortcuts",
            "tracked",
            false,
            PROXY_SHORTCUTS,
            &[PROXY_SHORTCUTS_NOTE],
        ),
        static_inventory_section(
            "local-adapters",
            "tracked",
            false,
            LOCAL_ADAPTERS,
            &[LOCAL_ADAPTERS_NOTE],
        ),
        static_inventory_section(
            "hooks",
            "tracked",
            false,
            HOOKS,
            &["Phase-50 expands hook audit/trust parity."],
        ),
        static_inventory_section(
            "discovery-learn",
            "planned",
            false,
            &[],
            &["Phase-49 brings RTK discover/learn concepts into Synapse diagnostics."],
        ),
        static_inventory_section(
            "analytics",
            "tracked",
            false,
            ANALYTICS,
            &["Phase-50 extends session and adapter economics release gates."],
        ),
    ];
    if let Some(path) = source_path.as_deref() {
        annotate_source_command_count(path, &mut sections)?;
    }
    let critical_passed = sections
        .iter()
        .filter(|section| section.critical)
        .all(|section| section.status == "pass");
    Ok(RtkParityReport {
        critical_passed,
        full_standalone_parity: false,
        source_path: source_path.map(|path| path.display().to_string()),
        sections,
    })
}
// END_build_rtk_parity_report

// START_CONTRACT_resolve_rtk_source
// PURPOSE: Resolve an explicit, environment, or nearby rtk-develop source path for source-derived parity checks
// INPUTS: { explicit_source: Option<&Path> }
// OUTPUTS: { Option<PathBuf> }
// START_resolve_rtk_source
fn resolve_rtk_source(explicit_source: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit_source {
        return Some(path.to_path_buf());
    }
    if let Ok(path) = std::env::var("SYNAPSE_RTK_SOURCE") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    let current = std::env::current_dir().ok()?;
    let candidates = [
        current.join("../GRACEme/rtk-develop"),
        current.join("../rtk-develop"),
        current.join("../../GRACEme/rtk-develop"),
    ];
    candidates.into_iter().find(|path| path.exists())
}
// END_resolve_rtk_source

// START_CONTRACT_filter_parity_section
// PURPOSE: Compare Synapse built-in RTK filter names with source-derived RTK filter names
// INPUTS: { source_filters: Option<&BTreeSet<String>> }
// OUTPUTS: { RtkParitySection }
// START_filter_parity_section
fn filter_parity_section(source_filters: Option<&BTreeSet<String>>) -> RtkParitySection {
    let synapse = synapse_filter_names().unwrap_or_default();
    let Some(source) = source_filters else {
        return RtkParitySection {
            name: "filters".into(),
            status: "unknown".into(),
            critical: true,
            synapse_count: synapse.len(),
            source_count: None,
            missing: Vec::new(),
            extra: Vec::new(),
            items: synapse.into_iter().collect(),
            notes: vec![
                "No rtk-develop source path resolved; pass --source or set SYNAPSE_RTK_SOURCE."
                    .into(),
            ],
        };
    };
    let missing = set_difference(source, &synapse);
    let extra = set_difference(&synapse, source);
    RtkParitySection {
        name: "filters".into(),
        status: if missing.is_empty() { "pass" } else { "fail" }.into(),
        critical: true,
        synapse_count: synapse.len(),
        source_count: Some(source.len()),
        missing,
        extra,
        items: synapse.into_iter().collect(),
        notes: vec![
            "Critical gate: every rtk-develop src/filters/*.toml name must exist in Synapse."
                .into(),
        ],
    }
}
// END_filter_parity_section

// START_CONTRACT_router_family_section
// PURPOSE: Validate CommandRouter family coverage against the planned RTK adapter catalogue
// OUTPUTS: { RtkParitySection }
// START_router_family_section
fn router_family_section() -> RtkParitySection {
    let router = crate::proxy::router::CommandRouter::new();
    let families = router
        .supported_adapters()
        .into_iter()
        .map(|adapter| adapter.family.to_string())
        .collect::<BTreeSet<_>>();
    let required = REQUIRED_ROUTER_FAMILIES
        .iter()
        .map(|family| (*family).to_string())
        .collect::<BTreeSet<_>>();
    let missing = set_difference(&required, &families);
    let extra = set_difference(&families, &required);
    RtkParitySection {
        name: "router-families".into(),
        status: if missing.is_empty() { "pass" } else { "fail" }.into(),
        critical: true,
        synapse_count: families.len(),
        source_count: Some(required.len()),
        missing,
        extra,
        items: families.into_iter().collect(),
        notes: vec!["Critical gate: router must expose every planned RTK adapter family.".into()],
    }
}
// END_router_family_section

// START_CONTRACT_static_inventory_section
// PURPOSE: Build a non-critical parity section for tracked or planned RTK capability domains
// INPUTS: { name: &str }, { status: &str }, { critical: bool }, { items: &[&str] }, { notes: &[&str] }
// OUTPUTS: { RtkParitySection }
// START_static_inventory_section
fn static_inventory_section(
    name: &str,
    status: &str,
    critical: bool,
    items: &[&str],
    notes: &[&str],
) -> RtkParitySection {
    RtkParitySection {
        name: name.into(),
        status: status.into(),
        critical,
        synapse_count: items.len(),
        source_count: None,
        missing: Vec::new(),
        extra: Vec::new(),
        items: items.iter().map(|item| (*item).to_string()).collect(),
        notes: notes.iter().map(|note| (*note).to_string()).collect(),
    }
}
// END_static_inventory_section

// START_CONTRACT_annotate_source_command_count
// PURPOSE: Add source command-module count context to non-critical adapter sections
// INPUTS: { source: &Path }, { sections: &mut [RtkParitySection] }
// OUTPUTS: { anyhow::Result<()> }
// START_annotate_source_command_count
fn annotate_source_command_count(
    source: &Path,
    sections: &mut [RtkParitySection],
) -> anyhow::Result<()> {
    let count = source_command_module_count(source)?;
    if let Some(section) = sections
        .iter_mut()
        .find(|section| section.name == "local-adapters")
    {
        section
            .notes
            .push(format!("rtk-develop command modules observed: {count}"));
    }
    Ok(())
}
// END_annotate_source_command_count

// START_CONTRACT_source_filter_names
// PURPOSE: Read RTK source filter names from src/filters/*.toml filenames
// INPUTS: { source: &Path }
// OUTPUTS: { anyhow::Result<BTreeSet<String>> }
// START_source_filter_names
fn source_filter_names(source: &Path) -> anyhow::Result<BTreeSet<String>> {
    let filter_dir = source.join("src").join("filters");
    let entries = std::fs::read_dir(&filter_dir)
        .map_err(|err| anyhow::anyhow!("read {}: {}", filter_dir.display(), err))?;
    let mut names = BTreeSet::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
            names.insert(stem.to_string());
        }
    }
    Ok(names)
}
// END_source_filter_names

// START_CONTRACT_synapse_filter_names
// PURPOSE: Read Synapse built-in RTK filter names from embedded TOML
// OUTPUTS: { anyhow::Result<BTreeSet<String>> }
// START_synapse_filter_names
fn synapse_filter_names() -> anyhow::Result<BTreeSet<String>> {
    let parsed: toml::Value = toml::from_str(SYNAPSE_RTK_FILTERS_TOML)?;
    let Some(filters) = parsed.get("filters").and_then(toml::Value::as_table) else {
        anyhow::bail!("embedded RTK filter pack has no [filters] table");
    };
    Ok(filters.keys().cloned().collect())
}
// END_synapse_filter_names

// START_CONTRACT_source_command_module_count
// PURPOSE: Count source RTK command modules below src/cmds for inventory context
// INPUTS: { source: &Path }
// OUTPUTS: { anyhow::Result<usize> }
// START_source_command_module_count
fn source_command_module_count(source: &Path) -> anyhow::Result<usize> {
    let root = source.join("src").join("cmds");
    if !root.exists() {
        return Ok(0);
    }
    count_rs_files(&root)
}
// END_source_command_module_count

// START_CONTRACT_count_rs_files
// PURPOSE: Recursively count Rust files below a source directory, excluding module glue files
// INPUTS: { root: &Path }
// OUTPUTS: { anyhow::Result<usize> }
// START_count_rs_files
fn count_rs_files(root: &Path) -> anyhow::Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            count += count_rs_files(&path)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs")
            && path.file_name().and_then(|value| value.to_str()) != Some("mod.rs")
        {
            count += 1;
        }
    }
    Ok(count)
}
// END_count_rs_files

// START_CONTRACT_set_difference
// PURPOSE: Return sorted values present in left but not right
// INPUTS: { left: &BTreeSet<String> }, { right: &BTreeSet<String> }
// OUTPUTS: { Vec<String> }
// START_set_difference
fn set_difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}
// END_set_difference

// START_CONTRACT_print_rtk_parity_report
// PURPOSE: Render compact human-readable RTK parity report
// INPUTS: { report: &RtkParityReport }
// OUTPUTS: { stdout lines }
// SIDE_EFFECTS: writes to stdout
// START_print_rtk_parity_report
fn print_rtk_parity_report(report: &RtkParityReport) {
    println!("RTK parity inventory");
    println!(
        "critical: {}",
        if report.critical_passed {
            "pass"
        } else {
            "fail"
        }
    );
    println!(
        "source: {}",
        report.source_path.as_deref().unwrap_or("not resolved")
    );
    for section in &report.sections {
        let source = section
            .source_count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "-".into());
        println!(
            "- {}: {} synapse={} source={} critical={}",
            section.name, section.status, section.synapse_count, source, section.critical
        );
        if !section.missing.is_empty() {
            println!("  missing: {}", section.missing.join(", "));
        }
        if !section.extra.is_empty() {
            println!("  extra: {}", section.extra.join(", "));
        }
        for note in &section.notes {
            println!("  note: {note}");
        }
    }
    println!("full-standalone-parity: planned");
}
// END_print_rtk_parity_report

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parity_report_passes_when_source_filters_match_synapse() {
        let source = tempfile::tempdir().expect("tempdir");
        let filter_dir = source.path().join("src").join("filters");
        std::fs::create_dir_all(&filter_dir).expect("filter dir");
        for name in synapse_filter_names().expect("synapse filters") {
            std::fs::write(filter_dir.join(format!("{name}.toml")), "").expect("filter file");
        }

        let report = build_rtk_parity_report(Some(source.path())).expect("report");

        assert!(report.critical_passed);
        let filters = report
            .sections
            .iter()
            .find(|section| section.name == "filters")
            .expect("filters section");
        assert_eq!(filters.status, "pass");
        assert_eq!(filters.source_count, Some(filters.synapse_count));
        assert!(filters.missing.is_empty());
    }

    #[test]
    fn parity_report_detects_missing_synapse_filter() {
        let source = tempfile::tempdir().expect("tempdir");
        let filter_dir = source.path().join("src").join("filters");
        std::fs::create_dir_all(&filter_dir).expect("filter dir");
        std::fs::write(filter_dir.join("not-in-synapse.toml"), "").expect("filter file");

        let report = build_rtk_parity_report(Some(source.path())).expect("report");

        assert!(!report.critical_passed);
        let filters = report
            .sections
            .iter()
            .find(|section| section.name == "filters")
            .expect("filters section");
        assert_eq!(filters.status, "fail");
        assert_eq!(filters.missing, vec!["not-in-synapse".to_string()]);
    }

    #[test]
    fn router_family_section_covers_required_families() {
        let section = router_family_section();

        assert_eq!(section.status, "pass");
        assert!(section.missing.is_empty());
        assert!(section.items.contains(&"python".to_string()));
        assert!(section.items.contains(&"cloud".to_string()));
    }

    #[test]
    fn proxy_shortcut_inventory_covers_expanded_phase45_surface() {
        let section =
            static_inventory_section("proxy-shortcuts", "tracked", false, PROXY_SHORTCUTS, &[]);

        assert_eq!(section.synapse_count, 42);
        for shortcut in [
            "gh", "aws", "go", "golangci", "dotnet", "make", "helm", "kubectl", "ruff", "mypy",
            "uv", "tsc", "vitest", "gradlew",
        ] {
            assert!(section.items.contains(&shortcut.to_string()), "{shortcut}");
        }
    }

    #[test]
    fn local_adapter_inventory_covers_phase46_system_adapters() {
        let section =
            static_inventory_section("local-adapters", "tracked", false, LOCAL_ADAPTERS, &[]);

        assert_eq!(section.synapse_count, 7);
        for adapter in ["json", "deps", "env", "wc", "pipe", "log", "smart"] {
            assert!(section.items.contains(&adapter.to_string()), "{adapter}");
        }
    }
}
