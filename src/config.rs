// MODULE_CONTRACT
// MODULE_ID: M-CONFIG
// PURPOSE: Config model and explicit load/default/init semantics for synapsec.toml
// SCOPE: Config struct definitions including embedding provider settings, budget and context-window settings, observability runtime bounds, telemetry defaults, LSP server overrides, read-only TOML load, default fallback, explicit default config generation, typed key lookup/update, path resolution, persistence
// DEPENDS: N/A
// LINKS: synapsec.toml

// START_MODULE_MAP
// Config — Top-level config struct (project, index, search, proxy, compress, tracking, budget, observability, telemetry, graphrag, embedding, lsp)
// ProjectConfig — Project metadata (name, version, strictness)
// IndexConfig — Indexer settings (chunk_size, chunk_overlap, require_git)
// SearchConfig — Search settings (max_results, similarity_threshold, hybrid_enabled)
// ProxyConfig — Proxy settings (enabled, passthrough_max_chars, capture caps, command timeout)
// CompressConfig — Compress settings (output_level, input_enabled)
// TrackingConfig — Tracking settings (enabled, history_days)
// BudgetConfig — Per-session token budget and context-window settings
// ObservabilityConfig — Dashboard/MCP observability settings and runtime bounds
// TelemetryConfig — Optional OpenTelemetry OTLP export settings
// GraphRagConfig — GraphRAG settings (enabled, use_llm)
// EmbeddingConfig — Local semantic embedding settings (enabled, provider, model, cache_dir, batch_size)
// LspConfig — Language server command overrides
// ConfigKeySpec — Supported scalar config key metadata
// ConfigEntry — Resolved scalar config key/value pair
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.13.0 - Added default-off telemetry config]
// END_CHANGE_SUMMARY

use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod observability;
mod telemetry;
pub use observability::ObservabilityConfig;
pub use telemetry::TelemetryConfig;

// START_public_api

