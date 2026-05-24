// MODULE_CONTRACT
// MODULE_ID: M-INDEXER
// PURPOSE: Code indexer — walks, parses, stores full snapshots, and searches code blocks with guarded storage health checks
// SCOPE: Indexer struct, SearchResult, guarded storage locks, index_directory, gitignore-aware indexing, stale-entry pruning, search, hybrid_search, storage health propagation, view_signatures
// DEPENDS: M-INDEXER-WALKER, M-INDEXER-PARSER, M-INDEXER-STORAGE, M-INDEXER-STORAGE-SEARCH, M-INDEXER-STORAGE-TYPES, M-CONFIG
// LINKS: N/A

// START_MODULE_MAP
// SearchResult — Search result with path, language, name, kind, lines, content, score
// Indexer — Main indexer combining walker, parser, and storage
// Indexer::storage_count — Returns loaded storage count through guarded lock access
// Indexer::index_directory_with_gitignore — Rebuilds index snapshot with configurable gitignore handling
// ensure_storage_ready — Converts storage load-health errors into actionable search errors
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.2.0 - Named index size and signature preview thresholds]
// END_CHANGE_SUMMARY

pub mod parser;
pub mod storage;
mod storage_search;
mod storage_types;
pub mod walker;

use crate::config::Config;
use crate::graphrag::builder::GraphBuilder;
use std::path::Path;
use std::sync::RwLock;
use storage::Storage;

const MAX_INDEXABLE_FILE_BYTES: usize = 100_000;
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
        let parser = parser::ParserEngine::new();

        tracing::info!("Indexing {} files in {}", total, root.display());

        let mut storage_guard = self.get_storage(root)?;
        let storage = storage_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("index storage unavailable after initialization"))?;

        let mut all_stored: Vec<storage::StoredBlock> = Vec::new();
        for (i, file) in files.iter().enumerate() {
            let full_path = root.join(&file.path);
            let code = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            if code.len() > MAX_INDEXABLE_FILE_BYTES {
                tracing::debug!("Skipping large file: {} ({} bytes)", file.path, code.len());
                continue;
            }

            let blocks = parser.parse(&code, &file.language);

            let stored: Vec<storage::StoredBlock> = blocks
                .iter()
                .map(|b| storage::StoredBlock {
                    id: format!("{}:{}", file.path, b.start_line),
                    path: file.path.clone(),
                    language: file.language.clone(),
                    name: b.name.clone(),
                    kind: b.kind.clone(),
                    content: b.content.clone(),
                    start_line: b.start_line,
                    end_line: b.end_line,
                })
                .collect();

            all_stored.extend(stored);

            if (i + 1) % 50 == 0 || i == total - 1 {
                tracing::info!("  indexed {}/{} files", i + 1, total);
            }
        }

        storage.replace_all_blocks(all_stored)?;
        tracing::info!(
            "Index complete: {} blocks from {} files",
            storage.count(),
            total
        );
        Ok(())
    }
    // END_indexer_index_directory_with_gitignore

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
        let storage_guard = self.get_storage(&root)?;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("index storage unavailable after initialization"))?;
        ensure_storage_ready(storage)?;
        let results = storage.search_with_scores(query, max_results);
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
                explanation: "lexical match".into(),
            })
            .collect())
    }
    // END_indexer_search

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
        let storage_guard = self.get_storage(&root)?;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("index storage unavailable after initialization"))?;
        ensure_storage_ready(storage)?;

        // Get BM25 results
        let bm25 = storage.search_with_scores(query, max_results * 2);
        // Get vector results
        let vector = storage.vector_search(query, max_results * 2);

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
        let graph = GraphBuilder::build(&root).ok();
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
    // END_indexer_hybrid_search

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

fn graph_proximity_boost(
    graph: &crate::graphrag::types::CodeGraph,
    block: &storage::StoredBlock,
    query_words: &[String],
) -> Option<(f64, String)> {
    let node = graph
        .nodes
        .iter()
        .find(|node| node.id == block.id || node.name == block.name)?;
    let mut best_boost = 0.0_f64;
    let mut reason = String::new();
    for relationship in &graph.relationships {
        if relationship.source_id != node.id && relationship.target_id != node.id {
            continue;
        }
        let other_id = if relationship.source_id == node.id {
            &relationship.target_id
        } else {
            &relationship.source_id
        };
        let Some(other_node) = graph
            .nodes
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
        for second_hop in &graph.relationships {
            if second_hop.source_id != *other_id && second_hop.target_id != *other_id {
                continue;
            }
            let second_id = if second_hop.source_id == *other_id {
                &second_hop.target_id
            } else {
                &second_hop.source_id
            };
            let Some(second_node) = graph
                .nodes
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
        let _cwd = crate::utils::test_cwd_lock().lock().await;
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
        let old_cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("set cwd");
        let indexer = Indexer::new(&Config::default());
        indexer.index_directory(dir.path()).await.expect("index");
        let results = indexer
            .hybrid_search("first_step", 5)
            .await
            .expect("search");
        std::env::set_current_dir(old_cwd).expect("restore cwd");
        assert!(results
            .iter()
            .any(|result| result.name.contains("second_step")));
    }

    #[tokio::test]
    async fn test_search_expands_module_identifier_terms() {
        let _cwd = crate::utils::test_cwd_lock().lock().await;
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
        let old_cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("set cwd");
        let indexer = Indexer::new(&Config::default());
        indexer.index_directory(dir.path()).await.expect("index");
        let results = indexer
            .search("runner workflow M-WORKFLOW", 5)
            .await
            .expect("search");
        std::env::set_current_dir(old_cwd).expect("restore cwd");
        assert!(results
            .iter()
            .any(|result| result.name.contains("first_step")));
    }

    #[tokio::test]
    async fn test_hybrid_search_uses_multi_hop_graph_proximity() {
        let _cwd = crate::utils::test_cwd_lock().lock().await;
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
        let old_cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("set cwd");
        let indexer = Indexer::new(&Config::default());
        indexer.index_directory(dir.path()).await.expect("index");
        let results = indexer
            .hybrid_search("third_step", 10)
            .await
            .expect("search");
        std::env::set_current_dir(old_cwd).expect("restore cwd");
        assert!(results.iter().any(
            |result| result.name.contains("second_step") || result.name.contains("first_step")
        ));
    }

    // END_test_search_reports_poisoned_storage_lock
}
// END_public_api
