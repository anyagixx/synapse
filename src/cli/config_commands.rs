// MODULE_CONTRACT
// MODULE_ID: M-CLI-CONFIG-COMMANDS
// PURPOSE: Explicit CLI config get, set, list, unset, path, and edit command handlers
// SCOPE: ConfigCmd dispatch, scalar key rendering, persisted config mutation through M-CONFIG
// DEPENDS: M-CLI, M-CONFIG
// LINKS:
//   → M-CLI (depends) - ConfigCmd and ConfigAction schemas
//   → M-CONFIG (depends) - typed config key lookup, update, defaults, and persistence
//   → UC-002 (implements) - CLI exposes bounded execution diagnostics

// START_MODULE_MAP
// ConfigCmd::run — Dispatches explicit config subcommands
// run_config_cmd — Executes path, edit, list, get, set, and unset actions
// render_config_list — Renders supported scalar config keys in stable order
// render_config_entry — Renders one scalar config entry
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Added persistent explicit config CLI commands]
// END_CHANGE_SUMMARY

use super::{ConfigAction, ConfigCmd};
use crate::config::{Config, ConfigEntry};

// START_public_api

impl ConfigCmd {
    // START_CONTRACT_ConfigCmd::run
    // PURPOSE: Execute explicit config path, edit, list, get, set, or unset subcommands
    // INPUTS: { config: Config }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: may spawn configured editor or persist synapsec.toml
    // START_config_cmd_run
    pub async fn run(self, config: Config) -> anyhow::Result<()> {
        run_config_cmd(self, config).await
    }
    // END_config_cmd_run
}

// START_CONTRACT_run_config_cmd
// PURPOSE: Execute explicit config actions without leaking unrelated config state for scalar get/set/unset
// INPUTS: { cmd: ConfigCmd }, { config: Config }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: may spawn configured editor or persist synapsec.toml
// START_run_config_cmd
async fn run_config_cmd(cmd: ConfigCmd, mut config: Config) -> anyhow::Result<()> {
    match cmd.action {
        None => println!("{}", toml::to_string_pretty(&config)?),
        Some(ConfigAction::Path) => println!("{}", Config::path()?.display()),
        Some(ConfigAction::Edit) => {
            let path = Config::path()?;
            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| "vim".into());
            std::process::Command::new(editor).arg(&path).status()?;
        }
        Some(ConfigAction::List) => print!("{}", render_config_list(&config)),
        Some(ConfigAction::Get { key }) => {
            let entry = config.get_config_key(&key)?;
            println!("{}", entry.value);
        }
        Some(ConfigAction::Set { key, value }) => {
            let entry = config.set_config_key(&key, &value)?;
            config.save()?;
            println!("{}", render_config_entry(&entry));
        }
        Some(ConfigAction::Unset { key }) => {
            let entry = config.unset_config_key(&key)?;
            config.save()?;
            println!("{}", render_config_entry(&entry));
        }
    }
    Ok(())
}
// END_run_config_cmd

// START_CONTRACT_render_config_list
// PURPOSE: Render supported scalar config entries in stable key order
// INPUTS: { config: &Config }
// OUTPUTS: { String }
// START_render_config_list
fn render_config_list(config: &Config) -> String {
    config
        .list_config_entries()
        .iter()
        .map(render_config_entry)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
// END_render_config_list

// START_CONTRACT_render_config_entry
// PURPOSE: Render one config entry as key = value for CLI output
// INPUTS: { entry: &ConfigEntry }
// OUTPUTS: { String }
// START_render_config_entry
fn render_config_entry(entry: &ConfigEntry) -> String {
    format!("{} = {}", entry.key, entry.value)
}
// END_render_config_entry

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_config_list_renders_supported_keys
    // PURPOSE: Verify config list renders supported typed keys without TOML sections
    // START_test_config_list_renders_supported_keys
    #[test]
    fn test_config_list_renders_supported_keys() {
        let rendered = render_config_list(&Config::default());

        assert!(rendered.contains("project.name = my-project"));
        assert!(rendered.contains("tracking.enabled = true"));
        assert!(!rendered.contains("[project]"));
    }
    // END_test_config_list_renders_supported_keys

    // START_CONTRACT_test_config_entry_rendering_is_single_key
    // PURPOSE: Verify get/set/unset output can print only one resulting config value
    // START_test_config_entry_rendering_is_single_key
    #[test]
    fn test_config_entry_rendering_is_single_key() {
        let entry = Config::default()
            .get_config_key("search.max_results")
            .expect("entry");

        assert_eq!(render_config_entry(&entry), "search.max_results = 20");
    }
    // END_test_config_entry_rendering_is_single_key
}