// START_Config
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Config {
    pub project: ProjectConfig,
    pub index: IndexConfig,
    pub search: SearchConfig,
    pub proxy: ProxyConfig,
    pub compress: CompressConfig,
    pub tracking: TrackingConfig,
    #[serde(default)]
    pub budget: BudgetConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub telemetry: TelemetryConfig,
    pub graphrag: GraphRagConfig,
    #[serde(default)]
    pub embedding: EmbeddingConfig,
    #[serde(default)]
    pub lsp: LspConfig,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub strictness: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct IndexConfig {
    pub chunk_size: u32,
    pub chunk_overlap: u32,
    pub require_git: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct SearchConfig {
    pub max_results: u32,
    pub similarity_threshold: f64,
    pub hybrid_enabled: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct ProxyConfig {
    pub enabled: bool,
    pub passthrough_max_chars: u32,
    #[serde(default = "default_proxy_capture_cap_bytes")]
    pub capture_cap_bytes: usize,
    #[serde(default = "default_proxy_command_timeout_secs")]
    pub command_timeout_secs: u64,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct CompressConfig {
    pub output_level: String,
    pub input_enabled: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TrackingConfig {
    #[serde(default = "default_tracking_enabled")]
    pub enabled: bool,
    pub history_days: u32,
}

// START_CONTRACT_default_tracking_enabled
// PURPOSE: Provide serde default for tracking.enabled
// OUTPUTS: { bool — true when config omits tracking.enabled }
fn default_tracking_enabled() -> bool {
    true
}

impl TrackingConfig {
    // START_CONTRACT_TrackingConfig::enabled
    // PURPOSE: Return whether token tracking is enabled
    // OUTPUTS: { bool }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct BudgetConfig {
    #[serde(default = "default_session_token_limit")]
    pub session_token_limit: u64,
    #[serde(default = "default_context_window_limit")]
    pub context_window_limit: u64,
    #[serde(default = "default_budget_warn_at_pct")]
    pub warn_at_pct: u64,
    #[serde(default = "default_budget_block_at_pct")]
    pub block_at_pct: u64,
    #[serde(default = "default_budget_reset_on_new_session")]
    pub reset_on_new_session: bool,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            session_token_limit: default_session_token_limit(),
            context_window_limit: default_context_window_limit(),
            warn_at_pct: default_budget_warn_at_pct(),
            block_at_pct: default_budget_block_at_pct(),
            reset_on_new_session: default_budget_reset_on_new_session(),
        }
    }
}

// START_CONTRACT_default_session_token_limit
// PURPOSE: Provide serde/default per-session token limit, where zero means unlimited.
// OUTPUTS: { u64 }
fn default_session_token_limit() -> u64 {
    0
}

// START_CONTRACT_default_context_window_limit
// PURPOSE: Provide serde/default context window limit used for pressure estimates.
// OUTPUTS: { u64 }
fn default_context_window_limit() -> u64 {
    200_000
}

// START_CONTRACT_default_budget_warn_at_pct
// PURPOSE: Provide serde/default budget warning threshold percentage.
// OUTPUTS: { u64 }
fn default_budget_warn_at_pct() -> u64 {
    80
}

// START_CONTRACT_default_budget_block_at_pct
// PURPOSE: Provide serde/default budget blocking threshold percentage.
// OUTPUTS: { u64 }
fn default_budget_block_at_pct() -> u64 {
    100
}

// START_CONTRACT_default_budget_reset_on_new_session
// PURPOSE: Provide serde/default behavior for resetting budget accounting on new sessions.
// OUTPUTS: { bool }
fn default_budget_reset_on_new_session() -> bool {
    true
}

// START_CONTRACT_default_proxy_capture_cap_bytes
// PURPOSE: Provide serde/default proxy stdout/stderr capture cap in bytes
// OUTPUTS: { usize — 10 MiB default cap }
fn default_proxy_capture_cap_bytes() -> usize {
    10_485_760
}

// START_CONTRACT_default_proxy_command_timeout_secs
// PURPOSE: Provide serde/default proxy command timeout in seconds
// OUTPUTS: { u64 — 300 second default timeout, 0 disables timeout when configured explicitly }
fn default_proxy_command_timeout_secs() -> u64 {
    300
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct GraphRagConfig {
    pub enabled: bool,
    pub use_llm: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct EmbeddingConfig {
    #[serde(default = "default_embedding_enabled")]
    pub enabled: bool,
    #[serde(default = "default_embedding_provider")]
    pub provider: String,
    #[serde(default = "default_embedding_model")]
    pub model: String,
    #[serde(default)]
    pub cache_dir: String,
    #[serde(default = "default_embedding_batch_size")]
    pub batch_size: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            enabled: default_embedding_enabled(),
            provider: default_embedding_provider(),
            model: default_embedding_model(),
            cache_dir: String::new(),
            batch_size: default_embedding_batch_size(),
        }
    }
}

// START_CONTRACT_default_embedding_enabled
// PURPOSE: Provide serde/default embedding enablement
// OUTPUTS: { bool — false so model downloads are explicit opt-in }
fn default_embedding_enabled() -> bool {
    false
}

// START_CONTRACT_default_embedding_provider
// PURPOSE: Provide serde/default embedding provider name
// OUTPUTS: { String }
fn default_embedding_provider() -> String {
    "fastembed".into()
}

// START_CONTRACT_default_embedding_model
// PURPOSE: Provide serde/default embedding model name
// OUTPUTS: { String }
fn default_embedding_model() -> String {
    "AllMiniLML6V2".into()
}

// START_CONTRACT_default_embedding_batch_size
// PURPOSE: Provide serde/default embedding batch size
// OUTPUTS: { usize }
fn default_embedding_batch_size() -> usize {
    256
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default)]
pub struct LspConfig {
    #[serde(default)]
    pub servers: HashMap<String, Vec<String>>,
}

// START_ConfigKeySpec
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfigKeySpec {
    pub key: &'static str,
    pub value_type: &'static str,
}
// END_ConfigKeySpec

// START_ConfigEntry
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
    pub value_type: &'static str,
}
// END_ConfigEntry

const CONFIG_KEY_SPECS: &[ConfigKeySpec] = &[
    ConfigKeySpec {
        key: "project.name",
        value_type: "string",
    },
    ConfigKeySpec {
        key: "project.version",
        value_type: "string",
    },
    ConfigKeySpec {
        key: "project.strictness",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "index.chunk_size",
        value_type: "u32",
    },
    ConfigKeySpec {
        key: "index.chunk_overlap",
        value_type: "u32",
    },
    ConfigKeySpec {
        key: "index.require_git",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "search.max_results",
        value_type: "u32",
    },
    ConfigKeySpec {
        key: "search.similarity_threshold",
        value_type: "f64",
    },
    ConfigKeySpec {
        key: "search.hybrid_enabled",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "proxy.enabled",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "proxy.passthrough_max_chars",
        value_type: "u32",
    },
    ConfigKeySpec {
        key: "proxy.capture_cap_bytes",
        value_type: "usize",
    },
    ConfigKeySpec {
        key: "proxy.command_timeout_secs",
        value_type: "u64",
    },
    ConfigKeySpec {
        key: "compress.output_level",
        value_type: "string",
    },
    ConfigKeySpec {
        key: "compress.input_enabled",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "tracking.enabled",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "tracking.history_days",
        value_type: "u32",
    },
    ConfigKeySpec {
        key: "budget.session_token_limit",
        value_type: "u64",
    },
    ConfigKeySpec {
        key: "budget.context_window_limit",
        value_type: "u64",
    },
    ConfigKeySpec {
        key: "budget.warn_at_pct",
        value_type: "u64",
    },
    ConfigKeySpec {
        key: "budget.block_at_pct",
        value_type: "u64",
    },
    ConfigKeySpec {
        key: "budget.reset_on_new_session",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "observability.enabled",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "observability.mcp_metrics_enabled",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "observability.mcp_metrics_retention_days",
        value_type: "u32",
    },
    ConfigKeySpec {
        key: "observability.mcp_stats_limit",
        value_type: "usize",
    },
    ConfigKeySpec {
        key: "observability.readiness_mcp_stats_limit",
        value_type: "usize",
    },
    ConfigKeySpec {
        key: "observability.pipeline_response_queue_capacity",
        value_type: "usize",
    },
    ConfigKeySpec {
        key: "observability.pipeline_max_concurrent_requests",
        value_type: "usize",
    },
    ConfigKeySpec {
        key: "observability.pipeline_max_line_bytes",
        value_type: "usize",
    },
    ConfigKeySpec {
        key: "telemetry.enabled",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "graphrag.enabled",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "graphrag.use_llm",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "embedding.enabled",
        value_type: "bool",
    },
    ConfigKeySpec {
        key: "embedding.provider",
        value_type: "string",
    },
    ConfigKeySpec {
        key: "embedding.model",
        value_type: "string",
    },
    ConfigKeySpec {
        key: "embedding.cache_dir",
        value_type: "string",
    },
    ConfigKeySpec {
        key: "embedding.batch_size",
        value_type: "usize",
    },
];

impl Config {
    // START_CONTRACT_Config::load
    // PURPOSE: Load the user config file and fail if it is missing or invalid
    // OUTPUTS: { anyhow::Result<Config> }
    // SIDE_EFFECTS: reads synapsec.toml
    /// Read-only load: returns existing config, errors if missing
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            anyhow::bail!(
                "Config not found at {}. Run 'syn init' first.",
                path.display()
            );
        }
        let content = std::fs::read_to_string(&path)?;
        toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid config at {}: {}", path.display(), e))
    }

    // START_CONTRACT_Config::load_or_default
    // PURPOSE: Load config if available, otherwise return default config without writing it
    // OUTPUTS: { Config }
    /// Load existing config or return default (does NOT write to disk)
    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_default()
    }

    // START_CONTRACT_Config::init_default
    // PURPOSE: Write default configuration to the user config path
    // OUTPUTS: { anyhow::Result<Config> }
    // SIDE_EFFECTS: creates config directory and writes synapsec.toml
    /// Initialize/overwrite config file with defaults
    pub fn init_default() -> anyhow::Result<Self> {
        let path = Self::path()?;
        let config = Self::default();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(&config)?;
        std::fs::write(&path, &toml_str)?;
        tracing::info!("Created default config at {}", path.display());
        Ok(config)
    }

    // START_CONTRACT_Config::path
    // PURPOSE: Resolve the user config file path
    // OUTPUTS: { anyhow::Result<PathBuf> }
    pub fn path() -> anyhow::Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find config directory"))?
            .join("synapse");
        Ok(config_dir.join("synapsec.toml"))
    }

    // START_CONTRACT_Config::supported_key_specs
    // PURPOSE: Return the supported scalar config keys that may be read or persisted through the CLI
    // OUTPUTS: { &'static [ConfigKeySpec] }
    // START_config_supported_key_specs
    pub fn supported_key_specs() -> &'static [ConfigKeySpec] {
        CONFIG_KEY_SPECS
    }
    // END_config_supported_key_specs

    // START_CONTRACT_Config::list_config_entries
    // PURPOSE: Return all supported scalar config key/value pairs in stable order
    // OUTPUTS: { Vec<ConfigEntry> }
    // START_config_list_config_entries
    pub fn list_config_entries(&self) -> Vec<ConfigEntry> {
        CONFIG_KEY_SPECS
            .iter()
            .filter_map(|spec| self.get_config_key(spec.key).ok())
            .collect()
    }
    // END_config_list_config_entries

    // START_CONTRACT_Config::get_config_key
    // PURPOSE: Read one supported scalar config key as a typed display value
    // INPUTS: { key: &str }
    // OUTPUTS: { anyhow::Result<ConfigEntry> }
    // START_config_get_config_key
    pub fn get_config_key(&self, key: &str) -> anyhow::Result<ConfigEntry> {
        let canonical = canonical_config_key(key)?;
        let value = match canonical.as_str() {
            "project.name" => self.project.name.clone(),
            "project.version" => self.project.version.clone(),
            "project.strictness" => self.project.strictness.to_string(),
            "index.chunk_size" => self.index.chunk_size.to_string(),
            "index.chunk_overlap" => self.index.chunk_overlap.to_string(),
            "index.require_git" => self.index.require_git.to_string(),
            "search.max_results" => self.search.max_results.to_string(),
            "search.similarity_threshold" => self.search.similarity_threshold.to_string(),
            "search.hybrid_enabled" => self.search.hybrid_enabled.to_string(),
            "proxy.enabled" => self.proxy.enabled.to_string(),
            "proxy.passthrough_max_chars" => self.proxy.passthrough_max_chars.to_string(),
            "proxy.capture_cap_bytes" => self.proxy.capture_cap_bytes.to_string(),
            "proxy.command_timeout_secs" => self.proxy.command_timeout_secs.to_string(),
            "compress.output_level" => self.compress.output_level.clone(),
            "compress.input_enabled" => self.compress.input_enabled.to_string(),
            "tracking.enabled" => self.tracking.enabled.to_string(),
            "tracking.history_days" => self.tracking.history_days.to_string(),
            "budget.session_token_limit" => self.budget.session_token_limit.to_string(),
            "budget.context_window_limit" => self.budget.context_window_limit.to_string(),
            "budget.warn_at_pct" => self.budget.warn_at_pct.to_string(),
            "budget.block_at_pct" => self.budget.block_at_pct.to_string(),
            "budget.reset_on_new_session" => self.budget.reset_on_new_session.to_string(),
            "observability.enabled" => self.observability.enabled.to_string(),
            "observability.mcp_metrics_enabled" => {
                self.observability.mcp_metrics_enabled.to_string()
            }
            "observability.mcp_metrics_retention_days" => {
                self.observability.mcp_metrics_retention_days.to_string()
            }
            "observability.mcp_stats_limit" => self.observability.mcp_stats_limit.to_string(),
            "observability.readiness_mcp_stats_limit" => {
                self.observability.readiness_mcp_stats_limit.to_string()
            }
            "observability.pipeline_response_queue_capacity" => self
                .observability
                .pipeline_response_queue_capacity
                .to_string(),
            "observability.pipeline_max_concurrent_requests" => self
                .observability
                .pipeline_max_concurrent_requests
                .to_string(),
            "observability.pipeline_max_line_bytes" => {
                self.observability.pipeline_max_line_bytes.to_string()
            }
            "telemetry.enabled" => self.telemetry.enabled.to_string(),
            "graphrag.enabled" => self.graphrag.enabled.to_string(),
            "graphrag.use_llm" => self.graphrag.use_llm.to_string(),
            "embedding.enabled" => self.embedding.enabled.to_string(),
            "embedding.provider" => self.embedding.provider.clone(),
            "embedding.model" => self.embedding.model.clone(),
            "embedding.cache_dir" => self.embedding.cache_dir.clone(),
            "embedding.batch_size" => self.embedding.batch_size.to_string(),
            _ => unreachable!("canonical config key must be supported"),
        };
        let spec = config_key_spec(&canonical)
            .ok_or_else(|| anyhow::anyhow!("unsupported config key `{}`", canonical))?;
        Ok(ConfigEntry {
            key: spec.key.to_string(),
            value,
            value_type: spec.value_type,
        })
    }
    // END_config_get_config_key

    // START_CONTRACT_Config::set_config_key
    // PURPOSE: Set one supported scalar config key from a typed string value
    // INPUTS: { key: &str }, { value: &str }
    // OUTPUTS: { anyhow::Result<ConfigEntry> }
    // SIDE_EFFECTS: mutates the in-memory Config only
    // START_config_set_config_key
    pub fn set_config_key(&mut self, key: &str, value: &str) -> anyhow::Result<ConfigEntry> {
        let canonical = canonical_config_key(key)?;
        match canonical.as_str() {
            "project.name" => self.project.name = parse_non_empty_string(&canonical, value)?,
            "project.version" => self.project.version = parse_non_empty_string(&canonical, value)?,
            "project.strictness" => self.project.strictness = parse_bool(&canonical, value)?,
            "index.chunk_size" => self.index.chunk_size = parse_u32(&canonical, value)?,
            "index.chunk_overlap" => self.index.chunk_overlap = parse_u32(&canonical, value)?,
            "index.require_git" => self.index.require_git = parse_bool(&canonical, value)?,
            "search.max_results" => self.search.max_results = parse_u32(&canonical, value)?,
            "search.similarity_threshold" => {
                self.search.similarity_threshold = parse_f64(&canonical, value)?
            }
            "search.hybrid_enabled" => self.search.hybrid_enabled = parse_bool(&canonical, value)?,
            "proxy.enabled" => self.proxy.enabled = parse_bool(&canonical, value)?,
            "proxy.passthrough_max_chars" => {
                self.proxy.passthrough_max_chars = parse_u32(&canonical, value)?
            }
            "proxy.capture_cap_bytes" => {
                self.proxy.capture_cap_bytes = parse_usize(&canonical, value)?
            }
            "proxy.command_timeout_secs" => {
                self.proxy.command_timeout_secs = parse_u64(&canonical, value)?
            }
            "compress.output_level" => {
                self.compress.output_level = parse_non_empty_string(&canonical, value)?
            }
            "compress.input_enabled" => {
                self.compress.input_enabled = parse_bool(&canonical, value)?
            }
            "tracking.enabled" => self.tracking.enabled = parse_bool(&canonical, value)?,
            "tracking.history_days" => self.tracking.history_days = parse_u32(&canonical, value)?,
            "budget.session_token_limit" => {
                self.budget.session_token_limit = parse_u64(&canonical, value)?
            }
            "budget.context_window_limit" => {
                self.budget.context_window_limit = parse_u64_range(&canonical, value, 1, 2_000_000)?
            }
            "budget.warn_at_pct" => {
                self.budget.warn_at_pct = parse_u64_range(&canonical, value, 0, 100)?
            }
            "budget.block_at_pct" => {
                self.budget.block_at_pct = parse_u64_range(&canonical, value, 1, 100)?
            }
            "budget.reset_on_new_session" => {
                self.budget.reset_on_new_session = parse_bool(&canonical, value)?
            }
            "observability.enabled" => self.observability.enabled = parse_bool(&canonical, value)?,
            "observability.mcp_metrics_enabled" => {
                self.observability.mcp_metrics_enabled = parse_bool(&canonical, value)?
            }
            "observability.mcp_metrics_retention_days" => {
                self.observability.mcp_metrics_retention_days =
                    parse_u32_range(&canonical, value, 1, 3650)?
            }
            "observability.mcp_stats_limit" => {
                self.observability.mcp_stats_limit = parse_usize_range(&canonical, value, 1, 100)?
            }
            "observability.readiness_mcp_stats_limit" => {
                self.observability.readiness_mcp_stats_limit =
                    parse_usize_range(&canonical, value, 1, 50)?
            }
            "observability.pipeline_response_queue_capacity" => {
                self.observability.pipeline_response_queue_capacity =
                    parse_usize_range(&canonical, value, 1, 4096)?
            }
            "observability.pipeline_max_concurrent_requests" => {
                self.observability.pipeline_max_concurrent_requests =
                    parse_usize_range(&canonical, value, 1, 256)?
            }
            "observability.pipeline_max_line_bytes" => {
                self.observability.pipeline_max_line_bytes =
                    parse_usize_range(&canonical, value, 1024, 67_108_864)?
            }
            "telemetry.enabled" => self.telemetry.enabled = parse_bool(&canonical, value)?,
            "graphrag.enabled" => self.graphrag.enabled = parse_bool(&canonical, value)?,
            "graphrag.use_llm" => self.graphrag.use_llm = parse_bool(&canonical, value)?,
            "embedding.enabled" => self.embedding.enabled = parse_bool(&canonical, value)?,
            "embedding.provider" => {
                self.embedding.provider = parse_non_empty_string(&canonical, value)?
            }
            "embedding.model" => self.embedding.model = parse_non_empty_string(&canonical, value)?,
            "embedding.cache_dir" => {
                self.embedding.cache_dir = parse_non_empty_string(&canonical, value)?
            }
            "embedding.batch_size" => {
                self.embedding.batch_size = parse_embedding_batch_size(&canonical, value)?
            }
            _ => unreachable!("canonical config key must be supported"),
        }
        self.get_config_key(&canonical)
    }
    // END_config_set_config_key

    // START_CONTRACT_Config::unset_config_key
    // PURPOSE: Reset one supported scalar config key to the built-in default value
    // INPUTS: { key: &str }
    // OUTPUTS: { anyhow::Result<ConfigEntry> }
    // SIDE_EFFECTS: mutates the in-memory Config only
    // START_config_unset_config_key
    pub fn unset_config_key(&mut self, key: &str) -> anyhow::Result<ConfigEntry> {
        let canonical = canonical_config_key(key)?;
        let defaults = Self::default();
        match canonical.as_str() {
            "project.name" => self.project.name = defaults.project.name,
            "project.version" => self.project.version = defaults.project.version,
            "project.strictness" => self.project.strictness = defaults.project.strictness,
            "index.chunk_size" => self.index.chunk_size = defaults.index.chunk_size,
            "index.chunk_overlap" => self.index.chunk_overlap = defaults.index.chunk_overlap,
            "index.require_git" => self.index.require_git = defaults.index.require_git,
            "search.max_results" => self.search.max_results = defaults.search.max_results,
            "search.similarity_threshold" => {
                self.search.similarity_threshold = defaults.search.similarity_threshold
            }
            "search.hybrid_enabled" => self.search.hybrid_enabled = defaults.search.hybrid_enabled,
            "proxy.enabled" => self.proxy.enabled = defaults.proxy.enabled,
            "proxy.passthrough_max_chars" => {
                self.proxy.passthrough_max_chars = defaults.proxy.passthrough_max_chars
            }
            "proxy.capture_cap_bytes" => {
                self.proxy.capture_cap_bytes = defaults.proxy.capture_cap_bytes
            }
            "proxy.command_timeout_secs" => {
                self.proxy.command_timeout_secs = defaults.proxy.command_timeout_secs
            }
            "compress.output_level" => self.compress.output_level = defaults.compress.output_level,
            "compress.input_enabled" => {
                self.compress.input_enabled = defaults.compress.input_enabled
            }
            "tracking.enabled" => self.tracking.enabled = defaults.tracking.enabled,
            "tracking.history_days" => self.tracking.history_days = defaults.tracking.history_days,
            "budget.session_token_limit" => {
                self.budget.session_token_limit = defaults.budget.session_token_limit
            }
            "budget.context_window_limit" => {
                self.budget.context_window_limit = defaults.budget.context_window_limit
            }
            "budget.warn_at_pct" => self.budget.warn_at_pct = defaults.budget.warn_at_pct,
            "budget.block_at_pct" => self.budget.block_at_pct = defaults.budget.block_at_pct,
            "budget.reset_on_new_session" => {
                self.budget.reset_on_new_session = defaults.budget.reset_on_new_session
            }
            "observability.enabled" => self.observability.enabled = defaults.observability.enabled,
            "observability.mcp_metrics_enabled" => {
                self.observability.mcp_metrics_enabled = defaults.observability.mcp_metrics_enabled
            }
            "observability.mcp_metrics_retention_days" => {
                self.observability.mcp_metrics_retention_days =
                    defaults.observability.mcp_metrics_retention_days
            }
            "observability.mcp_stats_limit" => {
                self.observability.mcp_stats_limit = defaults.observability.mcp_stats_limit
            }
            "observability.readiness_mcp_stats_limit" => {
                self.observability.readiness_mcp_stats_limit =
                    defaults.observability.readiness_mcp_stats_limit
            }
            "observability.pipeline_response_queue_capacity" => {
                self.observability.pipeline_response_queue_capacity =
                    defaults.observability.pipeline_response_queue_capacity
            }
            "observability.pipeline_max_concurrent_requests" => {
                self.observability.pipeline_max_concurrent_requests =
                    defaults.observability.pipeline_max_concurrent_requests
            }
            "observability.pipeline_max_line_bytes" => {
                self.observability.pipeline_max_line_bytes =
                    defaults.observability.pipeline_max_line_bytes
            }
            "telemetry.enabled" => self.telemetry.enabled = defaults.telemetry.enabled,
            "graphrag.enabled" => self.graphrag.enabled = defaults.graphrag.enabled,
            "graphrag.use_llm" => self.graphrag.use_llm = defaults.graphrag.use_llm,
            "embedding.enabled" => self.embedding.enabled = defaults.embedding.enabled,
            "embedding.provider" => self.embedding.provider = defaults.embedding.provider,
            "embedding.model" => self.embedding.model = defaults.embedding.model,
            "embedding.cache_dir" => self.embedding.cache_dir = defaults.embedding.cache_dir,
            "embedding.batch_size" => self.embedding.batch_size = defaults.embedding.batch_size,
            _ => unreachable!("canonical config key must be supported"),
        }
        self.get_config_key(&canonical)
    }
    // END_config_unset_config_key

    // START_CONTRACT_Config::save
    // PURPOSE: Persist this config atomically to synapsec.toml
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: creates the config directory and writes synapsec.toml
    // START_config_save
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path()?;
        write_config_to_path(self, &path)
    }
    // END_config_save
}

