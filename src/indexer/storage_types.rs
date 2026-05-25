// MODULE_CONTRACT
// MODULE_ID: M-INDEXER-STORAGE-TYPES
// PURPOSE: Shared storage data types for indexed code blocks
// SCOPE: StoredBlock serialization shape
// DEPENDS: N/A
// LINKS: docs/modules/M-INDEXER-STORAGE.xml

// START_MODULE_MAP
// StoredBlock — Serializable code block for JSON storage
// SearchFilters — Optional language and relative-path filters for indexed block search
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.7.0 — Added reusable search filter metadata]
// END_CHANGE_SUMMARY

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
}
// END_StoredBlock

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
}
