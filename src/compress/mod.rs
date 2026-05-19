// MODULE_CONTRACT
// MODULE_ID: M-COMPRESS
// PURPOSE: Caveman text compressor — multi-level compression (lite, full, ultra) for AI context efficiency
// SCOPE: Compressor struct, compress_output, compress_file, restore_file, lite/full/ultra levels
// DEPENDS: M-CONFIG
// LINKS: N/A

// START_MODULE_MAP
// Compressor — Text compression engine with lite/full/ultra levels and file I/O
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.1.0 — Regex compression paths use cached fallbacks instead of unwrap panics]
// END_CHANGE_SUMMARY

use crate::config::Config;
use std::sync::OnceLock;

static COLLAPSE_NEWLINES_RE: OnceLock<Result<regex::Regex, String>> = OnceLock::new();
static COLLAPSE_SPACES_RE: OnceLock<Result<regex::Regex, String>> = OnceLock::new();
static ARTICLES_RE: OnceLock<Result<regex::Regex, String>> = OnceLock::new();

// START_public_api

// START_Compressor
pub struct Compressor {
    config: Config,
}

// END_Compressor

impl Compressor {
    // START_CONTRACT_Compressor::new
    // PURPOSE: Create a new Compressor with the given config
    // INPUTS: { config: &Config — application config }
    // OUTPUTS: { Self — new Compressor instance }
    // START_compressor_new
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }
    // END_compressor_new

    // START_CONTRACT_Compressor::compress_output
    // PURPOSE: Compress AI output text using the configured level
    // INPUTS: { input: &str — text to compress }
    // OUTPUTS: { String — compressed text }
    // START_compress_output
    pub fn compress_output(&self, input: &str) -> String {
        let level = self.config.compress.output_level.as_str();
        match level {
            "ultra" => self.compress_ultra(input),
            "lite" => self.compress_lite(input),
            _ => self.compress_full(input),
        }
    }
    // END_compress_output

    fn compress_lite(&self, input: &str) -> String {
        let mut result = input.to_string();
        let fillers = [
            "I think ",
            "I believe ",
            "In my opinion ",
            "It's worth noting that ",
            "It should be noted that ",
            "As you can see, ",
            "Obviously, ",
            "Essentially, ",
            "Basically, ",
            "Interestingly, ",
            "Importantly, ",
            "Additionally, ",
            "Furthermore, ",
            "Moreover, ",
            "However, ",
            "Nevertheless, ",
            "Nonetheless, ",
            "Therefore, ",
            "Thus, ",
            "Consequently, ",
            "In addition, ",
            "In other words, ",
            "That being said, ",
            "Having said that, ",
            "At the end of the day, ",
            "In conclusion, ",
            "To summarize, ",
        ];
        for filler in &fillers {
            result = result.replace(filler, "");
        }
        result
    }

    fn compress_full(&self, input: &str) -> String {
        let mut result = self.compress_lite(input);

        // Remove pleasantries
        let pleasantries = [
            "you're welcome",
            "you are welcome",
            "no problem",
            "happy to help",
            "glad to help",
            "let me know",
            "feel free to",
            "don't hesitate",
            "please ",
            " thanks",
            "thank you",
        ];
        for p in &pleasantries {
            result = result.replace(p, "");
        }

        // Collapse multiple newlines
        result = replace_all_cached(
            &COLLAPSE_NEWLINES_RE,
            "\n{3,}",
            &result,
            "\n\n",
            "compress_full",
        );

        // Collapse multiple spaces
        result = replace_all_cached(&COLLAPSE_SPACES_RE, " {2,}", &result, " ", "compress_full");

        result.trim().to_string()
    }

    /// Ultra: maximum compression, telegraphic
    fn compress_ultra(&self, input: &str) -> String {
        let result = self.compress_full(input);

        // Remove articles
        let result = replace_all_cached(
            &ARTICLES_RE,
            "\\b(the|a|an|this|that|these|those)\\b",
            &result,
            "",
            "compress_ultra",
        );

        // Collapse whitespace again
        let result =
            replace_all_cached(&COLLAPSE_SPACES_RE, " {2,}", &result, " ", "compress_ultra");

        // Shorten common words
        let mut result = result;
        let shorts = [
            ("because", "b/c"),
            ("with", "w/"),
            ("without", "w/o"),
            ("about", "re"),
            ("regarding", "re"),
            ("something", "sth"),
            ("someone", "sb"),
            ("information", "info"),
            ("application", "app"),
            ("function", "fn"),
            ("parameter", "param"),
            ("variable", "var"),
            ("implementation", "impl"),
            ("configuration", "config"),
            ("documentation", "docs"),
            ("previous", "prev"),
            ("current", "cur"),
            ("between", "btwn"),
            ("message", "msg"),
            ("number", "num"),
        ];
        for (from, to) in &shorts {
            result = result.replace(from, to);
        }

        result.trim().to_string()
    }

    // START_CONTRACT_Compressor::compress_file
    // PURPOSE: Compress a file for AI context, creating backup before overwriting
    // INPUTS: { path: &Path — file to compress }
    // OUTPUTS: { anyhow::Result<()> — ok on success }
    // START_compress_file
    pub async fn compress_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = tokio::fs::read_to_string(path).await?;
        if content.len() < 50 {
            return Ok(()); // Very small files, skip
        }

        let backup = path.with_extension("original.md");
        if !backup.exists() {
            tokio::fs::write(&backup, &content).await?;
        }

        let compressed = self.compress_output(&content);
        tokio::fs::write(path, &compressed).await?;
        tracing::info!(
            "Compressed {}: {} chars → {} chars ({}%)",
            path.display(),
            content.len(),
            compressed.len(),
            if content.is_empty() {
                0
            } else {
                100 - compressed.len() * 100 / content.len()
            },
        );
        Ok(())
    }
    // END_compress_file

    // START_CONTRACT_Compressor::restore_file
    // PURPOSE: Restore original file from backup
    // INPUTS: { path: &Path — compressed file to restore }
    // OUTPUTS: { anyhow::Result<()> — ok on success }
    // START_restore_file
    pub async fn restore_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let backup = path.with_extension("original.md");
        if backup.exists() {
            let content = tokio::fs::read_to_string(&backup).await?;
            tokio::fs::write(path, &content).await?;
            tokio::fs::remove_file(&backup).await?;
            tracing::info!("Restored {} from backup", path.display());
        }
        Ok(())
    }
    // END_restore_file
}

