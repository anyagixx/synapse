// MODULE_CONTRACT
// MODULE_ID: M-INDEXER-PIPELINE
// PURPOSE: Indexing pipeline helpers — convert discovered files into deterministic parallel StoredBlock updates
// SCOPE: Rayon-backed collect_index_blocks, process_index_file, build_stored_blocks, size-bounded source reads
// DEPENDS: M-INDEXER-WALKER, M-INDEXER-PARSER, M-INDEXER-STORAGE-TYPES
// LINKS:
//   → M-INDEXER-WALKER (depends) — source file discovery types
//   → M-INDEXER-PARSER (depends) — code block extraction
//   → M-INDEXER-STORAGE-TYPES (depends) — persisted block shape
//   → Phase-64 (implements) — UPGRADE_1 indexing pipeline foundation

// START_MODULE_MAP
// MAX_INDEXABLE_FILE_BYTES — Maximum source bytes processed by index pipeline
// collect_index_blocks — Builds deterministic stored blocks for a full file list
// process_index_file — Reads and parses one discovered source file
// build_stored_blocks — Converts parsed CodeBlocks into StoredBlock records
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.2.0 — Initializes empty embedding metadata for new stored blocks]
// END_CHANGE_SUMMARY

use super::parser::{CodeBlock, ParserEngine};
use super::storage::StoredBlock;
use super::walker::IndexFile;
use rayon::prelude::*;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const MAX_INDEXABLE_FILE_BYTES: usize = 100_000;

// START_public_api

// START_CONTRACT_collect_index_blocks
// PURPOSE: Convert discovered files into a deterministic stored block snapshot using parallel file processing
// INPUTS: { root: &Path — project root }, { files: &[IndexFile] — files discovered by walker }
// OUTPUTS: { Vec<StoredBlock> — sorted stored blocks }
// SIDE_EFFECTS: reads source files from disk and emits indexing progress logs
// START_collect_index_blocks
pub fn collect_index_blocks(root: &Path, files: &[IndexFile]) -> Vec<StoredBlock> {
    let total = files.len();
    tracing::info!(
        "[IndexerPipeline][collect_index_blocks][PARALLEL_START] processing {} files",
        total
    );
    let completed = AtomicUsize::new(0);
    let mut all_stored: Vec<StoredBlock> = files
        .par_iter()
        .filter_map(|file| {
            let stored = process_index_file(root, file);
            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            if done % 50 == 0 || done == total {
                tracing::info!(
                    "[IndexerPipeline][collect_index_blocks][PROGRESS] indexed {}/{} files",
                    done,
                    total
                );
            }
            stored
        })
        .flatten()
        .collect();

    if total == 0 {
        tracing::info!("[IndexerPipeline][collect_index_blocks][PROGRESS] indexed 0/0 files");
    }

    sort_stored_blocks(&mut all_stored);
    all_stored
}
// END_collect_index_blocks

// START_CONTRACT_collect_index_blocks_serial
// PURPOSE: Build a sequential stored block snapshot for deterministic parity tests
// INPUTS: { root: &Path — project root }, { files: &[IndexFile] — files discovered by walker }
// OUTPUTS: { Vec<StoredBlock> — sorted stored blocks }
// SIDE_EFFECTS: reads source files from disk
// START_collect_index_blocks_serial
#[cfg(test)]
fn collect_index_blocks_serial(root: &Path, files: &[IndexFile]) -> Vec<StoredBlock> {
    let mut all_stored = Vec::new();
    for file in files {
        if let Some(stored) = process_index_file(root, file) {
            all_stored.extend(stored);
        }
    }
    sort_stored_blocks(&mut all_stored);
    all_stored
}
// END_collect_index_blocks_serial

// START_CONTRACT_process_index_file
// PURPOSE: Read and parse one discovered source file into stored blocks
// INPUTS: { root: &Path — project root }, { file: &IndexFile — discovered source file }
// OUTPUTS: { Option<Vec<StoredBlock>> — parsed blocks, or None when file is unreadable or too large }
// SIDE_EFFECTS: reads one source file and emits skip logs for oversized files
// START_process_index_file
pub fn process_index_file(root: &Path, file: &IndexFile) -> Option<Vec<StoredBlock>> {
    let full_path = root.join(&file.path);
    let code = match std::fs::read_to_string(&full_path) {
        Ok(code) => code,
        Err(error) => {
            tracing::debug!(
                "[IndexerPipeline][process_index_file][READ_SKIP] skipping {}: {}",
                file.path,
                error
            );
            return None;
        }
    };

    if code.len() > MAX_INDEXABLE_FILE_BYTES {
        tracing::debug!(
            "[IndexerPipeline][process_index_file][SIZE_SKIP] skipping large file: {} ({} bytes)",
            file.path,
            code.len()
        );
        return None;
    }

    let parser = ParserEngine::new();
    let blocks = parser.parse(&code, &file.language);
    Some(build_stored_blocks(file, &blocks))
}
// END_process_index_file

