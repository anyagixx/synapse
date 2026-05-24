// MODULE_CONTRACT
// MODULE_ID: M-HOOKS
// PURPOSE: Hook manager for AI agents — installs/uninstalls/audits Synapse integration files for OpenCode and RTK-style multi-agent hook processors
// SCOPE: HookManager struct, install/uninstall/status/audit for opencode, claude, cursor, gemini, copilot, and all targets; JSONC-aware MCP config merge, syn rewrite delegating plugin, shell hook generation, and trusted processor manifests
// DEPENDS: M-CONFIG, M-CLI-RTK-COMMANDS
// LINKS: .opencode/, .synapse/hooks/, docs/phases/Phase-27.xml, docs/phases/Phase-55.xml

// START_MODULE_MAP
// HookManager — Manages Synapse hook files for AI agent integration
// HookAuditReport — Reports trusted hook installation checks for agents
// HookAuditItem — One hook audit check result
// AgentHookSpec — Describes a trusted local hook processor manifest target
// merge_synapse_mcp_config — Adds mcp.synapse to an existing OpenCode config without dropping sibling keys
// strip_jsonc_comments — Removes JSONC comments before safe config parsing
// strip_trailing_commas — Removes JSONC trailing commas before safe config parsing
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.3.0 — Added multi-agent hook install and audit targets]
// END_CHANGE_SUMMARY

use crate::config::Config;
use std::path::Path;

const AGENT_HOOK_DIR: &str = ".synapse/hooks";
const OPENCODE_AGENT: &str = "opencode";
const ALL_AGENTS: &str = "all";

// START_public_api

// START_CONTRACT_merge_synapse_mcp_config
// PURPOSE: Merge Synapse MCP configuration into an OpenCode JSON/JSONC config while preserving unrelated keys and MCP siblings
// INPUTS: { existing: &str — current opencode.jsonc contents, possibly empty }
// OUTPUTS: { anyhow::Result<String> — pretty JSON config with mcp.synapse configured }
// START_merge_synapse_mcp_config
pub fn merge_synapse_mcp_config(existing: &str) -> anyhow::Result<String> {
    let mut current = if existing.trim().is_empty() {
        serde_json::json!({
            "$schema": "https://opencode.ai/config.json"
        })
    } else {
        let without_comments = strip_jsonc_comments(existing);
        let normalized = strip_trailing_commas(&without_comments);
        serde_json::from_str::<serde_json::Value>(&normalized).map_err(|e| {
            anyhow::anyhow!(
                "cannot parse existing opencode.jsonc for safe merge: {}. Fix the config or remove it, then rerun syn init.",
                e
            )
        })?
    };

    let root = current
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("opencode.jsonc root must be a JSON object"))?;
    let mcp = root
        .entry("mcp".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !mcp.is_object() {
        *mcp = serde_json::json!({});
    }
    let mcp_obj = mcp
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("opencode.jsonc mcp must be a JSON object"))?;
    mcp_obj.insert(
        "synapse".to_string(),
        serde_json::json!({
            "type": "local",
            "command": ["syn", "mcp"],
            "enabled": true
        }),
    );

    Ok(serde_json::to_string_pretty(&current)?)
}
// END_merge_synapse_mcp_config

// START_CONTRACT_strip_jsonc_comments
// PURPOSE: Remove // and /* */ comments outside JSON strings before parsing JSONC-like config
// INPUTS: { input: &str }
// OUTPUTS: { String }
// START_strip_jsonc_comments
fn strip_jsonc_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }

        if ch == '/' {
            match chars.peek().copied() {
                Some('/') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if next == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut previous = '\0';
                    for next in chars.by_ref() {
                        if previous == '*' && next == '/' {
                            break;
                        }
                        previous = next;
                    }
                    continue;
                }
                _ => {}
            }
        }

        out.push(ch);
    }

    out
}
// END_strip_jsonc_comments

// START_CONTRACT_strip_trailing_commas
// PURPOSE: Remove trailing commas before object and array terminators outside JSON strings
// INPUTS: { input: &str }
// OUTPUTS: { String }
// START_strip_trailing_commas
fn strip_trailing_commas(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut index = 0usize;

    while index < chars.len() {
        let ch = chars[index];
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            index += 1;
            continue;
        }

        if ch == ',' {
            let mut lookahead = index + 1;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }
            if lookahead < chars.len() && matches!(chars[lookahead], '}' | ']') {
                index += 1;
                continue;
            }
        }

        out.push(ch);
        index += 1;
    }

    out
}
// END_strip_trailing_commas

