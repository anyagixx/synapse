// MODULE_CONTRACT
// MODULE_ID: M-HOOKS
// PURPOSE: Hook manager for AI agents — installs/uninstalls Synapse integration files for OpenCode with safe MCP config merge and RTK-style shell autoproxy
// SCOPE: HookManager struct, install/uninstall/status for opencode agent, JSONC-aware MCP config merge, route-aware plugin and shell hook generation
// DEPENDS: M-CONFIG
// LINKS: .opencode/

// START_MODULE_MAP
// HookManager — Manages Synapse hook files for AI agent integration
// merge_synapse_mcp_config — Adds mcp.synapse to an existing OpenCode config without dropping sibling keys
// strip_jsonc_comments — Removes JSONC comments before safe config parsing
// strip_trailing_commas — Removes JSONC trailing commas before safe config parsing
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.0.0 — Expanded OpenCode shell autoproxy command coverage and session tracking]
// END_CHANGE_SUMMARY

use crate::config::Config;
use std::path::Path;

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
    // PURPOSE: Install Synapse hooks for a given agent (opencode/all)
    // INPUTS: { agent: &str — target agent name }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: creates files in .opencode/, prints status
    // START_hookmanager_install
    pub fn install(&self, agent: &str) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        match agent {
            "opencode" => self.install_opencode(&root),
            "all" => {
                self.install_opencode(&root)?;
                println!("All hooks installed for project at {}", root.display());
                Ok(())
            }
            _ => {
                println!("Unknown agent: {}. Supported: opencode, all", agent);
                println!(
                    "For generic integration, use `syn proxy -- <cmd>` to filter shell output."
                );
                Ok(())
            }
        }
    }
    // END_hookmanager_install

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

syn_proxy_should_proxy() {
    local cmd="$*"
    [[ "$cmd" =~ ^(syn\ proxy|rtk\ ) ]] && return 1
    [[ "$cmd" =~ ^(git|cargo|npm|npx|pnpm|yarn|bun|deno|uv|pytest|python\ -m\ pytest|ruff|mypy|basedpyright|pip|pip3|ls|cat|find|grep|rg|tree|docker|kubectl|helm|terraform|tofu|gh|glab|make|just|go\ build|go\ test|golangci-lint|gradle|gradlew|\./gradlew|dotnet|rake|rspec|pwd|which|du|wc|journalctl|systemctl) ]] && return 0
    return 1
}

syn_proxy_exec() {
    if syn_proxy_should_proxy "$@"; then
        syn proxy -- "$@"
    else
        "$@"
    fi
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

    // START_CONTRACT_HookManager::uninstall
    // PURPOSE: Uninstall Synapse hooks for a given agent
    // INPUTS: { agent: &str — target agent name }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: removes hook files from .opencode/
    // START_hookmanager_uninstall
    pub fn uninstall(&self, agent: &str) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        match agent {
            "opencode" | "all" => {
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
                println!("Uninstalled Synapse hooks from {}", root.display());
                Ok(())
            }
            _ => {
                println!("Unknown agent: {}. Nothing to uninstall.", agent);
                Ok(())
            }
        }
    }
    // END_hookmanager_uninstall

    // START_CONTRACT_HookManager::status
    // PURPOSE: Report the installation status of all Synapse hook files
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: prints status to stdout
    // START_hookmanager_status
    pub fn status(&self) -> anyhow::Result<()> {
        let root = std::env::current_dir()?;
        println!("Synapse Hook Status for {}", root.display());
        println!();

        let checks = [
            (".opencode/rules/synapse.md", "Rules (AI command reference)"),
            (
                ".opencode/plugins/synapse.ts",
                "Plugin (proxy + system transform)",
            ),
            (
                ".opencode/hooks/synapse-proxy.sh",
                "Shell hook (bash proxy)",
            ),
            ("opencode.jsonc", "MCP config (auto-start)"),
            (".opencode/package.json", "Plugin dependencies"),
        ];

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
            println!("\nRun `syn hooks install opencode` to install all hooks.");
        } else {
            println!("\nAll hooks installed. OpenCode will auto-start syn mcp.");
        }

        Ok(())
    }
    // END_hookmanager_status
}
// END_public_api

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
}
