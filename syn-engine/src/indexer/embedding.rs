// MODULE_CONTRACT
// MODULE_ID: M-INDEXER-EMBEDDING
// PURPOSE: Embedding provider specification and helpers for local semantic code search
// SCOPE: provider dependency selection, model identity, cache path, batch limits, block/query embedding, fallback policy
// DEPENDS: M-CONFIG, M-INDEXER-STORAGE-TYPES
// LINKS:
//   → M-CONFIG (depends) - embedding enablement, provider, model, cache, and batch settings
//   → M-INDEXER-STORAGE-TYPES (depends) - embedding metadata stored with indexed blocks
//   → M-INDEXER-STORAGE-SEARCH (depends) - query/block vector compatibility checks

// START_MODULE_MAP
// EmbeddingProviderSpec — Exact local embedding provider and model contract
// EmbeddingFallbackPolicy — Runtime behavior when embeddings are disabled or unavailable
// EmbeddingRuntimeConfig — Validated runtime embedding settings copied from Config
// EmbeddedText — Query embedding result with model compatibility metadata
// EmbeddingRunSummary — Summary of block embedding attachment work
// EmbeddingProvider — Provider trait for fastembed runtime and deterministic unit fakes
// FastembedEmbeddingProvider — fastembed-backed local ONNX text embedding provider
// configure_dynamic_ort_path — Selects bundled ONNX Runtime dylib for dynamic macOS Intel builds
// initialize_text_embedding — Initializes fastembed while converting ONNX Runtime panics into errors
// default_provider_spec — Returns the selected provider/model/version contract
// default_model_cache_dir — Returns the XDG data cache directory for embedding assets
// validate_embedding_batch_size — Validates configured embedding batch size bounds
// embed_blocks_from_config — Best-effort runtime entry for indexing blocks
// embed_query_from_config — Runtime entry for embedding a search query
// attach_embeddings — Attaches provider vectors to StoredBlock values
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.2.0 - Added macOS Intel dynamic ONNX Runtime build support]
// END_CHANGE_SUMMARY

use super::storage_types::StoredBlock;
use syn_core::config::Config;
use anyhow::Context as _;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::path::PathBuf;

pub const EMBEDDING_PROVIDER: &str = "fastembed";
pub const EMBEDDING_PROVIDER_VERSION: &str = "5.13.4";

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub const EMBEDDING_PROVIDER_FEATURES: &[&str] = &["ort-load-dynamic", "hf-hub-rustls-tls"];

#[cfg(not(all(target_os = "macos", target_arch = "x86_64")))]
pub const EMBEDDING_PROVIDER_FEATURES: &[&str] =
    &["ort-download-binaries-rustls-tls", "hf-hub-rustls-tls"];

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub const EMBEDDING_RUNTIME_LINKAGE: &str = "dynamic-onnxruntime";

#[cfg(not(all(target_os = "macos", target_arch = "x86_64")))]
pub const EMBEDDING_RUNTIME_LINKAGE: &str = "linked-onnxruntime";

pub const EMBEDDING_MODEL_NAME: &str = "AllMiniLML6V2";
pub const EMBEDDING_MODEL_REPOSITORY: &str = "Qdrant/all-MiniLM-L6-v2-onnx";
pub const EMBEDDING_MODEL_FILE: &str = "model.onnx";
pub const EMBEDDING_MODEL_FORMAT: &str = "onnx";
pub const EMBEDDING_MODEL_DIMENSIONS: usize = 384;
pub const DEFAULT_EMBEDDING_BATCH_SIZE: usize = 256;
pub const MIN_EMBEDDING_BATCH_SIZE: usize = 1;
pub const MAX_EMBEDDING_BATCH_SIZE: usize = 1024;

// START_public_api

// START_EmbeddingProviderSpec
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddingProviderSpec {
    pub provider: &'static str,
    pub provider_version: &'static str,
    pub provider_features: &'static [&'static str],
    pub model_name: &'static str,
    pub model_repository: &'static str,
    pub model_file: &'static str,
    pub model_format: &'static str,
    pub dimensions: usize,
    pub default_batch_size: usize,
}
// END_EmbeddingProviderSpec

// START_EmbeddingFallbackPolicy
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmbeddingFallbackPolicy {
    LexicalOnly,
}
// END_EmbeddingFallbackPolicy