// START_HookManager
pub struct HookManager {
    #[allow(dead_code)]
    config: Config,
}
// END_HookManager

// START_HookAuditReport
#[derive(Debug, Clone, serde::Serialize)]
pub struct HookAuditReport {
    pub agent: String,
    pub root: String,
    pub ok: bool,
    pub checks: Vec<HookAuditItem>,
}
// END_HookAuditReport

// START_HookAuditItem
#[derive(Debug, Clone, serde::Serialize)]
pub struct HookAuditItem {
    pub name: String,
    pub path: String,
    pub ok: bool,
    pub status: String,
    pub detail: String,
}
// END_HookAuditItem

// START_AgentHookSpec
#[derive(Debug, Clone, Copy)]
struct AgentHookSpec {
    agent: &'static str,
    event: &'static str,
    file_name: &'static str,
    install_hint: &'static str,
}
// END_AgentHookSpec

// START_CONTRACT_processor_hook_specs
// PURPOSE: Return trusted non-OpenCode hook processor install targets
// OUTPUTS: { &'static [AgentHookSpec] }
// LINKS:
//   → M-HOOKS (depends) - multi-agent hook install/audit inventory
//   → Phase-55 (implements) - install/audit targets beyond OpenCode
// START_processor_hook_specs
fn processor_hook_specs() -> &'static [AgentHookSpec] {
    &[
        AgentHookSpec {
            agent: "claude",
            event: "PreToolUse",
            file_name: "claude.json",
            install_hint: "Reference this manifest from Claude Code PreToolUse Bash hooks.",
        },
        AgentHookSpec {
            agent: "cursor",
            event: "preToolUse",
            file_name: "cursor.json",
            install_hint: "Reference this manifest from Cursor Agent shell hook configuration.",
        },
        AgentHookSpec {
            agent: "gemini",
            event: "BeforeTool",
            file_name: "gemini.json",
            install_hint: "Reference this manifest from Gemini CLI before-tool hooks.",
        },
        AgentHookSpec {
            agent: "copilot",
            event: "preToolUse",
            file_name: "copilot.json",
            install_hint: "Reference this manifest from GitHub Copilot hook/task integration.",
        },
    ]
}
// END_processor_hook_specs

// START_CONTRACT_processor_hook_spec
// PURPOSE: Find one trusted hook processor manifest spec by agent name
// INPUTS: { agent: &str }
// OUTPUTS: { Option<&'static AgentHookSpec> }
// LINKS:
//   → M-HOOKS (depends) - multi-agent hook target lookup
// START_processor_hook_spec
fn processor_hook_spec(agent: &str) -> Option<&'static AgentHookSpec> {
    processor_hook_specs()
        .iter()
        .find(|spec| spec.agent == agent)
}
// END_processor_hook_spec

// START_CONTRACT_supported_hook_agents
// PURPOSE: Render the supported hook target list for CLI diagnostics
// OUTPUTS: { &'static str }
// START_supported_hook_agents
fn supported_hook_agents() -> &'static str {
    "opencode, claude, cursor, gemini, copilot, all"
}
// END_supported_hook_agents

