// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-GRACE-TOOLS
// PURPOSE: MCP handlers for MyGRACE verification, review, status, refresh, requirements, technology, development plan, mental tests, traceability, agent-based testing, self-heal, log analysis, belief extraction, compression, tracking, skills, and response economy
// SCOPE: profile-aware verify_project/review_code with response economy, project_status response economy, self_heal, analyze_logs response economy, extract_belief_state, generate_requirements, generate_technology, generate_development_plan, mental_test_run response economy, traceability_report response economy, run_test_guide, submit_test_report, token_savings with adapter/session stats and response economy, compress_text, refresh_project response economy, language-aware suggest_contract, grace_* handlers
// DEPENDS: M-GRACE, M-GRACE-BELIEF-STATE, M-GRACE-DEVELOPMENT-PLAN, M-GRACE-MENTAL-TEST, M-GRACE-TRACEABILITY, M-GRACE-TESTING, M-GRACE-LOG, M-GRACE-REQUIREMENTS, M-GRACE-TECHNOLOGY, M-RUNNER-SELF-HEAL, M-TRACKING, M-COMPRESS, M-SKILLS-ENGINE, M-MCP-SERVER-RESPONSE
// LINKS:
//   -> docs/modules/M-MCP-SERVER.xml (depends) - MCP server parent module
//   -> UC-002 (implements) - exposes verified AI engineering workflows to MCP clients
//   -> NFR-002 (traces_to) - MCP handlers return explicit structured failures

// START_MODULE_MAP
// handle_verify — Runs profile-aware MyGRACE verification and formats failure packets
// handle_review — Runs profile-aware MyGRACE review
// handle_status — Returns project status JSON
// handle_analyze_logs — Runs structured LOG trajectory/anomaly/compare analysis
// handle_extract_belief_state — Stores observable AI belief state scaffold for a module
// handle_generate_requirements — Generates and validates docs/requirements.xml with AAG notation
// handle_generate_technology — Generates and validates docs/technology.xml with exact versions
// handle_generate_development_plan — Generates and validates docs/development-plan.xml with DataFlows and GenerationOrder
// handle_mental_test_run — Runs one DevelopmentPlan MentalTest and persists trace output
// handle_traceability_report — Builds end-to-end traceability matrix and updates docs/traceability-index.xml
// handle_run_test_guide — Runs a natural-language tester-agent guide and persists report artifacts
// handle_submit_test_report — Delivers a tester-agent failure report summary to a developer-agent recipient
// handle_self_heal — Runs one bounded self-heal iteration for a persisted run
// handle_gain — Returns token savings stats
// handle_compress — Compresses input text
// handle_refresh — Reports or fixes MyGRACE drift
// handle_grace_skill — Delegates grace_* MCP tools to SkillEngine
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.25.0 - Added response economy to high-output GRACE MCP handlers]
// END_CHANGE_SUMMARY

use super::server_response::{error, result, suggest_fix, text_result, FailurePacket};
use crate::grace::GraceProfile;
use crate::skills::{SkillEngine, SkillRequest};
use std::path::PathBuf;

// START_public_api

// START_CONTRACT_handle_verify
// PURPOSE: Execute verify_project and format pass/fail checks with failure suggestions
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_verify
pub(crate) async fn handle_verify(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let root = match std::env::current_dir() {
        Ok(r) => r,
        Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
    };
    let level = args["level"].as_str().unwrap_or("all");
    let profile = match parse_grace_profile_arg(args) {
        Ok(profile) => profile,
        Err(message) => return error(id, -32602, message),
    };
    match crate::grace::GraceEngine::verify_project_with_profile(&root, profile).await {
        Ok(results) => {
            let filtered: Vec<_> = if level == "all" {
                results
            } else {
                results.into_iter().filter(|r| r.level == level).collect()
            };
            let mut text = String::new();
            let mut failures = Vec::new();
            for r in &filtered {
                let status = if r.passed { "PASS" } else { "FAIL" };
                text.push_str(&format!("\n[{}] {}\n", status, r.level));
                for c in &r.checks {
                    let mark = if c.passed { "✓" } else { "✗" };
                    text.push_str(&format!("  {} {} — {}\n", mark, c.name, c.details));
                    if !c.passed {
                        failures.push(FailurePacket {
                            check: c.name.clone(),
                            details: c.details.clone(),
                            suggested: suggest_fix(&c.name),
                        });
                    }
                }
            }
            if !failures.is_empty() {
                text.push_str("\n--- FAILURE PACKETS ---\n");
                for (i, fp) in failures.iter().enumerate() {
                    text.push_str(&format!(
                        "\n{}. {} FAILED\n   Observed: {}\n   Suggested: {}\n",
                        i + 1,
                        fp.check,
                        fp.details,
                        fp.suggested
                    ));
                }
            }
            text.push_str(&format!("\nProfile: {}\n", profile.as_str()));
            text_result(id, text, args)
        }
        Err(e) => error(id, -32603, format!("Verify error: {}", e)),
    }
}
// END_handle_verify

