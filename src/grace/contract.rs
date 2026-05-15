use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModuleContract {
    pub file_path: String,
    pub module_id: Option<String>,
    pub purpose: Option<String>,
    pub scope: Option<String>,
    pub depends: Vec<String>,
    pub links: Vec<String>,
    pub has_contract: bool,
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ContractReport {
    pub total_files: usize,
    pub with_contract: usize,
    pub without_contract: usize,
    pub valid: usize,
    pub invalid: usize,
    pub contracts: Vec<ModuleContract>,
}

pub struct ContractValidator;

impl ContractValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_file(path: &Path, content: &str) -> ModuleContract {
        let file_path = path.to_string_lossy().to_string();
        let mut mc = ModuleContract {
            file_path,
            module_id: None,
            purpose: None,
            scope: None,
            depends: Vec::new(),
            links: Vec::new(),
            has_contract: false,
            valid: false,
            errors: Vec::new(),
        };

        let contract_block = Self::find_contract_block(content);

        let block = match contract_block {
            Some(b) => b,
            None => return mc,
        };

        mc.has_contract = true;

        // Extract module ID from filename or contract
        let lines: Vec<&str> = block.lines().collect();
        for line in &lines {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("#") {
                let val = trimmed.trim_start_matches('/').trim_start_matches('#').trim();
                Self::parse_field(&mut mc, val);
            }
            // Also match /* ... */ style
            if trimmed.starts_with('*') {
                let val = trimmed.trim_start_matches('*').trim();
                Self::parse_field(&mut mc, val);
            }
        }

        mc.valid = mc.purpose.is_some();
        if mc.purpose.is_none() {
            mc.errors.push("Missing PURPOSE in MODULE_CONTRACT".into());
        }
        mc
    }

    fn find_contract_block(content: &str) -> Option<&str> {
        let markers = ["// MODULE_CONTRACT\n", "// MODULE_CONTRACT\r",
                        "/* MODULE_CONTRACT", "# MODULE_CONTRACT",
                        "// MODULE_CONTRACT\r\n"];
        for marker in &markers {
            if let Some(start) = content.find(marker) {
                // Check it's on its own (the line starts with the comment, not the text before it)
                let line_start = content[..start].rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                // Check it's not part of another word
                if start > 0 {
                    let prev = content.as_bytes()[start - 1];
                    if prev != b'\n' && prev != b'\r' && prev != b' ' { continue; }
                }
                let line_end = content[start..].find('\n')
                    .map(|i| start + i)
                    .unwrap_or(content.len());
                return Some(&content[line_start..line_end]);
            }
        }
        None
    }

    fn parse_field(mc: &mut ModuleContract, val: &str) {
        if let Some(p) = val.strip_prefix("PURPOSE:") {
            mc.purpose = Some(p.trim().to_string());
        } else if let Some(s) = val.strip_prefix("SCOPE:") {
            mc.scope = Some(s.trim().to_string());
        } else if let Some(d) = val.strip_prefix("DEPENDS:") {
            mc.depends = d.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        } else if let Some(l) = val.strip_prefix("LINKS:") {
            mc.links = l.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        } else if let Some(id) = val.strip_prefix("MODULE_ID:") {
            mc.module_id = Some(id.trim().to_string());
        }
    }

    pub fn validate_project(root: &Path) -> anyhow::Result<ContractReport> {
        let mut contracts = Vec::new();
        let walker = crate::indexer::walker::Walker::new(root);
        let files = walker.walk();

        let source_extensions = ["rs", "py", "ts", "tsx", "js", "jsx", "go", "rb", "java", "php", "cpp", "hpp", "c", "h", "css", "scss", "lua", "sh", "bash", "zsh", "svelte"];
        for file in &files {
            let ext = std::path::Path::new(&file.path).extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            if !source_extensions.contains(&ext) {
                continue;
            }
            let full_path = root.join(&file.path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let mc = Self::scan_file(&full_path, &content);
                contracts.push(mc);
            }
        }

        let total = contracts.len();
        let with = contracts.iter().filter(|c| c.has_contract).count();
        let valid = contracts.iter().filter(|c| c.valid).count();

        Ok(ContractReport {
            total_files: total,
            with_contract: with,
            without_contract: total - with,
            valid,
            invalid: with - valid,
            contracts,
        })
    }

    pub fn enforce_500_lines(content: &str) -> bool {
        content.lines().count() <= 500
    }
}
