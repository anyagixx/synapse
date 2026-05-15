use std::process::{Command, Stdio};

pub struct CommandRunner {
    cmd: String,
    args: Vec<String>,
}

impl CommandRunner {
    pub fn new(parts: &[String]) -> Self {
        let cmd = parts[0].clone();
        let args = if parts.len() > 1 {
            parts[1..].to_vec()
        } else {
            Vec::new()
        };
        Self { cmd, args }
    }

    pub fn execute(&self) -> anyhow::Result<String> {
        let output = Command::new(&self.cmd)
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to execute '{}': {}", self.cmd, e))?;

        let result = output.wait_with_output()?;

        let mut text = String::new();

        // Prefer stdout, but include stderr if non-empty
        if !result.stdout.is_empty() {
            text.push_str(&String::from_utf8_lossy(&result.stdout));
        }
        if !result.stderr.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&String::from_utf8_lossy(&result.stderr));
        }

        if text.is_empty() {
            if result.status.success() {
                text = "ok".into();
            } else {
                text = format!("Exit code: {}", result.status.code().unwrap_or(-1));
            }
        }

        Ok(text)
    }
}
