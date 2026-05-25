// MODULE_CONTRACT
// MODULE_ID: M-INDEXER
// PURPOSE: Code indexer — walks, delegates pipeline block construction, stores full or delta snapshots, and searches code blocks with guarded storage health checks
// SCOPE: Indexer struct, SearchResult, guarded storage locks, index_directory, index_delta, gitignore-aware indexing, embedding attachment, stale-entry pruning, GraphBuilder cache invalidation, filtered search, search_in_root, hybrid_search, hybrid_search_in_root, storage health propagation, view_signatures
// DEPENDS: M-INDEXER-EMBEDDING, M-INDEXER-PIPELINE, M-INDEXER-WALKER, M-INDEXER-PARSER, M-INDEXER-STORAGE, M-INDEXER-STORAGE-SEARCH, M-INDEXER-STORAGE-TYPES, M-CONFIG
// LINKS:
//   → M-INDEXER-EMBEDDING (depends) — semantic embedding provider specification
//   → M-INDEXER-PIPELINE (depends) — deterministic full-index block construction
//   → M-INDEXER-WALKER (depends) — source discovery
//   → M-INDEXER-PARSER (depends) — signature parsing
//   → M-INDEXER-STORAGE (depends) — index persistence and search
//   → M-CONFIG (depends) — runtime configuration

// START_MODULE_MAP
// SearchResult — Search result with path, language, name, kind, lines, content, score
// Indexer — Main indexer combining walker, parser, and storage
// Indexer::storage_count — Returns loaded storage count through guarded lock access
// Indexer::index_directory_with_gitignore — Rebuilds index snapshot with configurable gitignore handling
// Indexer::index_delta — Applies changed and deleted file updates without a full project rebuild
// Indexer::embed_blocks_for_indexing — Best-effort embedding attachment before block persistence
// Indexer::embed_query_for_search — Best-effort query embedding for semantic search
// Indexer::search_with_filters — Searches indexed blocks with optional language/path filters
// Indexer::search_in_root — Searches indexed blocks for an explicit project root
// Indexer::hybrid_search_in_root — Runs graph-aware hybrid search for an explicit project root
// ensure_storage_ready — Converts storage load-health errors into actionable search errors
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v4.0.0 - Added best-effort embedding generation for indexing and search]
// END_CHANGE_SUMMARY

pub mod embedding;
pub mod parser;
pub mod pipeline;
pub mod storage;
mod storage_search;
mod storage_types;
pub mod walker;

use crate::config::Config;
use crate::graphrag::builder::GraphBuilder;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use storage::{SearchFilters, Storage};

const SIGNATURE_CONTENT_PREVIEW_LIMIT: usize = 200;

// START_public_api

// START_SearchResult
pub struct SearchResult {
    pub path: String,
    pub language: String,
    pub name: String,
    pub kind: String,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
    pub score: f64,
    pub explanation: String,
}
// END_SearchResult

// START_Indexer
pub struct Indexer {
    pub config: Config,
    pub storage: RwLock<Option<Storage>>,
}
// END_Indexer

