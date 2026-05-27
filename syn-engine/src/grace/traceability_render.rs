// MODULE_CONTRACT
// MODULE_ID: M-GRACE-TRACEABILITY
// PURPOSE: XML rendering and persisted index writer for traceability reports
// SCOPE: write_traceability_index, format_traceability_report, compact index rendering, scoped matrix rendering, XML escaping
// DEPENDS: M-GRACE-TRACEABILITY, M-GRACE-LAYOUT
// LINKS:
//   -> V-M-GRACE-TRACEABILITY (verified_by) - traceability render and index tests
//   -> UC-002 (implements) - verify and review bounded changes with traceable artifacts
//   -> NFR-002 (traces_to) - verification and review must not panic on malformed project state

// START_MODULE_MAP
// write_traceability_index - Persists docs/traceability-index.xml from a report
// format_traceability_report - Renders a scoped MCP-friendly traceability matrix
// render_traceability_index - Renders the compact persisted XML index
// select_chains - Selects project/module/requirement traceability chains
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Extracted traceability rendering from traceability core]
// END_CHANGE_SUMMARY

use super::traceability::{ArtifactType, TraceLink, TraceabilityChain, TraceabilityReport};
use std::path::{Path, PathBuf};

const TRACE_MATRIX_DISPLAY_LIMIT: usize = 80;
const TRACE_INDEX_IMPLEMENTER_LIMIT: usize = 10;

// START_public_api

// START_CONTRACT_write_traceability_index
// PURPOSE: Persist docs/traceability-index.xml from the computed traceability report
// INPUTS: { root: &Path }, { report: &TraceabilityReport }
// OUTPUTS: { anyhow::Result<PathBuf> }
// SIDE_EFFECTS: writes docs/traceability-index.xml
// START_write_traceability_index
pub fn write_traceability_index(
    root: &Path,
    report: &TraceabilityReport,
) -> anyhow::Result<PathBuf> {
    let layout = crate::grace::layout::DocsLayout::new(root);
    std::fs::create_dir_all(layout.docs_dir())?;
    let path = layout.traceability_index_path();
    std::fs::write(&path, render_traceability_index(report))?;
    Ok(path)
}
// END_write_traceability_index

// START_CONTRACT_format_traceability_report
// PURPOSE: Render a scoped traceability query as compact XML-like text for MCP output
// INPUTS: { report: &TraceabilityReport }, { scope: &str }, { target: Option<&str> }, { direction: &str }
// OUTPUTS: { String }
// START_format_traceability_report
pub fn format_traceability_report(
    report: &TraceabilityReport,
    scope: &str,
    target: Option<&str>,
    direction: &str,
) -> String {
    let selected = select_chains(report, scope, target);
    let mut output = format!(
        "<TraceabilityReport scope=\"{}\" direction=\"{}\" target=\"{}\" enforcement=\"{}\">\n",
        xml_attr(scope),
        xml_attr(direction),
        xml_attr(target.unwrap_or("")),
        xml_attr(&report.enforcement_mode)
    );
    output.push_str(&format!(
        "  <Summary requirements=\"{}\" use_cases=\"{}\" traced_modules=\"{}\" traced_functions=\"{}/{}\" traced_logs=\"{}/{}\" score=\"{:.3}\" />\n",
        report.requirements_total,
        report.use_cases_total,
        report.modules_with_traceability,
        report.functions_with_traceability,
        report.total_functions,
        report.logs_with_traceability,
        report.total_logs,
        report.traceability_score
    ));
    output.push_str("  <TraceMatrix>\n");
    for chain in selected.iter().take(TRACE_MATRIX_DISPLAY_LIMIT) {
        output.push_str(&format!(
            "    <Artifact id=\"{}\" type=\"{}\">\n",
            xml_attr(&chain.artifact_id),
            chain.artifact_type.as_str()
        ));
        render_links(
            &mut output,
            "TracesTo",
            if direction == "down" {
                &[]
            } else {
                &chain.traces_to
            },
        );
        render_links(
            &mut output,
            "TracedBy",
            if direction == "up" {
                &[]
            } else {
                &chain.traced_by
            },
        );
        output.push_str("    </Artifact>\n");
    }
    if selected.len() > TRACE_MATRIX_DISPLAY_LIMIT {
        output.push_str(&format!(
            "    <Truncated remaining=\"{}\" />\n",
            selected.len() - TRACE_MATRIX_DISPLAY_LIMIT
        ));
    }
    output.push_str("  </TraceMatrix>\n");
    output.push_str("  <Gaps>\n");
    for gap in report.gaps.iter().take(TRACE_MATRIX_DISPLAY_LIMIT) {
        if target.is_some_and(|needle| !gap.source_id.contains(needle)) && scope != "project" {
            continue;
        }
        output.push_str(&format!(
            "    <Gap type=\"{}\" source=\"{}\">{}</Gap>\n",
            xml_attr(&gap.gap_type),
            xml_attr(&gap.source_id),
            xml_text(&gap.description)
        ));
    }
    output.push_str("  </Gaps>\n</TraceabilityReport>");
    output
}
// END_format_traceability_report

