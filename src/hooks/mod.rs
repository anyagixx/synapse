use crate::config::Config;
use std::path::Path;

pub struct HookManager {
    #[allow(dead_code)]
    config: Config,
}

impl HookManager {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

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
        let mcp_config = serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "mcp": {
                "synapse": {
                    "type": "local",
                    "command": ["syn", "mcp"],
                    "enabled": true
                }
            }
        });
        let oc_config_path = opencode_dir.join("opencode.jsonc");
        let existing = std::fs::read_to_string(&oc_config_path).unwrap_or_default();
        let mut current: serde_json::Value =
            serde_json::from_str(&existing).unwrap_or(serde_json::json!({}));
        if current.get("mcp").is_none() {
            current["mcp"] = mcp_config["mcp"].clone();
            std::fs::write(&oc_config_path, &serde_json::to_string_pretty(&current)?)?;
        }
        println!("  .opencode/opencode.jsonc   (MCP auto-start)");

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

syn_proxy_exec() {
    local cmd="$*"
    if [[ "$cmd" =~ ^(git|cargo|npm|npx|pnpm|yarn|ls|cat|find|grep|tree|docker|make|go\ build|go\ test|pwd|which|du|wc) ]]; then
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
alias make='syn_proxy_exec make'
alias go='syn_proxy_exec go'

echo "[synapse] Shell proxy hooks loaded"
"#;
        let bash_path = hooks_dir.join("synapse-proxy.sh");
        if !bash_path.exists() {
            std::fs::write(&bash_path, bash_hook)?;
        }
        println!("  .opencode/hooks/synapse-proxy.sh   (source this for shell proxy)");

        Ok(())
    }

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
                let oc_config_path = root.join(".opencode/opencode.jsonc");
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
            (".opencode/opencode.jsonc", "MCP config (auto-start)"),
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
}
