pub mod parser;
pub mod storage;
pub mod walker;

use crate::config::Config;
use std::path::Path;
use std::sync::Mutex;
use storage::Storage;

pub struct SearchResult {
    pub path: String,
    pub language: String,
    pub name: String,
    pub kind: String,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
    pub score: f64,
}

pub struct Indexer {
    pub config: Config,
    pub storage: Mutex<Option<Storage>>,
}

impl Indexer {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            storage: Mutex::new(None),
        }
    }

    #[allow(clippy::needless_lifetimes)]
    fn get_storage(&self, root: &Path) -> std::sync::MutexGuard<'_, Option<Storage>> {
        let mut guard = self.storage.lock().unwrap();
        if guard.is_none() {
            *guard = Some(Storage::new(root));
        }
        guard
    }

    pub async fn index_directory(&self, root: &Path) -> anyhow::Result<()> {
        let walker = walker::Walker::new(root);
        let files = walker.walk();
        let total = files.len();
        let parser = parser::ParserEngine::new();

        tracing::info!("Indexing {} files in {}", total, root.display());

        let mut storage = self.get_storage(root);
        let storage = storage.as_mut().unwrap();

        for (i, file) in files.iter().enumerate() {
            let full_path = root.join(&file.path);
            let code = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            if code.len() > 100_000 {
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

            storage.store_blocks(stored)?;

            if (i + 1) % 50 == 0 || i == total - 1 {
                tracing::info!("  indexed {}/{} files", i + 1, total);
            }
        }

        tracing::info!(
            "Index complete: {} blocks from {} files",
            storage.count(),
            total
        );
        Ok(())
    }

    pub async fn search(
        &self,
        query: &str,
        max_results: usize,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let root = std::env::current_dir()?;
        let mut guard = self.storage.lock().unwrap();
        if guard.is_none() {
            *guard = Some(crate::indexer::storage::Storage::new(&root));
        }
        let storage = guard.as_ref().unwrap();
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
            })
            .collect())
    }

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
                if b.content.len() > 200 {
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
