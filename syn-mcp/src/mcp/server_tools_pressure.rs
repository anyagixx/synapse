// MODULE_CONTRACT
// MODULE_ID: M-MCP-SERVER-TOOLS
// PURPOSE: Choose tools/list schema verbosity based on current context pressure.
// SCOPE: effective tools/list schema style selection for high and critical context pressure.
// DEPENDS: M-CONFIG, M-TRACKING, M-MCP-SERVER-TOOLS
// LINKS:
//   -> M-CONFIG (depends) - reads configured context_window_limit
//   -> M-TRACKING (depends) - queries current session context pressure
//   -> M-MCP-SERVER-TOOLS (depends) - returns ToolSchemaStyle
//   -> NFR-003 (traces_to) - pressure-aware schema economy reduces repeated context load

// START_MODULE_MAP
// effective_schema_style - Force terse tools/list schemas when pressure is high or critical
// pressure_forces_terse - Convert pressure level into schema style policy
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added pressure-aware tools/list style policy]
// END_CHANGE_SUMMARY

use super::server_tools::ToolSchemaStyle;
use syn_core::config::Config;
use syn_core::tracking::{PressureLevel, Tracker};

// START_public_api

// START_CONTRACT_effective_schema_style
// PURPOSE: Return requested tools/list style unless current context pressure requires terse schemas.
// INPUTS: { config: &Config }, { tracker: &Tracker }, { requested: ToolSchemaStyle }
// OUTPUTS: { ToolSchemaStyle }
// LINKS:
//   -> M-TRACKING (depends) - context pressure determines forced terse mode
// START_effective_schema_style
pub(crate) async fn effective_schema_style(
    config: &Config,
    tracker: &Tracker,
    requested: ToolSchemaStyle,
) -> ToolSchemaStyle {
    match tracker
        .context_pressure(config.budget.context_window_limit)
        .await
    {
        Ok(pressure) if pressure_forces_terse(&pressure.level) => ToolSchemaStyle::Terse,
        Ok(_) => requested,
        Err(error) => {
            tracing::warn!(
                "[SynapseHandler][handle_message][MCP_TOOLS_LIST_PRESSURE] {}",
                error
            );
            requested
        }
    }
}
// END_effective_schema_style

// START_CONTRACT_pressure_forces_terse
// PURPOSE: Return whether a context pressure level should force terse tool schemas.
// INPUTS: { level: &PressureLevel }
// OUTPUTS: { bool }
// START_pressure_forces_terse
fn pressure_forces_terse(level: &PressureLevel) -> bool {
    matches!(level, PressureLevel::High | PressureLevel::Critical)
}
// END_pressure_forces_terse

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_pressure_tracker
    // PURPOSE: Create isolated config and tracker state for tools/list pressure tests.
    // OUTPUTS: { (tempfile::TempDir, tempfile::TempDir, Config, Tracker) }
    // START_pressure_tracker
    fn pressure_tracker() -> (tempfile::TempDir, tempfile::TempDir, Config, Tracker) {
        let data_home = tempfile::tempdir().expect("data home");
        let project_root = tempfile::tempdir().expect("project root");
        let mut config = Config::default();
        config.budget.context_window_limit = 1_000;
        let tracker = Tracker::new_for_test(
            &config,
            data_home.path(),
            project_root.path(),
            Some("tools-list-pressure"),
        );
        (data_home, project_root, config, tracker)
    }
    // END_pressure_tracker

    // START_CONTRACT_high_pressure_forces_terse_tools_list_style
    // PURPOSE: Verify high context pressure forces terse tools/list schemas even when full was requested.
    // START_high_pressure_forces_terse_tools_list_style
    #[tokio::test]
    async fn high_pressure_forces_terse_tools_list_style() {
        let (_data_home, _project_root, config, tracker) = pressure_tracker();
        tracker
            .record("pressure", 700, 0)
            .await
            .expect("seed spend");

        let style = effective_schema_style(&config, &tracker, ToolSchemaStyle::Full).await;

        assert_eq!(style, ToolSchemaStyle::Terse);
    }
    // END_high_pressure_forces_terse_tools_list_style

    // START_CONTRACT_low_pressure_preserves_requested_tools_list_style
    // PURPOSE: Verify low context pressure preserves the requested tools/list schema style.
    // START_low_pressure_preserves_requested_tools_list_style
    #[tokio::test]
    async fn low_pressure_preserves_requested_tools_list_style() {
        let (_data_home, _project_root, config, tracker) = pressure_tracker();
        tracker
            .record("pressure", 100, 0)
            .await
            .expect("seed spend");

        let style = effective_schema_style(&config, &tracker, ToolSchemaStyle::Full).await;

        assert_eq!(style, ToolSchemaStyle::Full);
    }
    // END_low_pressure_preserves_requested_tools_list_style
}
