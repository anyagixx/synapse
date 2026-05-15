use std::path::Path;

#[derive(serde::Deserialize, Clone, Default)]
pub struct FilterDef {
    pub match_command: Option<String>,
    pub match_regex: Option<String>,
    pub strip_ansi: Option<bool>,
    pub replace: Option<Vec<ReplaceRule>>,
    pub match_output: Option<String>,
    pub strip_lines: Option<Vec<String>>,
    pub keep_lines: Option<Vec<String>>,
    pub truncate_lines_at: Option<usize>,
    pub head_lines: Option<usize>,
    pub tail_lines: Option<usize>,
    pub max_lines: Option<usize>,
    pub on_empty: Option<String>,
}

#[derive(serde::Deserialize, Clone)]
pub struct ReplaceRule {
    pub pattern: String,
    pub replacement: String,
}

#[derive(serde::Deserialize, Default)]
pub struct FilterFile {
    pub filters: Vec<FilterDef>,
}

pub enum FilterSource {
    BuiltIn,
    User,
    Project,
}

pub struct FilterEngine {
    filters: Vec<(FilterDef, FilterSource)>,
}

impl FilterEngine {
    pub fn new() -> Self {
        let mut engine = Self { filters: Vec::new() };

        // 1. Load built-in filters (compiled into binary)
        if let Some(filters) = Self::load_builtin() {
            for f in filters {
                engine.filters.push((f, FilterSource::BuiltIn));
            }
        }

        // 2. Load user global filters (~/.config/synapse/filters.toml)
        if let Some(dir) = dirs::config_dir() {
            let path = dir.join("synapse").join("filters.toml");
            if let Some(filters) = Self::load_file(&path) {
                for f in filters {
                    engine.filters.push((f, FilterSource::User));
                }
            }
        }

        // 3. Load project-local filters (.synapse/filters.toml)
        let project_path = Path::new(".synapse").join("filters.toml");
        if let Some(filters) = Self::load_file(&project_path) {
            for f in filters {
                engine.filters.push((f, FilterSource::Project));
            }
        }

        engine
    }

    fn load_builtin() -> Option<Vec<FilterDef>> {
        let toml_str = include_str!("builtin_filters.toml");
        let file: FilterFile = toml::from_str(toml_str).ok()?;
        Some(file.filters)
    }

    fn load_file(path: &Path) -> Option<Vec<FilterDef>> {
        let content = std::fs::read_to_string(path).ok()?;
        let file: FilterFile = toml::from_str(&content).ok()?;
        Some(file.filters)
    }

    pub fn find_filter(&self, cmd: &str) -> Option<&FilterDef> {
        let cmd = cmd.trim();
        let re = regex::Regex::new(r"^(\w+|-+)+").ok()?;
        let cmd_base = re.find(cmd).map(|m| m.as_str()).unwrap_or(cmd);

        // Check project-local first, then user, then built-in
        for (filter, _source) in &self.filters {
            if let Some(ref pattern) = filter.match_command {
                if cmd.starts_with(pattern) || cmd_base == pattern {
                    return Some(filter);
                }
            }
            if let Some(ref pattern) = filter.match_regex {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(cmd) {
                        return Some(filter);
                    }
                }
            }
        }
        None
    }

    pub fn apply(&self, filter: &FilterDef, output: &str) -> String {
        let mut result = output.to_string();

        // Stage 1: strip_ansi
        if filter.strip_ansi.unwrap_or(false) {
            result = Self::remove_ansi(&result);
        }

        // Stage 2: replace
        if let Some(ref rules) = filter.replace {
            for rule in rules {
                if let Ok(re) = regex::Regex::new(&rule.pattern) {
                    result = re.replace_all(&result, &rule.replacement[..]).to_string();
                }
            }
        }

        // Stage 3: match_output — if output matches, return short message
        if let Some(ref pattern) = filter.match_output {
            if let Ok(re) = regex::Regex::new(pattern) {
                if re.is_match(&result) {
                    if let Some(ref msg) = filter.on_empty {
                        return msg.clone();
                    }
                    return result;
                }
            }
        }

        // Stage 4: strip_lines / keep_lines
        let lines: Vec<&str> = result.lines().collect();
        let filtered: Vec<&str> = if let Some(ref keep) = filter.keep_lines {
            lines.iter().filter(|line| {
                keep.iter().any(|pat| {
                    regex::Regex::new(pat).map(|re| re.is_match(line)).unwrap_or(false)
                })
            }).copied().collect()
        } else if let Some(ref strip) = filter.strip_lines {
            lines.iter().filter(|line| {
                !strip.iter().any(|pat| {
                    regex::Regex::new(pat).map(|re| re.is_match(line)).unwrap_or(false)
                })
            }).copied().collect()
        } else {
            lines
        };

        // Stage 5: truncate_lines_at
        let truncated: Vec<String> = if let Some(max_len) = filter.truncate_lines_at {
            filtered.iter().map(|line| {
                if line.len() > max_len {
                    format!("{}...", &line[..max_len])
                } else {
                    line.to_string()
                }
            }).collect()
        } else {
            filtered.iter().map(|l| l.to_string()).collect()
        };

        // Stage 6: head_lines / tail_lines
        let sliced: Vec<&str> = match (filter.head_lines, filter.tail_lines) {
            (Some(h), Some(t)) => {
                let mut v: Vec<&str> = Vec::new();
                let total = truncated.len();
                for i in 0..h.min(total) {
                    v.push(&truncated[i]);
                }
                if h + t < total {
                    v.push("...");
                }
                for i in total.saturating_sub(t)..total {
                    v.push(&truncated[i]);
                }
                v
            }
            (Some(h), None) => truncated.iter().take(h).map(|s| s.as_str()).collect(),
            (None, Some(t)) => {
                let total = truncated.len();
                truncated.iter().skip(total.saturating_sub(t)).map(|s| s.as_str()).collect()
            }
            (None, None) => truncated.iter().map(|s| s.as_str()).collect(),
        };

        // Stage 7: max_lines
        let final_lines: Vec<&str> = if let Some(max) = filter.max_lines {
            let mut v: Vec<&str> = sliced.iter().take(max).copied().collect();
            if sliced.len() > max {
                v.push("...");
            }
            v
        } else {
            sliced
        };

        // Stage 8: on_empty
        if final_lines.is_empty() || final_lines.iter().all(|l| l.trim().is_empty()) {
            return filter.on_empty.clone().unwrap_or_default();
        }

        final_lines.join("\n")
    }

    fn remove_ansi(s: &str) -> String {
        let re = regex::Regex::new("\x1b\\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }
}