impl Indexer {
    // START_CONTRACT_Indexer::new
    // PURPOSE: Create a new Indexer
    // OUTPUTS: { Self }
    // START_indexer_new
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            storage: RwLock::new(None),
        }
    }
    // END_indexer_new

    // START_CONTRACT_Indexer::get_storage
    // PURPOSE: Return initialized storage under a guarded write lock without panicking on poisoned locks
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<RwLockWriteGuard<Option<Storage>>> }
    // SIDE_EFFECTS: initializes storage when missing
    // START_indexer_get_storage
    fn get_storage(
        &self,
        root: &Path,
    ) -> anyhow::Result<std::sync::RwLockWriteGuard<'_, Option<Storage>>> {
        let mut guard = self
            .storage
            .write()
            .map_err(|_| anyhow::anyhow!("indexer storage lock poisoned"))?;
        if guard.is_none() {
            *guard = Some(Storage::new(root));
        }
        Ok(guard)
    }
    // END_indexer_get_storage

    // START_CONTRACT_Indexer::storage_count
    // PURPOSE: Return the loaded storage block count through guarded read access
    // OUTPUTS: { anyhow::Result<usize> }
    // START_indexer_storage_count
    pub fn storage_count(&self) -> anyhow::Result<usize> {
        let guard = self
            .storage
            .read()
            .map_err(|_| anyhow::anyhow!("indexer storage lock poisoned"))?;
        Ok(guard.as_ref().map(|storage| storage.count()).unwrap_or(0))
    }
    // END_indexer_storage_count

    // START_CONTRACT_Indexer::index_directory
    // PURPOSE: Walk, parse, and index all source files in a directory
    // INPUTS: { root: &Path — project root }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes indexed blocks to storage
    // START_indexer_index_directory
    pub async fn index_directory(&self, root: &Path) -> anyhow::Result<()> {
        self.index_directory_with_gitignore(root, true).await
    }
    // END_indexer_index_directory

    // START_CONTRACT_Indexer::index_directory_with_gitignore
    // PURPOSE: Walk, parse, and replace the full index snapshot for all source files in a directory
    // INPUTS: { root: &Path — project root }, { respect_gitignore: bool — whether .gitignore rules apply }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: rewrites indexed blocks and purges entries for deleted files
    // START_indexer_index_directory_with_gitignore
    pub async fn index_directory_with_gitignore(
        &self,
        root: &Path,
        respect_gitignore: bool,
    ) -> anyhow::Result<()> {
        let walker = walker::Walker::new_with_gitignore(root, respect_gitignore);
        let files = walker.walk();
        let total = files.len();

        tracing::info!("Indexing {} files in {}", total, root.display());

        let mut all_stored = pipeline::collect_index_blocks(root, &files);
        self.embed_blocks_for_indexing(&mut all_stored, "full");

        let mut storage_guard = self.get_storage(root)?;
        let storage = storage_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("index storage unavailable after initialization"))?;
        storage.replace_all_blocks(all_stored)?;
        GraphBuilder::invalidate_cache(root);
        tracing::info!(
            "Index complete: {} blocks from {} files",
            storage.count(),
            total
        );
        Ok(())
    }
    // END_indexer_index_directory_with_gitignore

    // START_CONTRACT_Indexer::index_delta
    // PURPOSE: Apply changed and deleted file updates without rebuilding the full project index
    // INPUTS: { root: &Path — project root }, { changed_files: &[PathBuf] — changed source paths }, { deleted_files: &[PathBuf] — deleted source paths }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: updates stored blocks for changed/deleted files and invalidates GraphBuilder cache for root
    // START_indexer_index_delta
    pub async fn index_delta(
        &self,
        root: &Path,
        changed_files: &[PathBuf],
        deleted_files: &[PathBuf],
    ) -> anyhow::Result<()> {
        let deleted_paths = normalize_delta_paths(root, deleted_files)?;
        let changed_paths = normalize_delta_paths(root, changed_files)?;
        let mut changed_updates = Vec::new();
        for path in changed_paths
            .iter()
            .filter(|path| !deleted_paths.contains(*path))
        {
            let full_path = root.join(path);
            let mut blocks = if let Some(language) = walker::detect_language(&full_path) {
                let file = walker::IndexFile {
                    path: path.clone(),
                    language,
                };
                pipeline::process_index_file(root, &file).unwrap_or_default()
            } else {
                Vec::new()
            };
            self.embed_blocks_for_indexing(&mut blocks, "delta");
            changed_updates.push((path.clone(), blocks));
        }

        let mut storage_guard = self.get_storage(root)?;
        let storage = storage_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("index storage unavailable after initialization"))?;

        let deleted: Vec<String> = deleted_paths.iter().cloned().collect();
        let removed_count = storage.remove_files_blocks(&deleted)?;
        let mut changed_count = 0usize;
        for (path, blocks) in changed_updates {
            storage.upsert_file_blocks(&path, blocks)?;
            changed_count += 1;
        }

        if changed_count > 0 || removed_count > 0 || !deleted_paths.is_empty() {
            GraphBuilder::invalidate_cache_for_delta(root, changed_count, deleted_paths.len());
        }
        tracing::info!(
            "[Indexer][index_delta][DELTA_APPLY] changed={} deleted={} removed_blocks={}",
            changed_count,
            deleted_paths.len(),
            removed_count
        );
        Ok(())
    }
    // END_indexer_index_delta

    // START_CONTRACT_Indexer::embed_blocks_for_indexing
    // PURPOSE: Attach configured embeddings to blocks before persistence without blocking lexical index storage
    // INPUTS: { blocks: &mut [StoredBlock] }, { scope: &str }
    // SIDE_EFFECTS: may download embedding model assets, mutates block metadata, logs fallback warnings
    // START_indexer_embed_blocks_for_indexing
    fn embed_blocks_for_indexing(&self, blocks: &mut [storage::StoredBlock], scope: &str) {
        match embedding::embed_blocks_from_config(blocks, &self.config) {
            Ok(summary) if summary.enabled => {
                tracing::info!(
                    "[Indexer][embed_blocks_for_indexing][EMBEDDING] scope={} embedded={} skipped={} model={}",
                    scope,
                    summary.embedded_blocks,
                    summary.skipped_blocks,
                    summary.model_id.as_deref().unwrap_or("unknown")
                );
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(
                    "[Indexer][embed_blocks_for_indexing][EMBEDDING_FALLBACK] scope={} error={}; continuing with lexical index",
                    scope,
                    error
                );
                for block in blocks {
                    block.clear_embedding();
                }
            }
        }
    }
    // END_indexer_embed_blocks_for_indexing

    // START_CONTRACT_Indexer::embed_query_for_search
    // PURPOSE: Best-effort query embedding for semantic vector search
    // INPUTS: { query: &str }
    // OUTPUTS: { Option<embedding::EmbeddedText> }
    // SIDE_EFFECTS: may download embedding model assets and logs fallback warnings
    // START_indexer_embed_query_for_search
    fn embed_query_for_search(&self, query: &str) -> Option<embedding::EmbeddedText> {
        match embedding::embed_query_from_config(query, &self.config) {
            Ok(query_embedding) => query_embedding,
            Err(error) => {
                tracing::warn!(
                    "[Indexer][embed_query_for_search][EMBEDDING_FALLBACK] error={}; using lexical search",
                    error
                );
                None
            }
        }
    }
    // END_indexer_embed_query_for_search

    // START_CONTRACT_Indexer::search
    // PURPOSE: BM25 search across indexed code blocks
    // INPUTS: { query: &str }, { max_results: usize }
    // OUTPUTS: { anyhow::Result<Vec<SearchResult>> }
    // START_indexer_search
    pub async fn search(
        &self,
        query: &str,
        max_results: usize,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let root = std::env::current_dir()?;
        self.search_in_root(&root, query, max_results).await
    }
    // END_indexer_search

    // START_CONTRACT_Indexer::search_with_filters
    // PURPOSE: BM25 search across indexed code blocks using optional metadata filters
    // INPUTS: { query: &str }, { max_results: usize }, { filters: &SearchFilters }
    // OUTPUTS: { anyhow::Result<Vec<SearchResult>> }
    // START_indexer_search_with_filters
    pub async fn search_with_filters(
        &self,
        query: &str,
        max_results: usize,
        filters: &SearchFilters,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let root = std::env::current_dir()?;
        self.search_in_root_with_filters(&root, query, max_results, filters)
            .await
    }
    // END_indexer_search_with_filters

    // START_CONTRACT_Indexer::search_in_root
    // PURPOSE: BM25 search across indexed code blocks for an explicit project root
    // INPUTS: { root: &Path }, { query: &str }, { max_results: usize }
    // OUTPUTS: { anyhow::Result<Vec<SearchResult>> }
    // START_indexer_search_in_root
    pub async fn search_in_root(
        &self,
        root: &Path,
        query: &str,
        max_results: usize,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let filters = SearchFilters::default();
        self.search_in_root_with_filters(root, query, max_results, &filters)
            .await
    }
    // END_indexer_search_in_root

    // START_CONTRACT_Indexer::search_in_root_with_filters
    // PURPOSE: BM25 search across indexed code blocks for an explicit project root using optional metadata filters
    // INPUTS: { root: &Path }, { query: &str }, { max_results: usize }, { filters: &SearchFilters }
    // OUTPUTS: { anyhow::Result<Vec<SearchResult>> }
    // START_indexer_search_in_root_with_filters
    pub async fn search_in_root_with_filters(
        &self,
        root: &Path,
        query: &str,
        max_results: usize,
        filters: &SearchFilters,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let query_embedding = self.embed_query_for_search(query);
        let storage_guard = self.get_storage(root)?;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("index storage unavailable after initialization"))?;
        ensure_storage_ready(storage)?;

        let (results, explanation) = if let Some(query_embedding) = query_embedding {
            let semantic = storage.embedding_vector_search_with_filters(
                &query_embedding.vector,
                &query_embedding.model_id,
                query_embedding.dimensions,
                max_results,
                filters,
            );
            if semantic.is_empty() {
                tracing::warn!(
                    "[Indexer][search_in_root_with_filters][EMBEDDING_FALLBACK] no compatible block embeddings; using lexical search"
                );
                (
                    storage.search_with_scores_and_filters(query, max_results, filters),
                    "lexical fallback",
                )
            } else {
                (semantic, "semantic embedding match")
            }
        } else {
            (
                storage.search_with_scores_and_filters(query, max_results, filters),
                "lexical match",
            )
        };
        Ok(results
            .into_iter()
            .map(|(b, score)| SearchResult {
                path: b.path,
                language: b.language,
                name: b.name,
                kind: b.kind,
                start_line: b.start_line as u32,
                end_line: b.end_line as u32,
                content: b.content,
                score,
                explanation: explanation.into(),
            })
            .collect())
    }
    // END_indexer_search_in_root_with_filters

    // START_CONTRACT_Indexer::hybrid_search
    // PURPOSE: Combined BM25 + vector search with deduplication
    // INPUTS: { query: &str }, { max_results: usize }
    // OUTPUTS: { anyhow::Result<Vec<SearchResult>> }
    // START_indexer_hybrid_search
    pub async fn hybrid_search(
        &self,
        query: &str,
        max_results: usize,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let root = std::env::current_dir()?;
        self.hybrid_search_in_root(&root, query, max_results).await
    }
    // END_indexer_hybrid_search

    // START_CONTRACT_Indexer::hybrid_search_in_root
    // PURPOSE: Combined BM25 + vector search with graph proximity for an explicit project root
    // INPUTS: { root: &Path }, { query: &str }, { max_results: usize }
    // OUTPUTS: { anyhow::Result<Vec<SearchResult>> }
    // START_indexer_hybrid_search_in_root
    pub async fn hybrid_search_in_root(
        &self,
        root: &Path,
        query: &str,
        max_results: usize,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let storage_guard = self.get_storage(root)?;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("index storage unavailable after initialization"))?;
        ensure_storage_ready(storage)?;

        // Get BM25 results
        let bm25 = storage.search_with_scores(query, max_results * 2);
        // Get semantic vectors when configured, otherwise deterministic n-gram fallback.
        let vector = if let Some(query_embedding) = self.embed_query_for_search(query) {
            let semantic = storage.embedding_vector_search(
                &query_embedding.vector,
                &query_embedding.model_id,
                query_embedding.dimensions,
                max_results * 2,
            );
            if semantic.is_empty() {
                tracing::warn!(
                    "[Indexer][hybrid_search_in_root][EMBEDDING_FALLBACK] no compatible block embeddings; using ngram vector fallback"
                );
                storage.vector_search(query, max_results * 2)
            } else {
                semantic
            }
        } else {
            storage.vector_search(query, max_results * 2)
        };

        // Combine and deduplicate
        let mut combined: std::collections::HashMap<
            String,
            (f64, &crate::indexer::storage::StoredBlock),
        > = std::collections::HashMap::new();
        for (b, score) in &bm25 {
            combined.entry(b.id.clone()).or_insert((*score, b));
            if let Some(e) = combined.get_mut(&b.id) {
                e.0 = e.0.max(*score);
            }
        }
        for (b, score) in &vector {
            let entry = combined.entry(b.id.clone()).or_insert((*score, b));
            entry.0 += score * 0.5; // Add vector signal to existing
        }

        let mut results: Vec<_> = combined.into_values().collect();
        let graph = GraphBuilder::build(root).ok();
        let query_words: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|w| w.to_string())
            .collect();
        let mut explanations: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        if let Some(graph) = &graph {
            for (score, block) in &mut results {
                if let Some((boost, explanation)) =
                    graph_proximity_boost(graph, block, &query_words)
                {
                    *score += boost;
                    explanations.insert(block.id.clone(), explanation);
                }
            }
        }
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(max_results);

        Ok(results
            .into_iter()
            .map(|(score, b)| SearchResult {
                path: b.path.clone(),
                language: b.language.clone(),
                name: b.name.clone(),
                kind: b.kind.clone(),
                start_line: b.start_line as u32,
                end_line: b.end_line as u32,
                content: b.content.clone(),
                score,
                explanation: explanations
                    .remove(&b.id)
                    .unwrap_or_else(|| "lexical/vector match".into()),
            })
            .collect())
    }
    // END_indexer_hybrid_search_in_root

    // START_CONTRACT_Indexer::view_signatures
    // PURPOSE: Parse and return function/class signatures from a file
    // INPUTS: { path_str: &str — file path }
    // OUTPUTS: { anyhow::Result<Vec<String>> — list of signature strings }
    // START_indexer_view_signatures
    pub async fn view_signatures(&self, path_str: &str) -> anyhow::Result<Vec<String>> {
        let full_path = std::env::current_dir()?.join(path_str);
        let code = tokio::fs::read_to_string(&full_path).await?;
        let parser = parser::ParserEngine::new();
        let ext = full_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let lang = match ext {
            "rs" => "rust",
            "py" => "python",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" | "mjs" | "cjs" => "javascript",
            "go" => "go",
            _ => return Ok(fallback_signatures(&code)),
        };
        let blocks = parser.parse(&code, lang);
        if blocks.is_empty() {
            return Ok(fallback_signatures(&code));
        }
        Ok(blocks
            .iter()
            .map(|b| {
                if b.content.len() > SIGNATURE_CONTENT_PREVIEW_LIMIT {
                    format!(
                        "{}:{} — {} ({} lines)",
                        b.name,
                        b.start_line,
                        b.kind,
                        b.end_line - b.start_line + 1
                    )
                } else {
                    format!(
                        "{}:{} — {}",
                        b.name,
                        b.start_line,
                        b.content.lines().next().unwrap_or("")
                    )
                }
            })
            .collect())
    }
}
// END_indexer_view_signatures

