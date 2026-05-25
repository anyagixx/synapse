// MODULE_CONTRACT
// MODULE_ID: M-INDEXER-STORAGE
// PURPOSE: JSON block storage facade with load-health reporting, snapshot and delta persistence, filtered BM25, and vector search APIs
// SCOPE: Storage struct, StoredBlock, SearchFilters, JSON persistence, full snapshot replacement, per-file upsert/removal, load-health reporting, filtered search API orchestration
// DEPENDS: M-INDEXER-STORAGE-SEARCH, M-INDEXER-STORAGE-TYPES
// LINKS: N/A

// START_MODULE_MAP
// StoredBlock — Re-exported serializable code block for JSON storage
// SearchFilters — Re-exported search filter metadata for storage callers
// Storage — JSON-backed block store with load-health, filtered BM25, and vector search
// replace_all_blocks — Replaces the full index snapshot and removes stale file entries
// upsert_file_blocks — Replaces one file's indexed blocks without a full snapshot rewrite
// remove_file_blocks — Removes one deleted file's indexed blocks without a full snapshot rewrite
// load_blocks — Reads and validates stored JSON blocks
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.8.0 — Added filtered search and vector search APIs]
// END_CHANGE_SUMMARY

use super::storage_search::{
    block_matches_filters, cosine_similarity, expand_query_terms, ngram_vectorize, score_block,
};
use std::path::{Path, PathBuf};

// START_public_api

// START_StoredBlock
pub use super::storage_types::{SearchFilters, StoredBlock};
// END_StoredBlock

// START_Storage
pub struct Storage {
    db_path: PathBuf,
    blocks: Vec<StoredBlock>,
    load_error: Option<String>,
}
// END_Storage

impl Storage {
    // START_CONTRACT_Storage::new
    // PURPOSE: Create or load a Storage for a project root
    // OUTPUTS: { Self }
    // SIDE_EFFECTS: reads/writes blocks.json
    // START_storage_new
    pub fn new(project_root: &Path) -> Self {
        let db_path = Self::db_path_for_root(project_root);
        let Some(db_dir) = db_path.parent().map(Path::to_path_buf) else {
            return Self {
                db_path,
                blocks: Vec::new(),
                load_error: Some("index path has no parent directory".into()),
            };
        };

        let mut load_error = None;
        if let Err(e) = std::fs::create_dir_all(&db_dir) {
            let message = format!("cannot create index directory {}: {}", db_dir.display(), e);
            tracing::warn!("[Storage][new][INIT] {}", message);
            load_error = Some(message);
        }

        let (blocks, read_error) = if load_error.is_none() {
            load_blocks(&db_path)
        } else {
            (Vec::new(), None)
        };
        if read_error.is_some() {
            load_error = read_error;
        }

        Self {
            db_path,
            blocks,
            load_error,
        }
    }
    // END_storage_new

    // START_CONTRACT_Storage::db_path_for_root
    // PURPOSE: Return the deterministic JSON storage path for a project root
    // INPUTS: { project_root: &Path }
    // OUTPUTS: { PathBuf }
    // START_storage_db_path_for_root
    pub fn db_path_for_root(project_root: &Path) -> PathBuf {
        let hash = simple_hash(project_root);
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("synapse")
            .join("index")
            .join(hash)
            .join("blocks.json")
    }
    // END_storage_db_path_for_root

    // START_CONTRACT_Storage::db_path
    // PURPOSE: Return this storage instance's JSON database path
    // OUTPUTS: { &Path }
    // START_storage_db_path
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
    // END_storage_db_path