// START_CONTRACT_handle_review
// PURPOSE: Execute review_code and format review sections with issues
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_review
pub(crate) async fn handle_review(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let root = match std::env::current_dir() {
        Ok(r) => r,
        Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
    };
    let mode = args["mode"].as_str().unwrap_or("scoped");
    let profile = match parse_grace_profile_arg(args) {
        Ok(profile) => profile,
        Err(message) => return error(id, -32602, message),
    };
    match crate::grace::review::Reviewer::review_with_profile(&root, mode, profile) {
        Ok(report) => {
            let mut text = format!(
                "=== GRACE Review ({}, profile={}) ===\n",
                report.mode,
                profile.as_str()
            );
            for s in &report.sections {
                let status = if s.passed { "✓" } else { "✗" };
                text.push_str(&format!("{} {} — {}\n", status, s.name, s.details));
                for issue in &s.issues {
                    text.push_str(&format!("  ⚠ {}\n", issue));
                }
            }
            if report.passed {
                text.push_str("\nAll checks passed.");
            }
            text_result(id, text, args)
        }
        Err(e) => error(id, -32603, format!("Review error: {}", e)),
    }
}
// END_handle_review

// START_CONTRACT_handle_status
// PURPOSE: Execute project_status and return status JSON as MCP text
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_status
pub(crate) async fn handle_status(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let root = match std::env::current_dir() {
        Ok(r) => r,
        Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
    };
    match crate::grace::status::StatusCollector::collect(&root).await {
        Ok(report) => {
            let text = serde_json::to_string_pretty(&report).unwrap_or_default();
            text_result(id, text, args)
        }
        Err(e) => error(id, -32603, format!("Status error: {}", e)),
    }
}
// END_handle_status

// START_CONTRACT_handle_analyze_logs
// PURPOSE: Analyze a structured LOG file and return trajectory/anomaly/compare XML
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_analyze_logs
pub(crate) async fn handle_analyze_logs(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let log_path = match args["log_path"].as_str() {
        Some(path) if !path.trim().is_empty() => path.trim(),
        _ => return error(id, -32602, "Missing 'log_path' parameter"),
    };
    let mode = args["mode"].as_str().unwrap_or("trajectory");
    if !matches!(mode, "trajectory" | "anomaly" | "compare") {
        return error(
            id,
            -32602,
            "Invalid mode. Use trajectory | anomaly | compare.",
        );
    }
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
    };
    let path = resolve_log_path(&root, log_path);
    let contract_ref = args["contract_ref"].as_str();
    match crate::grace::log::analyze_log_file(&path, mode, contract_ref) {
        Ok(report) => text_result(id, report.to_xml(), args),
        Err(e) => error(id, -32603, format!("Log analysis error: {}", e)),
    }
}
// END_handle_analyze_logs

