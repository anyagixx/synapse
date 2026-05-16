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
    pub has_module_map: bool,
    pub has_change_summary: bool,
    pub function_contracts: Vec<FunctionContract>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FunctionContract {
    pub name: String,
    pub purpose: Option<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub side_effects: Vec<String>,
    pub links: Vec<String>,
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

impl Default for ContractValidator {
    fn default() -> Self {
        Self::new()
    }
}

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
            has_module_map: false,
            has_change_summary: false,
            function_contracts: Vec::new(),
            errors: Vec::new(),
        };

        let contract_block = Self::find_contract_block(content);
        let block = match contract_block {
            Some(b) => b,
            None => return mc,
        };

        mc.has_contract = true;

        // Check for MODULE_MAP
        mc.has_module_map =
            content.contains("// START_MODULE_MAP") || content.contains("# START_MODULE_MAP");

        // Check for CHANGE_SUMMARY
        mc.has_change_summary = content.contains("// START_CHANGE_SUMMARY")
            || content.contains("# START_CHANGE_SUMMARY");

        // Extract function contracts
        mc.function_contracts = Self::extract_function_contracts(content);

        let lines: Vec<&str> = block.lines().collect();
        for line in &lines {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("#") {
                let val = trimmed
                    .trim_start_matches('/')
                    .trim_start_matches('#')
                    .trim();
                Self::parse_field(&mut mc, val);
            }
            if trimmed.starts_with('*') {
                let val = trimmed.trim_start_matches('*').trim();
                Self::parse_field(&mut mc, val);
            }
        }

        mc.valid = mc.purpose.is_some();
        if mc.purpose.is_none() {
            mc.errors.push("Missing PURPOSE in MODULE_CONTRACT".into());
        }
        if !mc.has_module_map {
            mc.errors.push("Missing MODULE_MAP".into());
        }
        if !mc.has_change_summary {
            mc.errors.push("Missing CHANGE_SUMMARY".into());
        }
        if mc.function_contracts.is_empty() {
            mc.errors
                .push("Missing function contracts (START_CONTRACT_name)".into());
        }
        mc
    }

    fn extract_function_contracts(content: &str) -> Vec<FunctionContract> {
        let mut contracts = Vec::new();
        extract_contracts_style(content, "//", &mut contracts);
        extract_contracts_style(content, "#", &mut contracts);
        contracts
    }

    fn find_contract_block(content: &str) -> Option<String> {
        let start_marker = content.find("// MODULE_CONTRACT")?;
        let block_start = content[..start_marker]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let mut block_end = start_marker;
        let after_marker = &content[start_marker..];
        for line in after_marker.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with('*') {
                block_end += line.len() + 1;
            } else if trimmed.is_empty() {
                block_end += line.len() + 1;
            } else {
                break;
            }
        }
        Some(content[block_start..block_end.min(content.len())].to_string())
    }

    fn parse_field(mc: &mut ModuleContract, val: &str) {
        if let Some(p) = val.strip_prefix("PURPOSE:") {
            mc.purpose = Some(p.trim().to_string());
        } else if let Some(s) = val.strip_prefix("SCOPE:") {
            mc.scope = Some(s.trim().to_string());
        } else if let Some(d) = val.strip_prefix("DEPENDS:") {
            mc.depends = d
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if let Some(l) = val.strip_prefix("LINKS:") {
            mc.links = l
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if let Some(id) = val.strip_prefix("MODULE_ID:") {
            mc.module_id = Some(id.trim().to_string());
        }
    }

    pub fn validate_project(root: &Path) -> anyhow::Result<ContractReport> {
        let mut contracts = Vec::new();
        let walker = crate::indexer::walker::Walker::new(root);
        let files = walker.walk();

        let source_extensions = [
            "rs", "py", "ts", "tsx", "js", "jsx", "go", "rb", "java", "php", "cpp", "hpp", "c",
            "h", "css", "scss", "lua", "sh", "bash", "zsh", "svelte",
        ];
        for file in &files {
            let ext = std::path::Path::new(&file.path)
                .extension()
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
}

fn extract_contracts_style(content: &str, prefix: &str, contracts: &mut Vec<FunctionContract>) {
    let pattern = format!(r"{} START_CONTRACT_(\w+)", prefix);
    let re = regex::Regex::new(&pattern).unwrap();
    let mut starts = Vec::new();
    for cap in re.captures_iter(content) {
        let name = cap[1].to_string();
        if contracts.iter().any(|c| c.name == name) {
            continue;
        }
        if let Some(m) = cap.get(0) {
            starts.push((m.start(), name));
        }
    }
    for (start_pos, name) in &starts {
        let after_start = &content[*start_pos..];
        let end_marker = format!("{} END_{}", prefix, name);
        let alt_prefix = if prefix == "//" { "#" } else { "//" };
        let alt_marker = format!("{} END_{}", alt_prefix, name);
        let end_pos = after_start
            .find(&end_marker)
            .or_else(|| after_start.find(&alt_marker));
        if let Some(end_off) = end_pos {
            let body = &after_start[..end_off];
            let body_start = body.find('\n').map(|i| i + 1).unwrap_or(0);
            let body_text = &body[body_start..];
            let mut fc = FunctionContract {
                name: name.clone(),
                purpose: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                side_effects: Vec::new(),
                links: Vec::new(),
            };
            for line in body_text.lines() {
                let t = line.trim().trim_start_matches(prefix).trim();
                let t = t.trim_start_matches(alt_prefix).trim();
                if let Some(p) = t.strip_prefix("PURPOSE:") {
                    fc.purpose = Some(p.trim().to_string());
                } else if let Some(i) = t.strip_prefix("INPUTS:") {
                    fc.inputs = i
                        .split("}, {")
                        .map(|s| s.trim().trim_matches('{').trim_matches('}').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                } else if let Some(o) = t.strip_prefix("OUTPUTS:") {
                    fc.outputs = o
                        .split("}, {")
                        .map(|s| s.trim().trim_matches('{').trim_matches('}').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                } else if let Some(s) = t.strip_prefix("SIDE_EFFECTS:") {
                    fc.side_effects = s
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                } else if let Some(l) = t.strip_prefix("LINKS:") {
                    fc.links = l
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
            contracts.push(fc);
        }
    }
}