// START_EmbeddingRuntimeConfig
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddingRuntimeConfig {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub cache_dir: PathBuf,
    pub batch_size: usize,
}
// END_EmbeddingRuntimeConfig

// START_EmbeddedText
#[derive(Clone, Debug, PartialEq)]
pub struct EmbeddedText {
    pub model_id: String,
    pub dimensions: usize,
    pub vector: Vec<f32>,
}
// END_EmbeddedText

// START_EmbeddingRunSummary
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EmbeddingRunSummary {
    pub enabled: bool,
    pub attempted_blocks: usize,
    pub embedded_blocks: usize,
    pub skipped_blocks: usize,
    pub model_id: Option<String>,
}
// END_EmbeddingRunSummary

// START_EmbeddingProvider
pub trait EmbeddingProvider {
    fn model_id(&self) -> &str;
    fn dimensions(&self) -> usize;
    fn embed_texts(&mut self, texts: &[String], batch_size: usize)
        -> anyhow::Result<Vec<Vec<f32>>>;
}
// END_EmbeddingProvider

// START_FastembedEmbeddingProvider
pub struct FastembedEmbeddingProvider {
    model_id: String,
    dimensions: usize,
    inner: TextEmbedding,
}
// END_FastembedEmbeddingProvider

impl EmbeddingRuntimeConfig {
    // START_CONTRACT_EmbeddingRuntimeConfig::from_config
    // PURPOSE: Copy and validate embedding runtime settings from Config
    // INPUTS: { config: &Config }
    // OUTPUTS: { anyhow::Result<EmbeddingRuntimeConfig> }
    // START_embedding_runtime_config_from_config
    pub fn from_config(config: &Config) -> anyhow::Result<Self> {
        let batch_size = validate_embedding_batch_size(config.embedding.batch_size)?;
        let cache_dir = resolve_model_cache_dir(&config.embedding.cache_dir)?;
        Ok(Self {
            enabled: config.embedding.enabled,
            provider: config.embedding.provider.trim().to_string(),
            model: config.embedding.model.trim().to_string(),
            cache_dir,
            batch_size,
        })
    }
    // END_embedding_runtime_config_from_config

    // START_CONTRACT_EmbeddingRuntimeConfig::model_id
    // PURPOSE: Return the persisted embedding model identifier used for compatibility checks
    // OUTPUTS: { anyhow::Result<String> }
    // START_embedding_runtime_config_model_id
    pub fn model_id(&self) -> anyhow::Result<String> {
        self.ensure_supported()?;
        Ok(default_model_id())
    }
    // END_embedding_runtime_config_model_id

    // START_CONTRACT_EmbeddingRuntimeConfig::ensure_supported
    // PURPOSE: Verify this runtime config names the embedded fastembed model supported by Synapse
    // OUTPUTS: { anyhow::Result<()> }
    // START_embedding_runtime_config_ensure_supported
    fn ensure_supported(&self) -> anyhow::Result<()> {
        if !self.provider.eq_ignore_ascii_case(EMBEDDING_PROVIDER) {
            anyhow::bail!(
                "unsupported embedding provider `{}`; supported provider is `{}`",
                self.provider,
                EMBEDDING_PROVIDER
            );
        }
        if parse_embedding_model(&self.model).is_none() {
            anyhow::bail!(
                "unsupported embedding model `{}`; supported model is `{}`",
                self.model,
                EMBEDDING_MODEL_NAME
            );
        }
        Ok(())
    }
    // END_embedding_runtime_config_ensure_supported
}

impl FastembedEmbeddingProvider {
    // START_CONTRACT_FastembedEmbeddingProvider::new
    // PURPOSE: Initialize a fastembed text model from validated runtime config
    // INPUTS: { runtime: &EmbeddingRuntimeConfig }
    // OUTPUTS: { anyhow::Result<FastembedEmbeddingProvider> }
    // SIDE_EFFECTS: creates model cache directory and may download model assets on first use
    // START_fastembed_embedding_provider_new
    pub fn new(runtime: &EmbeddingRuntimeConfig) -> anyhow::Result<Self> {
        runtime.ensure_supported()?;
        std::fs::create_dir_all(&runtime.cache_dir)?;
        let model = parse_embedding_model(&runtime.model)
            .ok_or_else(|| anyhow::anyhow!("unsupported embedding model `{}`", runtime.model))?;
        configure_dynamic_ort_path();
        let options = InitOptions::new(model)
            .with_cache_dir(runtime.cache_dir.clone())
            .with_show_download_progress(false);
        let inner = initialize_text_embedding(options)?;
        Ok(Self {
            model_id: runtime.model_id()?,
            dimensions: EMBEDDING_MODEL_DIMENSIONS,
            inner,
        })
    }
    // END_fastembed_embedding_provider_new
}

