// MODULE_CONTRACT
// MODULE_ID: M-CONFIG
// PURPOSE: Observability config model and bounded runtime defaults for dashboard and MCP metrics
// SCOPE: ObservabilityConfig, serde defaults, normalized retention/stats limits, MCP pipeline bounds
// DEPENDS: N/A
// LINKS:
//   -> M-CONFIG (depends) - parent config facade exposes typed key helpers
//   -> NFR-002 (traces_to) - runtime bounds prevent unbounded queues and payloads
//   -> NFR-003 (traces_to) - MCP metrics quantify runtime reliability and economics

// START_MODULE_MAP
// ObservabilityConfig — Dashboard/MCP observability settings and runtime bounds
// default_observability_* — Serde default providers for existing config files
// bounded_u32_or_default — Normalize invalid deserialized u32 bounds
// bounded_usize_or_default — Normalize invalid deserialized usize bounds
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Extracted observability config bounds from M-CONFIG]
// END_CHANGE_SUMMARY

// START_public_api

// START_ObservabilityConfig
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct ObservabilityConfig {
    #[serde(default = "default_observability_enabled")]
    pub enabled: bool,
    #[serde(default = "default_observability_mcp_metrics_enabled")]
    pub mcp_metrics_enabled: bool,
    #[serde(default = "default_observability_mcp_metrics_retention_days")]
    pub mcp_metrics_retention_days: u32,
    #[serde(default = "default_observability_mcp_stats_limit")]
    pub mcp_stats_limit: usize,
    #[serde(default = "default_observability_readiness_mcp_stats_limit")]
    pub readiness_mcp_stats_limit: usize,
    #[serde(default = "default_observability_pipeline_response_queue_capacity")]
    pub pipeline_response_queue_capacity: usize,
    #[serde(default = "default_observability_pipeline_max_concurrent_requests")]
    pub pipeline_max_concurrent_requests: usize,
    #[serde(default = "default_observability_pipeline_max_line_bytes")]
    pub pipeline_max_line_bytes: usize,
}
// END_ObservabilityConfig

impl Default for ObservabilityConfig {
    // START_CONTRACT_ObservabilityConfig::default
    // PURPOSE: Return release-safe dashboard and MCP observability defaults
    // OUTPUTS: { ObservabilityConfig }
    // LINKS:
    //   -> NFR-002 (traces_to) - runtime observability defaults are bounded
    // START_observability_config_default
    fn default() -> Self {
        Self {
            enabled: default_observability_enabled(),
            mcp_metrics_enabled: default_observability_mcp_metrics_enabled(),
            mcp_metrics_retention_days: default_observability_mcp_metrics_retention_days(),
            mcp_stats_limit: default_observability_mcp_stats_limit(),
            readiness_mcp_stats_limit: default_observability_readiness_mcp_stats_limit(),
            pipeline_response_queue_capacity:
                default_observability_pipeline_response_queue_capacity(),
            pipeline_max_concurrent_requests:
                default_observability_pipeline_max_concurrent_requests(),
            pipeline_max_line_bytes: default_observability_pipeline_max_line_bytes(),
        }
    }
    // END_observability_config_default
}

impl ObservabilityConfig {
    // START_CONTRACT_ObservabilityConfig::enabled
    // PURPOSE: Return whether dashboard observability endpoints are enabled
    // OUTPUTS: { bool }
    // LINKS:
    //   -> UC-001 (implements) - users can inspect local runtime state
    // START_observability_enabled
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    // END_observability_enabled

    // START_CONTRACT_ObservabilityConfig::mcp_metrics_enabled
    // PURPOSE: Return whether MCP metrics should be recorded and served
    // OUTPUTS: { bool }
    // LINKS:
    //   -> NFR-003 (traces_to) - MCP metrics quantify runtime reliability and economics
    // START_observability_mcp_metrics_enabled
    pub fn mcp_metrics_enabled(&self) -> bool {
        self.enabled && self.mcp_metrics_enabled
    }
    // END_observability_mcp_metrics_enabled