// END_public_api

// START_CONTRACT_render_traceability_index
// PURPOSE: Render the compact persisted XML traceability index from a report
// INPUTS: { report: &TraceabilityReport }
// OUTPUTS: { String }
// START_render_traceability_index
fn render_traceability_index(report: &TraceabilityReport) -> String {
    let mut output = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<TRACEABILITY_INDEX enforcement=\"{}\">\n",
        xml_attr(&report.enforcement_mode)
    );
    output.push_str("  <META>\n    <MODEL>mygrace-sharded</MODEL>\n    <PRIMARY>true</PRIMARY>\n    <GENERATED_BY>M-GRACE-TRACEABILITY</GENERATED_BY>\n  </META>\n");
    output.push_str(&format!(
        "  <SUMMARY requirements=\"{}\" use_cases=\"{}\" traced_modules=\"{}\" traced_functions=\"{}\" traced_logs=\"{}\" score=\"{:.3}\" />\n",
        report.requirements_total,
        report.use_cases_total,
        report.modules_with_traceability,
        report.functions_with_traceability,
        report.logs_with_traceability,
        report.traceability_score
    ));
    output.push_str(&format!(
        "  <ENFORCEMENT mode=\"{}\">{}</ENFORCEMENT>\n",
        xml_attr(&report.enforcement_mode),
        if report.enforcement_mode == "strict" {
            "Traceability gaps fail verification."
        } else {
            "Traceability gaps are reported before strict rollout."
        }
    ));
    output.push_str("  <REQUIREMENTS>\n");
    for chain in report
        .chains
        .iter()
        .filter(|chain| chain.artifact_type == ArtifactType::Requirement)
    {
        render_index_chain(&mut output, "REQUIREMENT", chain);
    }
    output.push_str("  </REQUIREMENTS>\n  <USE_CASES>\n");
    for chain in report
        .chains
        .iter()
        .filter(|chain| chain.artifact_type == ArtifactType::UseCase)
    {
        render_index_chain(&mut output, "USE_CASE", chain);
    }
    output.push_str("  </USE_CASES>\n  <GAPS>\n");
    for gap in &report.gaps {
        output.push_str(&format!(
            "    <GAP type=\"{}\" id=\"{}\">{}</GAP>\n",
            xml_attr(&gap.gap_type),
            xml_attr(&gap.source_id),
            xml_text(&gap.description)
        ));
    }
    output.push_str("  </GAPS>\n</TRACEABILITY_INDEX>\n");
    output
}
// END_render_traceability_index