// START_CONTRACT_configure_dynamic_ort_path
// PURPOSE: Prefer a bundled ONNX Runtime dylib beside the syn executable for dynamic macOS Intel builds
// OUTPUTS: { none }
// SIDE_EFFECTS: may set ORT_DYLIB_PATH before fastembed initializes ONNX Runtime
// START_configure_dynamic_ort_path
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
fn configure_dynamic_ort_path() {
    if std::env::var_os("ORT_DYLIB_PATH").is_some() {
        return;
    }
    let Ok(exe_path) = std::env::current_exe() else {
        return;
    };
    let Some(exe_dir) = exe_path.parent() else {
        return;
    };
    let candidate = exe_dir.join("libonnxruntime.dylib");
    if candidate.is_file() {
        std::env::set_var("ORT_DYLIB_PATH", candidate);
    }
}

#[cfg(not(all(target_os = "macos", target_arch = "x86_64")))]
fn configure_dynamic_ort_path() {}
// END_configure_dynamic_ort_path

// START_CONTRACT_initialize_text_embedding
// PURPOSE: Initialize fastembed and return actionable errors when ONNX Runtime is unavailable
// INPUTS: { options: InitOptions }
// OUTPUTS: { anyhow::Result<TextEmbedding> }
// SIDE_EFFECTS: may initialize ONNX Runtime and download model assets through fastembed
// START_initialize_text_embedding
fn initialize_text_embedding(options: InitOptions) -> anyhow::Result<TextEmbedding> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        TextEmbedding::try_new(options)
    }));
    result
        .map_err(|_| {
            anyhow::anyhow!(
                "embedding provider failed to initialize ONNX Runtime using {}; set ORT_DYLIB_PATH to a compatible libonnxruntime.dylib or disable embeddings",
                EMBEDDING_RUNTIME_LINKAGE
            )
        })?
        .with_context(|| {
            format!(
                "embedding provider failed to initialize ONNX Runtime using {}; set ORT_DYLIB_PATH to a compatible libonnxruntime.dylib or disable embeddings",
                EMBEDDING_RUNTIME_LINKAGE
            )
        })
}
// END_initialize_text_embedding

impl EmbeddingProvider for FastembedEmbeddingProvider {
    // START_CONTRACT_FastembedEmbeddingProvider::model_id
    // PURPOSE: Return the persisted model id for this fastembed provider
    // OUTPUTS: { &str }
    // START_fastembed_embedding_provider_model_id
    fn model_id(&self) -> &str {
        &self.model_id
    }
    // END_fastembed_embedding_provider_model_id

    // START_CONTRACT_FastembedEmbeddingProvider::dimensions
    // PURPOSE: Return expected vector dimensionality for this fastembed provider
    // OUTPUTS: { usize }
    // START_fastembed_embedding_provider_dimensions
    fn dimensions(&self) -> usize {
        self.dimensions
    }
    // END_fastembed_embedding_provider_dimensions

    // START_CONTRACT_FastembedEmbeddingProvider::embed_texts
    // PURPOSE: Generate dense embeddings for a batch of text inputs
    // INPUTS: { texts: &[String] }, { batch_size: usize }
    // OUTPUTS: { anyhow::Result<Vec<Vec<f32>>> }
    // START_fastembed_embedding_provider_embed_texts
    fn embed_texts(
        &mut self,
        texts: &[String],
        batch_size: usize,
    ) -> anyhow::Result<Vec<Vec<f32>>> {
        self.inner.embed(texts, Some(batch_size))
    }
    // END_fastembed_embedding_provider_embed_texts
}