// END_Config

impl Default for Config {
    // START_CONTRACT_Config::default
    // PURPOSE: Return built-in Synapse configuration defaults
    // OUTPUTS: { Config }
    fn default() -> Self {
        Self {
            project: ProjectConfig {
                name: "my-project".into(),
                version: "0.1.0".into(),
                strictness: false,
            },
            index: IndexConfig {
                chunk_size: 2000,
                chunk_overlap: 100,
                require_git: true,
            },
            search: SearchConfig {
                max_results: 20,
                similarity_threshold: 0.65,
                hybrid_enabled: false,
            },
            proxy: ProxyConfig {
                enabled: true,
                passthrough_max_chars: 2000,
                capture_cap_bytes: default_proxy_capture_cap_bytes(),
                command_timeout_secs: default_proxy_command_timeout_secs(),
            },
            compress: CompressConfig {
                output_level: "full".into(),
                input_enabled: true,
            },
            tracking: TrackingConfig {
                enabled: true,
                history_days: 90,
            },
            budget: BudgetConfig::default(),
            observability: ObservabilityConfig::default(),
            telemetry: TelemetryConfig::default(),
            graphrag: GraphRagConfig {
                enabled: false,
                use_llm: false,
            },
            embedding: EmbeddingConfig::default(),
            lsp: LspConfig::default(),
        }
    }
}

