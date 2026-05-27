// MODULE_CONTRACT
// MODULE_ID: M-CLI-GRACE-COMMANDS
// PURPOSE: CLI MyGRACE command handlers
// SCOPE: VerifyCmd including --staged, ReviewCmd, StatusCmd, RefreshCmd, SkillsCmd, GRACE profile parsing
// DEPENDS: M-CONFIG, M-GRACE, M-GRACE-REFRESH, M-GRACE-REVIEW, M-GRACE-STATUS, M-SKILLS
// LINKS: docs/modules/M-CLI.xml

// START_MODULE_MAP
// VerifyCmd::run — Runs MyGRACE verification with optional strictness profile and staged mode
// ReviewCmd::run — Runs MyGRACE review with optional strictness profile
// StatusCmd::run — Prints MyGRACE status
// RefreshCmd::run — Reports or fixes MyGRACE drift
// SkillsCmd::run — Lists, shows, or runs MyGRACE skills
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.11.0 - Added verify --staged support for git pre-commit hooks]
// END_CHANGE_SUMMARY

use super::{RefreshCmd, ReviewCmd, SkillsCmd, StatusCmd, VerifyCmd};
use syn_core::config::Config;
use syn_engine::grace::bootstrap::resolve_module_scope;
use syn_engine::grace::GraceProfile;

// START_public_api

impl VerifyCmd {
    // START_CONTRACT_VerifyCmd::run
    // PURPOSE: Run MyGRACE verification and print JSON or human-readable checks, optionally scoped to staged git files
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // LINKS:
    //   -> Phase-90 (implements) - staged pre-commit verification
    //   -> NFR-002 (traces_to) - reliable release verification commands
    //   <- V-M-CLI-GRACE-COMMANDS (verified_by) - verify CLI coverage
    // START_verify_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let profile = parse_grace_profile(&self.profile)?;
        let mut results = if self.staged {
            syn_engine::grace::verify::Verifier::verify_staged_with_profile(&root, profile).await?
        } else {
            syn_engine::grace::GraceEngine::verify_project_with_profile(&root, profile).await?
        };
        if let Some(module) = self.r#mod.as_deref() {
            let scope = resolve_module_scope(&root, module);
            if !scope.is_empty() {
                results.retain(|r| {
                    r.checks.iter().any(|c| {
                        scope
                            .iter()
                            .any(|p| c.details.contains(&p.display().to_string()))
                            || c.details.contains(module)
                            || c.name.contains(module)
                            || r.level.contains(module)
                    })
                });
            }
        }

        let all_pass = results.iter().all(|r| r.passed);
        if self.json || self.ci {
            println!("{}", serde_json::to_string_pretty(&results)?);
            if all_pass {
                return Ok(());
            }
            anyhow::bail!("Verification failed. Fix issues above and re-run.");
        }

        for r in &results {
            let status = if r.passed { "✓ PASS" } else { "✗ FAIL" };
            println!("[{}] {}", status, r.level);
            for c in &r.checks {
                println!(
                    "  {} {} — {}",
                    if c.passed { "✓" } else { "✗" },
                    c.name,
                    c.details
                );
            }
        }
        if !all_pass {
            anyhow::bail!("Verification failed. Fix issues above and re-run.");
        }
        println!("All checks passed under {} profile.", profile.as_str());
        Ok(())
    }
    // END_verify_run
}