impl HookManager {
    // START_CONTRACT_HookManager::new
    // PURPOSE: Create a new HookManager
    // OUTPUTS: { Self }
    // START_hookmanager_new
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }
    // END_hookmanager_new

    // START_CONTRACT_HookManager::install
    // PURPOSE: Install Synapse hooks for a given agent or all supported hook targets
    // INPUTS: { agent: &str — target agent name }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: creates files in .opencode/ or .synapse/hooks/, prints status
    // START_hookmanager_install
    pub fn install(&self, agent: &str) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        match agent {
            OPENCODE_AGENT => self.install_opencode(&root),
            ALL_AGENTS => {
                self.install_opencode(&root)?;
                for spec in processor_hook_specs() {
                    self.install_agent_manifest(&root, spec)?;
                }
                println!("All hooks installed for project at {}", root.display());
                Ok(())
            }
            name if processor_hook_spec(name).is_some() => {
                let spec = processor_hook_spec(name)
                    .ok_or_else(|| anyhow::anyhow!("unknown hook agent: {name}"))?;
                self.install_agent_manifest(&root, spec)
            }
            _ => {
                anyhow::bail!(
                    "Unknown agent: {}. Supported: {}",
                    agent,
                    supported_hook_agents()
                )
            }
        }
    }
    // END_hookmanager_install

    // START_CONTRACT_HookManager::audit
    // PURPOSE: Audit hook installation and trusted content markers
    // INPUTS: { agent: &str — target agent name }, { json: bool — render machine-readable output }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: prints audit report and fails when required hook trust checks fail
    // LINKS:
    //   → M-HOOK-OPENCODE-REWRITE (depends) - audit verifies syn rewrite delegation markers
    //   → NFR-003 (traces_to) - audit confirms shell commands are auto-proxied for token savings
    // START_hookmanager_audit
    pub fn audit(&self, agent: &str, json: bool) -> anyhow::Result<()> {
        let report = self.audit_report(agent)?;
        if json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print_hook_audit_report(&report);
        }
        if !report.ok {
            anyhow::bail!("hook audit failed for {}", report.agent);
        }
        Ok(())
    }
    // END_hookmanager_audit

    // START_CONTRACT_HookManager::audit_report
    // PURPOSE: Build a hook audit report without printing
    // INPUTS: { agent: &str — target agent name }
    // OUTPUTS: { anyhow::Result<HookAuditReport> }
    // LINKS:
    //   → M-HOOKS (depends) - reuses installed OpenCode hook contract
    // START_hookmanager_audit_report
    pub fn audit_report(&self, agent: &str) -> anyhow::Result<HookAuditReport> {
        let root = std::env::current_dir()?;
        match agent {
            OPENCODE_AGENT => self.audit_opencode(&root),
            ALL_AGENTS => self.audit_all(&root),
            name if processor_hook_spec(name).is_some() => {
                let spec = processor_hook_spec(name)
                    .ok_or_else(|| anyhow::anyhow!("unknown hook agent: {name}"))?;
                Ok(self.audit_agent_manifest(&root, spec))
            }
            _ => anyhow::bail!(
                "Unknown agent: {}. Supported: {}",
                agent,
                supported_hook_agents()
            ),
        }
    }
    // END_hookmanager_audit_report

    // START_CONTRACT_HookManager::install_opencode
    // PURPOSE: Generate OpenCode rules, MCP config, plugin, package metadata, and shell proxy hook
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes .opencode files and opencode.jsonc
    // LINKS:
    //   → UC-001 (implements) - installs OpenCode integration assets
    //   → NFR-003 (traces_to) - shell hook autoproxies high-volume commands
    // START_hookmanager_install_opencode
    fn install_opencode(&self, root: &Path) -> anyhow::Result<()> {
        let opencode_dir = root.join(".opencode");
        std::fs::create_dir_all(&opencode_dir)?;

        // 1. Rules file — tells AI about Synapse commands
        let rules_dir = opencode_dir.join("rules");
        std::fs::create_dir_all(&rules_dir)?;
        let rules_content = include_str!("../../.opencode/rules/synapse.md");
        std::fs::write(rules_dir.join("synapse.md"), rules_content)?;
        println!("  .opencode/rules/synapse.md");

        // 2. MCP auto-start config
        let oc_config_path = root.join("opencode.jsonc");
        let existing = std::fs::read_to_string(&oc_config_path).unwrap_or_default();
        let merged = merge_synapse_mcp_config(&existing)?;
        if existing != merged {
            std::fs::write(&oc_config_path, merged)?;
        }
        println!("  opencode.jsonc              (project root) → MCP auto-start");

        // 3. Plugin file
        let plugins_dir = opencode_dir.join("plugins");
        std::fs::create_dir_all(&plugins_dir)?;
        let plugin_path = plugins_dir.join("synapse.ts");
        if !plugin_path.exists() {
            let plugin_content = include_str!("../../.opencode/plugins/synapse.ts");
            std::fs::write(&plugin_path, plugin_content)?;
        }
        println!("  .opencode/plugins/synapse.ts");

        // 4. Package.json for plugin dependencies
        let pkg_path = opencode_dir.join("package.json");
        if !pkg_path.exists() {
            std::fs::write(
                &pkg_path,
                r#"{"dependencies":{"@opencode-ai/plugin":"^1.15"}}"#,
            )?;
        }
        println!("  .opencode/package.json");

        // 5. Bash hook script for shell-level command rewriting
        let hooks_dir = root.join(".opencode").join("hooks");
        std::fs::create_dir_all(&hooks_dir)?;
        let bash_hook = r#"#!/bin/bash
# Synapse proxy hook — rewrites shell commands through `syn proxy`
# Source this file in your shell to enable: source .opencode/hooks/synapse-proxy.sh

export SYNAPSE_SESSION_ID="${SYNAPSE_SESSION_ID:-opencode-$(date +%Y%m%d%H%M%S)-$$}"

syn_proxy_exec() {
    local rewritten
    rewritten="$(syn rewrite "$@" 2>/dev/null)" || {
        "$@"
        return
    }
    if [[ -z "$rewritten" || "$rewritten" == "$*" ]]; then
        "$@"
        return
    fi
    bash -lc "$rewritten"
}

alias git='syn_proxy_exec git'
alias cargo='syn_proxy_exec cargo'
alias npm='syn_proxy_exec npm'
alias npx='syn_proxy_exec npx'
alias pnpm='syn_proxy_exec pnpm'
alias yarn='syn_proxy_exec yarn'
alias bun='syn_proxy_exec bun'
alias deno='syn_proxy_exec deno'
alias uv='syn_proxy_exec uv'
alias pytest='syn_proxy_exec pytest'
alias python='syn_proxy_exec python'
alias ruff='syn_proxy_exec ruff'
alias mypy='syn_proxy_exec mypy'
alias rg='syn_proxy_exec rg'
alias make='syn_proxy_exec make'
alias just='syn_proxy_exec just'
alias go='syn_proxy_exec go'
alias docker='syn_proxy_exec docker'
alias kubectl='syn_proxy_exec kubectl'
alias helm='syn_proxy_exec helm'
alias terraform='syn_proxy_exec terraform'
alias tofu='syn_proxy_exec tofu'
alias gh='syn_proxy_exec gh'
alias glab='syn_proxy_exec glab'

echo "[synapse] Shell proxy hooks loaded (session ${SYNAPSE_SESSION_ID})"
"#;
        let bash_path = hooks_dir.join("synapse-proxy.sh");
        if !bash_path.exists() {
            std::fs::write(&bash_path, bash_hook)?;
        }
        println!("  .opencode/hooks/synapse-proxy.sh   (source this for shell proxy)");

        Ok(())
    }
    // END_hookmanager_install_opencode

    // START_CONTRACT_HookManager::audit_opencode
    // PURPOSE: Check OpenCode hook files, MCP config, and trusted rewrite/session markers
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<HookAuditReport> }
    // LINKS:
    //   → M-HOOK-OPENCODE-REWRITE (depends) - trusted markers prove hooks delegate to syn rewrite
    // START_hookmanager_audit_opencode
    fn audit_opencode(&self, root: &Path) -> anyhow::Result<HookAuditReport> {
        let mut checks = vec![
            audit_file_exists(
                root,
                ".opencode/rules/synapse.md",
                "rules-file",
                "OpenCode has the Synapse/MyGRACE operating rules.",
            ),
            audit_file_contains(
                root,
                ".opencode/plugins/synapse.ts",
                "plugin-rewrite",
                "syn rewrite",
                "OpenCode plugin delegates command routing to syn rewrite.",
            ),
            audit_file_contains(
                root,
                ".opencode/plugins/synapse.ts",
                "plugin-session",
                "SYNAPSE_SESSION_ID",
                "OpenCode plugin propagates session identity for economics.",
            ),
            audit_file_contains(
                root,
                ".opencode/hooks/synapse-proxy.sh",
                "shell-rewrite",
                "syn rewrite",
                "Shell hook delegates routing to syn rewrite.",
            ),
            audit_file_contains(
                root,
                ".opencode/hooks/synapse-proxy.sh",
                "shell-session",
                "SYNAPSE_SESSION_ID",
                "Shell hook creates or propagates a token-economy session id.",
            ),
            audit_file_contains(
                root,
                ".opencode/hooks/synapse-proxy.sh",
                "shell-container-alias",
                "alias docker='syn_proxy_exec docker'",
                "Shell hook covers container commands added for RTK parity.",
            ),
            audit_file_exists(
                root,
                ".opencode/package.json",
                "plugin-dependencies",
                "OpenCode plugin dependency metadata is present.",
            ),
        ];
        checks.push(audit_opencode_mcp_config(root));
        let ok = checks.iter().all(|item| item.ok);
        Ok(HookAuditReport {
            agent: "opencode".into(),
            root: root.display().to_string(),
            ok,
            checks,
        })
    }
    // END_hookmanager_audit_opencode

    // START_CONTRACT_HookManager::install_agent_manifest
    // PURPOSE: Generate a trusted local manifest for a non-OpenCode hook processor
    // INPUTS: { root: &Path }, { spec: &AgentHookSpec }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes .synapse/hooks/<agent>.json and prints the installed path
    // LINKS:
    //   → M-HOOKS (depends) - install target for RTK-style hook processors
    //   → Phase-55 (implements) - non-OpenCode hook install parity
    // START_hookmanager_install_agent_manifest
    fn install_agent_manifest(&self, root: &Path, spec: &AgentHookSpec) -> anyhow::Result<()> {
        let hook_dir = root.join(AGENT_HOOK_DIR);
        std::fs::create_dir_all(&hook_dir)?;
        let manifest_path = hook_dir.join(spec.file_name);
        let processor_command = format!("syn hook {}", spec.agent);
        let manifest = serde_json::json!({
            "agent": spec.agent,
            "event": spec.event,
            "command": processor_command,
            "delegates_to": "syn rewrite",
            "token_savings": "RTK-style shell auto-proxy through Synapse",
            "install_hint": spec.install_hint
        });
        std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
        println!("  {}", manifest_path.strip_prefix(root)?.display());
        Ok(())
    }
    // END_hookmanager_install_agent_manifest

    // START_CONTRACT_HookManager::audit_agent_manifest
    // PURPOSE: Verify a non-OpenCode hook processor manifest contains trusted delegation markers
    // INPUTS: { root: &Path }, { spec: &AgentHookSpec }
    // OUTPUTS: { HookAuditReport }
    // LINKS:
    //   → M-HOOKS (depends) - audit target for RTK-style hook processors
    //   → Phase-55 (implements) - non-OpenCode hook audit parity
    // START_hookmanager_audit_agent_manifest
    fn audit_agent_manifest(&self, root: &Path, spec: &AgentHookSpec) -> HookAuditReport {
        let path = format!("{AGENT_HOOK_DIR}/{}", spec.file_name);
        let processor_marker = format!("syn hook {}", spec.agent);
        let checks = vec![
            audit_file_contains(
                root,
                &path,
                "processor-command",
                &processor_marker,
                "Manifest delegates the agent hook to the matching syn hook processor.",
            ),
            audit_file_contains(
                root,
                &path,
                "rewrite-delegation",
                "syn rewrite",
                "Manifest declares syn rewrite as the routing source of truth.",
            ),
        ];
        let ok = checks.iter().all(|item| item.ok);
        HookAuditReport {
            agent: spec.agent.into(),
            root: root.display().to_string(),
            ok,
            checks,
        }
    }
    // END_hookmanager_audit_agent_manifest

    // START_CONTRACT_HookManager::audit_all
    // PURPOSE: Audit OpenCode assets plus every trusted processor manifest as one aggregate report
    // INPUTS: { root: &Path }
    // OUTPUTS: { anyhow::Result<HookAuditReport> }
    // LINKS:
    //   → M-HOOKS (depends) - multi-agent hook audit aggregation
    //   → Phase-55 (implements) - all-target hook audit parity
    // START_hookmanager_audit_all
    fn audit_all(&self, root: &Path) -> anyhow::Result<HookAuditReport> {
        let mut checks = self.audit_opencode(root)?.checks;
        for spec in processor_hook_specs() {
            checks.extend(self.audit_agent_manifest(root, spec).checks);
        }
        let ok = checks.iter().all(|item| item.ok);
        Ok(HookAuditReport {
            agent: ALL_AGENTS.into(),
            root: root.display().to_string(),
            ok,
            checks,
        })
    }
    // END_hookmanager_audit_all

    // START_CONTRACT_HookManager::uninstall
    // PURPOSE: Uninstall Synapse hooks for a given agent
    // INPUTS: { agent: &str — target agent name }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: removes hook files from .opencode/
    // START_hookmanager_uninstall
    pub fn uninstall(&self, agent: &str) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        match agent {
            OPENCODE_AGENT | ALL_AGENTS => {
                let files = [
                    root.join(".opencode/rules/synapse.md"),
                    root.join(".opencode/plugins/synapse.ts"),
                    root.join(".opencode/hooks/synapse-proxy.sh"),
                ];
                for f in &files {
                    if f.exists() {
                        std::fs::remove_file(f)?;
                        println!("Removed {}", f.display());
                    }
                }
                // Don't remove opencode.jsonc (may have other config), just remove MCP section
                let oc_config_path = root.join("opencode.jsonc"); // Config is at project root
                if oc_config_path.exists() {
                    let existing = std::fs::read_to_string(&oc_config_path)?;
                    if let Ok(mut current) = serde_json::from_str::<serde_json::Value>(&existing) {
                        if let Some(obj) = current.as_object_mut() {
                            obj.remove("mcp");
                            if !obj.is_empty() {
                                std::fs::write(
                                    &oc_config_path,
                                    &serde_json::to_string_pretty(&current)?,
                                )?;
                            } else {
                                std::fs::remove_file(&oc_config_path)?;
                            }
                        }
                    }
                }
                if agent == ALL_AGENTS {
                    for spec in processor_hook_specs() {
                        self.uninstall_agent_manifest(&root, spec)?;
                    }
                }
                println!("Uninstalled Synapse hooks from {}", root.display());
                Ok(())
            }
            name if processor_hook_spec(name).is_some() => {
                let spec = processor_hook_spec(name)
                    .ok_or_else(|| anyhow::anyhow!("unknown hook agent: {name}"))?;
                self.uninstall_agent_manifest(&root, spec)
            }
            _ => {
                anyhow::bail!(
                    "Unknown agent: {}. Supported: {}",
                    agent,
                    supported_hook_agents()
                )
            }
        }
    }
    // END_hookmanager_uninstall

    // START_CONTRACT_HookManager::uninstall_agent_manifest
    // PURPOSE: Remove one non-OpenCode hook processor manifest if present
    // INPUTS: { root: &Path }, { spec: &AgentHookSpec }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: removes .synapse/hooks/<agent>.json and prints removal status
    // LINKS:
    //   → M-HOOKS (depends) - multi-agent hook uninstall target
    // START_hookmanager_uninstall_agent_manifest
    fn uninstall_agent_manifest(&self, root: &Path, spec: &AgentHookSpec) -> anyhow::Result<()> {
        let path = root.join(AGENT_HOOK_DIR).join(spec.file_name);
        if path.exists() {
            std::fs::remove_file(&path)?;
            println!("Removed {}", path.display());
        }
        Ok(())
    }
    // END_hookmanager_uninstall_agent_manifest

    // START_CONTRACT_HookManager::status
    // PURPOSE: Report the installation status of Synapse hook files for one target
    // INPUTS: { agent: &str — target agent name }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: prints status to stdout
    // START_hookmanager_status
    pub fn status(&self, agent: &str) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        println!("Synapse Hook Status for {agent} at {}", root.display());
        println!();

        let checks = status_checks(agent)?;

        let mut all_ok = true;
        for (path, desc) in &checks {
            let full = root.join(path);
            let status = if full.exists() {
                "INSTALLED"
            } else {
                "MISSING"
            };
            if status == "MISSING" {
                all_ok = false;
            }
            println!("  [{}] {} — {}", status, path, desc);
        }

        if !all_ok {
            println!("\nRun `syn hooks install {agent}` to install missing hooks.");
        } else {
            println!("\nRequested hooks are installed.");
        }

        Ok(())
    }
    // END_hookmanager_status
}
// END_public_api