    // START_CONTRACT_ObservabilityConfig::mcp_metrics_retention_days
    // PURPOSE: Return bounded MCP metrics retention days
    // OUTPUTS: { u32 }
    // LINKS:
    //   -> NFR-002 (traces_to) - metrics retention is bounded
    // START_observability_mcp_metrics_retention_days
    pub fn mcp_metrics_retention_days(&self) -> u32 {
        bounded_u32_or_default(
            self.mcp_metrics_retention_days,
            default_observability_mcp_metrics_retention_days(),
            1,
            3650,
        )
    }
    // END_observability_mcp_metrics_retention_days

    // START_CONTRACT_ObservabilityConfig::mcp_stats_limit
    // PURPOSE: Return bounded MCP dashboard stats row limit
    // OUTPUTS: { usize }
    // LINKS:
    //   -> NFR-002 (traces_to) - dashboard payload size is bounded
    // START_observability_mcp_stats_limit
    pub fn mcp_stats_limit(&self) -> usize {
        bounded_usize_or_default(
            self.mcp_stats_limit,
            default_observability_mcp_stats_limit(),
            1,
            100,
        )
    }
    // END_observability_mcp_stats_limit

    // START_CONTRACT_ObservabilityConfig::readiness_mcp_stats_limit
    // PURPOSE: Return bounded MCP stats limit for readiness payloads
    // OUTPUTS: { usize }
    // LINKS:
    //   -> NFR-002 (traces_to) - readiness payload size is bounded
    // START_observability_readiness_mcp_stats_limit
    pub fn readiness_mcp_stats_limit(&self) -> usize {
        bounded_usize_or_default(
            self.readiness_mcp_stats_limit,
            default_observability_readiness_mcp_stats_limit(),
            1,
            50,
        )
    }
    // END_observability_readiness_mcp_stats_limit

    // START_CONTRACT_ObservabilityConfig::pipeline_response_queue_capacity
    // PURPOSE: Return bounded MCP response queue capacity
    // OUTPUTS: { usize }
    // LINKS:
    //   -> NFR-002 (traces_to) - MCP response queue is bounded
    // START_observability_pipeline_response_queue_capacity
    pub fn pipeline_response_queue_capacity(&self) -> usize {
        bounded_usize_or_default(
            self.pipeline_response_queue_capacity,
            default_observability_pipeline_response_queue_capacity(),
            1,
            4096,
        )
    }
    // END_observability_pipeline_response_queue_capacity

    // START_CONTRACT_ObservabilityConfig::pipeline_max_concurrent_requests
    // PURPOSE: Return bounded MCP handler concurrency limit
    // OUTPUTS: { usize }
    // LINKS:
    //   -> NFR-002 (traces_to) - MCP handler concurrency is bounded
    // START_observability_pipeline_max_concurrent_requests
    pub fn pipeline_max_concurrent_requests(&self) -> usize {
        bounded_usize_or_default(
            self.pipeline_max_concurrent_requests,
            default_observability_pipeline_max_concurrent_requests(),
            1,
            256,
        )
    }
    // END_observability_pipeline_max_concurrent_requests

    // START_CONTRACT_ObservabilityConfig::pipeline_max_line_bytes
    // PURPOSE: Return bounded MCP JSON-RPC line limit
    // OUTPUTS: { usize }
    // LINKS:
    //   -> NFR-002 (traces_to) - MCP request size is bounded
    // START_observability_pipeline_max_line_bytes
    pub fn pipeline_max_line_bytes(&self) -> usize {
        bounded_usize_or_default(
            self.pipeline_max_line_bytes,
            default_observability_pipeline_max_line_bytes(),
            1024,
            67_108_864,
        )
    }
    // END_observability_pipeline_max_line_bytes
}

// END_public_api

// START_CONTRACT_default_observability_enabled
// PURPOSE: Provide serde/default dashboard observability enablement
// OUTPUTS: { bool }
// LINKS:
//   -> UC-001 (implements) - dashboard inspection is enabled by default
// START_default_observability_enabled
fn default_observability_enabled() -> bool {
    true
}
// END_default_observability_enabled

