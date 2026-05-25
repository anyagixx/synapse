// MODULE_CONTRACT
// MODULE_ID: M-INDEXER-EMBEDDING
// PURPOSE: Embedding provider specification and helpers for local semantic code search
// SCOPE: provider dependency selection, model identity, cache path, batch limits, fallback policy
// DEPENDS: M-CONFIG, M-INDEXER-STORAGE-TYPES
// LINKS:
//   → M-CONFIG (depends) - embedding enablement, provider, model, cache, and batch settings
//   → M-INDEXER-STORAGE-TYPES (depends) - embedding metadata stored with indexed blocks
//   → M-INDEXER-STORAGE-SEARCH (depends) - query/block vector compatibility checks

// START_MODULE_MAP
// EmbeddingProviderSpec — Exact local embedding provider and model contract
// EmbeddingFallbackPolicy — Runtime behavior when embeddings are disabled or unavailable
// default_provider_spec — Returns the selected provider/model/version contract
// default_model_cache_dir — Returns the XDG data cache directory for embedding assets
// validate_embedding_batch_size — Validates configured embedding batch size bounds
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Selected fastembed all-MiniLM-L6-v2 ONNX provider contract]
// END_CHANGE_SUMMARY

use std::path::PathBuf;

pub const EMBEDDING_PROVIDER: &str = "fastembed";
pub const EMBEDDING_PROVIDER_VERSION: &str = "5.13.4";
pub const EMBEDDING_PROVIDER_FEATURES: &[&str] =
    &["ort-download-binaries-rustls-tls", "hf-hub-rustls-tls"];
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

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_default_provider_spec_documents_fastembed_model
    // PURPOSE: Verify Phase-66 provider selection remains explicit and dimension-compatible
    // START_test_default_provider_spec_documents_fastembed_model
    #[test]
    fn test_default_provider_spec_documents_fastembed_model() {
        let spec = default_provider_spec();

        assert_eq!(spec.provider, "fastembed");
        assert_eq!(spec.provider_version, "5.13.4");
        assert_eq!(spec.model_name, "AllMiniLML6V2");
        assert_eq!(spec.model_repository, "Qdrant/all-MiniLM-L6-v2-onnx");
        assert_eq!(spec.model_format, "onnx");
        assert_eq!(spec.dimensions, 384);
        assert_eq!(spec.default_batch_size, 256);
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
}
