// MODULE_CONTRACT
// MODULE_ID: M-PROXY-FILTER
// PURPOSE: TOML filter engine — applies regex-based output transformations from TOML filter definitions
// SCOPE: FilterDef, ReplaceRule, FilterFile, FilterEngine with find_filter and apply, 8-stage pipeline
// DEPENDS: N/A
// LINKS: builtin_filters.toml, ~/.config/synapse/filters.toml, .synapse/filters.toml

// START_MODULE_MAP
// FilterDef — TOML filter definition with match, replace, strip, truncate rules
// ReplaceRule — Regex replace rule (pattern → replacement)
// FilterFile — TOML file containing filter definitions
// FilterSource — Source of a filter (BuiltIn, User, Project)
// FilterEngine — Loads and applies TOML output filters
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY

use std::path::Path;

// START_public_api

// START_FilterDef
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
// END_FilterDef

// START_ReplaceRule
#[derive(serde::Deserialize, Clone)]
pub struct ReplaceRule {
    pub pattern: String,
    pub replacement: String,
}
// END_ReplaceRule

// START_FilterFile
#[derive(serde::Deserialize, Default)]
pub struct FilterFile {
    pub filters: Vec<FilterDef>,
}
// END_FilterFile

// START_FilterSource
pub enum FilterSource {
    BuiltIn,
    User,
    Project,
}
// END_FilterSource

// START_FilterEngine
pub struct FilterEngine {
    filters: Vec<(FilterDef, FilterSource)>,
}
// END_FilterEngine

impl Default for FilterEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterEngine {
    // START_CONTRACT_FilterEngine::new
    // PURPOSE: Create a new FilterEngine, loading built-in, user, and project filters
    // OUTPUTS: { Self }
    // SIDE_EFFECTS: reads TOML filter files from disk
    // START_fe_new
    pub fn new() -> Self {
        let mut engine = Self {
            filters: Vec::new(),
        };

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
    // END_fe_new

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

    // START_CONTRACT_FilterEngine::find_filter
    // PURPOSE: Find a matching filter for a given command string
    // INPUTS: { cmd: &str — command to match }
    // OUTPUTS: { Option<&FilterDef> }
    // START_fe_find_filter
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
    // END_fe_find_filter

    // START_CONTRACT_FilterEngine::apply
    // PURPOSE: Apply a filter definition to command output via 8-stage pipeline
    // INPUTS: { filter: &FilterDef }, { output: &str — raw command output }
    // OUTPUTS: { String — filtered output }
    // START_fe_apply
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
            lines
                .iter()
                .filter(|line| {
                    keep.iter().any(|pat| {
                        regex::Regex::new(pat)
                            .map(|re| re.is_match(line))
                            .unwrap_or(false)
                    })
                })
                .copied()
                .collect()
        } else if let Some(ref strip) = filter.strip_lines {
            lines
                .iter()
                .filter(|line| {
                    !strip.iter().any(|pat| {
                        regex::Regex::new(pat)
                            .map(|re| re.is_match(line))
                            .unwrap_or(false)
                    })
                })
                .copied()
                .collect()
        } else {
            lines
        };

        // Stage 5: truncate_lines_at
        let truncated: Vec<String> = if let Some(max_len) = filter.truncate_lines_at {
            filtered
                .iter()
                .map(|line| {
                    if line.len() > max_len {
                        format!("{}...", &line[..max_len])
                    } else {
                        line.to_string()
                    }
                })
                .collect()
        } else {
            filtered.iter().map(|l| l.to_string()).collect()
        };

        // Stage 6: head_lines / tail_lines
        let sliced: Vec<&str> = match (filter.head_lines, filter.tail_lines) {
            (Some(h), Some(t)) => {
                let mut v: Vec<&str> = Vec::new();
                let total = truncated.len();
                for item in truncated.iter().take(h.min(total)) {
                    v.push(item);
                }
                if h + t < total {
                    v.push("...");
                }
                for item in truncated.iter().skip(total.saturating_sub(t)) {
                    v.push(item);
                }
                v
            }
            (Some(h), None) => truncated.iter().take(h).map(|s| s.as_str()).collect(),
            (None, Some(t)) => {
                let total = truncated.len();
                truncated
                    .iter()
                    .skip(total.saturating_sub(t))
                    .map(|s| s.as_str())
                    .collect()
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
    // END_fe_apply

    fn remove_ansi(s: &str) -> String {
        let re = regex::Regex::new("\x1b\\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_filters_load() {
        let engine = FilterEngine::new();
        let filter = engine.find_filter("cargo test");
        assert!(
            filter.is_some(),
            "Should find built-in filter for 'cargo test'"
        );
    }

    #[test]
    fn test_find_git_status_filter() {
        let engine = FilterEngine::new();
        let filter = engine.find_filter("git status");
        assert!(filter.is_some());
    }

    #[test]
    fn test_passthrough_unknown_command() {
        let engine = FilterEngine::new();
        let filter = engine.find_filter("some_unknown_command_xyz");
        assert!(filter.is_none());
    }

    #[test]
    fn test_apply_strip_lines() {
        let engine = FilterEngine::new();
        let filter = FilterDef {
            match_command: Some("ls".into()),
            strip_lines: Some(vec!["^$".into()]),
            ..Default::default()
        };
        let output = "file1.rs\n\nfile2.rs\n\n";
        let result = engine.apply(&filter, output);
        assert!(!result.contains("\n\n"));
    }

    #[test]
    fn test_apply_max_lines() {
        let engine = FilterEngine::new();
        let filter = FilterDef {
            max_lines: Some(3),
            ..Default::default()
        };
        let output = "line1\nline2\nline3\nline4\nline5\nline6";
        let result = engine.apply(&filter, output);
        assert!(result.lines().count() <= 4); // 3 + "..."
    }

    #[test]
    fn test_apply_on_empty() {
        let engine = FilterEngine::new();
        let filter = FilterDef {
            on_empty: Some("No output".into()),
            ..Default::default()
        };
        let result = engine.apply(&filter, "");
        assert_eq!(result, "No output");
    }

    #[test]
    fn test_apply_keep_lines() {
        let engine = FilterEngine::new();
        let filter = FilterDef {
            keep_lines: Some(vec!["error".into(), "FAIL".into()]),
            ..Default::default()
        };
        let output =
            "running tests\n  PASS test_a\n  FAIL test_b\n  error: something broke\nfinished";
        let result = engine.apply(&filter, output);
        assert!(result.contains("FAIL"));
        assert!(result.contains("error"));
    }
}
// END_public_api