// START_CONTRACT_render_index_chain
// PURPOSE: Render one requirement or use-case chain with capped implementing artifact samples
// INPUTS: { output: &mut String }, { tag: &str }, { chain: &TraceabilityChain }
// OUTPUTS: { none }
// SIDE_EFFECTS: appends XML text to output
// START_render_index_chain
fn render_index_chain(output: &mut String, tag: &str, chain: &TraceabilityChain) {
    output.push_str(&format!(
        "    <{} id=\"{}\">\n",
        tag,
        xml_attr(&chain.artifact_id)
    ));
    let implementers: Vec<&TraceLink> = chain
        .traced_by
        .iter()
        .filter(|link| {
            matches!(
                link.target_type.as_str(),
                "module" | "function" | "log_entry"
            )
        })
        .collect();
    let shown = implementers.len().min(TRACE_INDEX_IMPLEMENTER_LIMIT);
    output.push_str(&format!(
        "      <IMPLEMENTED_BY total=\"{}\" shown=\"{}\">\n",
        implementers.len(),
        shown
    ));
    for link in implementers.iter().take(TRACE_INDEX_IMPLEMENTER_LIMIT) {
        output.push_str(&format!(
            "        <ARTIFACT ref=\"{}\" type=\"{}\" relationship=\"{}\" />\n",
            xml_attr(&link.target_id),
            xml_attr(&link.target_type),
            xml_attr(&link.relationship)
        ));
    }
    if implementers.len() > TRACE_INDEX_IMPLEMENTER_LIMIT {
        output.push_str(&format!(
            "        <TRUNCATED remaining=\"{}\" />\n",
            implementers.len() - TRACE_INDEX_IMPLEMENTER_LIMIT
        ));
    }
    output.push_str("      </IMPLEMENTED_BY>\n    </");
    output.push_str(tag);
    output.push_str(">\n");
}
// END_render_index_chain

// START_CONTRACT_select_chains
// PURPOSE: Select traceability chains for project, module, or requirement scoped rendering
// INPUTS: { report: &TraceabilityReport }, { scope: &str }, { target: Option<&str> }
// OUTPUTS: { Vec<&TraceabilityChain> }
// START_select_chains
fn select_chains<'a>(
    report: &'a TraceabilityReport,
    scope: &str,
    target: Option<&str>,
) -> Vec<&'a TraceabilityChain> {
    let selected: Vec<_> = match scope {
        "module" => target
            .map(|module| {
                report
                    .chains
                    .iter()
                    .filter(|chain| {
                        chain.artifact_id == module
                            || chain.artifact_id.starts_with(&format!("{}::", module))
                            || chain.traces_to.iter().any(|link| link.target_id == module)
                            || chain.traced_by.iter().any(|link| link.target_id == module)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        "requirement" => target
            .map(|requirement| {
                report
                    .chains
                    .iter()
                    .filter(|chain| {
                        chain.artifact_id == requirement
                            || chain
                                .traces_to
                                .iter()
                                .any(|link| link.target_id == requirement)
                            || chain
                                .traced_by
                                .iter()
                                .any(|link| link.target_id == requirement)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        _ => report.chains.iter().collect(),
    };
    if selected.is_empty() && scope != "project" {
        report
            .chains
            .iter()
            .filter(|chain| target.is_some_and(|needle| chain.artifact_id.contains(needle)))
            .collect()
    } else {
        selected
    }
}
// END_select_chains

// START_CONTRACT_render_links
// PURPOSE: Render traceability links under one XML tag
// INPUTS: { output: &mut String }, { tag: &str }, { links: &[TraceLink] }
// OUTPUTS: { none }
// SIDE_EFFECTS: appends XML text to output
// START_render_links
fn render_links(output: &mut String, tag: &str, links: &[TraceLink]) {
    output.push_str(&format!("      <{}>\n", tag));
    for link in links {
        output.push_str(&format!(
            "        <Link target=\"{}\" type=\"{}\" relationship=\"{}\" />\n",
            xml_attr(&link.target_id),
            xml_attr(&link.target_type),
            xml_attr(&link.relationship)
        ));
    }
    output.push_str(&format!("      </{}>\n", tag));
}
// END_render_links

// START_CONTRACT_xml_attr
// PURPOSE: Escape a string for safe XML attribute usage
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_xml_attr
fn xml_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
// END_xml_attr

// START_CONTRACT_xml_text
// PURPOSE: Escape a string for safe XML text usage
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_xml_text
fn xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
// END_xml_text