// START_CONTRACT_ensure_storage_ready
// PURPOSE: Fail search operations with an actionable message when storage loaded from a corrupted index
// INPUTS: { storage: &Storage }
// OUTPUTS: { anyhow::Result<()> }
// START_ensure_storage_ready
fn ensure_storage_ready(storage: &Storage) -> anyhow::Result<()> {
    if let Some(error) = storage.load_error() {
        anyhow::bail!(
            "index storage is unreadable: {}. Run `syn index` to rebuild.",
            error
        );
    }
    Ok(())
}
// END_ensure_storage_ready

// START_CONTRACT_normalize_delta_paths
// PURPOSE: Normalize changed or deleted file paths to project-relative storage paths
// INPUTS: { root: &Path — project root }, { paths: &[PathBuf] — changed or deleted paths }
// OUTPUTS: { anyhow::Result<BTreeSet<String>> }
// START_normalize_delta_paths
fn normalize_delta_paths(root: &Path, paths: &[PathBuf]) -> anyhow::Result<BTreeSet<String>> {
    let mut normalized = BTreeSet::new();
    for path in paths {
        normalized.insert(normalize_delta_path(root, path)?);
    }
    Ok(normalized)
}
// END_normalize_delta_paths

// START_CONTRACT_normalize_delta_path
// PURPOSE: Convert one changed or deleted path into the project-relative storage path format
// INPUTS: { root: &Path — project root }, { path: &Path — changed or deleted path }
// OUTPUTS: { anyhow::Result<String> }
// START_normalize_delta_path
fn normalize_delta_path(root: &Path, path: &Path) -> anyhow::Result<String> {
    let relative = if path.is_absolute() {
        path.strip_prefix(root).unwrap_or(path)
    } else {
        path
    };
    let normalized = relative.to_string_lossy().replace('\\', "/");
    if normalized.is_empty() || normalized == "." {
        anyhow::bail!("index delta path must name a file");
    }
    Ok(normalized)
}
// END_normalize_delta_path