// START_CONTRACT_default_provider_spec
// PURPOSE: Return the exact selected local embedding provider and model contract
// OUTPUTS: { EmbeddingProviderSpec }
// START_default_provider_spec
pub fn default_provider_spec() -> EmbeddingProviderSpec {
    EmbeddingProviderSpec {
        provider: EMBEDDING_PROVIDER,
        provider_version: EMBEDDING_PROVIDER_VERSION,
        provider_features: EMBEDDING_PROVIDER_FEATURES,
        model_name: EMBEDDING_MODEL_NAME,
        model_repository: EMBEDDING_MODEL_REPOSITORY,
        model_file: EMBEDDING_MODEL_FILE,
        model_format: EMBEDDING_MODEL_FORMAT,
        dimensions: EMBEDDING_MODEL_DIMENSIONS,
        default_batch_size: DEFAULT_EMBEDDING_BATCH_SIZE,
    }
}
// END_default_provider_spec

// START_CONTRACT_default_fallback_policy
// PURPOSE: Return the deterministic search fallback when embeddings are unavailable
// OUTPUTS: { EmbeddingFallbackPolicy }
// START_default_fallback_policy
pub fn default_fallback_policy() -> EmbeddingFallbackPolicy {
    EmbeddingFallbackPolicy::LexicalOnly
}
// END_default_fallback_policy

// START_CONTRACT_default_model_cache_dir
// PURPOSE: Resolve the default cache directory for local embedding model assets
// OUTPUTS: { anyhow::Result<PathBuf> }
// START_default_model_cache_dir
pub fn default_model_cache_dir() -> anyhow::Result<PathBuf> {
    let data_dir = dirs::data_dir().ok_or_else(|| anyhow::anyhow!("Cannot find data directory"))?;
    Ok(data_dir.join("synapse").join("models").join("fastembed"))
}
// END_default_model_cache_dir

// START_CONTRACT_validate_embedding_batch_size
// PURPOSE: Validate configured embedding batch size bounds before indexing
// INPUTS: { batch_size: usize }
// OUTPUTS: { anyhow::Result<usize> }
// START_validate_embedding_batch_size
pub fn validate_embedding_batch_size(batch_size: usize) -> anyhow::Result<usize> {
    if !(MIN_EMBEDDING_BATCH_SIZE..=MAX_EMBEDDING_BATCH_SIZE).contains(&batch_size) {
        anyhow::bail!(
            "embedding batch size must be between {} and {}",
            MIN_EMBEDDING_BATCH_SIZE,
            MAX_EMBEDDING_BATCH_SIZE
        );
    }
    Ok(batch_size)
}
// END_validate_embedding_batch_size

// START_CONTRACT_embed_blocks_from_config
// PURPOSE: Attach configured embeddings to indexed blocks when embedding is enabled
// INPUTS: { blocks: &mut [StoredBlock] }, { config: &Config }
// OUTPUTS: { anyhow::Result<EmbeddingRunSummary> }
// SIDE_EFFECTS: may download local model assets and mutates StoredBlock embedding metadata
// START_embed_blocks_from_config
pub fn embed_blocks_from_config(
    blocks: &mut [StoredBlock],
    config: &Config,
) -> anyhow::Result<EmbeddingRunSummary> {
    let runtime = EmbeddingRuntimeConfig::from_config(config)?;
    if !runtime.enabled {
        return Ok(EmbeddingRunSummary {
            enabled: false,
            skipped_blocks: blocks.len(),
            ..EmbeddingRunSummary::default()
        });
    }
    let batch_size = runtime.batch_size;
    let mut provider = FastembedEmbeddingProvider::new(&runtime)?;
    attach_embeddings(blocks, &mut provider, batch_size)
}
// END_embed_blocks_from_config

// START_CONTRACT_embed_query_from_config
// PURPOSE: Generate a compatible embedding for a search query when embedding is enabled
// INPUTS: { query: &str }, { config: &Config }
// OUTPUTS: { anyhow::Result<Option<EmbeddedText>> }
// SIDE_EFFECTS: may download local model assets
// START_embed_query_from_config
pub fn embed_query_from_config(
    query: &str,
    config: &Config,
) -> anyhow::Result<Option<EmbeddedText>> {
    if query.trim().is_empty() {
        return Ok(None);
    }
    let runtime = EmbeddingRuntimeConfig::from_config(config)?;
    if !runtime.enabled {
        return Ok(None);
    }
    let batch_size = runtime.batch_size;
    let mut provider = FastembedEmbeddingProvider::new(&runtime)?;
    let texts = vec![query.to_string()];
    let mut embeddings = provider.embed_texts(&texts, batch_size)?;
    let vector = embeddings
        .pop()
        .ok_or_else(|| anyhow::anyhow!("embedding provider returned no query vector"))?;
    validate_vector_dimensions(&vector, provider.dimensions())?;
    Ok(Some(EmbeddedText {
        model_id: provider.model_id().to_string(),
        dimensions: provider.dimensions(),
        vector,
    }))
}
// END_embed_query_from_config

