// MODULE_CONTRACT
// MODULE_ID: M-INDEXER-STORAGE-TYPES
// PURPOSE: Shared storage data types for indexed code blocks and semantic embedding metadata
// SCOPE: StoredBlock serialization shape, backward-compatible embedding metadata, SearchFilters metadata
// DEPENDS: N/A
// LINKS: docs/modules/M-INDEXER-STORAGE.xml

// START_MODULE_MAP
// StoredBlock — Serializable code block for JSON storage
// StoredBlock::set_embedding — Attaches embedding vector metadata to a stored block
// StoredBlock::has_compatible_embedding — Checks embedding model/schema/dimension compatibility
// StoredBlock::clear_embedding — Removes embedding metadata from a stored block
// SearchFilters — Optional language and relative-path filters for indexed block search
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.9.0 — Added backward-compatible embedding metadata]
// END_CHANGE_SUMMARY

pub const CURRENT_EMBEDDING_SCHEMA_VERSION: u32 = 1;

// START_public_api

// START_StoredBlock
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct StoredBlock {
    pub id: String,
    pub path: String,
    pub language: String,
    pub name: String,
    pub kind: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_dimensions: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_schema_version: Option<u32>,
}
// END_StoredBlock

impl StoredBlock {
    // START_CONTRACT_StoredBlock::set_embedding
    // PURPOSE: Attach an embedding vector and compatibility metadata to this block
    // INPUTS: { model: &str }, { vector: Vec<f32> }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: mutates embedding metadata fields
    // START_stored_block_set_embedding
    pub fn set_embedding(&mut self, model: &str, vector: Vec<f32>) -> anyhow::Result<()> {
        if model.trim().is_empty() {
            anyhow::bail!("embedding model must not be empty");
        }
        if vector.is_empty() {
            anyhow::bail!("embedding vector must not be empty");
        }
        let dimensions = vector.len();
        self.embedding = Some(vector);
        self.embedding_model = Some(model.to_string());
        self.embedding_dimensions = Some(dimensions);
        self.embedding_schema_version = Some(CURRENT_EMBEDDING_SCHEMA_VERSION);
        Ok(())
    }
    // END_stored_block_set_embedding

    // START_CONTRACT_StoredBlock::has_compatible_embedding
    // PURPOSE: Check whether this block has an embedding compatible with the active model and dimension
    // INPUTS: { model: &str }, { dimensions: usize }
    // OUTPUTS: { bool }
    // START_stored_block_has_compatible_embedding
    pub fn has_compatible_embedding(&self, model: &str, dimensions: usize) -> bool {
        let Some(vector) = &self.embedding else {
            return false;
        };
        self.embedding_model.as_deref() == Some(model)
            && self.embedding_dimensions == Some(dimensions)
            && self.embedding_schema_version == Some(CURRENT_EMBEDDING_SCHEMA_VERSION)
            && vector.len() == dimensions
    }
    // END_stored_block_has_compatible_embedding

    // START_CONTRACT_StoredBlock::clear_embedding
    // PURPOSE: Remove embedding vector and metadata from this block
    // SIDE_EFFECTS: mutates embedding metadata fields
    // START_stored_block_clear_embedding
    pub fn clear_embedding(&mut self) {
        self.embedding = None;
        self.embedding_model = None;
        self.embedding_dimensions = None;
        self.embedding_schema_version = None;
    }
    // END_stored_block_clear_embedding
}

// START_SearchFilters
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchFilters {
    pub language: Option<String>,
    pub path_prefix: Option<String>,
    pub path_contains: Option<String>,
}
// END_SearchFilters

impl SearchFilters {
    // START_CONTRACT_SearchFilters::new
    // PURPOSE: Build normalized optional search filters for indexed blocks
    // INPUTS: { language: Option<String> }, { path_prefix: Option<String> }, { path_contains: Option<String> }
    // OUTPUTS: { SearchFilters }
    // START_search_filters_new
    pub fn new(
        language: Option<String>,
        path_prefix: Option<String>,
        path_contains: Option<String>,
    ) -> Self {
        Self {
            language: normalize_text_filter(language),
            path_prefix: normalize_path_filter(path_prefix),
            path_contains: normalize_path_filter(path_contains),
        }
    }
    // END_search_filters_new

    // START_CONTRACT_SearchFilters::is_empty
    // PURPOSE: Report whether no filters are configured
    // OUTPUTS: { bool }
    // START_search_filters_is_empty
    pub fn is_empty(&self) -> bool {
        self.language.is_none() && self.path_prefix.is_none() && self.path_contains.is_none()
    }
    // END_search_filters_is_empty