// START_CONTRACT_status_checks
// PURPOSE: Return expected hook files for one status target
// INPUTS: { agent: &str }
// OUTPUTS: { anyhow::Result<Vec<(String, String)>> }
// LINKS:
//   → M-HOOKS (depends) - multi-agent hook status output
// START_status_checks
fn status_checks(agent: &str) -> anyhow::Result<Vec<(String, String)>> {
    match agent {
        OPENCODE_AGENT => Ok(opencode_status_checks()),
        ALL_AGENTS => {
            let mut checks = opencode_status_checks();
            checks.extend(
                processor_hook_specs()
                    .iter()
                    .map(agent_manifest_status_check),
            );
            Ok(checks)
        }
        name if processor_hook_spec(name).is_some() => {
            let spec = processor_hook_spec(name)
                .ok_or_else(|| anyhow::anyhow!("unknown hook agent: {name}"))?;
            Ok(vec![agent_manifest_status_check(spec)])
        }
        _ => anyhow::bail!(
            "Unknown agent: {}. Supported: {}",
            agent,
            supported_hook_agents()
        ),
    }
}
// END_status_checks

// START_CONTRACT_opencode_status_checks
// PURPOSE: Return expected OpenCode hook files for status output
// OUTPUTS: { Vec<(String, String)> }
// LINKS:
//   → M-HOOK-OPENCODE-REWRITE (depends) - OpenCode install surface
// START_opencode_status_checks
fn opencode_status_checks() -> Vec<(String, String)> {
    vec![
        (
            ".opencode/rules/synapse.md".into(),
            "Rules (AI command reference)".into(),
        ),
        (
            ".opencode/plugins/synapse.ts".into(),
            "Plugin (proxy + system transform)".into(),
        ),
        (
            ".opencode/hooks/synapse-proxy.sh".into(),
            "Shell hook (bash proxy)".into(),
        ),
        ("opencode.jsonc".into(), "MCP config (auto-start)".into()),
        (
            ".opencode/package.json".into(),
            "Plugin dependencies".into(),
        ),
    ]
}
// END_opencode_status_checks