// START_CONTRACT_canonical_config_key
// PURPOSE: Normalize and validate a user-provided scalar config key
// INPUTS: { key: &str }
// OUTPUTS: { anyhow::Result<String> }
// START_canonical_config_key
fn canonical_config_key(key: &str) -> anyhow::Result<String> {
    let normalized = key.trim().to_ascii_lowercase().replace('-', "_");
    if config_key_spec(&normalized).is_some() {
        Ok(normalized)
    } else {
        anyhow::bail!(
            "unsupported config key `{}`. Supported keys: {}",
            key,
            supported_config_keys()
        )
    }
}
// END_canonical_config_key

// START_CONTRACT_config_key_spec
// PURPOSE: Find metadata for a supported scalar config key
// INPUTS: { key: &str }
// OUTPUTS: { Option<ConfigKeySpec> }
// START_config_key_spec
fn config_key_spec(key: &str) -> Option<ConfigKeySpec> {
    CONFIG_KEY_SPECS
        .iter()
        .copied()
        .find(|spec| spec.key == key)
}
// END_config_key_spec

// START_CONTRACT_supported_config_keys
// PURPOSE: Render supported scalar config keys for error messages
// OUTPUTS: { String }
// START_supported_config_keys
fn supported_config_keys() -> String {
    CONFIG_KEY_SPECS
        .iter()
        .map(|spec| spec.key)
        .collect::<Vec<_>>()
        .join(", ")
}
// END_supported_config_keys