// START_CONTRACT_attach_embeddings
// PURPOSE: Attach provider vectors and metadata to blocks missing compatible embeddings
// INPUTS: { blocks: &mut [StoredBlock] }, { provider: &mut impl EmbeddingProvider }, { batch_size: usize }
// OUTPUTS: { anyhow::Result<EmbeddingRunSummary> }
// SIDE_EFFECTS: mutates StoredBlock embedding metadata
// START_attach_embeddings
pub fn attach_embeddings<P: EmbeddingProvider>(
    blocks: &mut [StoredBlock],
    provider: &mut P,
    batch_size: usize,
) -> anyhow::Result<EmbeddingRunSummary> {
    validate_embedding_batch_size(batch_size)?;
    let model_id = provider.model_id().to_string();
    let dimensions = provider.dimensions();
    if dimensions == 0 {
        anyhow::bail!("embedding provider dimensions must not be zero");
    }

    let mut pending_indices = Vec::new();
    let mut texts = Vec::new();
    for (idx, block) in blocks.iter().enumerate() {
        if block.has_compatible_embedding(&model_id, dimensions) {
            continue;
        }
        pending_indices.push(idx);
        texts.push(block_embedding_text(block));
    }

    if texts.is_empty() {
        return Ok(EmbeddingRunSummary {
            enabled: true,
            skipped_blocks: blocks.len(),
            model_id: Some(model_id),
            ..EmbeddingRunSummary::default()
        });
    }

    let embeddings = provider.embed_texts(&texts, batch_size)?;
    if embeddings.len() != pending_indices.len() {
        anyhow::bail!(
            "embedding provider returned {} vectors for {} blocks",
            embeddings.len(),
            pending_indices.len()
        );
    }

    let attempted_blocks = pending_indices.len();
    for (idx, vector) in pending_indices.into_iter().zip(embeddings) {
        validate_vector_dimensions(&vector, dimensions)?;
        blocks[idx].set_embedding(&model_id, vector)?;
    }

    Ok(EmbeddingRunSummary {
        enabled: true,
        attempted_blocks,
        embedded_blocks: attempted_blocks,
        skipped_blocks: blocks.len().saturating_sub(attempted_blocks),
        model_id: Some(model_id),
    })
}
// END_attach_embeddings

// START_CONTRACT_default_model_id
// PURPOSE: Return the persisted embedding model id for the selected provider/model/version
// OUTPUTS: { String }
// START_default_model_id
pub fn default_model_id() -> String {
    format!(
        "{}:{}@{}",
        EMBEDDING_PROVIDER, EMBEDDING_MODEL_NAME, EMBEDDING_PROVIDER_VERSION
    )
}
// END_default_model_id

// START_CONTRACT_resolve_model_cache_dir
// PURPOSE: Resolve configured or default model cache directory
// INPUTS: { configured: &str }
// OUTPUTS: { anyhow::Result<PathBuf> }
// START_resolve_model_cache_dir
fn resolve_model_cache_dir(configured: &str) -> anyhow::Result<PathBuf> {
    let trimmed = configured.trim();
    if trimmed.is_empty() {
        return default_model_cache_dir();
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return Ok(home.join(rest));
        }
    }
    Ok(PathBuf::from(trimmed))
}
// END_resolve_model_cache_dir

// START_CONTRACT_parse_embedding_model
// PURPOSE: Map supported config model aliases to fastembed model enum values
// INPUTS: { model: &str }
// OUTPUTS: { Option<EmbeddingModel> }
// START_parse_embedding_model
fn parse_embedding_model(model: &str) -> Option<EmbeddingModel> {
    match model.trim().to_ascii_lowercase().as_str() {
        "allminilml6v2" | "all-minilm-l6-v2" | "sentence-transformers/all-minilm-l6-v2" => {
            Some(EmbeddingModel::AllMiniLML6V2)
        }
        _ => None,
    }
}
// END_parse_embedding_model