// START_CONTRACT_default_observability_mcp_metrics_enabled
// PURPOSE: Provide serde/default MCP metrics recording enablement
// OUTPUTS: { bool }
// LINKS:
//   -> NFR-003 (traces_to) - MCP metrics quantify runtime economics by default
// START_default_observability_mcp_metrics_enabled
fn default_observability_mcp_metrics_enabled() -> bool {
    true
}
// END_default_observability_mcp_metrics_enabled

// START_CONTRACT_default_observability_mcp_metrics_retention_days
// PURPOSE: Provide serde/default MCP metrics retention days
// OUTPUTS: { u32 }
// LINKS:
//   -> NFR-002 (traces_to) - metrics retention is release-bounded
// START_default_observability_mcp_metrics_retention_days
fn default_observability_mcp_metrics_retention_days() -> u32 {
    30
}
// END_default_observability_mcp_metrics_retention_days

// START_CONTRACT_default_observability_mcp_stats_limit
// PURPOSE: Provide serde/default MCP stats row limit
// OUTPUTS: { usize }
// LINKS:
//   -> NFR-002 (traces_to) - dashboard stats payload is bounded
// START_default_observability_mcp_stats_limit
fn default_observability_mcp_stats_limit() -> usize {
    12
}
// END_default_observability_mcp_stats_limit

// START_CONTRACT_default_observability_readiness_mcp_stats_limit
// PURPOSE: Provide serde/default readiness MCP stats row limit
// OUTPUTS: { usize }
// LINKS:
//   -> NFR-002 (traces_to) - readiness payload is bounded
// START_default_observability_readiness_mcp_stats_limit
fn default_observability_readiness_mcp_stats_limit() -> usize {
    5
}
// END_default_observability_readiness_mcp_stats_limit

// START_CONTRACT_default_observability_pipeline_response_queue_capacity
// PURPOSE: Provide serde/default MCP response queue capacity
// OUTPUTS: { usize }
// LINKS:
//   -> NFR-002 (traces_to) - MCP response queue is bounded
// START_default_observability_pipeline_response_queue_capacity
fn default_observability_pipeline_response_queue_capacity() -> usize {
    128
}
// END_default_observability_pipeline_response_queue_capacity

// START_CONTRACT_default_observability_pipeline_max_concurrent_requests
// PURPOSE: Provide serde/default MCP request handler concurrency
// OUTPUTS: { usize }
// LINKS:
//   -> NFR-002 (traces_to) - MCP concurrency is bounded
// START_default_observability_pipeline_max_concurrent_requests
fn default_observability_pipeline_max_concurrent_requests() -> usize {
    16
}
// END_default_observability_pipeline_max_concurrent_requests

// START_CONTRACT_default_observability_pipeline_max_line_bytes
// PURPOSE: Provide serde/default MCP JSON-RPC line byte limit
// OUTPUTS: { usize }
// LINKS:
//   -> NFR-002 (traces_to) - MCP request size is bounded
// START_default_observability_pipeline_max_line_bytes
fn default_observability_pipeline_max_line_bytes() -> usize {
    10_485_760
}
// END_default_observability_pipeline_max_line_bytes

// START_CONTRACT_bounded_u32_or_default
// PURPOSE: Clamp invalid deserialized u32 bounds back to a release-safe default
// INPUTS: { value: u32 }, { default: u32 }, { min: u32 }, { max: u32 }
// OUTPUTS: { u32 }
// LINKS:
//   -> NFR-002 (traces_to) - deserialized runtime bounds stay safe
// START_bounded_u32_or_default
fn bounded_u32_or_default(value: u32, default: u32, min: u32, max: u32) -> u32 {
    if (min..=max).contains(&value) {
        value
    } else {
        default
    }
}
// END_bounded_u32_or_default

// START_CONTRACT_bounded_usize_or_default
// PURPOSE: Clamp invalid deserialized usize bounds back to a release-safe default
// INPUTS: { value: usize }, { default: usize }, { min: usize }, { max: usize }
// OUTPUTS: { usize }
// LINKS:
//   -> NFR-002 (traces_to) - deserialized runtime bounds stay safe
// START_bounded_usize_or_default
fn bounded_usize_or_default(value: usize, default: usize, min: usize, max: usize) -> usize {
    if (min..=max).contains(&value) {
        value
    } else {
        default
    }
}
// END_bounded_usize_or_default