// START_CONTRACT_parse_non_empty_string
// PURPOSE: Parse a non-empty string config value
// INPUTS: { key: &str }, { value: &str }
// OUTPUTS: { anyhow::Result<String> }
// START_parse_non_empty_string
fn parse_non_empty_string(key: &str, value: &str) -> anyhow::Result<String> {
    if value.trim().is_empty() {
        anyhow::bail!("{} must not be empty", key);
    }
    Ok(value.to_string())
}
// END_parse_non_empty_string

// START_CONTRACT_parse_bool
// PURPOSE: Parse a bool config value
// INPUTS: { key: &str }, { value: &str }
// OUTPUTS: { anyhow::Result<bool> }
// START_parse_bool
fn parse_bool(key: &str, value: &str) -> anyhow::Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => anyhow::bail!("{} must be true or false", key),
    }
}
// END_parse_bool

// START_CONTRACT_parse_u32
// PURPOSE: Parse an unsigned 32-bit config value
// INPUTS: { key: &str }, { value: &str }
// OUTPUTS: { anyhow::Result<u32> }
// START_parse_u32
fn parse_u32(key: &str, value: &str) -> anyhow::Result<u32> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|_| anyhow::anyhow!("{} must be an unsigned integer", key))
}
// END_parse_u32

// START_CONTRACT_parse_u32_range
// PURPOSE: Parse a bounded unsigned 32-bit config value
// INPUTS: { key: &str }, { value: &str }, { min: u32 }, { max: u32 }
// OUTPUTS: { anyhow::Result<u32> }
// LINKS:
//   -> NFR-002 (traces_to) - user-supplied runtime bounds are validated
// START_parse_u32_range
fn parse_u32_range(key: &str, value: &str, min: u32, max: u32) -> anyhow::Result<u32> {
    let parsed = parse_u32(key, value)?;
    if !(min..=max).contains(&parsed) {
        anyhow::bail!("{} must be between {} and {}", key, min, max);
    }
    Ok(parsed)
}
// END_parse_u32_range

