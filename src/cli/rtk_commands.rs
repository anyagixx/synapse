// MODULE_CONTRACT
// MODULE_ID: M-CLI-RTK-COMMANDS
// PURPOSE: First-class RTK-style CLI shortcuts that execute common tools through the Synapse proxy
// SCOPE: RtkProxyCmd shortcut dispatch for read, ls, tree, find, rg, grep, git, cargo, npm, pnpm, npx, and pytest
// DEPENDS: M-CONFIG, M-CLI-RUNTIME-COMMANDS
// LINKS:
//   → M-CLI-RUNTIME-COMMANDS (depends) - delegates execution to ProxyCmd
//   → M-PROXY (depends) - proxy execution, filtering, tracking, and evidence
//   → UC-002 (implements) - token-saving command execution evidence
//   → NFR-003 (traces_to) - shorter command surface increases proxy adoption

// START_MODULE_MAP
// RtkProxyCmd::run_as — Prefixes a native command and delegates to ProxyCmd
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Added first-class RTK proxy shortcuts]
// END_CHANGE_SUMMARY

use super::{ProxyCmd, RtkProxyCmd};
use crate::config::Config;

// START_public_api

impl RtkProxyCmd {
    // START_CONTRACT_RtkProxyCmd::run_as
    // PURPOSE: Execute a first-class RTK-style shortcut by prefixing the native executable and delegating to syn proxy
    // INPUTS: { config: Config }, { executable: &str - native command to run }, { require_args: bool }, { usage: &str }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: may run external command through ProxyCmd
    // LINKS:
    //   → M-CLI-RUNTIME-COMMANDS (depends) - reuses ProxyCmd route/evidence/exit behavior
    //   → M-PROXY (depends) - applies token-saving routing, filtering, tracking, and evidence capture
    // START_rtk_proxy_cmd_run_as
    pub async fn run_as(
        &self,
        config: Config,
        executable: &str,
        require_args: bool,
        usage: &str,
    ) -> anyhow::Result<()> {
        if require_args && self.args.is_empty() {
            anyhow::bail!(usage.to_string());
        }

        let mut args = Vec::with_capacity(self.args.len() + 1);
        args.push(executable.to_string());
        args.extend(self.args.iter().cloned());

        ProxyCmd {
            route: self.route,
            evidence: self.evidence,
            args,
        }
        .run(config)
        .await
    }
    // END_rtk_proxy_cmd_run_as
}

// END_public_api
