// MODULE_CONTRACT
// MODULE_ID: M-INDEXER-WALKER
// PURPOSE: File system walker — discovers source files with configurable .gitignore and .synignore handling
// SCOPE: Walker struct, file discovery with language detection, gitignore toggle, ignore rules
// DEPENDS: N/A
// LINKS: .gitignore, .synignore

// START_MODULE_MAP
// IndexFile — Discovered source file with path and language
// Walker — File system walker with configurable gitignore-aware traversal
// Walker::new_with_gitignore — Creates a walker with explicit gitignore behavior
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.1.0 — Added explicit gitignore toggle for syn index --no-git]
// END_CHANGE_SUMMARY

use ignore::WalkBuilder;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

// START_public_api

// START_IndexFile
pub struct IndexFile {
    pub path: String,
    pub language: String,
}
// END_IndexFile

// START_Walker
pub struct Walker {
    root: std::path::PathBuf,
    respect_gitignore: bool,
    total: AtomicUsize,
}
// END_Walker

impl Walker {
    // START_CONTRACT_Walker::new
    // PURPOSE: Create a new Walker for a given root directory
    // OUTPUTS: { Self }
    // START_walker_new
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            respect_gitignore: true,
            total: AtomicUsize::new(0),
        }
    }
    // END_walker_new

    // START_CONTRACT_Walker::new_with_gitignore
    // PURPOSE: Create a new Walker with explicit gitignore handling
    // INPUTS: { root: &Path }, { respect_gitignore: bool }
    // OUTPUTS: { Self }
    // START_walker_new_with_gitignore
    pub fn new_with_gitignore(root: &Path, respect_gitignore: bool) -> Self {
        Self {
            root: root.to_path_buf(),
            respect_gitignore,
            total: AtomicUsize::new(0),
        }
    }
    // END_walker_new_with_gitignore

    // START_CONTRACT_Walker::total
    // PURPOSE: Return the total number of discovered files
    // OUTPUTS: { usize }
    // START_walker_total
    pub fn total(&self) -> usize {
        self.total.load(Ordering::Relaxed)
    }
    // END_walker_total

    // START_CONTRACT_Walker::walk
    // PURPOSE: Walk the directory tree and collect all source files
    // OUTPUTS: { Vec<IndexFile> — discovered files }
    // START_walker_walk
    pub fn walk(&self) -> Vec<IndexFile> {
        let mut files = Vec::new();
        let walker = WalkBuilder::new(&self.root)
            .standard_filters(true)
            .git_global(self.respect_gitignore)
            .git_ignore(self.respect_gitignore)
            .git_exclude(self.respect_gitignore)
            .add_custom_ignore_filename(".synignore")
            .follow_links(false)
            .build();

        for result in walker {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    if let Some(lang) = detect_language(path) {
                        let rel = path
                            .strip_prefix(&self.root)
                            .unwrap_or(path)
                            .to_string_lossy()
                            .to_string();
                        files.push(IndexFile {
                            path: rel,
                            language: lang,
                        });
                        self.total.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(_) => continue,
            }
        }
        files
    }
    // END_walker_walk
}

fn detect_language(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    let name = path.file_name()?.to_str()?;
    match ext.as_str() {
        "rs" => Some("rust".into()),
        "py" => Some("python".into()),
        "ts" | "tsx" => Some("typescript".into()),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript".into()),
        "go" => Some("go".into()),
        "php" => Some("php".into()),
        "cpp" | "hpp" | "cc" | "cxx" | "h" => Some("cpp".into()),
        "rb" => Some("ruby".into()),
        "java" => Some("java".into()),
        "sh" | "bash" | "zsh" => Some("bash".into()),
        "json" => Some("json".into()),
        "css" | "scss" => Some("css".into()),
        "lua" => Some("lua".into()),
        "md" => Some("markdown".into()),
        "svelte" => Some("svelte".into()),
        _ => match name {
            "Dockerfile" | "dockerfile" => Some("dockerfile".into()),
            "Makefile" | "makefile" => Some("makefile".into()),
            _ => None,
        },
    }
}
// END_public_api
