// MODULE_CONTRACT
// MODULE_ID: M-PROXY-RUNNER
// PURPOSE: Streaming command runner — executes shell commands, captures stdout/stderr with exit status, and tees raw output into evidence artifacts
// SCOPE: CommandRunner struct, CommandOutput status/evidence model, streaming stdout/stderr capture via std::process::Command, XDG data raw-output evidence files
// DEPENDS: N/A
// LINKS:
//   → Phase-25 (implements) - RTK streaming runner and raw evidence
//   ← V-M-PROXY-RUNNER (verified_by) - runner verification shard

// START_MODULE_MAP
// CommandOutput — Captured command output plus success, exit status, and raw evidence metadata
// CommandRunner — Executes a shell command and returns combined stdout/stderr
// EvidenceSink — Best-effort writer for raw command output artifacts
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.0.0 — Stream stdout/stderr and tee raw output into evidence artifacts]
// END_CHANGE_SUMMARY

use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

static EVIDENCE_COUNTER: AtomicU64 = AtomicU64::new(0);

// START_public_api

// START_CommandOutput
pub struct CommandOutput {
    pub text: String,
    pub status_code: i32,
    pub success: bool,
    pub evidence_path: Option<PathBuf>,
    pub raw_bytes: u64,
}
// END_CommandOutput

// START_CommandRunner
pub struct CommandRunner {
    cmd: String,
    args: Vec<String>,
}
// END_CommandRunner

impl CommandRunner {
    // START_CONTRACT_CommandRunner::new
    // PURPOSE: Create a CommandRunner from command parts
    // INPUTS: { parts: &[String] - command and arguments }
    // OUTPUTS: { Self }
    // START_runner_new
    pub fn new(parts: &[String]) -> Self {
        let cmd = parts[0].clone();
        let args = if parts.len() > 1 {
            parts[1..].to_vec()
        } else {
            Vec::new()
        };
        Self { cmd, args }
    }
    // END_runner_new

    // START_CONTRACT_CommandRunner::execute
    // PURPOSE: Execute the command with streaming stdout/stderr capture and raw evidence tee
    // OUTPUTS: { anyhow::Result<CommandOutput> - command output, status, and evidence metadata }
    // SIDE_EFFECTS: spawns child process, writes raw output evidence when data directory is available
    // LINKS:
    //   → Phase-25 (implements) - streaming runner and raw evidence
    // START_runner_execute
    pub fn execute(&self) -> anyhow::Result<CommandOutput> {
        let mut child = Command::new(&self.cmd)
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to execute '{}': {}", self.cmd, e))?;

        let mut evidence = EvidenceSink::new();
        let stdout_handle = child
            .stdout
            .take()
            .map(|stdout| spawn_capture(stdout, evidence.writer()));
        let stderr_handle = child
            .stderr
            .take()
            .map(|stderr| spawn_capture(stderr, evidence.writer()));

        let status = child.wait()?;
        let stdout = join_capture(stdout_handle)?;
        let stderr = join_capture(stderr_handle)?;
        let status_code = status.code().unwrap_or(1);
        let success = status.success();
        let raw_bytes = (stdout.len() + stderr.len()) as u64;
        let evidence_path = evidence.finish(raw_bytes)?;
        let text = combine_output(stdout, stderr, success, status_code);

        Ok(CommandOutput {
            text,
            status_code,
            success,
            evidence_path,
            raw_bytes,
        })
    }
    // END_runner_execute
}
// END_public_api

// START_EvidenceSink
struct EvidenceSink {
    path: Option<PathBuf>,
    writer: Option<Arc<Mutex<File>>>,
}
// END_EvidenceSink