// START_CONTRACT_block_embedding_text
// PURPOSE: Build stable text input for a code block embedding
// INPUTS: { block: &StoredBlock }
// OUTPUTS: { String }
// START_block_embedding_text
fn block_embedding_text(block: &StoredBlock) -> String {
    format!(
        "path: {}\nlanguage: {}\nkind: {}\nname: {}\ncode:\n{}",
        block.path, block.language, block.kind, block.name, block.content
    )
}
// END_block_embedding_text

// START_CONTRACT_validate_vector_dimensions
// PURPOSE: Verify provider vectors match configured model dimensions
// INPUTS: { vector: &[f32] }, { dimensions: usize }
// OUTPUTS: { anyhow::Result<()> }
// START_validate_vector_dimensions
fn validate_vector_dimensions(vector: &[f32], dimensions: usize) -> anyhow::Result<()> {
    if vector.len() != dimensions {
        anyhow::bail!(
            "embedding vector has {} dimensions, expected {}",
            vector.len(),
            dimensions
        );
    }
    Ok(())
}
// END_validate_vector_dimensions

// END_public_api

#[cfg(test)]
mod tests {
    use super::super::storage_types::CURRENT_EMBEDDING_SCHEMA_VERSION;
    use super::*;

    struct FakeEmbeddingProvider {
        model_id: String,
        dimensions: usize,
    }

    impl FakeEmbeddingProvider {
        fn new(model_id: &str, dimensions: usize) -> Self {
            Self {
                model_id: model_id.to_string(),
                dimensions,
            }
        }
    }

    impl EmbeddingProvider for FakeEmbeddingProvider {
        fn model_id(&self) -> &str {
            &self.model_id
        }

        fn dimensions(&self) -> usize {
            self.dimensions
        }

        fn embed_texts(
            &mut self,
            texts: &[String],
            _batch_size: usize,
        ) -> anyhow::Result<Vec<Vec<f32>>> {
            Ok(texts
                .iter()
                .enumerate()
                .map(|(idx, text)| vec![text.len() as f32, idx as f32 + 1.0, 1.0])
                .collect())
        }
    }

    fn stored_block(id: &str, name: &str) -> StoredBlock {
        StoredBlock {
            id: id.to_string(),
            path: "src/lib.rs".into(),
            language: "rust".into(),
            name: name.to_string(),
            kind: "function".into(),
            content: format!("fn {name}() {{}}"),
            start_line: 1,
            end_line: 1,
            embedding: None,
            embedding_model: None,
            embedding_dimensions: None,
            embedding_schema_version: None,
        }
    }

    // START_CONTRACT_test_default_provider_spec_documents_fastembed_model
    // PURPOSE: Verify Phase-66 provider selection remains explicit and dimension-compatible
    // START_test_default_provider_spec_documents_fastembed_model
    #[test]
    fn test_default_provider_spec_documents_fastembed_model() {
        let spec = default_provider_spec();

        assert_eq!(spec.provider, "fastembed");
        assert_eq!(spec.provider_version, "5.13.4");
        assert!(spec.provider_features.contains(&"hf-hub-rustls-tls"));
        assert_eq!(spec.model_name, "AllMiniLML6V2");
        assert_eq!(spec.model_repository, "Qdrant/all-MiniLM-L6-v2-onnx");
        assert_eq!(spec.model_format, "onnx");
        assert_eq!(spec.dimensions, 384);
        assert_eq!(spec.default_batch_size, 256);

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        assert!(spec.provider_features.contains(&"ort-load-dynamic"));

        #[cfg(not(all(target_os = "macos", target_arch = "x86_64")))]
        assert!(spec
            .provider_features
            .contains(&"ort-download-binaries-rustls-tls"));
    }
    // END_test_default_provider_spec_documents_fastembed_model

    // START_CONTRACT_test_validate_embedding_batch_size_bounds
    // PURPOSE: Verify configured embedding batches stay within bounded indexing limits
    // START_test_validate_embedding_batch_size_bounds
    #[test]
    fn test_validate_embedding_batch_size_bounds() {
        assert_eq!(validate_embedding_batch_size(1).expect("min"), 1);
        assert_eq!(validate_embedding_batch_size(1024).expect("max"), 1024);
        assert!(validate_embedding_batch_size(0).is_err());
        assert!(validate_embedding_batch_size(1025).is_err());
    }
    // END_test_validate_embedding_batch_size_bounds