impl ReviewCmd {
    // START_CONTRACT_ReviewCmd::run
    // PURPOSE: Run MyGRACE integrity review and print JSON or human-readable sections
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_review_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let mode = self.mode.as_deref().unwrap_or("scoped");
        let profile = parse_grace_profile(&self.profile)?;
        let report = syn_engine::grace::review::Reviewer::review_with_profile(&root, mode, profile)?;
        let mut sections = report.sections.clone();
        if let Some(module) = self.r#mod.as_deref() {
            let scope = resolve_module_scope(&root, module);
            if !scope.is_empty() {
                sections.retain(|s| {
                    s.name.contains(module)
                        || s.details.contains(module)
                        || s.issues.iter().any(|i| i.contains(module))
                        || scope
                            .iter()
                            .any(|p| s.details.contains(&p.display().to_string()))
                });
            }
        }
        let report = syn_engine::grace::review::ReviewReport {
            mode: report.mode,
            passed: sections.iter().all(|s| s.passed),
            sections,
        };
        if self.json || self.ci {
            println!("{}", serde_json::to_string_pretty(&report)?);
            if report.passed {
                return Ok(());
            }
            anyhow::bail!("Review found issues.");
        }

        println!(
            "=== GRACE Integrity Review ({}, profile={}) ===",
            report.mode,
            profile.as_str()
        );
        for s in &report.sections {
            let status = if s.passed { "✓" } else { "✗" };
            println!("{} {} — {}", status, s.name, s.details);
            for issue in &s.issues {
                println!("    ⚠ {}", issue);
            }
        }
        if report.passed {
            println!("Review passed.");
        } else {
            anyhow::bail!("Review found issues.");
        }
        Ok(())
    }
    // END_review_run
}

// START_CONTRACT_parse_grace_profile
// PURPOSE: Parse a CLI GRACE profile and return an actionable error for unsupported names
// INPUTS: { profile: &str — lite|balanced|strict }
// OUTPUTS: { anyhow::Result<GraceProfile> }
// START_parse_grace_profile
fn parse_grace_profile(profile: &str) -> anyhow::Result<GraceProfile> {
    GraceProfile::from_name(profile).ok_or_else(|| {
        anyhow::anyhow!(
            "Unsupported GRACE profile '{}'. Use one of: lite, balanced, strict.",
            profile
        )
    })
}
// END_parse_grace_profile

impl StatusCmd {
    // START_CONTRACT_StatusCmd::run
    // PURPOSE: Collect and print project status, optionally scoped to one module
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_status_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let report = syn_engine::grace::status::StatusCollector::collect(&root).await?;
        if let Some(module) = self.r#mod.as_deref() {
            let scope = resolve_module_scope(&root, module);
            if self.json || self.ci {
                let filtered = serde_json::json!({
                    "health": report.health,
                    "mygrace_issues": report.mygrace_issues,
                    "contracts": report.contracts.clone(),
                    "semantic": report.semantic.clone(),
                    "verification": report.verification.clone(),
                    "drift": report.drift.clone(),
                    "token_economy": report.token_economy,
                    "system": report.system,
                    "next_actions": report.next_actions.iter().filter(|a| a.contains(module) || scope.iter().any(|p| a.contains(&p.display().to_string()))).cloned().collect::<Vec<_>>()
                });
                println!("{}", serde_json::to_string_pretty(&filtered)?);
                return Ok(());
            }
            syn_engine::grace::status::StatusCollector::print_report(&report);
            return Ok(());
        }
        if self.json || self.ci {
            println!("{}", serde_json::to_string_pretty(&report)?);
            return Ok(());
        }
        syn_engine::grace::status::StatusCollector::print_report(&report);
        Ok(())
    }
    // END_status_run
}

impl RefreshCmd {
    // START_CONTRACT_RefreshCmd::run
    // PURPOSE: Report or fix canonical MyGRACE drift between code and shards
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_refresh_run
    pub async fn run(&self, _config: Config) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        let report = if self.fix {
            syn_engine::grace::refresh::Refresher::fix(&root)?
        } else {
            syn_engine::grace::refresh::Refresher::refresh(&root)?
        };