// START_CONTRACT_agent_manifest_status_check
// PURPOSE: Return one expected processor manifest file for status output
// INPUTS: { spec: &AgentHookSpec }
// OUTPUTS: { (String, String) }
// LINKS:
//   → M-HOOKS (depends) - processor manifest status
// START_agent_manifest_status_check
fn agent_manifest_status_check(spec: &AgentHookSpec) -> (String, String) {
    (
        format!("{AGENT_HOOK_DIR}/{}", spec.file_name),
        format!("{} hook processor manifest", spec.agent),
    )
}
// END_agent_manifest_status_check

// START_CONTRACT_audit_file_exists
// PURPOSE: Create a hook audit result for a required file
// INPUTS: { root: &Path }, { path: &str }, { name: &str }, { detail: &str }
// OUTPUTS: { HookAuditItem }
// START_audit_file_exists
fn audit_file_exists(root: &Path, path: &str, name: &str, detail: &str) -> HookAuditItem {
    let full = root.join(path);
    HookAuditItem {
        name: name.into(),
        path: path.into(),
        ok: full.exists(),
        status: if full.exists() { "trusted" } else { "missing" }.into(),
        detail: detail.into(),
    }
}
// END_audit_file_exists

// START_CONTRACT_audit_file_contains
// PURPOSE: Create a hook audit result for trusted content marker presence
// INPUTS: { root: &Path }, { path: &str }, { name: &str }, { marker: &str }, { detail: &str }
// OUTPUTS: { HookAuditItem }
// START_audit_file_contains
fn audit_file_contains(
    root: &Path,
    path: &str,
    name: &str,
    marker: &str,
    detail: &str,
) -> HookAuditItem {
    let full = root.join(path);
    let status = match std::fs::read_to_string(&full) {
        Ok(content) if content.contains(marker) => ("trusted", true),
        Ok(_) => ("stale", false),
        Err(_) => ("missing", false),
    };
    HookAuditItem {
        name: name.into(),
        path: path.into(),
        ok: status.1,
        status: status.0.into(),
        detail: detail.into(),
    }
}
// END_audit_file_contains

