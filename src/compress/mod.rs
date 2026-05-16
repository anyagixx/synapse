use crate::config::Config;

pub struct Compressor {
    config: Config,
}

impl Compressor {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Compress AI output text (caveman mode)
    pub fn compress_output(&self, input: &str) -> String {
        let level = self.config.compress.output_level.as_str();
        match level {
            "ultra" => self.compress_ultra(input),
            "lite" => self.compress_lite(input),
            _ => self.compress_full(input),
        }
    }

    /// Lite: remove filler words, keep grammar
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

    /// Full: drop articles, filler, pleasantries
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
        let re = regex::Regex::new("\n{3,}").unwrap();
        result = re.replace_all(&result, "\n\n").to_string();

        // Collapse multiple spaces
        let re = regex::Regex::new(" {2,}").unwrap();
        result = re.replace_all(&result, " ").to_string();

        result.trim().to_string()
    }

    /// Ultra: maximum compression, telegraphic
    fn compress_ultra(&self, input: &str) -> String {
        let result = self.compress_full(input);

        // Remove articles
        let re = regex::Regex::new("\\b(the|a|an|this|that|these|those)\\b").unwrap();
        let result = re.replace_all(&result, "").to_string();

        // Collapse whitespace again
        let re = regex::Regex::new(" {2,}").unwrap();
        let result = re.replace_all(&result, " ").to_string();

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

    /// Compress a file for AI context
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

    /// Restore original from backup
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
}