fn graph_proximity_boost(
    graph: &crate::graphrag::types::CodeGraph,
    block: &storage::StoredBlock,
    query_words: &[String],
) -> Option<(f64, String)> {
    let node = graph
        .nodes()
        .iter()
        .find(|node| node.id == block.id || node.name == block.name)?;
    let mut best_boost = 0.0_f64;
    let mut reason = String::new();
    for relationship in graph.relationships() {
        if relationship.source_id != node.id && relationship.target_id != node.id {
            continue;
        }
        let other_id = if relationship.source_id == node.id {
            &relationship.target_id
        } else {
            &relationship.source_id
        };
        let Some(other_node) = graph
            .nodes()
            .iter()
            .find(|candidate| candidate.id == *other_id)
        else {
            continue;
        };
        let other_name = other_node.name.to_lowercase();
        if query_words.iter().any(|query| other_name.contains(query)) {
            let boost = relationship.weight * 0.75;
            if boost > best_boost {
                best_boost = boost;
                reason = format!("graph proximity via {}", relationship.relation_type.label());
            }
        }
        for second_hop in graph.relationships() {
            if second_hop.source_id != *other_id && second_hop.target_id != *other_id {
                continue;
            }
            let second_id = if second_hop.source_id == *other_id {
                &second_hop.target_id
            } else {
                &second_hop.source_id
            };
            let Some(second_node) = graph
                .nodes()
                .iter()
                .find(|candidate| candidate.id == *second_id)
            else {
                continue;
            };
            let second_name = second_node.name.to_lowercase();
            if query_words.iter().any(|query| second_name.contains(query)) {
                let boost = (relationship.weight + second_hop.weight) * 0.35;
                if boost > best_boost {
                    best_boost = boost;
                    reason = format!(
                        "multi-hop graph proximity via {} → {}",
                        relationship.relation_type.label(),
                        second_hop.relation_type.label()
                    );
                }
            }
        }
    }
    if best_boost > 0.0 {
        Some((best_boost, reason))
    } else {
        None
    }
}