// START_CONTRACT_parse_u64
// PURPOSE: Parse an unsigned 64-bit config value
// INPUTS: { key: &str }, { value: &str }
// OUTPUTS: { anyhow::Result<u64> }
// START_parse_u64
fn parse_u64(key: &str, value: &str) -> anyhow::Result<u64> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("{} must be an unsigned integer", key))
}
// END_parse_u64

// START_CONTRACT_parse_u64_range
// PURPOSE: Parse a bounded unsigned 64-bit config value.
// INPUTS: { key: &str }, { value: &str }, { min: u64 }, { max: u64 }
// OUTPUTS: { anyhow::Result<u64> }
// LINKS:
//   -> NFR-002 (traces_to) - user-supplied budget bounds are validated
// START_parse_u64_range
fn parse_u64_range(key: &str, value: &str, min: u64, max: u64) -> anyhow::Result<u64> {
    let parsed = parse_u64(key, value)?;
    if !(min..=max).contains(&parsed) {
        anyhow::bail!("{} must be between {} and {}", key, min, max);
    }
    Ok(parsed)
}
// END_parse_u64_range

// START_CONTRACT_parse_usize
// PURPOSE: Parse a usize config value
// INPUTS: { key: &str }, { value: &str }
// OUTPUTS: { anyhow::Result<usize> }
// START_parse_usize
fn parse_usize(key: &str, value: &str) -> anyhow::Result<usize> {
    value
        .trim()
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("{} must be an unsigned integer", key))
}
// END_parse_usize