        if self.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
            return Ok(());
        }

        println!("=== GRACE Refresh Report ===");
        if report.fixed {
            println!("Canonical artifacts were rewritten from source MODULE_ID contracts.");
        }
        println!();
        println!("Code modules with contracts: {}", report.total_modules);
        println!("  In knowledge-graph:      {}", report.in_graph);
        println!("  In verification-plan:    {}", report.in_verification);
        println!();

        if !report.not_in_graph.is_empty() {
            println!("Modules NOT in knowledge-graph.xml:");
            for m in &report.not_in_graph {
                println!("  ✗ {}", m);
            }
            println!();
        }

        if !report.in_graph_not_in_code.is_empty() {
            println!("Stale entries in knowledge-graph.xml (not in code):");
            for m in &report.in_graph_not_in_code {
                println!("  ✗ {}", m);
            }
            println!();
        }

        if !report.not_in_verification.is_empty() {
            println!("Modules NOT in verification-plan.xml:");
            for m in &report.not_in_verification {
                println!("  ✗ {}", m);
            }
            println!();
        }

        if !report.contract_issues.is_empty() {
            println!("Contract issues:");
            for i in &report.contract_issues {
                println!("  ✗ {}", i);
            }
            println!();
        }

        if !report.canonical_drift.files_without_contract.is_empty() {
            println!("Governed files without MODULE_CONTRACT:");
            for path in &report.canonical_drift.files_without_contract {
                println!("  ✗ {}", path);
            }
            println!();
        }

        if !report.canonical_drift.duplicate_graph_ids.is_empty()
            || !report.canonical_drift.duplicate_verification_ids.is_empty()
            || !report.canonical_drift.graph_path_mismatches.is_empty()
            || !report
                .canonical_drift
                .verification_path_mismatches
                .is_empty()
            || !report.canonical_drift.missing_module_shards.is_empty()
            || !report
                .canonical_drift
                .missing_verification_shards
                .is_empty()
        {
            println!("Canonical MyGRACE drift:");
            for id in &report.canonical_drift.duplicate_graph_ids {
                println!("  ✗ duplicate graph id {}", id);
            }
            for id in &report.canonical_drift.duplicate_verification_ids {
                println!("  ✗ duplicate verification id {}", id);
            }
            for issue in &report.canonical_drift.graph_path_mismatches {
                println!("  ✗ {}", issue);
            }
            for issue in &report.canonical_drift.verification_path_mismatches {
                println!("  ✗ {}", issue);
            }
            for path in &report.canonical_drift.missing_module_shards {
                println!("  ✗ missing module shard {}", path);
            }
            for path in &report.canonical_drift.missing_verification_shards {
                println!("  ✗ missing verification shard {}", path);
            }
            println!();
        }

        if !report.suggested_actions.is_empty() {
            println!("Suggested actions:");
            for a in &report.suggested_actions {
                println!("  → {}", a);
            }
            println!();
        }

        if report.not_in_graph.is_empty()
            && report.not_in_verification.is_empty()
            && report.canonical_drift.is_clean()
            && report.contract_issues.is_empty()
        {
            println!("All modules synced. No drift detected.");
        }

        Ok(())
    }
    // END_refresh_run
}

impl SkillsCmd {
    // START_CONTRACT_SkillsCmd::run
    // PURPOSE: List, show, or execute registered MyGRACE workflow skills
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // START_skills_run
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        match self.action.as_str() {
            "list" => {
                println!("=== GRACE Skills ===");
                for skill in syn_skills::skills::registry::SKILL_DEFS {
                    println!("- {} — {}", skill.name, skill.description);
                }
                Ok(())
            }
            "show" => {
                let name = self
                    .name
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("Usage: syn skills show <name>"))?;
                match syn_skills::skills::registry::find_skill(name) {
                    Some(skill) => {
                        println!("{} — {}", skill.name, skill.description);
                        if !skill.args.is_empty() {
                            println!("Args:");
                            for arg in skill.args {
                                let req = if arg.required { "required" } else { "optional" };
                                println!("  - {} ({}) — {}", arg.name, req, arg.description);
                            }
                        }
                        Ok(())
                    }
                    None => anyhow::bail!("Unknown skill: {}", name),
                }
            }
            "run" => {
                let name = self.name.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("Usage: syn skills run <name> [key=value ...]")
                })?;
                let mut args = serde_json::Map::new();
                for raw in &self.args {
                    if let Some((k, v)) = raw.split_once('=') {
                        args.insert(k.to_string(), serde_json::Value::String(v.to_string()));
                    }
                }
                let engine = syn_skills::skills::SkillEngine::new(&config);
                let result = engine
                    .execute(syn_skills::skills::SkillRequest {
                        name: name.to_string(),
                        arguments: serde_json::Value::Object(args),
                    })
                    .await?;
                println!("{}\n\n{}", result.title, result.body);
                Ok(())
            }
            _ => anyhow::bail!("Usage: syn skills list|show|run <name> [key=value ...]"),
        }
    }
    // END_skills_run
}

// END_public_api