fn fallback_signatures(code: &str) -> Vec<String> {
    let mut sigs = Vec::new();
    for line in code.lines() {
        let t = line.trim();
        if ["fn ", "pub fn ", "def ", "class ", "function "]
            .iter()
            .any(|k| t.starts_with(k))
        {
            sigs.push(line.to_string());
        }
    }
    sigs
}

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_test_search_reports_poisoned_storage_lock
    // PURPOSE: Verify search returns an actionable error instead of panicking when the storage lock is poisoned
    // START_test_search_reports_poisoned_storage_lock
    #[tokio::test]
    async fn test_search_reports_poisoned_storage_lock() {
        let indexer = Indexer::new(&Config::default());
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = indexer.storage.write().unwrap();
            panic!("poison indexer storage lock");
        }))
        .is_err();
        std::panic::set_hook(previous_hook);
        assert!(poisoned);

        let err = match indexer.search("anything", 1).await {
            Ok(_) => panic!("search should report poisoned storage lock"),
            Err(e) => e.to_string(),
        };
        assert!(
            err.contains("indexer storage lock poisoned"),
            "unexpected error: {}",
            err
        );
    }
    #[tokio::test]
    async fn test_hybrid_search_uses_graph_call_edges_for_related_results() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).expect("create src");
        std::fs::write(
            src.join("workflow.rs"),
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-WORKFLOW\n",
                "// PURPOSE: Workflow module\n",
                "// SCOPE: Graph-aware hybrid search test\n",
                "// DEPENDS: N/A\n",
                "// LINKS:\n",
                "\n",
                "// START_MODULE_MAP\n",
                "// first_step — first step\n",
                "// second_step — second step\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 — Initial]\n",
                "// END_CHANGE_SUMMARY\n",
                "\n",
                "// START_CONTRACT_first_step\n",
                "// PURPOSE: First step\n",
                "// START_first_step\n",
                "pub fn first_step() { second_step(); }\n",
                "// END_first_step\n",
                "\n",
                "// START_CONTRACT_second_step\n",
                "// PURPOSE: Second step\n",
                "// START_second_step\n",
                "pub fn second_step() {}\n",
                "// END_second_step\n",
            ),
        )
        .expect("write source");
        let indexer = Indexer::new(&Config::default());
        indexer.index_directory(dir.path()).await.expect("index");
        let results = indexer
            .hybrid_search_in_root(dir.path(), "first_step", 5)
            .await
            .expect("search");
        assert!(results
            .iter()
            .any(|result| result.name.contains("second_step")));
    }

    #[tokio::test]
    async fn test_search_expands_module_identifier_terms() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).expect("create src");
        std::fs::write(
            src.join("workflow.rs"),
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-WORKFLOW\n",
                "// PURPOSE: Workflow module\n",
                "// SCOPE: Query expansion search test\n",
                "// DEPENDS: N/A\n",
                "// LINKS:\n",
                "\n",
                "// START_MODULE_MAP\n",
                "// first_step — first step\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 — Initial]\n",
                "// END_CHANGE_SUMMARY\n",
                "\n",
                "// START_CONTRACT_first_step\n",
                "// PURPOSE: First step\n",
                "// START_first_step\n",
                "pub fn first_step() {}\n",
                "// END_first_step\n",
            ),
        )
        .expect("write source");
        let indexer = Indexer::new(&Config::default());
        indexer.index_directory(dir.path()).await.expect("index");
        let results = indexer
            .search_in_root(dir.path(), "runner workflow M-WORKFLOW", 5)
            .await
            .expect("search");
        assert!(results
            .iter()
            .any(|result| result.name.contains("first_step")));
    }

    // START_CONTRACT_test_search_in_root_with_filters_applies_metadata
    // PURPOSE: Verify explicit-root index searches apply SearchFilters before returning results
    // START_test_search_in_root_with_filters_applies_metadata
    #[tokio::test]
    async fn test_search_in_root_with_filters_applies_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        let tests = dir.path().join("tests");
        std::fs::create_dir_all(&src).expect("create src");
        std::fs::create_dir_all(&tests).expect("create tests");
        std::fs::write(src.join("workflow.rs"), "pub fn shared_login() {}\n").expect("write src");
        std::fs::write(
            tests.join("workflow_test.rs"),
            "pub fn shared_login_test() {}\n",
        )
        .expect("write tests");

        let indexer = Indexer::new(&Config::default());
        indexer.index_directory(dir.path()).await.expect("index");
        let filters = SearchFilters::new(None, Some("src".into()), None);
        let results = indexer
            .search_in_root_with_filters(dir.path(), "shared_login", 10, &filters)
            .await
            .expect("search");

        assert!(!results.is_empty());
        assert!(results.iter().all(|result| result.path.starts_with("src/")));
    }
    // END_test_search_in_root_with_filters_applies_metadata

    // START_CONTRACT_test_index_delta_upserts_changed_files
    // PURPOSE: Verify incremental indexing replaces changed-file blocks without a full rebuild
    // START_test_index_delta_upserts_changed_files
    #[tokio::test]
    async fn test_index_delta_upserts_changed_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).expect("create src");
        let path = src.join("workflow.rs");
        std::fs::write(&path, "pub fn old_step() {\n}\n").expect("write old source");

        let indexer = Indexer::new(&Config::default());
        indexer.index_directory(dir.path()).await.expect("index");
        assert!(!indexer
            .search_in_root(dir.path(), "old", 5)
            .await
            .expect("search old")
            .is_empty());

        std::fs::write(&path, "pub fn new_step() {\n}\n").expect("write new source");
        indexer
            .index_delta(dir.path(), &[PathBuf::from("src/workflow.rs")], &[])
            .await
            .expect("index delta");

        assert!(indexer
            .search_in_root(dir.path(), "old", 5)
            .await
            .expect("search old after delta")
            .is_empty());
        assert!(!indexer
            .search_in_root(dir.path(), "new", 5)
            .await
            .expect("search new after delta")
            .is_empty());
    }
    // END_test_index_delta_upserts_changed_files

    // START_CONTRACT_test_index_delta_removes_deleted_files
    // PURPOSE: Verify incremental indexing removes deleted-file blocks while preserving unrelated files
    // START_test_index_delta_removes_deleted_files
    #[tokio::test]
    async fn test_index_delta_removes_deleted_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).expect("create src");
        let deleted_path = src.join("deleted.rs");
        let keep_path = src.join("keep.rs");
        std::fs::write(&deleted_path, "pub fn deleted_step() {\n}\n").expect("write deleted");
        std::fs::write(&keep_path, "pub fn keep_step() {\n}\n").expect("write keep");

        let indexer = Indexer::new(&Config::default());
        indexer.index_directory(dir.path()).await.expect("index");
        std::fs::remove_file(&deleted_path).expect("remove deleted");
        indexer
            .index_delta(dir.path(), &[], &[PathBuf::from("src/deleted.rs")])
            .await
            .expect("index deleted delta");

        assert!(indexer
            .search_in_root(dir.path(), "deleted", 5)
            .await
            .expect("search deleted")
            .is_empty());
        assert!(!indexer
            .search_in_root(dir.path(), "keep", 5)
            .await
            .expect("search keep")
            .is_empty());
    }
    // END_test_index_delta_removes_deleted_files

    #[tokio::test]
    async fn test_hybrid_search_uses_multi_hop_graph_proximity() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).expect("create src");
        std::fs::write(
            src.join("workflow.rs"),
            concat!(
                "// MODULE_CONTRACT\n",
                "// MODULE_ID: M-WORKFLOW\n",
                "// PURPOSE: Workflow module\n",
                "// SCOPE: Multi-hop graph-aware hybrid search test\n",
                "// DEPENDS: N/A\n",
                "// LINKS:\n",
                "\n",
                "// START_MODULE_MAP\n",
                "// first_step — first step\n",
                "// second_step — second step\n",
                "// third_step — third step\n",
                "// END_MODULE_MAP\n",
                "\n",
                "// START_CHANGE_SUMMARY\n",
                "// LAST_CHANGE: [v1.0.0 — Initial]\n",
                "// END_CHANGE_SUMMARY\n",
                "\n",
                "// START_CONTRACT_first_step\n",
                "// PURPOSE: First step\n",
                "// START_first_step\n",
                "pub fn first_step() { second_step(); }\n",
                "// END_first_step\n",
                "\n",
                "// START_CONTRACT_second_step\n",
                "// PURPOSE: Second step\n",
                "// START_second_step\n",
                "pub fn second_step() { third_step(); }\n",
                "// END_second_step\n",
                "\n",
                "// START_CONTRACT_third_step\n",
                "// PURPOSE: Third step\n",
                "// START_third_step\n",
                "pub fn third_step() {}\n",
                "// END_third_step\n",
            ),
        )
        .expect("write source");
        let indexer = Indexer::new(&Config::default());
        indexer.index_directory(dir.path()).await.expect("index");
        let results = indexer
            .hybrid_search_in_root(dir.path(), "third_step", 10)
            .await
            .expect("search");
        assert!(results.iter().any(
            |result| result.name.contains("second_step") || result.name.contains("first_step")
        ));
    }

    // END_test_search_reports_poisoned_storage_lock
}
// END_public_api