// START_CONTRACT_parse_usize_range
// PURPOSE: Parse a bounded usize config value
// INPUTS: { key: &str }, { value: &str }, { min: usize }, { max: usize }
// OUTPUTS: { anyhow::Result<usize> }
// LINKS:
//   -> NFR-002 (traces_to) - user-supplied runtime bounds are validated
// START_parse_usize_range
fn parse_usize_range(key: &str, value: &str, min: usize, max: usize) -> anyhow::Result<usize> {
    let parsed = parse_usize(key, value)?;
    if !(min..=max).contains(&parsed) {
        anyhow::bail!("{} must be between {} and {}", key, min, max);
    }
    Ok(parsed)
}
// END_parse_usize_range

// START_CONTRACT_parse_embedding_batch_size
// PURPOSE: Parse a bounded embedding batch size config value
// INPUTS: { key: &str }, { value: &str }
// OUTPUTS: { anyhow::Result<usize> }
// START_parse_embedding_batch_size
fn parse_embedding_batch_size(key: &str, value: &str) -> anyhow::Result<usize> {
    let parsed = parse_usize(key, value)?;
    if !(1..=1024).contains(&parsed) {
        anyhow::bail!("{} must be between 1 and 1024", key);
    }
    Ok(parsed)
}
// END_parse_embedding_batch_size

// START_CONTRACT_parse_f64
// PURPOSE: Parse a finite floating-point config value
// INPUTS: { key: &str }, { value: &str }
// OUTPUTS: { anyhow::Result<f64> }
// START_parse_f64
fn parse_f64(key: &str, value: &str) -> anyhow::Result<f64> {
    let parsed = value
        .trim()
        .parse::<f64>()
        .map_err(|_| anyhow::anyhow!("{} must be a number", key))?;
    if !parsed.is_finite() {
        anyhow::bail!("{} must be finite", key);
    }
    Ok(parsed)
}
// END_parse_f64

// START_CONTRACT_write_config_to_path
// PURPOSE: Atomically persist config TOML to a specific path
// INPUTS: { config: &Config }, { path: &Path }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: creates parent directories and writes the target TOML file
// START_write_config_to_path
fn write_config_to_path(config: &Config, path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let toml_str = toml::to_string_pretty(config)?;
    let tmp_path = path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, toml_str)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}