// START_CONTRACT_audit_opencode_mcp_config
// PURPOSE: Validate opencode.jsonc contains a trusted Synapse MCP server declaration
// INPUTS: { root: &Path }
// OUTPUTS: { HookAuditItem }
// LINKS:
//   → M-MCP (depends) - OpenCode auto-starts syn mcp from this config
// START_audit_opencode_mcp_config
fn audit_opencode_mcp_config(root: &Path) -> HookAuditItem {
    let path = "opencode.jsonc";
    let full = root.join(path);
    let Ok(existing) = std::fs::read_to_string(&full) else {
        return HookAuditItem {
            name: "mcp-config".into(),
            path: path.into(),
            ok: false,
            status: "missing".into(),
            detail: "OpenCode MCP config is missing.".into(),
        };
    };
    let without_comments = strip_jsonc_comments(&existing);
    let normalized = strip_trailing_commas(&without_comments);
    let value = serde_json::from_str::<serde_json::Value>(&normalized);
    let ok = value
        .ok()
        .and_then(|value| {
            let synapse = value.get("mcp")?.get("synapse")?;
            let command = synapse.get("command")?.as_array()?;
            let enabled = synapse
                .get("enabled")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);
            let matches_command = command.len() == 2
                && command[0].as_str() == Some("syn")
                && command[1].as_str() == Some("mcp");
            Some(enabled && matches_command)
        })
        .unwrap_or(false);

    HookAuditItem {
        name: "mcp-config".into(),
        path: path.into(),
        ok,
        status: if ok { "trusted" } else { "stale" }.into(),
        detail: "OpenCode config starts Synapse MCP with command [\"syn\", \"mcp\"].".into(),
    }
}
// END_audit_opencode_mcp_config

