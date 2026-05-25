// MODULE_CONTRACT
// MODULE_ID: M-INDEXER-STORAGE-SEARCH
// PURPOSE: Search scoring helpers for indexer block storage
// SCOPE: BM25-style scoring, tokenization, n-gram vectorization, cosine similarity
// DEPENDS: M-INDEXER-STORAGE-TYPES
// LINKS: docs/modules/M-INDEXER-STORAGE.xml, docs/modules/M-INDEXER-STORAGE-TYPES.xml

// START_MODULE_MAP
// block_matches_filters — Checks optional search filters before scoring
// score_block — Computes text relevance score for a stored block
// expand_query_terms — Builds expanded query vocabulary from identifiers and module/file hints
// tokenize — Splits identifiers and paths into searchable tokens
// ngram_vectorize — Builds normalized 3-gram vectors
// cosine_similarity — Computes vector dot-product similarity
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.7.0 - Added pre-ranking search filter matching]
// END_CHANGE_SUMMARY

use super::storage_types::{SearchFilters, StoredBlock};
use std::collections::HashMap;

const MIN_QUERY_WORD_CHARS: usize = 2;
const MIN_NGRAM_CHARS: usize = 3;
const SHORT_BLOCK_TOKEN_BONUS_LIMIT: f64 = 500.0;
const SHORT_BLOCK_TOKEN_BONUS_DIVISOR: f64 = 1000.0;

// START_public_api

// START_CONTRACT_block_matches_filters
// PURPOSE: Check whether a stored block is in scope for optional search filters before scoring
// INPUTS: { block: &StoredBlock }, { filters: &SearchFilters }
// OUTPUTS: { bool }
// START_block_matches_filters
pub(crate) fn block_matches_filters(block: &StoredBlock, filters: &SearchFilters) -> bool {
    filters.is_empty() || filters.matches(block)
}
// END_block_matches_filters

// START_CONTRACT_score_block
// PURPOSE: Score a stored block against a normalized query using name, path, and content signals
// INPUTS: { block: &StoredBlock }, { query_lower: &str }, { query_words: &[&str] }
// OUTPUTS: { f64 }
// START_score_block
pub(crate) fn score_block(block: &StoredBlock, query_lower: &str, query_words: &[&str]) -> f64 {
    let name_lower = block.name.to_lowercase();
    let content_lower = block.content.to_lowercase();
    let path_lower = block.path.to_lowercase();
    let mut score = 0.0_f64;

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
        if word.is_empty() || word.len() < MIN_QUERY_WORD_CHARS {
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

            if wt.len() >= MIN_NGRAM_CHARS {
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
    if total_content_tokens < SHORT_BLOCK_TOKEN_BONUS_LIMIT && score > 0.0 {
        score *= 1.0
            + (SHORT_BLOCK_TOKEN_BONUS_LIMIT - total_content_tokens).max(0.0)
                / SHORT_BLOCK_TOKEN_BONUS_DIVISOR;
    }
    score
}
// END_score_block

// START_CONTRACT_expand_query_terms
// PURPOSE: Expand query text into normalized identifier and module/file hint terms
// INPUTS: { query: &str }
// OUTPUTS: { Vec<String> }
// START_expand_query_terms
pub(crate) fn expand_query_terms(query: &str) -> Vec<String> {
    let mut expanded = Vec::new();
    let lower = query.to_lowercase();
    for raw in lower.split_whitespace() {
        expanded.push(raw.to_string());
        for token in tokenize(raw) {
            if !expanded.contains(&token) {
                expanded.push(token.clone());
            }
            if let Some(stripped) = token.strip_prefix("m-") {
                let stripped = stripped.to_string();
                if !expanded.contains(&stripped) {
                    expanded.push(stripped);
                }
            }
            if token.ends_with(".rs")
                || token.ends_with(".py")
                || token.ends_with(".ts")
                || token.ends_with(".js")
            {
                let file_stem = token.split('.').next().unwrap_or(&token).to_string();
                if !expanded.contains(&file_stem) {
                    expanded.push(file_stem);
                }
            }
        }
    }
    expanded
}
// END_expand_query_terms

// START_CONTRACT_tokenize
// PURPOSE: Split code identifiers, paths, and punctuation-delimited text into normalized tokens
// INPUTS: { text: &str }
// OUTPUTS: { Vec<String> }
// START_tokenize
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
// END_tokenize

// START_CONTRACT_ngram_vectorize
// PURPOSE: Build a normalized character 3-gram vector for approximate text matching
// INPUTS: { text: &str }
// OUTPUTS: { HashMap<u64, f64> }
// START_ngram_vectorize
pub(crate) fn ngram_vectorize(text: &str) -> HashMap<u64, f64> {
    let chars: Vec<char> = text.to_lowercase().chars().collect();
    if chars.len() < MIN_NGRAM_CHARS {
        return HashMap::new();
    }
    let mut vec = HashMap::new();
    for i in 0..chars.len().saturating_sub(2) {
        let hash = ((chars[i] as u64) << 16) | ((chars[i + 1] as u64) << 8) | (chars[i + 2] as u64);
        *vec.entry(hash).or_insert(0.0) += 1.0;
    }
    let norm: f64 = vec.values().map(|v| v * v).sum::<f64>().sqrt();
    if norm > 0.0 {
        for v in vec.values_mut() {
            *v /= norm;
        }
    }
    vec
}
// END_ngram_vectorize

// START_CONTRACT_cosine_similarity
// PURPOSE: Compute cosine similarity for already normalized sparse n-gram vectors
// INPUTS: { a: &HashMap<u64, f64> }, { b: &HashMap<u64, f64> }
// OUTPUTS: { f64 }
// START_cosine_similarity
pub(crate) fn cosine_similarity(a: &HashMap<u64, f64>, b: &HashMap<u64, f64>) -> f64 {
    let mut dot = 0.0;
    for (k, va) in a {
        if let Some(vb) = b.get(k) {
            dot += va * vb;
        }
    }
    dot
}
// END_cosine_similarity

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_query_terms_adds_identifier_variants() {
        let terms = expand_query_terms("M-RUNNER start_run.rs");
        assert!(terms.contains(&"m-runner".to_string()));
        assert!(terms.contains(&"runner".to_string()));
        assert!(
            terms.contains(&"start_run.rs".to_string()) || terms.contains(&"start_run".to_string())
        );
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