// START_CONTRACT_replace_all_cached
// PURPOSE: Apply a built-in regex replacement without panicking if regex initialization fails
// INPUTS: { cache: &OnceLock<Result<regex::Regex, String>> — regex cache }, { pattern: &str — static regex pattern }, { input: &str — source text }, { replacement: &str — replacement text }, { caller: &str — caller label for logs }
// OUTPUTS: { String — transformed text, or original text when the built-in regex is invalid }
// SIDE_EFFECTS: emits [Compressor][replace_all_cached][REGEX] warning on invalid built-in regex
// START_replace_all_cached
fn replace_all_cached(
    cache: &'static OnceLock<Result<regex::Regex, String>>,
    pattern: &'static str,
    input: &str,
    replacement: &str,
    caller: &str,
) -> String {
    match cache.get_or_init(|| regex::Regex::new(pattern).map_err(|error| error.to_string())) {
        Ok(re) => re.replace_all(input, replacement).to_string(),
        Err(error) => {
            tracing::warn!(
                "[Compressor][replace_all_cached][REGEX] {} skipped invalid built-in regex {:?}: {}",
                caller,
                pattern,
                error
            );
            input.to_string()
        }
    }
}
// END_replace_all_cached

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_compression_collapses_spacing_without_panics() {
        let mut config = Config::default();
        config.compress.output_level = "full".into();
        let compressor = Compressor::new(&config);

        let compressed =
            compressor.compress_output("I think hello   world\n\n\n\nplease keep this useful");

        assert_eq!(compressed, "hello world\n\nkeep this useful");
    }

    #[test]
    fn test_ultra_compression_removes_articles_without_panics() {
        let mut config = Config::default();
        config.compress.output_level = "ultra".into();
        let compressor = Compressor::new(&config);

        let compressed = compressor.compress_output("the application with this configuration");

        assert_eq!(compressed, "app w/ config");
    }
}
// END_public_api