// END_write_config_to_path

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_config_deserializes_runtime_defaults_for_existing_files() {
        let config: Config = toml::from_str(
            r#"
[project]
name = "my-project"
version = "0.1.0"
strictness = false

[index]
chunk_size = 2000
chunk_overlap = 100
require_git = true

[search]
max_results = 20
similarity_threshold = 0.65
hybrid_enabled = false

[proxy]
enabled = true
passthrough_max_chars = 2000

[compress]
output_level = "full"
input_enabled = true

[tracking]
enabled = true
history_days = 90

[graphrag]
enabled = false
use_llm = false
"#,
        )
        .expect("parse config");

        assert_eq!(config.proxy.capture_cap_bytes, 10_485_760);
        assert_eq!(config.proxy.command_timeout_secs, 300);
        assert!(config.observability.enabled());
        assert!(config.observability.mcp_metrics_enabled());
        assert_eq!(config.observability.mcp_metrics_retention_days(), 30);
        assert_eq!(config.observability.mcp_stats_limit(), 12);
        assert_eq!(config.observability.readiness_mcp_stats_limit(), 5);
        assert_eq!(config.observability.pipeline_response_queue_capacity(), 128);
        assert_eq!(config.observability.pipeline_max_concurrent_requests(), 16);
        assert_eq!(config.observability.pipeline_max_line_bytes(), 10_485_760);
        assert!(!config.embedding.enabled);
        assert_eq!(config.embedding.provider, "fastembed");
        assert_eq!(config.embedding.model, "AllMiniLML6V2");
        assert_eq!(config.embedding.batch_size, 256);
        assert_eq!(config.budget.session_token_limit, 0);
        assert_eq!(config.budget.context_window_limit, 200_000);
        assert_eq!(config.budget.warn_at_pct, 80);
        assert_eq!(config.budget.block_at_pct, 100);
        assert!(config.budget.reset_on_new_session);
    }

    #[test]
    fn default_proxy_runtime_limits_are_release_defaults() {
        let config = Config::default();

        assert_eq!(config.proxy.capture_cap_bytes, 10_485_760);
        assert_eq!(config.proxy.command_timeout_secs, 300);
    }

    #[test]
    fn default_observability_runtime_limits_are_release_defaults() {
        let config = Config::default();

        assert!(config.observability.enabled());
        assert!(config.observability.mcp_metrics_enabled());
        assert_eq!(config.observability.mcp_metrics_retention_days(), 30);
        assert_eq!(config.observability.mcp_stats_limit(), 12);
        assert_eq!(config.observability.readiness_mcp_stats_limit(), 5);
        assert_eq!(config.observability.pipeline_response_queue_capacity(), 128);
        assert_eq!(config.observability.pipeline_max_concurrent_requests(), 16);
        assert_eq!(config.observability.pipeline_max_line_bytes(), 10_485_760);
    }

    #[test]
    fn default_budget_runtime_limits_are_unlimited() {
        let config = Config::default();

        assert_eq!(config.budget.session_token_limit, 0);
        assert_eq!(config.budget.context_window_limit, 200_000);
        assert_eq!(config.budget.warn_at_pct, 80);
        assert_eq!(config.budget.block_at_pct, 100);
        assert!(config.budget.reset_on_new_session);
    }

    #[test]
    fn config_key_helpers_get_set_and_unset_typed_values() {
        let mut config = Config::default();

        let entry = config
            .get_config_key("telemetry.enabled")
            .expect("get telemetry default");
        assert_eq!(entry.value, "false");

        let entry = config
            .set_config_key("search.max-results", "42")
            .expect("set key");
        assert_eq!(entry.key, "search.max_results");
        assert_eq!(entry.value, "42");
        assert_eq!(config.search.max_results, 42);

        let entry = config
            .set_config_key("tracking.enabled", "false")
            .expect("set bool");
        assert_eq!(entry.value, "false");
        assert!(!config.tracking.enabled);

        let entry = config
            .set_config_key("telemetry.enabled", "true")
            .expect("set telemetry toggle");
        assert_eq!(entry.value, "true");
        assert!(config.telemetry.enabled);

        let entry = config
            .set_config_key("embedding.enabled", "true")
            .expect("set embedding enabled");
        assert_eq!(entry.value, "true");
        assert!(config.embedding.enabled);

        let entry = config
            .set_config_key("embedding.batch-size", "128")
            .expect("set embedding batch size");
        assert_eq!(entry.key, "embedding.batch_size");
        assert_eq!(entry.value, "128");
        assert_eq!(config.embedding.batch_size, 128);

        let entry = config
            .set_config_key("observability.pipeline-max-concurrent-requests", "32")
            .expect("set MCP pipeline concurrency");
        assert_eq!(entry.key, "observability.pipeline_max_concurrent_requests");
        assert_eq!(entry.value, "32");
        assert_eq!(config.observability.pipeline_max_concurrent_requests(), 32);

        let entry = config
            .set_config_key("observability.mcp-metrics-enabled", "false")
            .expect("set MCP metrics toggle");
        assert_eq!(entry.value, "false");
        assert!(!config.observability.mcp_metrics_enabled());

        let entry = config
            .set_config_key("budget.session-token-limit", "500000")
            .expect("set budget limit");
        assert_eq!(entry.key, "budget.session_token_limit");
        assert_eq!(entry.value, "500000");
        assert_eq!(config.budget.session_token_limit, 500_000);

        let entry = config
            .set_config_key("budget.context-window-limit", "180000")
            .expect("set context window limit");
        assert_eq!(entry.key, "budget.context_window_limit");
        assert_eq!(entry.value, "180000");
        assert_eq!(config.budget.context_window_limit, 180_000);

        let entry = config
            .set_config_key("budget.warn-at-pct", "75")
            .expect("set budget warning threshold");
        assert_eq!(entry.value, "75");
        assert_eq!(config.budget.warn_at_pct, 75);

        let entry = config
            .set_config_key("budget.block-at-pct", "95")
            .expect("set budget block threshold");
        assert_eq!(entry.value, "95");
        assert_eq!(config.budget.block_at_pct, 95);

        let entry = config
            .set_config_key("budget.reset-on-new-session", "false")
            .expect("set budget reset toggle");
        assert_eq!(entry.value, "false");
        assert!(!config.budget.reset_on_new_session);

        let entry = config
            .unset_config_key("search.max_results")
            .expect("unset key");
        assert_eq!(
            entry.value,
            Config::default().search.max_results.to_string()
        );
        assert_eq!(
            config.search.max_results,
            Config::default().search.max_results
        );
    }

    #[test]
    fn config_key_helpers_reject_unknown_or_invalid_values() {
        let mut config = Config::default();

        assert!(config.get_config_key("unknown.key").is_err());
        assert!(config.set_config_key("tracking.enabled", "maybe").is_err());
        assert!(config.set_config_key("project.name", " ").is_err());
        assert!(config
            .set_config_key("search.similarity_threshold", "NaN")
            .is_err());
        assert!(config.set_config_key("embedding.batch_size", "0").is_err());
        assert!(config
            .set_config_key("embedding.batch_size", "1025")
            .is_err());
        assert!(config
            .set_config_key("observability.mcp_metrics_retention_days", "0")
            .is_err());
        assert!(config
            .set_config_key("observability.mcp_stats_limit", "101")
            .is_err());
        assert!(config
            .set_config_key("observability.pipeline_max_concurrent_requests", "0")
            .is_err());
        assert!(config
            .set_config_key("observability.pipeline_max_line_bytes", "512")
            .is_err());
        assert!(config.set_config_key("budget.warn_at_pct", "101").is_err());
        assert!(config.set_config_key("budget.block_at_pct", "0").is_err());
        assert!(config
            .set_config_key("budget.context_window_limit", "0")
            .is_err());
    }

    #[test]
    fn config_key_helpers_persist_typed_update_to_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("synapsec.toml");
        let mut config = Config::default();
        config
            .set_config_key("tracking.enabled", "false")
            .expect("set key");

        write_config_to_path(&config, &path).expect("write config");
        let persisted = std::fs::read_to_string(&path).expect("read config");
        let loaded: Config = toml::from_str(&persisted).expect("parse persisted config");

        assert!(!loaded.tracking.enabled);
        assert!(!loaded.telemetry.enabled);
        assert_eq!(loaded.budget.session_token_limit, 0);
        assert_eq!(loaded.budget.context_window_limit, 200_000);
    }
}
// END_public_api
