// MODULE_CONTRACT
// MODULE_ID: M-CONFIG
// PURPOSE: Telemetry config model for optional OpenTelemetry OTLP export.
// SCOPE: TelemetryConfig defaults, endpoint/service/sampling/export interval settings, and resource attributes.
// DEPENDS: N/A
// LINKS:
//   -> docs/phases/Phase-93.xml (implements) - OpenTelemetry config
//   -> M-TELEMETRY (depends) - runtime telemetry initialization

// START_MODULE_MAP
// TelemetryConfig - Optional OTLP export settings
// default_* - Backward-compatible telemetry defaults
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added Phase-93 telemetry config defaults]
// END_CHANGE_SUMMARY

use std::collections::BTreeMap;

// START_public_api

// START_TelemetryConfig
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TelemetryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_telemetry_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_telemetry_service_name")]
    pub service_name: String,
    #[serde(default = "default_telemetry_sampling")]
    pub sampling: String,
    #[serde(default = "default_telemetry_batch_interval_ms")]
    pub batch_interval_ms: u64,
    #[serde(default)]
    pub resource: BTreeMap<String, String>,
}
// END_TelemetryConfig

impl Default for TelemetryConfig {
    // START_CONTRACT_TelemetryConfig::default
    // PURPOSE: Return default-off telemetry settings.
    // OUTPUTS: { TelemetryConfig }
    // START_telemetry_config_default
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: default_telemetry_endpoint(),
            service_name: default_telemetry_service_name(),
            sampling: default_telemetry_sampling(),
            batch_interval_ms: default_telemetry_batch_interval_ms(),
            resource: BTreeMap::new(),
        }
    }
    // END_telemetry_config_default
}

// START_CONTRACT_default_telemetry_endpoint
// PURPOSE: Default OTLP gRPC collector endpoint.
// OUTPUTS: { String }
// START_default_telemetry_endpoint
fn default_telemetry_endpoint() -> String {
    "http://localhost:4317".into()
}
// END_default_telemetry_endpoint

// START_CONTRACT_default_telemetry_service_name
// PURPOSE: Default service.name resource attribute.
// OUTPUTS: { String }
// START_default_telemetry_service_name
fn default_telemetry_service_name() -> String {
    "synapse".into()
}
// END_default_telemetry_service_name

// START_CONTRACT_default_telemetry_sampling
// PURPOSE: Default sampler mode for enabled telemetry.
// OUTPUTS: { String }
// START_default_telemetry_sampling
fn default_telemetry_sampling() -> String {
    "always_on".into()
}
// END_default_telemetry_sampling

// START_CONTRACT_default_telemetry_batch_interval_ms
// PURPOSE: Default batch export interval in milliseconds.
// OUTPUTS: { u64 }
// START_default_telemetry_batch_interval_ms
fn default_telemetry_batch_interval_ms() -> u64 {
    2_000
}
// END_default_telemetry_batch_interval_ms

// END_public_api
