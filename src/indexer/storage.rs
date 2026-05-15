use std::path::{Path, PathBuf};

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

pub struct Storage {
    db_path: PathBuf,
    blocks: Vec<StoredBlock>,
}

impl Storage {
    pub fn new(project_root: &Path) -> Self {
        let hash = simple_hash(project_root);
        let db_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("synapse")
            .join("index")
            .join(hash);
        std::fs::create_dir_all(&db_dir).unwrap_or_default();
        let db_path = db_dir.join("blocks.json");
        let blocks = if db_path.exists() {
            std::fs::read_to_string(&db_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Self { db_path, blocks }
    }

    pub fn count(&self) -> usize {
        self.blocks.len()
    }

    pub fn all_blocks(&self) -> &[StoredBlock] {
        &self.blocks
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn store_blocks(&mut self, new_blocks: Vec<StoredBlock>) -> anyhow::Result<()> {
        let mut seen_files: std::collections::HashSet<String> = std::collections::HashSet::new();
        for b in &new_blocks {
            seen_files.insert(b.path.clone());
        }
        self.blocks.retain(|b| !seen_files.contains(&b.path));
        self.blocks.extend(new_blocks);
        self.flush()
    }

    pub fn flush(&self) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(&self.blocks)?;
        std::fs::write(&self.db_path, &json)?;
        Ok(())
    }

    pub fn search(&self, query: &str, max_results: usize) -> Vec<StoredBlock> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(f64, &StoredBlock)> = self.blocks
            .iter()
            .filter_map(|b| {
                let score = score_block(b, &query_lower, &query_words);
                if score > 0.0 { Some((score, b)) } else { None }
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(max_results);
        scored.into_iter().map(|(_, b)| b.clone()).collect()
    }
}

fn score_block(block: &StoredBlock, query_lower: &str, query_words: &[&str]) -> f64 {
    let name_lower = block.name.to_lowercase();
    let content_lower = block.content.to_lowercase();
    let path_lower = block.path.to_lowercase();
    let mut score = 0.0;

    // Exact name match (highest priority)
    if name_lower.contains(query_lower) {
        score += 15.0;
        return score;
    }

    let total_words = content_lower.split_whitespace().count() as f64;

    for word in query_words {
        if word.is_empty() || word.len() < 2 { continue; }

        // Name match (strong signal)
        if name_lower.contains(word) { score += 5.0; }
        // Path match
        if path_lower.contains(word) { score += 3.0; }

        // BM25-like term frequency with inverse document frequency heuristic
        let tf = content_lower.matches(word).count() as f64;
        if tf > 0.0 {
            let bm25 = tf * (2.2) / (tf + 1.2 * (1.0 - 0.75 + 0.75 * total_words / 100.0));
            score += bm25;
        }
    }

    // Path prefix bonus
    if path_lower.contains(query_lower) { score += 2.0; }
    score
}

fn simple_hash(path: &Path) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