// START_CONTRACT_handle_extract_belief_state
// PURPOSE: Generate and persist a validated BELIEF_STATE scaffold for a module
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// SIDE_EFFECTS: writes docs/belief-states/M-XXX.xml
// START_handle_extract_belief_state
pub(crate) async fn handle_extract_belief_state(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let module_id = match args["module_id"].as_str() {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => return error(id, -32602, "Missing 'module_id' parameter"),
    };
    let context = args["context"]
        .as_str()
        .unwrap_or("Read MODULE_CONTRACT + DevelopmentPlan");
    let root = match args["project_root"].as_str() {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => match std::env::current_dir() {
            Ok(root) => root,
            Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
        },
    };
    match crate::grace::belief_state::extract_and_store_belief_state(&root, module_id, context) {
        Ok(report) => {
            let text = format!(
                "<BeliefStateExtraction module=\"{}\" valid=\"{}\">\n  <Path>{}</Path>\n  <Message>BELIEF_STATE scaffold stored and validated. Refine it before substantial code generation if module intent changed.</Message>\n</BeliefStateExtraction>",
                report.module_id, report.valid, report.path
            );
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Belief state extraction error: {}", e)),
    }
}
// END_handle_extract_belief_state

// START_CONTRACT_handle_generate_requirements
// PURPOSE: Generate and validate a complete RequirementsAnalysis artifact from project description inputs
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// SIDE_EFFECTS: writes docs/requirements.xml
// START_handle_generate_requirements
pub(crate) async fn handle_generate_requirements(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let project_description = match args["project_description"].as_str() {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => return error(id, -32602, "Missing 'project_description' parameter"),
    };
    let domain = args["domain"].as_str().unwrap_or("application").trim();
    let detail_level = args["detail_level"].as_str().unwrap_or("standard").trim();
    if !matches!(detail_level, "quick" | "standard" | "detailed") {
        return error(
            id,
            -32602,
            "Invalid detail_level. Use quick | standard | detailed.",
        );
    }
    let root = match args["project_root"].as_str() {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => match std::env::current_dir() {
            Ok(root) => root,
            Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
        },
    };

    match crate::grace::requirements::generate_requirements_file(
        &root,
        project_description,
        domain,
        detail_level,
    ) {
        Ok(report) => {
            let text = format!(
                "<RequirementsGeneration valid=\"{}\">\n  <Path>{}</Path>\n  <Goals>{}</Goals>\n  <Entities>{}</Entities>\n  <Actors>{}</Actors>\n  <UseCases>{}</UseCases>\n  <NonFunctionalRequirements>{}</NonFunctionalRequirements>\n  <Constraints>{}</Constraints>\n  <GlossaryTerms>{}</GlossaryTerms>\n</RequirementsGeneration>",
                report.valid,
                report.path,
                report.goals,
                report.entities.len(),
                report.actors,
                report.use_cases.len(),
                report.non_functional_requirements,
                report.constraints,
                report.glossary_terms.len(),
            );
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Requirements generation error: {}", e)),
    }
}
// END_handle_generate_requirements

// START_CONTRACT_handle_generate_technology
// PURPOSE: Generate and validate a complete Technology artifact from detected dependency manifests
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// SIDE_EFFECTS: writes docs/technology.xml
// START_handle_generate_technology
pub(crate) async fn handle_generate_technology(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let detect_existing = args["detect_existing"].as_bool().unwrap_or(true);
    let compatibility_check = args["compatibility_check"].as_bool().unwrap_or(true);
    let base = match args["project_root"].as_str() {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => match std::env::current_dir() {
            Ok(root) => root,
            Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
        },
    };
    let project_path = args["project_path"].as_str().unwrap_or(".").trim();
    let requested = PathBuf::from(project_path);
    let root = if requested.is_absolute() {
        requested
    } else {
        base.join(requested)
    };

    match crate::grace::technology::generate_technology_file(
        &root,
        detect_existing,
        compatibility_check,
    ) {
        Ok(report) => {
            let text = format!(
                "<TechnologyGeneration valid=\"{}\">\n  <Path>{}</Path>\n  <Languages>{}</Languages>\n  <Components>{}</Components>\n  <CompatibilityChecks>{}</CompatibilityChecks>\n  <KnownIssues>{}</KnownIssues>\n  <DetectedDependencies>{}</DetectedDependencies>\n</TechnologyGeneration>",
                report.valid,
                report.path,
                report.languages.len(),
                report.components.len(),
                report.compatibility_checks.len(),
                report.known_issues,
                report.detected_dependencies.len(),
            );
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Technology generation error: {}", e)),
    }
}
// END_handle_generate_technology

// START_CONTRACT_handle_generate_development_plan
// PURPOSE: Generate and validate a complete DevelopmentPlan artifact from requirements, technology, and graph modules
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// SIDE_EFFECTS: writes docs/development-plan.xml
// START_handle_generate_development_plan
pub(crate) async fn handle_generate_development_plan(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let from_requirements = args["from_requirements"].as_bool().unwrap_or(true);
    let data_flow_analysis = args["data_flow_analysis"].as_str().unwrap_or("auto").trim();
    if !matches!(data_flow_analysis, "auto" | "guided" | "manual") {
        return error(
            id,
            -32602,
            "Invalid data_flow_analysis. Use auto | guided | manual.",
        );
    }
    let generation_order = args["generation_order"]
        .as_str()
        .unwrap_or("topological")
        .trim();
    if !matches!(generation_order, "topological" | "phase" | "manual") {
        return error(
            id,
            -32602,
            "Invalid generation_order. Use topological | phase | manual.",
        );
    }
    let root = match args["project_root"].as_str() {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => match std::env::current_dir() {
            Ok(root) => root,
            Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
        },
    };

    match crate::grace::development_plan::generate_development_plan_file(
        &root,
        from_requirements,
        data_flow_analysis,
        generation_order,
    ) {
        Ok(report) => {
            let text = format!(
                "<DevelopmentPlanGeneration valid=\"{}\">\n  <Path>{}</Path>\n  <ArchitectureModules>{}</ArchitectureModules>\n  <DataFlows>{}</DataFlows>\n  <GenerationModules>{}</GenerationModules>\n  <NonHumanPatterns>{}</NonHumanPatterns>\n  <ContractGuidelines>{}</ContractGuidelines>\n</DevelopmentPlanGeneration>",
                report.valid,
                report.path,
                report.architecture_modules.len(),
                report.data_flows.len(),
                report.generation_modules.len(),
                report.non_human_patterns,
                report.contract_guidelines,
            );
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(
            id,
            -32603,
            format!("DevelopmentPlan generation error: {}", e),
        ),
    }
}
// END_handle_generate_development_plan

// START_CONTRACT_handle_mental_test_run
// PURPOSE: Run one DevelopmentPlan MentalTest and return persisted trace XML
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// SIDE_EFFECTS: writes docs/mental-tests/<mental_test_id>.xml
// START_handle_mental_test_run
pub(crate) async fn handle_mental_test_run(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let module_id = match args["module_id"].as_str() {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => return error(id, -32602, "Missing 'module_id' parameter"),
    };
    let mental_test_id = match args["mental_test_id"].as_str() {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => return error(id, -32602, "Missing 'mental_test_id' parameter"),
    };
    let step_by_step = args["step_by_step"].as_bool().unwrap_or(true);
    let root = match args["project_root"].as_str() {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => match std::env::current_dir() {
            Ok(root) => root,
            Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
        },
    };

    match crate::grace::mental_test::run_mental_test(&root, module_id, mental_test_id, step_by_step)
    {
        Ok(report) => text_result(id, report.to_xml(), args),
        Err(e) => error(id, -32603, format!("Mental test error: {}", e)),
    }
}
// END_handle_mental_test_run

// START_CONTRACT_handle_traceability_report
// PURPOSE: Generate an end-to-end traceability matrix for project, module, or requirement scope
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// SIDE_EFFECTS: writes docs/traceability-index.xml
// START_handle_traceability_report
pub(crate) async fn handle_traceability_report(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let scope = args["scope"].as_str().unwrap_or("project").trim();
    if !matches!(scope, "project" | "module" | "requirement") {
        return error(
            id,
            -32602,
            "Invalid scope. Use project | module | requirement.",
        );
    }
    let direction = args["direction"].as_str().unwrap_or("up").trim();
    if !matches!(direction, "up" | "down") {
        return error(id, -32602, "Invalid direction. Use up | down.");
    }
    let target = args["target"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if scope != "project" && target.is_none() {
        return error(
            id,
            -32602,
            "Missing 'target' for module or requirement scope",
        );
    }
    let root = match args["project_root"].as_str() {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => match std::env::current_dir() {
            Ok(root) => root,
            Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
        },
    };
    match crate::grace::traceability::scan_project_traceability(&root) {
        Ok(report) => {
            let index_path = crate::grace::traceability::write_traceability_index(&root, &report)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "docs/traceability-index.xml not written".into());
            let mut text = crate::grace::traceability::format_traceability_report(
                &report, scope, target, direction,
            );
            text.push_str(&format!(
                "\n<TraceabilityIndex>{}</TraceabilityIndex>",
                index_path
            ));
            text_result(id, text, args)
        }
        Err(e) => error(id, -32603, format!("Traceability error: {}", e)),
    }
}
// END_handle_traceability_report

// START_CONTRACT_handle_run_test_guide
// PURPOSE: Run a natural-language testing guide and return tester-agent summary
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// SIDE_EFFECTS: writes docs/tests/results/run-*/summary.json and optional failure XML
// START_handle_run_test_guide
pub(crate) async fn handle_run_test_guide(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let guide_path = match args["guide_path"].as_str() {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => return error(id, -32602, "Missing 'guide_path' parameter"),
    };
    let application_url = args["application_url"]
        .as_str()
        .unwrap_or("mock://pass")
        .trim();
    let agent_console_url = args["agent_console_url"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let collect_logs = args["collect_logs"].as_bool().unwrap_or(true);
    let output_report = args["output_report"].as_bool().unwrap_or(true);
    let root = match args["project_root"].as_str() {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => match std::env::current_dir() {
            Ok(root) => root,
            Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
        },
    };
    match crate::grace::testing::run_test_guide(
        &root,
        guide_path,
        application_url,
        agent_console_url,
        collect_logs,
        output_report,
    ) {
        Ok(report) => {
            let text = serde_json::to_string_pretty(&report).unwrap_or_default();
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Test guide error: {}", e)),
    }
}
// END_handle_run_test_guide

// START_CONTRACT_handle_submit_test_report
// PURPOSE: Submit a tester-agent failure report to a developer-agent recipient
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_submit_test_report
pub(crate) async fn handle_submit_test_report(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let report_path = match args["report"]
        .as_str()
        .or_else(|| args["report_path"].as_str())
    {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => return error(id, -32602, "Missing 'report' parameter"),
    };
    let to = args["to"].as_str().unwrap_or("developer").trim();
    let root = match args["project_root"].as_str() {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => match std::env::current_dir() {
            Ok(root) => root,
            Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
        },
    };
    match crate::grace::testing::submit_test_report(&root, report_path, to) {
        Ok(report) => {
            let text = serde_json::to_string_pretty(&report).unwrap_or_default();
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Submit test report error: {}", e)),
    }
}
// END_handle_submit_test_report

// START_CONTRACT_handle_self_heal
// PURPOSE: Execute one bounded self-heal iteration for a persisted autonomous run
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// SIDE_EFFECTS: updates docs/runs/<run_id>.json metadata and may escalate exhausted runs
// START_handle_self_heal
pub(crate) async fn handle_self_heal(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let run_id = match args["run_id"].as_str() {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => return error(id, -32602, "Missing 'run_id' parameter"),
    };
    let profile = args["profile"].as_str().unwrap_or("strict").trim();
    if GraceProfile::from_name(profile).is_none() {
        return error(
            id,
            -32602,
            format!(
                "Unsupported GRACE profile '{}'. Use one of: lite, balanced, strict.",
                profile
            ),
        );
    }
    let root = match args["project_root"].as_str() {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => match std::env::current_dir() {
            Ok(root) => root,
            Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
        },
    };
    let manager = crate::run::RunManager::new(root);
    match manager.execute_self_heal(run_id, profile) {
        Ok(result_report) => {
            let text = serde_json::to_string_pretty(&result_report).unwrap_or_default();
            result(
                id,
                serde_json::json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
            )
        }
        Err(e) => error(id, -32603, format!("Self-heal error: {}", e)),
    }
}
// END_handle_self_heal

// START_CONTRACT_handle_gain
// PURPOSE: Execute token_savings and return tracking statistics
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_gain
pub(crate) async fn handle_gain(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let config = crate::config::Config::load().unwrap_or_default();
    let tracker = crate::tracking::Tracker::new(&config);
    match tracker.get_stats().await {
        Ok(stats) => {
            let mut text = String::new();
            text.push_str(&format!("Commands tracked:  {}\n", stats.total_commands));
            text.push_str(&format!(
                "Input tokens:      {}\n",
                stats.total_input_tokens
            ));
            text.push_str(&format!(
                "Output tokens:     {}\n",
                stats.total_output_tokens
            ));
            text.push_str(&format!(
                "Tokens saved:      {}\n",
                stats.total_saved_tokens
            ));
            text.push_str(&format!(
                "Avg savings:       {:.1}%\n",
                stats.avg_savings_pct
            ));
            if stats.total_commands > 0 {
                let est = stats.total_saved_tokens as f64 * 0.000003;
                text.push_str(&format!("Est. cost saved:   ${:.4}\n", est));
            }
            if !stats.top_adapters.is_empty() {
                text.push_str("\nTop adapters:\n");
                for item in &stats.top_adapters {
                    text.push_str(&format!(
                        "- {}: {} runs, {} saved, avg {:.1}%\n",
                        item.adapter, item.count, item.saved_tokens, item.avg_savings_pct
                    ));
                }
            }
            if !stats.recent_sessions.is_empty() {
                text.push_str("\nRecent sessions:\n");
                for item in &stats.recent_sessions {
                    text.push_str(&format!(
                        "- {}: {} runs, {} saved\n",
                        item.session_id, item.count, item.saved_tokens
                    ));
                }
            }
            text_result(id, text, args)
        }
        Err(e) => error(id, -32603, format!("Gain error: {}", e)),
    }
}
// END_handle_gain

// START_CONTRACT_handle_compress
// PURPOSE: Execute compress_text with selected compression level and return compressed text
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_compress
pub(crate) async fn handle_compress(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let text = match args["text"].as_str() {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => return error(id, -32602, "Missing 'text' parameter"),
    };
    let level = args["level"].as_str().unwrap_or("full");
    let config = crate::config::Config::load().unwrap_or_default();
    let mut cfg = config;
    cfg.compress.output_level = level.to_string();
    let compressor = crate::compress::Compressor::new(&cfg);
    let result_text = compressor.compress_output(&text);
    let saved = text.len().saturating_sub(result_text.len());
    let pct = if text.is_empty() {
        0
    } else {
        saved * 100 / text.len()
    };
    let info = format!(
        "({} → {} chars, {}% saved)\n\n{}",
        text.len(),
        result_text.len(),
        pct,
        result_text
    );
    result(
        id,
        serde_json::json!({
            "content": [{"type": "text", "text": info}],
            "isError": false
        }),
    )
}
// END_handle_compress

// START_CONTRACT_handle_refresh
// PURPOSE: Execute refresh_project in report or fix mode and return drift report JSON
// INPUTS: { id: Option<serde_json::Value> }, { args: &serde_json::Value }
// OUTPUTS: { serde_json::Value }
// START_handle_refresh
pub(crate) async fn handle_refresh(
    id: Option<serde_json::Value>,
    args: &serde_json::Value,
) -> serde_json::Value {
    let root = match std::env::current_dir() {
        Ok(r) => r,
        Err(e) => return error(id, -32603, format!("cwd error: {}", e)),
    };
    let fix = args["fix"].as_bool().unwrap_or(false);
    let result_report = if fix {
        crate::grace::refresh::Refresher::fix(&root)
    } else {
        crate::grace::refresh::Refresher::refresh(&root)
    };
    match result_report {
        Ok(report) => {
            let text = serde_json::to_string_pretty(&report).unwrap_or_default();
            text_result(id, text, args)
        }
        Err(e) => error(id, -32603, format!("Refresh error: {}", e)),
    }
}
// END_handle_refresh

// START_CONTRACT_parse_grace_profile_arg
// PURPOSE: Parse an optional MCP GRACE profile argument
// INPUTS: { args: &serde_json::Value — tool arguments }
// OUTPUTS: { Result<GraceProfile, String> }
// START_parse_grace_profile_arg
fn parse_grace_profile_arg(args: &serde_json::Value) -> Result<GraceProfile, String> {
    let profile = args["profile"].as_str().unwrap_or("strict");
    GraceProfile::from_name(profile).ok_or_else(|| {
        format!(
            "Unsupported GRACE profile '{}'. Use one of: lite, balanced, strict.",
            profile
        )
    })
}
// END_parse_grace_profile_arg

fn resolve_log_path(root: &std::path::Path, log_path: &str) -> PathBuf {
    let path = PathBuf::from(log_path);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

// START_CONTRACT_handle_grace_skill
// PURPOSE: Execute a registered grace_* skill through SkillEngine and return title/body text
// INPUTS: { skill_engine: &SkillEngine }, { request: SkillRequest }, { id: Option<serde_json::Value> }
// OUTPUTS: { serde_json::Value }
// START_handle_grace_skill
pub(crate) async fn handle_grace_skill(
    skill_engine: &SkillEngine,
    request: SkillRequest,
    id: Option<serde_json::Value>,
) -> serde_json::Value {
    match skill_engine.execute(request).await {
        Ok(result_value) => result(
            id,
            serde_json::json!({
                "content": [{"type": "text", "text": format!("{}\n\n{}", result_value.title, result_value.body)}],
                "isError": false
            }),
        ),
        Err(e) => error(id, -32603, format!("Skill error: {}", e)),
    }
}
// END_handle_grace_skill

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_analyze_logs_returns_report
    // PURPOSE: Verify analyze_logs reads a structured LOG file and returns an XML report
    // START_test_handle_analyze_logs_returns_report
    async fn test_handle_analyze_logs_returns_report() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log_path = dir.path().join("sample.log");
        std::fs::write(
            &log_path,
            concat!(
                "<LOG id=\"sample-001\" level=\"INFO\" ref=\"run\" module=\"M-SAMPLE\" contract=\"run\">\n",
                "  EVENT: run_started\n",
                "  DECISION: Execute sample\n",
                "  EXPECTATION: Sample succeeds\n",
                "  RESULT: success\n",
                "  TRACEABILITY:\n",
                "    UC-002: MCP log analysis supports verified project changes\n",
                "</LOG>\n",
            ),
        )
        .expect("write log");
        let response = handle_analyze_logs(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "log_path": log_path.to_string_lossy(),
                "mode": "trajectory",
                "contract_ref": "M-SAMPLE"
            }),
        )
        .await;
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text response");
        assert!(text.contains("<LogAnalysisReport"));
        assert!(text.contains("<TotalEvents>1</TotalEvents>"));
        assert_eq!(response["result"]["style"], "full");
        assert_eq!(response["result"]["was_trimmed"], false);
        assert!(response["result"]["tokens"]["shown"].as_u64().unwrap_or(0) > 0);
    }
    // END_test_handle_analyze_logs_returns_report

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_extract_belief_state_writes_artifact
    // PURPOSE: Verify extract_belief_state writes a valid docs/belief-states artifact
    // START_test_handle_extract_belief_state_writes_artifact
    async fn test_handle_extract_belief_state_writes_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = crate::grace::layout::DocsLayout::new(dir.path());
        std::fs::create_dir_all(layout.modules_dir()).expect("modules dir");
        std::fs::write(
            layout.modules_dir().join("M-SAMPLE.xml"),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
                "<MODULE id=\"M-SAMPLE\" type=\"CORE_LOGIC\" status=\"active\">\n",
                "  <PURPOSE>Sample module purpose</PURPOSE>\n",
                "  <SCOPE>Sample module scope</SCOPE>\n",
                "</MODULE>\n",
            ),
        )
        .expect("write module shard");

        let response = handle_extract_belief_state(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "module_id": "M-SAMPLE",
                "context": "Read MODULE_CONTRACT + DevelopmentPlan",
                "project_root": dir.path().to_string_lossy()
            }),
        )
        .await;

        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text response");
        assert!(text.contains("<BeliefStateExtraction"));
        assert!(dir.path().join("docs/belief-states/M-SAMPLE.xml").exists());
    }
    // END_test_handle_extract_belief_state_writes_artifact

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_generate_requirements_writes_valid_artifact
    // PURPOSE: Verify generate_requirements writes a valid RequirementsAnalysis artifact
    // START_test_handle_generate_requirements_writes_valid_artifact
    async fn test_handle_generate_requirements_writes_valid_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        let response = handle_generate_requirements(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "project_description": "Developer automation platform",
                "domain": "developer tooling",
                "detail_level": "standard",
                "project_root": dir.path().to_string_lossy()
            }),
        )
        .await;

        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text response");
        assert!(text.contains("<RequirementsGeneration valid=\"true\""));
        assert!(dir.path().join("docs/requirements.xml").exists());
    }
    // END_test_handle_generate_requirements_writes_valid_artifact

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_generate_technology_writes_valid_artifact
    // PURPOSE: Verify generate_technology writes a valid Technology artifact
    // START_test_handle_generate_technology_writes_valid_artifact
    async fn test_handle_generate_technology_writes_valid_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        let response = handle_generate_technology(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "project_path": ".",
                "detect_existing": false,
                "compatibility_check": true,
                "project_root": dir.path().to_string_lossy()
            }),
        )
        .await;

        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text response");
        assert!(text.contains("<TechnologyGeneration valid=\"true\""));
        assert!(dir.path().join("docs/technology.xml").exists());
    }
    // END_test_handle_generate_technology_writes_valid_artifact

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_generate_development_plan_writes_artifact
    // PURPOSE: Verify generate_development_plan writes a DevelopmentPlan artifact
    // START_test_handle_generate_development_plan_writes_artifact
    async fn test_handle_generate_development_plan_writes_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("docs")).expect("docs");
        std::fs::write(
            dir.path().join("docs/graph-index.xml"),
            r#"<GRAPH_INDEX><MODULES><MODULE id="M-CORE" path="docs/modules/M-CORE.xml" status="active" /></MODULES></GRAPH_INDEX>"#,
        )
        .expect("graph");
        let response = handle_generate_development_plan(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "from_requirements": false,
                "data_flow_analysis": "auto",
                "generation_order": "topological",
                "project_root": dir.path().to_string_lossy()
            }),
        )
        .await;

        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text response");
        assert!(text.contains("<DevelopmentPlanGeneration"));
        assert!(dir.path().join("docs/development-plan.xml").exists());
    }
    // END_test_handle_generate_development_plan_writes_artifact

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_mental_test_run_writes_trace
    // PURPOSE: Verify mental_test_run executes a DevelopmentPlan MentalTest and persists trace output
    // START_test_handle_mental_test_run_writes_trace
    async fn test_handle_mental_test_run_writes_trace() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("docs")).expect("docs");
        std::fs::write(
            dir.path().join("docs/development-plan.xml"),
            concat!(
                "<DevelopmentPlan><ArchitectureGraph><Module id=\"M-SAMPLE\" critical=\"true\" /></ArchitectureGraph>",
                "<MentalTests><MentalTest id=\"MT-001\" target=\"M-SAMPLE::run\" status=\"pass\">",
                "<Description>Simulation</Description><Scenario>GIVEN x WHEN y</Scenario>",
                "<Steps><Step n=\"1\" action=\"run\" expected=\"ok\" actual=\"PASS — ok\">Simulate.</Step></Steps>",
                "<EdgeCases><Case id=\"EC-001\" description=\"edge\" expectation=\"handled\" /></EdgeCases>",
                "<Result>pass</Result><Rationale>Deterministic.</Rationale>",
                "</MentalTest></MentalTests></DevelopmentPlan>",
            ),
        )
        .expect("plan");

        let response = handle_mental_test_run(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "module_id": "M-SAMPLE",
                "mental_test_id": "MT-001",
                "step_by_step": true,
                "project_root": dir.path().to_string_lossy()
            }),
        )
        .await;

        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text response");
        assert!(text.contains("<MentalTestRun"));
        assert!(text.contains("passed=\"true\""));
        assert!(dir.path().join("docs/mental-tests/MT-001.xml").exists());
    }
    // END_test_handle_mental_test_run_writes_trace

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_traceability_report_writes_index
    // PURPOSE: Verify traceability_report returns a matrix and persists docs/traceability-index.xml
    // START_test_handle_traceability_report_writes_index
    async fn test_handle_traceability_report_writes_index() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("docs")).expect("docs");
        std::fs::create_dir_all(dir.path().join("src")).expect("src");
        std::fs::write(
            dir.path().join("docs/requirements.xml"),
            concat!(
                "<RequirementsAnalysis><DomainModel><Entity name=\"Order\" /></DomainModel>",
                "<UseCases><UseCase id=\"UC-001\"><Actor>User</Actor><Action>Order</Action><Goal>Done</Goal></UseCase></UseCases>",
                "<NonFunctionalRequirements><Requirement id=\"REQ-001\"><Description>Traceable order</Description></Requirement></NonFunctionalRequirements>",
                "</RequirementsAnalysis>",
            ),
        )
        .expect("requirements");
        std::fs::write(
            dir.path().join("src/order.rs"),
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-ORDER\n",
                "// PURPOSE: Order module\n",
                "// SCOPE: Order flow\n",
                "// DEPENDS: N/A\n",
                "// LINKS:\n",
                "//   -> UC-001 (implements) - order use case\n",
                "\n",
                "// START_MODULE_MAP\n",
                "// place_order - Places order\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 - Initial]\n",
                "// END_CHANGE_SUMMARY\n",
                "\n",
                "// START_CONTRACT_place_order\n",
                "// PURPOSE: Place order\n",
                "// LINKS:\n",
                "//   -> UC-001 (implements) - order use case\n",
                "// START_place_order\n",
                "pub fn place_order() {}\n",
                "// END_place_order\n",
            ),
        )
        .expect("source");

        let response = handle_traceability_report(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "scope": "module",
                "target": "M-ORDER",
                "direction": "up",
                "project_root": dir.path().to_string_lossy()
            }),
        )
        .await;

        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text response");
        assert!(text.contains("<TraceabilityReport"));
        assert!(text.contains("M-ORDER::place_order"));
        assert!(dir.path().join("docs/traceability-index.xml").exists());
    }
    // END_test_handle_traceability_report_writes_index

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_run_test_guide_writes_summary
    // PURPOSE: Verify run_test_guide parses Markdown and writes tester-agent summary artifacts
    // START_test_handle_run_test_guide_writes_summary
    async fn test_handle_run_test_guide_writes_summary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let guide_dir = dir.path().join("docs/tests/guides");
        std::fs::create_dir_all(&guide_dir).expect("guides");
        std::fs::write(
            guide_dir.join("sample.md"),
            concat!(
                "# Testing Guide: MCP Sample\n\n",
                "## Test: Happy Path\n\n",
                "### Steps\n1. Execute action\n\n",
                "### Expected Behavior\n- Action succeeds\n\n",
                "### Data to Capture\n- LOG output\n",
            ),
        )
        .expect("guide");

        let response = handle_run_test_guide(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "guide_path": "docs/tests/guides/sample.md",
                "application_url": "mock://pass",
                "collect_logs": true,
                "output_report": true,
                "project_root": dir.path().to_string_lossy()
            }),
        )
        .await;

        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text response");
        assert!(text.contains("\"passed\": true"));
        assert!(dir.path().join("docs/tests/index.xml").exists());
    }
    // END_test_handle_run_test_guide_writes_summary

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_submit_test_report_highlights_log_refs
    // PURPOSE: Verify submit_test_report returns LOG evidence refs for developer handoff
    // START_test_handle_submit_test_report_highlights_log_refs
    async fn test_handle_submit_test_report_highlights_log_refs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let report_dir = dir.path().join("docs/tests/results/run-1");
        std::fs::create_dir_all(&report_dir).expect("results");
        std::fs::write(
            report_dir.join("failure.xml"),
            concat!(
                "<TestFailureReport><Test name=\"Sample\">",
                "<Actual>Observed mismatch</Actual>",
                "<LogEvidence><LOG ref=\"sample-001\">TRACEABILITY: UC-002<Actual>failed</Actual></LOG></LogEvidence>",
                "</Test></TestFailureReport>",
            ),
        )
        .expect("report");

        let response = handle_submit_test_report(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "report": "docs/tests/results/run-1/failure.xml",
                "to": "developer",
                "project_root": dir.path().to_string_lossy()
            }),
        )
        .await;

        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text response");
        assert!(text.contains("\"has_log_evidence\": true"));
        assert!(text.contains("sample-001"));
    }
    // END_test_handle_submit_test_report_highlights_log_refs

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_self_heal
    // PURPOSE: Verify self_heal validates required run_id before touching project state
    // START_test_handle_self_heal
    async fn test_handle_self_heal() {
        let response = handle_self_heal(Some(serde_json::json!(1)), &serde_json::json!({})).await;

        assert_eq!(response["error"]["code"], -32602);
        assert!(response["error"]["message"]
            .as_str()
            .expect("message")
            .contains("run_id"));
    }
    // END_test_handle_self_heal

    #[tokio::test(flavor = "current_thread")]
    // START_CONTRACT_test_handle_repair_contract
    // PURPOSE: Verify Phase-71 repair_contract handler is reachable through the MCP GRACE tool test surface
    // START_test_handle_repair_contract
    async fn test_handle_repair_contract() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("src")).expect("src");
        std::fs::write(dir.path().join("src/sample.rs"), "pub fn run() {}\n").expect("source");

        let response = crate::mcp::server_contract_tools::handle_repair_contract(
            Some(serde_json::json!(1)),
            &serde_json::json!({
                "file_path": "src/sample.rs",
                "module_id": "M-SAMPLE",
                "purpose": "Sample repair",
                "dry_run": true,
                "project_root": dir.path().to_string_lossy()
            }),
        )
        .await;

        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("text response");
        assert!(text.contains("MODULE_ID: M-SAMPLE"));
    }
    // END_test_handle_repair_contract
}

// END_public_api