impl EvidenceSink {
    // START_CONTRACT_EvidenceSink::new
    // PURPOSE: Create a best-effort evidence sink under the Synapse XDG data directory
    // OUTPUTS: { Self }
    // SIDE_EFFECTS: creates proxy-evidence directory and log file when possible
    // START_evidence_sink_new
    fn new() -> Self {
        let Some(dir) = dirs::data_dir().map(|dir| dir.join("synapse").join("proxy-evidence"))
        else {
            return Self::disabled();
        };
        if std::fs::create_dir_all(&dir).is_err() {
            return Self::disabled();
        }
        let path = dir.join(evidence_file_name());
        match File::create(&path) {
            Ok(file) => Self {
                path: Some(path),
                writer: Some(Arc::new(Mutex::new(file))),
            },
            Err(_) => Self::disabled(),
        }
    }
    // END_evidence_sink_new

    fn disabled() -> Self {
        Self {
            path: None,
            writer: None,
        }
    }

    fn writer(&self) -> Option<Arc<Mutex<File>>> {
        self.writer.as_ref().map(Arc::clone)
    }

    // START_CONTRACT_EvidenceSink::finish
    // PURPOSE: Flush evidence output and remove empty artifacts
    // INPUTS: { raw_bytes: u64 }
    // OUTPUTS: { anyhow::Result<Option<PathBuf>> }
    // SIDE_EFFECTS: flushes or removes evidence file
    // START_evidence_sink_finish
    fn finish(&mut self, raw_bytes: u64) -> anyhow::Result<Option<PathBuf>> {
        if let Some(writer) = self.writer.take() {
            writer
                .lock()
                .map_err(|_| anyhow::anyhow!("raw evidence writer lock poisoned"))?
                .flush()?;
        }
        let path = self.path.take();
        if raw_bytes == 0 {
            if let Some(path) = &path {
                let _ = std::fs::remove_file(path);
            }
            return Ok(None);
        }
        Ok(path)
    }
    // END_evidence_sink_finish
}

// START_CONTRACT_spawn_capture
// PURPOSE: Read one child output stream in chunks, tee chunks to evidence, and return captured bytes
// INPUTS: { stream: impl Read }, { writer: Option<Arc<Mutex<File>>> }
// OUTPUTS: { JoinHandle<anyhow::Result<Vec<u8>>> }
// SIDE_EFFECTS: writes raw bytes to evidence file
// START_spawn_capture
fn spawn_capture<R>(
    mut stream: R,
    writer: Option<Arc<Mutex<File>>>,
) -> JoinHandle<anyhow::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut captured = Vec::new();
        let mut buf = [0_u8; 8192];
        loop {
            let read = stream.read(&mut buf)?;
            if read == 0 {
                break;
            }
            if let Some(writer) = &writer {
                writer
                    .lock()
                    .map_err(|_| anyhow::anyhow!("raw evidence writer lock poisoned"))?
                    .write_all(&buf[..read])?;
            }
            captured.extend_from_slice(&buf[..read]);
        }
        Ok(captured)
    })
}
// END_spawn_capture

fn join_capture(handle: Option<JoinHandle<anyhow::Result<Vec<u8>>>>) -> anyhow::Result<Vec<u8>> {
    match handle {
        Some(handle) => handle
            .join()
            .map_err(|_| anyhow::anyhow!("command output capture thread panicked"))?,
        None => Ok(Vec::new()),
    }
}

fn combine_output(stdout: Vec<u8>, stderr: Vec<u8>, success: bool, status_code: i32) -> String {
    let mut text = String::new();
    if !stdout.is_empty() {
        text.push_str(&String::from_utf8_lossy(&stdout));
    }
    if !stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(&stderr));
    }
    if text.is_empty() {
        if success {
            "ok".into()
        } else {
            format!("Exit code: {}", status_code)
        }
    } else {
        text
    }
}

fn evidence_file_name() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let seq = EVIDENCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("proxy-{millis}-{}-{seq}.log", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combine_output_preserves_exit_fallback() {
        let text = combine_output(Vec::new(), Vec::new(), false, 17);
        assert_eq!(text, "Exit code: 17");
    }
}
