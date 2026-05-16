use ignore::WalkBuilder;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct IndexFile {
    pub path: String,
    pub language: String,
}

pub struct Walker {
    root: std::path::PathBuf,
    total: AtomicUsize,
}

impl Walker {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            total: AtomicUsize::new(0),
        }
    }

    pub fn total(&self) -> usize {
        self.total.load(Ordering::Relaxed)
    }

    pub fn walk(&self) -> Vec<IndexFile> {
        let mut files = Vec::new();
        let walker = WalkBuilder::new(&self.root)
            .standard_filters(true)
            .git_global(true)
            .git_ignore(true)
            .git_exclude(true)
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