    // START_CONTRACT_Storage::load_error
    // PURPOSE: Return the storage load error when persisted JSON could not be read or parsed
    // OUTPUTS: { Option<&str> }
    // START_storage_load_error
    pub fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }
    // END_storage_load_error

    // START_CONTRACT_Storage::count
    // PURPOSE: Return the number of stored blocks
    // OUTPUTS: { usize }
    // START_storage_count
    pub fn count(&self) -> usize {
        self.blocks.len()
    }
    // END_storage_count

    // START_CONTRACT_Storage::all_blocks
    // PURPOSE: Return a reference to all stored blocks
    // OUTPUTS: { &[StoredBlock] }
    // START_storage_all_blocks
    pub fn all_blocks(&self) -> &[StoredBlock] {
        &self.blocks
    }
    // END_storage_all_blocks

    // START_CONTRACT_Storage::is_empty
    // PURPOSE: Check if storage has no blocks
    // OUTPUTS: { bool }
    // START_storage_is_empty
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
    // END_storage_is_empty

    // START_CONTRACT_Storage::store_blocks
    // PURPOSE: Replace blocks for given file paths and persist to JSON
    // INPUTS: { new_blocks: Vec<StoredBlock> }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: flushes to blocks.json
    // START_storage_store_blocks
    pub fn store_blocks(&mut self, new_blocks: Vec<StoredBlock>) -> anyhow::Result<()> {
        let mut seen_files: std::collections::HashSet<String> = std::collections::HashSet::new();
        for b in &new_blocks {
            seen_files.insert(b.path.clone());
        }
        self.blocks.retain(|b| !seen_files.contains(&b.path));
        self.blocks.extend(new_blocks);
        self.flush()
    }
    // END_storage_store_blocks

    // START_CONTRACT_Storage::upsert_file_blocks
    // PURPOSE: Replace all blocks for one file path and persist without rewriting unrelated files
    // INPUTS: { path: &str — project-relative source path }, { new_blocks: Vec<StoredBlock> — replacement blocks for path }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: flushes to blocks.json
    // START_storage_upsert_file_blocks
    pub fn upsert_file_blocks(
        &mut self,
        path: &str,
        new_blocks: Vec<StoredBlock>,
    ) -> anyhow::Result<()> {
        if let Some(mismatched) = new_blocks.iter().find(|block| block.path != path) {
            anyhow::bail!(
                "cannot upsert blocks for {}: block {} belongs to {}",
                path,
                mismatched.id,
                mismatched.path
            );
        }
        self.blocks.retain(|block| block.path != path);
        self.blocks.extend(new_blocks);
        self.flush()
    }
    // END_storage_upsert_file_blocks

    // START_CONTRACT_Storage::remove_file_blocks
    // PURPOSE: Remove all stored blocks for one deleted file path and persist
    // INPUTS: { path: &str — project-relative source path }
    // OUTPUTS: { anyhow::Result<usize> — number of removed blocks }
    // SIDE_EFFECTS: flushes to blocks.json
    // START_storage_remove_file_blocks
    pub fn remove_file_blocks(&mut self, path: &str) -> anyhow::Result<usize> {
        let before = self.blocks.len();
        self.blocks.retain(|block| block.path != path);
        let removed = before.saturating_sub(self.blocks.len());
        self.flush()?;
        Ok(removed)
    }
    // END_storage_remove_file_blocks

    // START_CONTRACT_Storage::remove_files_blocks
    // PURPOSE: Remove stored blocks for multiple deleted file paths and persist once
    // INPUTS: { paths: &[String] — project-relative source paths }
    // OUTPUTS: { anyhow::Result<usize> — number of removed blocks }
    // SIDE_EFFECTS: flushes to blocks.json
    // START_storage_remove_files_blocks
    pub fn remove_files_blocks(&mut self, paths: &[String]) -> anyhow::Result<usize> {
        let removed_paths: std::collections::HashSet<&str> =
            paths.iter().map(String::as_str).collect();
        let before = self.blocks.len();
        self.blocks
            .retain(|block| !removed_paths.contains(block.path.as_str()));
        let removed = before.saturating_sub(self.blocks.len());
        self.flush()?;
        Ok(removed)
    }
    // END_storage_remove_files_blocks

    // START_CONTRACT_Storage::replace_all_blocks
    // PURPOSE: Replace the complete stored block snapshot after a full project index
    // INPUTS: { blocks: Vec<StoredBlock> }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: removes stale blocks for deleted files and flushes blocks.json
    // START_storage_replace_all_blocks
    pub fn replace_all_blocks(&mut self, blocks: Vec<StoredBlock>) -> anyhow::Result<()> {
        self.blocks = blocks;
        self.flush()
    }
    // END_storage_replace_all_blocks

    // START_CONTRACT_Storage::flush
    // PURPOSE: Write blocks to JSON file on disk
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes to blocks.json
    // START_storage_flush
    pub fn flush(&self) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(&self.blocks)?;
        // Atomic write: write to temp, then rename
        let tmp_path = self.db_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &json)?;
        std::fs::rename(&tmp_path, &self.db_path)?;
        Ok(())
    }
    // END_storage_flush

    // START_CONTRACT_Storage::search
    // PURPOSE: BM25 text search over stored blocks
    // INPUTS: { query: &str }, { max_results: usize }
    // OUTPUTS: { Vec<StoredBlock> }
    // START_storage_search
    pub fn search(&self, query: &str, max_results: usize) -> Vec<StoredBlock> {
        let filters = SearchFilters::default();
        self.search_with_filters(query, max_results, &filters)
    }
    // END_storage_search

    // START_CONTRACT_Storage::search_with_filters
    // PURPOSE: BM25 text search over stored blocks after applying metadata filters
    // INPUTS: { query: &str }, { max_results: usize }, { filters: &SearchFilters }
    // OUTPUTS: { Vec<StoredBlock> }
    // START_storage_search_with_filters
    pub fn search_with_filters(
        &self,
        query: &str,
        max_results: usize,
        filters: &SearchFilters,
    ) -> Vec<StoredBlock> {
        if query.trim().is_empty() {
            return Vec::new();
        }
        self.search_with_scores_and_filters(query, max_results, filters)
            .into_iter()
            .map(|(b, _)| b)
            .collect()
    }
    // END_storage_search_with_filters

    // START_CONTRACT_Storage::search_with_scores
    // PURPOSE: BM25 search returning scored results
    // INPUTS: { query: &str }, { max_results: usize }
    // OUTPUTS: { Vec<(StoredBlock, f64)> }
    // START_storage_search_with_scores
    pub fn search_with_scores(&self, query: &str, max_results: usize) -> Vec<(StoredBlock, f64)> {
        let filters = SearchFilters::default();
        self.search_with_scores_and_filters(query, max_results, &filters)
    }
    // END_storage_search_with_scores

    // START_CONTRACT_Storage::search_with_scores_and_filters
    // PURPOSE: BM25 search returning scored results after applying metadata filters before ranking
    // INPUTS: { query: &str }, { max_results: usize }, { filters: &SearchFilters }
    // OUTPUTS: { Vec<(StoredBlock, f64)> }
    // START_storage_search_with_scores_and_filters
    pub fn search_with_scores_and_filters(
        &self,
        query: &str,
        max_results: usize,
        filters: &SearchFilters,
    ) -> Vec<(StoredBlock, f64)> {
        let query_lower = query.to_lowercase();
        if query_lower.trim().is_empty() {
            return Vec::new();
        }
        let expanded_terms = expand_query_terms(&query_lower);
        let query_words: Vec<&str> = expanded_terms.iter().map(|term| term.as_str()).collect();

        let mut scored: Vec<(f64, StoredBlock)> = self
            .blocks
            .iter()
            .filter(|b| block_matches_filters(b, filters))
            .filter_map(|b| {
                let score = score_block(b, &query_lower, &query_words);
                if score > 0.0 {
                    Some((score, b.clone()))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(max_results);
        scored.into_iter().map(|(s, b)| (b, s)).collect()
    }
    // END_storage_search_with_scores_and_filters

    // START_CONTRACT_Storage::vector_search
    // PURPOSE: N-gram vector search with cosine similarity
    // INPUTS: { query: &str }, { max_results: usize }
    // OUTPUTS: { Vec<(StoredBlock, f64)> }
    // START_storage_vector_search
    pub fn vector_search(&self, query: &str, max_results: usize) -> Vec<(StoredBlock, f64)> {
        let filters = SearchFilters::default();
        self.vector_search_with_filters(query, max_results, &filters)
    }
    // END_storage_vector_search

    // START_CONTRACT_Storage::vector_search_with_filters
    // PURPOSE: N-gram vector search with cosine similarity after applying metadata filters before ranking
    // INPUTS: { query: &str }, { max_results: usize }, { filters: &SearchFilters }
    // OUTPUTS: { Vec<(StoredBlock, f64)> }
    // START_storage_vector_search_with_filters
    pub fn vector_search_with_filters(
        &self,
        query: &str,
        max_results: usize,
        filters: &SearchFilters,
    ) -> Vec<(StoredBlock, f64)> {
        let expanded_terms = expand_query_terms(query);
        let query_text = expanded_terms.join(" ");
        let query_vec = ngram_vectorize(&query_text);
        if query_vec.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(f64, StoredBlock)> = self
            .blocks
            .iter()
            .filter(|b| block_matches_filters(b, filters))
            .filter_map(|b| {
                let content_vec = ngram_vectorize(&b.content);
                let name_vec = ngram_vectorize(&b.name);
                if content_vec.is_empty() && name_vec.is_empty() {
                    return None;
                }
                let content_sim = cosine_similarity(&query_vec, &content_vec);
                let name_sim = cosine_similarity(&query_vec, &name_vec);
                let score = content_sim * 0.7 + name_sim * 0.3;
                if score > 0.02 {
                    Some((score, b.clone()))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(max_results);
        scored.into_iter().map(|(s, b)| (b, s)).collect()
    }
    // END_storage_vector_search_with_filters
}

// START_CONTRACT_load_blocks
// PURPOSE: Read persisted JSON blocks and return a visible load error for unreadable or corrupted storage
// INPUTS: { db_path: &Path }
// OUTPUTS: { (Vec<StoredBlock>, Option<String>) }
// START_load_blocks
fn load_blocks(db_path: &Path) -> (Vec<StoredBlock>, Option<String>) {
    if !db_path.exists() {
        return (Vec::new(), None);
    }

    let content = match std::fs::read_to_string(db_path) {
        Ok(content) => content,
        Err(e) => {
            let message = format!("cannot read index {}: {}", db_path.display(), e);
            tracing::warn!("[Storage][load_blocks][READ] {}", message);
            return (Vec::new(), Some(message));
        }
    };

    if content.trim().is_empty() {
        return (Vec::new(), None);
    }

    match serde_json::from_str(&content) {
        Ok(blocks) => (blocks, None),
        Err(e) => {
            let message = format!("corrupted index {}: {}", db_path.display(), e);
            tracing::warn!("[Storage][load_blocks][PARSE] {}", message);
            (Vec::new(), Some(message))
        }
    }
}
// END_load_blocks

fn simple_hash(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    result
        .iter()
        .take(8)
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block(id: &str, path: &str, name: &str, kind: &str, content: &str) -> StoredBlock {
        make_block_with_language(id, path, "rust", name, kind, content)
    }

    fn make_block_with_language(
        id: &str,
        path: &str,
        language: &str,
        name: &str,
        kind: &str,
        content: &str,
    ) -> StoredBlock {
        StoredBlock {
            id: id.to_string(),
            path: path.to_string(),
            language: language.to_string(),
            name: name.to_string(),
            kind: kind.to_string(),
            content: content.to_string(),
            start_line: 1,
            end_line: content.lines().count(),
        }
    }

    fn tmp_storage() -> Storage {
        let dir = std::env::temp_dir().join(format!("syn-test-{}", uuid::Uuid::new_v4()));
        Storage::new(&dir)
    }

    #[test]
    fn test_corrupted_index_reports_load_error() {
        let dir = std::env::temp_dir().join(format!("syn-corrupt-{}", uuid::Uuid::new_v4()));
        let hash = simple_hash(&dir);
        let db_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("synapse")
            .join("index")
            .join(hash);
        let db_path = db_dir.join("blocks.json");
        std::fs::create_dir_all(&db_dir).unwrap();
        std::fs::write(&db_path, "{ invalid json").unwrap();

        let storage = Storage::new(&dir);
        assert_eq!(storage.count(), 0);
        assert!(
            storage
                .load_error()
                .is_some_and(|e| e.contains("corrupted index")),
            "corruption should be visible: {:?}",
            storage.load_error()
        );

        let _ = std::fs::remove_dir_all(db_dir);
    }

    #[test]
    fn test_store_and_count() {
        let mut s = tmp_storage();
        s.store_blocks(vec![make_block(
            "a:1",
            "src/lib.rs",
            "login",
            "function",
            "fn login() { check_password() }",
        )])
        .unwrap();
        assert_eq!(s.count(), 1);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_store_replaces_same_file() {
        let mut s = tmp_storage();
        s.store_blocks(vec![make_block(
            "a:1",
            "src/lib.rs",
            "old",
            "fn",
            "fn old() {}",
        )])
        .unwrap();
        assert_eq!(s.count(), 1);
        s.store_blocks(vec![
            make_block("a:2", "src/lib.rs", "new1", "fn", "fn new1() {}"),
            make_block("a:5", "src/lib.rs", "new2", "fn", "fn new2() {}"),
        ])
        .unwrap();
        assert_eq!(s.count(), 2);
    }

    #[test]
    fn test_upsert_file_blocks_replaces_empty_file() {
        let mut s = tmp_storage();
        s.store_blocks(vec![
            make_block("a:1", "src/lib.rs", "old", "fn", "fn old() {}"),
            make_block("b:1", "src/other.rs", "keep", "fn", "fn keep() {}"),
        ])
        .unwrap();

        s.upsert_file_blocks("src/lib.rs", Vec::new()).unwrap();

        assert_eq!(s.count(), 1);
        assert!(s.search("old", 10).is_empty());
        assert_eq!(s.search("keep", 10).len(), 1);
    }

    #[test]
    fn test_upsert_file_blocks_rejects_mismatched_paths() {
        let mut s = tmp_storage();
        let err = s
            .upsert_file_blocks(
                "src/lib.rs",
                vec![make_block(
                    "a:1",
                    "src/other.rs",
                    "other",
                    "fn",
                    "fn other() {}",
                )],
            )
            .expect_err("mismatched path should fail");

        assert!(err.to_string().contains("belongs to src/other.rs"));
    }

    #[test]
    fn test_remove_file_blocks_removes_deleted_file_only() {
        let mut s = tmp_storage();
        s.store_blocks(vec![
            make_block("a:1", "src/deleted.rs", "deleted", "fn", "fn deleted() {}"),
            make_block("b:1", "src/keep.rs", "keep", "fn", "fn keep() {}"),
        ])
        .unwrap();

        let removed = s.remove_file_blocks("src/deleted.rs").unwrap();

        assert_eq!(removed, 1);
        assert_eq!(s.count(), 1);
        assert!(s.search("deleted", 10).is_empty());
        assert_eq!(s.search("keep", 10).len(), 1);
    }

    #[test]
    fn test_remove_files_blocks_flushes_once_for_deleted_paths() {
        let mut s = tmp_storage();
        s.store_blocks(vec![
            make_block("a:1", "src/a.rs", "alpha", "fn", "fn alpha() {}"),
            make_block("b:1", "src/b.rs", "beta", "fn", "fn beta() {}"),
            make_block("c:1", "src/c.rs", "gamma", "fn", "fn gamma() {}"),
        ])
        .unwrap();

        let removed = s
            .remove_files_blocks(&["src/a.rs".to_string(), "src/b.rs".to_string()])
            .unwrap();

        assert_eq!(removed, 2);
        assert_eq!(s.count(), 1);
        assert_eq!(s.search("gamma", 10).len(), 1);
    }

    #[test]
    fn test_replace_all_blocks_removes_stale_files() {
        let mut s = tmp_storage();
        s.store_blocks(vec![
            make_block("a:1", "src/old.rs", "old", "fn", "fn old() {}"),
            make_block("b:1", "src/keep.rs", "keep", "fn", "fn keep() {}"),
        ])
        .unwrap();

        s.replace_all_blocks(vec![make_block(
            "b:1",
            "src/keep.rs",
            "keep",
            "fn",
            "fn keep() {}",
        )])
        .unwrap();

        assert_eq!(s.count(), 1);
        assert!(s.search("old", 10).is_empty());
        assert_eq!(s.search("keep", 10).len(), 1);
    }

    #[test]
    fn test_search_by_name() {
        let mut s = tmp_storage();
        s.store_blocks(vec![
            make_block("a:1", "src/main.rs", "main", "function", "fn main() {}"),
            make_block(
                "a:3",
                "src/auth.rs",
                "login",
                "function",
                "fn login(user: &str) -> bool { true }",
            ),
        ])
        .unwrap();
        let results = s.search("login", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "login");
    }

    #[test]
    fn test_search_by_content() {
        let mut s = tmp_storage();
        s.store_blocks(vec![
            make_block(
                "a:1",
                "src/main.rs",
                "main",
                "fn",
                "fn main() { let db = Database::new(); }",
            ),
            make_block(
                "a:3",
                "src/db.rs",
                "Database",
                "struct",
                "pub struct Database { conn: SqliteConnection }",
            ),
        ])
        .unwrap();
        let results = s.search("database", 10);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_empty_query_returns_empty() {
        let mut s = tmp_storage();
        s.store_blocks(vec![make_block(
            "a:1",
            "src/main.rs",
            "main",
            "fn",
            "fn main() {}",
        )])
        .unwrap();
        assert!(s.search("", 10).is_empty());
    }

    #[test]
    fn test_max_results_limit() {
        let mut s = tmp_storage();
        let blocks: Vec<_> = (0..10)
            .map(|i| {
                make_block(
                    &format!("a:{}", i),
                    &format!("f{}.rs", i),
                    &format!("fn{}", i),
                    "fn",
                    "fn test() {}",
                )
            })
            .collect();
        s.store_blocks(blocks).unwrap();
        assert_eq!(s.search("test", 3).len(), 3);
    }

    #[test]
    fn test_search_filters_by_language() {
        let mut s = tmp_storage();
        s.store_blocks(vec![
            make_block_with_language("a:1", "src/lib.rs", "rust", "login", "fn", "fn login() {}"),
            make_block_with_language(
                "b:1",
                "src/login.py",
                "python",
                "login",
                "function",
                "def login(): pass",
            ),
        ])
        .unwrap();

        let filters = SearchFilters::new(Some("rust".into()), None, None);
        let results = s.search_with_scores_and_filters("login", 10, &filters);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.path, "src/lib.rs");
    }

    #[test]
    fn test_search_filters_by_path_prefix() {
        let mut s = tmp_storage();
        s.store_blocks(vec![
            make_block(
                "a:1",
                "src/indexer/storage.rs",
                "search_storage",
                "fn",
                "fn search_storage() {}",
            ),
            make_block(
                "b:1",
                "src/mcp/server.rs",
                "search_server",
                "fn",
                "fn search_server() {}",
            ),
        ])
        .unwrap();

        let filters = SearchFilters::new(None, Some("src/indexer".into()), None);
        let results = s.search_with_scores_and_filters("search", 10, &filters);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.path, "src/indexer/storage.rs");
    }

    #[test]
    fn test_db_path_for_root_matches_storage_instance_path() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new(dir.path());

        assert_eq!(storage.db_path(), Storage::db_path_for_root(dir.path()));
    }
}
// END_public_api