// START_CONTRACT_build_stored_blocks
// PURPOSE: Convert parser CodeBlocks into persisted StoredBlock values for one source file
// INPUTS: { file: &IndexFile — source file metadata }, { blocks: &[CodeBlock] — parsed blocks }
// OUTPUTS: { Vec<StoredBlock> }
// START_build_stored_blocks
pub fn build_stored_blocks(file: &IndexFile, blocks: &[CodeBlock]) -> Vec<StoredBlock> {
    blocks
        .iter()
        .map(|block| StoredBlock {
            id: format!("{}:{}", file.path, block.start_line),
            path: file.path.clone(),
            language: file.language.clone(),
            name: block.name.clone(),
            kind: block.kind.clone(),
            content: block.content.clone(),
            start_line: block.start_line,
            end_line: block.end_line,
            embedding: None,
            embedding_model: None,
            embedding_dimensions: None,
            embedding_schema_version: None,
        })
        .collect()
}
// END_build_stored_blocks

// END_public_api

// START_CONTRACT_sort_stored_blocks
// PURPOSE: Normalize stored block ordering for deterministic snapshots
// INPUTS: { blocks: &mut Vec<StoredBlock> }
// SIDE_EFFECTS: sorts blocks in place by path, start line, and id
// START_sort_stored_blocks
fn sort_stored_blocks(blocks: &mut [StoredBlock]) {
    blocks.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.start_line.cmp(&b.start_line))
            .then(a.id.cmp(&b.id))
    });
}
// END_sort_stored_blocks

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_index_file
    // PURPOSE: Build IndexFile test data
    // INPUTS: { path: &str }, { language: &str }
    // OUTPUTS: { IndexFile }
    // START_index_file
    fn index_file(path: &str, language: &str) -> IndexFile {
        IndexFile {
            path: path.to_string(),
            language: language.to_string(),
        }
    }
    // END_index_file

    // START_CONTRACT_test_build_stored_blocks_uses_file_metadata
    // PURPOSE: Verify StoredBlock construction preserves file metadata and parser line data
    // START_test_build_stored_blocks_uses_file_metadata
    #[test]
    fn test_build_stored_blocks_uses_file_metadata() {
        let file = index_file("src/lib.rs", "rust");
        let blocks = vec![CodeBlock {
            name: "run".into(),
            kind: "function_item".into(),
            start_line: 3,
            end_line: 5,
            content: "fn run() {}".into(),
        }];

        let stored = build_stored_blocks(&file, &blocks);

        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, "src/lib.rs:3");
        assert_eq!(stored[0].path, "src/lib.rs");
        assert_eq!(stored[0].language, "rust");
        assert_eq!(stored[0].name, "run");
        assert!(stored[0].embedding.is_none());
        assert!(stored[0].embedding_model.is_none());
    }
    // END_test_build_stored_blocks_uses_file_metadata

    // START_CONTRACT_test_collect_index_blocks_sorts_snapshot
    // PURPOSE: Verify full snapshot block ordering is deterministic regardless of input file order
    // START_test_collect_index_blocks_sorts_snapshot
    #[test]
    fn test_collect_index_blocks_sorts_snapshot() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).expect("create src");
        std::fs::write(src.join("b.rs"), "fn beta() {}\n").expect("write beta");
        std::fs::write(src.join("a.rs"), "fn alpha() {}\n").expect("write alpha");

        let files = vec![
            index_file("src/b.rs", "rust"),
            index_file("src/a.rs", "rust"),
        ];
        let stored = collect_index_blocks(dir.path(), &files);
        let paths: Vec<_> = stored.iter().map(|block| block.path.as_str()).collect();

        assert_eq!(paths, vec!["src/a.rs", "src/b.rs"]);
    }
    // END_test_collect_index_blocks_sorts_snapshot

    // START_CONTRACT_test_parallel_collection_matches_serial_snapshot
    // PURPOSE: Verify Rayon collection produces the same deterministic snapshot as sequential collection
    // START_test_parallel_collection_matches_serial_snapshot
    #[test]
    fn test_parallel_collection_matches_serial_snapshot() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).expect("create src");
        for idx in 0..24 {
            std::fs::write(
                src.join(format!("file_{idx:02}.rs")),
                format!("fn function_{idx:02}() {{}}\n"),
            )
            .expect("write source");
        }
        let files: Vec<_> = (0..24)
            .rev()
            .map(|idx| index_file(&format!("src/file_{idx:02}.rs"), "rust"))
            .collect();

        let parallel = collect_index_blocks(dir.path(), &files);
        let serial = collect_index_blocks_serial(dir.path(), &files);
        let parallel_ids: Vec<_> = parallel.iter().map(|block| block.id.as_str()).collect();
        let serial_ids: Vec<_> = serial.iter().map(|block| block.id.as_str()).collect();

        assert_eq!(parallel_ids, serial_ids);
    }
    // END_test_parallel_collection_matches_serial_snapshot

    // START_CONTRACT_test_process_index_file_skips_large_files
    // PURPOSE: Verify oversized files are skipped before parser work
    // START_test_process_index_file_skips_large_files
    #[test]
    fn test_process_index_file_skips_large_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).expect("create src");
        std::fs::write(
            src.join("large.rs"),
            "x".repeat(MAX_INDEXABLE_FILE_BYTES + 1),
        )
        .expect("write large file");

        let stored = process_index_file(dir.path(), &index_file("src/large.rs", "rust"));

        assert!(stored.is_none());
    }
    // END_test_process_index_file_skips_large_files
}
