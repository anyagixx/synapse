// MODULE_CONTRACT
// MODULE_ID: M-CONFIG
// PURPOSE: Config model and explicit load/default/init semantics for synapsec.toml
// SCOPE: Config struct definitions, read-only TOML load, default fallback, explicit default config generation, path resolution
// DEPENDS: N/A
// LINKS: synapsec.toml

// START_MODULE_MAP
// Config — Top-level config struct (project, index, search, proxy, compress, tracking, graphrag)
// ProjectConfig — Project metadata (name, version, strictness)
// IndexConfig — Indexer settings (chunk_size, chunk_overlap, require_git)
// SearchConfig — Search settings (max_results, similarity_threshold, hybrid_enabled)
// ProxyConfig — Proxy settings (enabled, passthrough_max_chars)
// CompressConfig — Compress settings (output_level, input_enabled)
// TrackingConfig — Tracking settings (enabled, history_days)
// GraphRagConfig — GraphRAG settings (enabled, use_llm)
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.8.0 — Clarified read-only load, default fallback, and explicit init semantics]
// END_CHANGE_SUMMARY

use std::path::PathBuf;

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
    pub graphrag: GraphRagConfig,
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
pub struct GraphRagConfig {
    pub enabled: bool,
    pub use_llm: bool,
}

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
            },
            compress: CompressConfig {
                output_level: "full".into(),
                input_enabled: true,
            },
            tracking: TrackingConfig {
                enabled: true,
                history_days: 90,
            },
            graphrag: GraphRagConfig {
                enabled: false,
                use_llm: false,
            },
        }
    }
}
// END_public_api