    // START_CONTRACT_test_runtime_config_uses_safe_disabled_default
    // PURPOSE: Verify Config defaults do not trigger implicit model downloads
    // START_test_runtime_config_uses_safe_disabled_default
    #[test]
    fn test_runtime_config_uses_safe_disabled_default() {
        let runtime = EmbeddingRuntimeConfig::from_config(&Config::default()).expect("runtime");

        assert!(!runtime.enabled);
        assert_eq!(runtime.provider, EMBEDDING_PROVIDER);
        assert_eq!(runtime.model, EMBEDDING_MODEL_NAME);
        assert_eq!(runtime.batch_size, DEFAULT_EMBEDDING_BATCH_SIZE);
    }
    // END_test_runtime_config_uses_safe_disabled_default

    // START_CONTRACT_test_runtime_config_rejects_unsupported_provider_or_model
    // PURPOSE: Verify provider/model validation fails before indexing with incompatible embeddings
    // START_test_runtime_config_rejects_unsupported_provider_or_model
    #[test]
    fn test_runtime_config_rejects_unsupported_provider_or_model() {
        let mut config = Config::default();
        config.embedding.enabled = true;
        config.embedding.provider = "other".into();
        let runtime = EmbeddingRuntimeConfig::from_config(&config).expect("runtime");
        assert!(runtime.model_id().is_err());

        config.embedding.provider = EMBEDDING_PROVIDER.into();
        config.embedding.model = "unknown-model".into();
        let runtime = EmbeddingRuntimeConfig::from_config(&config).expect("runtime");
        assert!(runtime.model_id().is_err());
    }
    // END_test_runtime_config_rejects_unsupported_provider_or_model

    // START_CONTRACT_test_attach_embeddings_sets_metadata
    // PURPOSE: Verify embedding attachment stores vectors with compatibility metadata
    // START_test_attach_embeddings_sets_metadata
    #[test]
    fn test_attach_embeddings_sets_metadata() {
        let mut provider = FakeEmbeddingProvider::new("fake:test", 3);
        let mut blocks = vec![
            stored_block("src/lib.rs:1", "alpha"),
            stored_block("src/lib.rs:2", "beta"),
        ];

        let summary = attach_embeddings(&mut blocks, &mut provider, 2).expect("attach");

        assert!(summary.enabled);
        assert_eq!(summary.attempted_blocks, 2);
        assert_eq!(summary.embedded_blocks, 2);
        assert_eq!(summary.model_id.as_deref(), Some("fake:test"));
        assert!(blocks[0].has_compatible_embedding("fake:test", 3));
        assert_eq!(blocks[0].embedding_dimensions, Some(3));
        assert_eq!(
            blocks[0].embedding_schema_version,
            Some(CURRENT_EMBEDDING_SCHEMA_VERSION)
        );
    }
    // END_test_attach_embeddings_sets_metadata

    // START_CONTRACT_test_attach_embeddings_skips_compatible_blocks
    // PURPOSE: Verify existing compatible embeddings are not regenerated
    // START_test_attach_embeddings_skips_compatible_blocks
    #[test]
    fn test_attach_embeddings_skips_compatible_blocks() {
        let mut provider = FakeEmbeddingProvider::new("fake:test", 3);
        let mut blocks = vec![
            stored_block("src/lib.rs:1", "alpha"),
            stored_block("src/lib.rs:2", "beta"),
        ];
        blocks[0]
            .set_embedding("fake:test", vec![1.0, 0.0, 0.0])
            .expect("seed embedding");

        let summary = attach_embeddings(&mut blocks, &mut provider, 2).expect("attach");

        assert_eq!(summary.attempted_blocks, 1);
        assert_eq!(summary.embedded_blocks, 1);
        assert_eq!(summary.skipped_blocks, 1);
        assert_eq!(blocks[0].embedding.as_deref(), Some(&[1.0, 0.0, 0.0][..]));
        assert!(blocks[1].has_compatible_embedding("fake:test", 3));
    }
    // END_test_attach_embeddings_skips_compatible_blocks
}