// START_CONTRACT_print_hook_audit_report
// PURPOSE: Render a compact human-readable hook audit report
// INPUTS: { report: &HookAuditReport }
// OUTPUTS: { stdout audit lines }
// SIDE_EFFECTS: writes to stdout
// START_print_hook_audit_report
fn print_hook_audit_report(report: &HookAuditReport) {
    println!("Synapse Hook Audit for {} at {}", report.agent, report.root);
    for item in &report.checks {
        println!(
            "  [{}] {} — {} ({})",
            item.status.to_ascii_uppercase(),
            item.path,
            item.name,
            item.detail
        );
    }
    if report.ok {
        println!("\nHook audit passed. Hooks are trusted for RTK routing.");
    } else {
        println!(
            "\nHook audit failed. Run `syn hooks install {}` and rerun audit.",
            report.agent
        );
    }
}
// END_print_hook_audit_report

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_synapse_mcp_config_preserves_existing_mcp_sibling() {
        let existing = r#"{
          "$schema": "https://opencode.ai/config.json",
          "mcp": {
            "other": {
              "type": "local",
              "command": ["other"],
              "enabled": true
            }
          },
          "theme": "dark"
        }"#;

        let merged = merge_synapse_mcp_config(existing);
        assert!(merged.is_ok(), "merge should succeed: {:?}", merged.err());
        let merged = merged.unwrap_or_default();
        let value = serde_json::from_str::<serde_json::Value>(&merged);
        assert!(value.is_ok(), "merged config should parse as JSON");
        let value = value.unwrap_or_default();
        assert!(value["mcp"]["other"].is_object());
        assert_eq!(value["mcp"]["synapse"]["command"][0], "syn");
        assert_eq!(value["theme"], "dark");
    }

    #[test]
    fn test_merge_synapse_mcp_config_accepts_jsonc_comments_and_trailing_commas() {
        let existing = r#"{
          // user comment
          "mcp": {
            "other": {"type": "local", "command": ["other"], "enabled": true,},
          },
        }"#;

        let merged = merge_synapse_mcp_config(existing);
        assert!(
            merged.is_ok(),
            "JSONC merge should succeed: {:?}",
            merged.err()
        );
        let merged = merged.unwrap_or_default();
        let value = serde_json::from_str::<serde_json::Value>(&merged);
        assert!(value.is_ok(), "merged config should parse as JSON");
        let value = value.unwrap_or_default();
        assert!(value["mcp"]["other"].is_object());
        assert_eq!(value["mcp"]["synapse"]["enabled"], true);
    }

    #[test]
    fn test_hook_audit_reports_missing_opencode_assets() {
        let root = tempfile::tempdir().unwrap();
        let manager = HookManager::new(&Config::default());
        let report = manager.audit_opencode(root.path()).unwrap();

        assert!(!report.ok);
        assert!(report
            .checks
            .iter()
            .any(|item| item.name == "mcp-config" && item.status == "missing"));
    }

    #[test]
    fn test_hook_audit_trusts_generated_opencode_assets() {
        let root = tempfile::tempdir().unwrap();
        let manager = HookManager::new(&Config::default());
        manager.install_opencode(root.path()).unwrap();
        let report = manager.audit_opencode(root.path()).unwrap();

        assert!(report.ok);
        assert!(report
            .checks
            .iter()
            .any(|item| item.name == "shell-container-alias" && item.ok));
        assert!(report
            .checks
            .iter()
            .any(|item| item.name == "mcp-config" && item.status == "trusted"));
    }

    #[test]
    fn test_hook_audit_trusts_generated_agent_manifest() {
        let root = tempfile::tempdir().unwrap();
        let manager = HookManager::new(&Config::default());
        let spec = processor_hook_spec("claude").unwrap();

        manager.install_agent_manifest(root.path(), spec).unwrap();
        let report = manager.audit_agent_manifest(root.path(), spec);

        assert!(report.ok);
        assert!(report
            .checks
            .iter()
            .any(|item| item.name == "processor-command" && item.ok));
        assert!(report
            .checks
            .iter()
            .any(|item| item.name == "rewrite-delegation" && item.ok));
    }

    #[test]
    fn test_hook_audit_all_trusts_opencode_and_processor_manifests() {
        let root = tempfile::tempdir().unwrap();
        let manager = HookManager::new(&Config::default());

        manager.install_opencode(root.path()).unwrap();
        for spec in processor_hook_specs() {
            manager.install_agent_manifest(root.path(), spec).unwrap();
        }
        let report = manager.audit_all(root.path()).unwrap();

        assert!(report.ok);
        assert!(report
            .checks
            .iter()
            .any(|item| item.path == ".synapse/hooks/copilot.json" && item.ok));
    }
}