    // START_CONTRACT_SearchFilters::matches
    // PURPOSE: Check whether a stored block satisfies all configured filters
    // INPUTS: { block: &StoredBlock }
    // OUTPUTS: { bool }
    // START_search_filters_matches
    pub fn matches(&self, block: &StoredBlock) -> bool {
        if let Some(language) = &self.language {
            if !block.language.eq_ignore_ascii_case(language) {
                return false;
            }
        }

        let block_path = normalize_path(&block.path);
        if let Some(prefix) = &self.path_prefix {
            if !block_path.starts_with(prefix) {
                return false;
            }
        }
        if let Some(contains) = &self.path_contains {
            if !block_path.contains(contains) {
                return false;
            }
        }
        true
    }
    // END_search_filters_matches
}

// START_CONTRACT_normalize_text_filter
// PURPOSE: Normalize optional text filters while dropping empty values
// INPUTS: { value: Option<String> }
// OUTPUTS: { Option<String> }
// START_normalize_text_filter
fn normalize_text_filter(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
}
// END_normalize_text_filter

// START_CONTRACT_normalize_path_filter
// PURPOSE: Normalize optional relative path filters while dropping empty values
// INPUTS: { value: Option<String> }
// OUTPUTS: { Option<String> }
// START_normalize_path_filter
fn normalize_path_filter(value: Option<String>) -> Option<String> {
    value.map(|v| normalize_path(&v)).filter(|v| !v.is_empty())
}
// END_normalize_path_filter

// START_CONTRACT_normalize_path
// PURPOSE: Normalize project-relative paths for stable filter comparisons
// INPUTS: { value: &str }
// OUTPUTS: { String }
// START_normalize_path
fn normalize_path(value: &str) -> String {
    let mut normalized = value.trim().replace('\\', "/").to_ascii_lowercase();
    while normalized.starts_with("./") {
        normalized = normalized[2..].to_string();
    }
    normalized = normalized.trim_start_matches('/').to_string();
    normalized.trim_end_matches('/').to_string()
}
// END_normalize_path

// START_CONTRACT_public_api
// PURPOSE: Export persisted code block data and reusable search filter metadata
// OUTPUTS: { StoredBlock, SearchFilters }
// END_public_api

#[cfg(test)]
mod tests {
    use super::*;

    fn block(path: &str, language: &str) -> StoredBlock {
        StoredBlock {
            id: format!("{path}:1"),
            path: path.to_string(),
            language: language.to_string(),
            name: "sample".into(),
            kind: "function".into(),
            content: "fn sample() {}".into(),
            start_line: 1,
            end_line: 1,
            embedding: None,
            embedding_model: None,
            embedding_dimensions: None,
            embedding_schema_version: None,
        }
    }

    #[test]
    fn test_search_filters_empty_matches_any_block() {
        let filters = SearchFilters::default();

        assert!(filters.is_empty());
        assert!(filters.matches(&block("src/lib.rs", "rust")));
    }

    #[test]
    fn test_search_filters_match_language_case_insensitive() {
        let filters = SearchFilters::new(Some("Rust".into()), None, None);

        assert!(filters.matches(&block("src/lib.rs", "rust")));
        assert!(!filters.matches(&block("src/main.py", "python")));
    }

    #[test]
    fn test_search_filters_match_relative_path_prefix() {
        let filters = SearchFilters::new(None, Some("./src/indexer".into()), None);

        assert!(filters.matches(&block("src/indexer/storage.rs", "rust")));
        assert!(!filters.matches(&block("tests/indexer.rs", "rust")));
    }

    #[test]
    fn test_search_filters_match_path_contains() {
        let filters = SearchFilters::new(None, None, Some("server_code".into()));

        assert!(filters.matches(&block("src/mcp/server_code_tools.rs", "rust")));
        assert!(!filters.matches(&block("src/mcp/server_tools.rs", "rust")));
    }

    #[test]
    fn test_stored_block_embedding_metadata_roundtrip() {
        let mut block = block("src/lib.rs", "rust");

        block
            .set_embedding("AllMiniLML6V2", vec![0.1, 0.2, 0.3])
            .expect("set embedding");

        assert!(block.has_compatible_embedding("AllMiniLML6V2", 3));
        assert!(!block.has_compatible_embedding("OtherModel", 3));
        assert!(!block.has_compatible_embedding("AllMiniLML6V2", 4));
    }

    #[test]
    fn test_stored_block_embedding_metadata_defaults_for_legacy_json() {
        let block: StoredBlock = serde_json::from_str(
            r#"{
              "id": "src/lib.rs:1",
              "path": "src/lib.rs",
              "language": "rust",
              "name": "sample",
              "kind": "function",
              "content": "fn sample() {}",
              "start_line": 1,
              "end_line": 1
            }"#,
        )
        .expect("legacy stored block");

        assert!(block.embedding.is_none());
        assert!(!block.has_compatible_embedding("AllMiniLML6V2", 384));
    }

    #[test]
    fn test_stored_block_embedding_rejects_empty_vector() {
        let mut block = block("src/lib.rs", "rust");

        assert!(block.set_embedding("AllMiniLML6V2", Vec::new()).is_err());
        assert!(block.set_embedding("", vec![0.1]).is_err());
    }
}
