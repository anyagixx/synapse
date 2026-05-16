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
        if query.trim().is_empty() {
            return Vec::new();
        }
        self.search_with_scores(query, max_results)
            .into_iter()
            .map(|(b, _)| b)
            .collect()
    }

    pub fn search_with_scores(&self, query: &str, max_results: usize) -> Vec<(StoredBlock, f64)> {
        let query_lower = query.to_lowercase();
        if query_lower.trim().is_empty() {
            return Vec::new();
        }
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(f64, StoredBlock)> = self
            .blocks
            .iter()
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

    pub fn vector_search(&self, query: &str, max_results: usize) -> Vec<(StoredBlock, f64)> {
        let query_vec = ngram_vectorize(query);
        if query_vec.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(f64, StoredBlock)> = self
            .blocks
            .iter()
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
}

fn score_block(block: &StoredBlock, query_lower: &str, query_words: &[&str]) -> f64 {
    let name_lower = block.name.to_lowercase();
    let content_lower = block.content.to_lowercase();
    let path_lower = block.path.to_lowercase();
    let mut score = 0.0_f64;

    // Exact name match (highest priority)
    if name_lower == *query_lower {
        return 50.0;
    }

    if name_lower.starts_with(query_lower) {
        score += 25.0;
    } else if name_lower.contains(query_lower) {
        score += 18.0;
    }

    let name_tokens = tokenize(&name_lower);
    let path_tokens = tokenize(&path_lower);
    let content_tokens = tokenize(&content_lower);
    let total_content_tokens = content_tokens.len() as f64;
    if total_content_tokens < 1.0 {
        return score;
    }

    let avg_block_len = 200.0_f64;

    for word in query_words {
        if word.is_empty() || word.len() < 2 {
            continue;
        }
        let word_tokens = tokenize(word);
        if word_tokens.is_empty() {
            continue;
        }

        let name_matches = name_tokens
            .iter()
            .filter(|t| word_tokens.iter().any(|wt| t.contains(wt)))
            .count();
        if name_matches > 0 {
            score += 6.0 * name_matches as f64;
        }

        let path_matches = path_tokens
            .iter()
            .filter(|t| word_tokens.iter().any(|wt| t.contains(wt)))
            .count();
        if path_matches > 0 {
            score += 3.5 * path_matches as f64;
        }

        for wt in &word_tokens {
            let tf = content_tokens.iter().filter(|t| t.contains(wt)).count() as f64;
            if tf > 0.0 {
                let dl_ratio = total_content_tokens / avg_block_len;
                let k1 = 1.5;
                let b = 0.75;
                let bm25 = tf * (k1 + 1.0) / (tf + k1 * (1.0 - b + b * dl_ratio));
                score += bm25 * 2.5;

                let first_lines: Vec<&str> = content_lower.lines().take(5).collect();
                if first_lines.iter().any(|l| l.contains(wt)) {
                    score += 3.0;
                }
            }

            if wt.len() >= 3 {
                let ngram_matches = content_lower.matches(wt).count() as f64;
                if ngram_matches > 0.0 {
                    score += ngram_matches * 0.3;
                }
                for nt in &name_tokens {
                    if nt.contains(wt) {
                        score += 2.0;
                    }
                }
            }
        }
    }

    if content_lower.contains(query_lower) {
        score += 4.0;
    }
    if total_content_tokens < 500.0 && score > 0.0 {
        score *= 1.0 + (500.0 - total_content_tokens).max(0.0) / 1000.0;
    }
    score
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let parts: Vec<&str> = text
        .split(&[
            ' ', '\t', '\n', '_', '-', '.', '/', '\\', ':', ',', ';', '(', ')', '[', ']', '{', '}',
            '"', '\'', '!', '?', '=', '+', '*', '&', '|', '^', '~', '<', '>',
        ] as &[_])
        .collect();
    for part in parts {
        if part.is_empty() {
            continue;
        }
        let mut start = 0;
        for (i, c) in part.char_indices().skip(1) {
            if c.is_uppercase() {
                tokens.push(part[start..i].to_lowercase());
                start = i;
            }
        }
        tokens.push(part[start..].to_lowercase());
    }
    tokens.retain(|t| t.len() >= 2 || t.chars().all(|c| c.is_alphanumeric()));
    tokens
}

fn simple_hash(path: &Path) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn ngram_vectorize(text: &str) -> std::collections::HashMap<u64, f64> {
    let chars: Vec<char> = text.to_lowercase().chars().collect();
    if chars.len() < 3 {
        return std::collections::HashMap::new();
    }
    let mut vec = std::collections::HashMap::new();
    for i in 0..chars.len().saturating_sub(2) {
        let hash = ((chars[i] as u64) << 16) | ((chars[i + 1] as u64) << 8) | (chars[i + 2] as u64);
        *vec.entry(hash).or_insert(0.0) += 1.0;
    }
    // Normalize
    let norm: f64 = vec.values().map(|v| v * v).sum::<f64>().sqrt();
    if norm > 0.0 {
        for v in vec.values_mut() {
            *v /= norm;
        }
    }
    vec
}

fn cosine_similarity(
    a: &std::collections::HashMap<u64, f64>,
    b: &std::collections::HashMap<u64, f64>,
) -> f64 {
    let mut dot = 0.0;
    for (k, va) in a {
        if let Some(vb) = b.get(k) {
            dot += va * vb;
        }
    }
    dot
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block(id: &str, path: &str, name: &str, kind: &str, content: &str) -> StoredBlock {
        StoredBlock {
            id: id.to_string(),
            path: path.to_string(),
            language: "rust".to_string(),
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
    fn test_tokenize_camel_case() {
        let t = tokenize("MyFunctionTest");
        assert!(t.iter().any(|x| x == "my"), "expected 'my' in {:?}", t);
        assert!(
            t.iter().any(|x| x == "function"),
            "expected 'function' in {:?}",
            t
        );
        assert!(t.iter().any(|x| x == "test"), "expected 'test' in {:?}", t);
    }

    #[test]
    fn test_tokenize_snake_case() {
        let t = tokenize("snake_case_var");
        assert!(t.contains(&"snake".to_string()));
        assert!(t.contains(&"case".to_string()));
        assert!(t.contains(&"var".to_string()));
    }
}
