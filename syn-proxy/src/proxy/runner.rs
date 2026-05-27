// MODULE_CONTRACT
// MODULE_ID: M-PROXY-RUNNER
// PURPOSE: Streaming command runner — executes shell commands, captures capped stdout/stderr with exit status, and tees bounded raw output into evidence artifacts
// SCOPE: CommandRunner struct, CommandOutput status/evidence model, RTK-style capped streaming stdout/stderr capture via std::process::Command, XDG data raw-output evidence files
// DEPENDS: N/A
// LINKS:
//   → Phase-25 (implements) - RTK streaming runner and raw evidence
//   → Phase-52 (implements) - RTK-style raw capture caps
//   ← V-M-PROXY-RUNNER (verified_by) - runner verification shard

// START_MODULE_MAP
// CommandOutput — Captured command output plus success, exit status, and raw evidence metadata
// CommandRunner — Executes a shell command and returns combined capped stdout/stderr
// CommandRunner::from_config — Creates a runner using proxy capture and timeout settings
// EvidenceSink — Best-effort writer for raw command output artifacts
// CaptureResult — Bounded stream capture plus raw byte accounting
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.2.0 — Made command execution async-friendly with configurable caps and timeout]
// END_CHANGE_SUMMARY

use syn_core::config::Config;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static EVIDENCE_COUNTER: AtomicU64 = AtomicU64::new(0);
const DEFAULT_CAPTURE_CAP_BYTES: usize = 10_485_760;
const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 300;
const CAPTURE_BUFFER_BYTES: usize = 8192;
const COMMAND_WAIT_POLL_MS: u64 = 25;

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
    capture_cap_bytes: usize,
    command_timeout_secs: u64,
}
// END_CommandRunner

impl CommandRunner {
    // START_CONTRACT_CommandRunner::new
    // PURPOSE: Create a CommandRunner from command parts
    // INPUTS: { parts: &[String] - command and arguments }
    // OUTPUTS: { Self }
    // START_runner_new
    pub fn new(parts: &[String]) -> Self {
        Self::with_limits(
            parts,
            DEFAULT_CAPTURE_CAP_BYTES,
            DEFAULT_COMMAND_TIMEOUT_SECS,
        )
    }
    // END_runner_new

    // START_CONTRACT_CommandRunner::from_config
    // PURPOSE: Create a CommandRunner from command parts and proxy runtime config
    // INPUTS: { parts: &[String] - command and arguments }, { config: &Config }
    // OUTPUTS: { Self }
    // START_runner_from_config
    pub fn from_config(parts: &[String], config: &Config) -> Self {
        let capture_cap_bytes = if config.proxy.capture_cap_bytes == 0 {
            usize::MAX
        } else {
            config.proxy.capture_cap_bytes
        };
        Self::with_limits(parts, capture_cap_bytes, config.proxy.command_timeout_secs)
    }
    // END_runner_from_config

    // START_CONTRACT_CommandRunner::with_limits
    // PURPOSE: Create a CommandRunner from explicit capture and timeout limits
    // INPUTS: { parts: &[String] }, { capture_cap_bytes: usize }, { command_timeout_secs: u64 }
    // OUTPUTS: { Self }
    // START_runner_with_limits
    fn with_limits(parts: &[String], capture_cap_bytes: usize, command_timeout_secs: u64) -> Self {
        let cmd = parts[0].clone();
        let args = if parts.len() > 1 {
            parts[1..].to_vec()
        } else {
            Vec::new()
        };
        Self {
            cmd,
            args,
            capture_cap_bytes,
            command_timeout_secs,
        }
    }
    // END_runner_with_limits

    // START_CONTRACT_CommandRunner::execute
    // PURPOSE: Execute the command with streaming stdout/stderr capture and raw evidence tee
    // OUTPUTS: { anyhow::Result<CommandOutput> - command output, status, and evidence metadata }
    // SIDE_EFFECTS: spawns child process, writes raw output evidence when data directory is available
    // LINKS:
    //   → Phase-25 (implements) - streaming runner and raw evidence
    //   → Phase-52 (implements) - bounded raw capture
    // START_runner_execute
    pub async fn execute(&self) -> anyhow::Result<CommandOutput> {
        let cmd = self.cmd.clone();
        let args = self.args.clone();
        let capture_cap_bytes = self.capture_cap_bytes;
        let command_timeout_secs = self.command_timeout_secs;

        tokio::task::spawn_blocking(move || {
            execute_blocking(cmd, args, capture_cap_bytes, command_timeout_secs)
        })
        .await
        .map_err(|e| anyhow::anyhow!("command execution task failed: {}", e))?
    }
    // END_runner_execute
}
// END_public_api

// START_CONTRACT_execute_blocking
// PURPOSE: Execute a child process and capture output inside a blocking worker thread
// INPUTS: { cmd: String }, { args: Vec<String> }, { capture_cap_bytes: usize }, { command_timeout_secs: u64 }
// OUTPUTS: { anyhow::Result<CommandOutput> }
// SIDE_EFFECTS: spawns child process, capture threads, and evidence file writer
// START_execute_blocking
fn execute_blocking(
    cmd: String,
    args: Vec<String>,
    capture_cap_bytes: usize,
    command_timeout_secs: u64,
) -> anyhow::Result<CommandOutput> {
    let mut child = Command::new(&cmd)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to execute '{}': {}", cmd, e))?;

    let mut evidence = EvidenceSink::new();
    let stdout_handle = child
        .stdout
        .take()
        .map(|stdout| spawn_capture(stdout, evidence.writer(), "stdout", capture_cap_bytes));
    let stderr_handle = child
        .stderr
        .take()
        .map(|stderr| spawn_capture(stderr, evidence.writer(), "stderr", capture_cap_bytes));

    let status = wait_for_child(&mut child, command_timeout_secs, &cmd)?;
    let stdout = join_capture(stdout_handle)?;
    let stderr = join_capture(stderr_handle)?;
    let status_code = status.code().unwrap_or(1);
    let success = status.success();
    let raw_bytes = stdout.raw_bytes.saturating_add(stderr.raw_bytes);
    let evidence_bytes = stdout.evidence_bytes.saturating_add(stderr.evidence_bytes);
    let evidence_path = evidence.finish(evidence_bytes)?;
    let text = combine_output(stdout.captured, stderr.captured, success, status_code);

    Ok(CommandOutput {
        text,
        status_code,
        success,
        evidence_path,
        raw_bytes,
    })
}
// END_execute_blocking

// START_CONTRACT_wait_for_child
// PURPOSE: Wait for child exit, killing it when a nonzero timeout elapses
// INPUTS: { child: &mut Child }, { command_timeout_secs: u64 }, { cmd: &str }
// OUTPUTS: { anyhow::Result<ExitStatus> }
// SIDE_EFFECTS: may kill timed-out child process
// START_wait_for_child
fn wait_for_child(
    child: &mut Child,
    command_timeout_secs: u64,
    cmd: &str,
) -> anyhow::Result<ExitStatus> {
    if command_timeout_secs == 0 {
        return Ok(child.wait()?);
    }

    let deadline = Instant::now() + Duration::from_secs(command_timeout_secs);
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("command timed out after {command_timeout_secs}s: {cmd}");
        }
        std::thread::sleep(Duration::from_millis(COMMAND_WAIT_POLL_MS));
    }
}
// END_wait_for_child

// START_CaptureResult
struct CaptureResult {
    captured: Vec<u8>,
    raw_bytes: u64,
    evidence_bytes: u64,
}
// END_CaptureResult

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
// PURPOSE: Read one child output stream in chunks, tee bounded chunks to evidence, and return capped captured bytes with raw byte accounting
// INPUTS: { stream: impl Read }, { writer: Option<Arc<Mutex<File>>> }, { stream_name: &'static str }, { capture_cap_bytes: usize }
// OUTPUTS: { JoinHandle<anyhow::Result<CaptureResult>> }
// SIDE_EFFECTS: writes bounded raw bytes and a truncation marker to evidence file
// START_spawn_capture
fn spawn_capture<R>(
    mut stream: R,
    writer: Option<Arc<Mutex<File>>>,
    stream_name: &'static str,
    capture_cap_bytes: usize,
) -> JoinHandle<anyhow::Result<CaptureResult>>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut captured = Vec::new();
        let mut raw_bytes = 0_u64;
        let mut evidence_bytes = 0_u64;
        let mut truncated = false;
        let mut buf = [0_u8; CAPTURE_BUFFER_BYTES];
        loop {
            let read = stream.read(&mut buf)?;
            if read == 0 {
                break;
            }
            raw_bytes = raw_bytes.saturating_add(read as u64);
            let remaining = capture_cap_bytes.saturating_sub(captured.len());
            let keep = remaining.min(read);
            if keep > 0 {
                if let Some(writer) = &writer {
                    writer
                        .lock()
                        .map_err(|_| anyhow::anyhow!("raw evidence writer lock poisoned"))?
                        .write_all(&buf[..keep])?;
                    evidence_bytes = evidence_bytes.saturating_add(keep as u64);
                }
                captured.extend_from_slice(&buf[..keep]);
            }
            if keep < read && !truncated {
                truncated = true;
                let marker = truncation_marker(stream_name, capture_cap_bytes);
                if let Some(writer) = &writer {
                    writer
                        .lock()
                        .map_err(|_| anyhow::anyhow!("raw evidence writer lock poisoned"))?
                        .write_all(marker.as_bytes())?;
                    evidence_bytes = evidence_bytes.saturating_add(marker.len() as u64);
                }
                captured.extend_from_slice(marker.as_bytes());
            }
        }
        Ok(CaptureResult {
            captured,
            raw_bytes,
            evidence_bytes,
        })
    })
}
// END_spawn_capture

fn join_capture(
    handle: Option<JoinHandle<anyhow::Result<CaptureResult>>>,
) -> anyhow::Result<CaptureResult> {
    match handle {
        Some(handle) => handle
            .join()
            .map_err(|_| anyhow::anyhow!("command output capture thread panicked"))?,
        None => Ok(CaptureResult {
            captured: Vec::new(),
            raw_bytes: 0,
            evidence_bytes: 0,
        }),
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

fn truncation_marker(stream_name: &str, capture_cap_bytes: usize) -> String {
    format!(
        "\n[synapse] warning: {stream_name} capture exceeded {capture_cap_bytes} bytes; output truncated\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_combine_output_preserves_exit_fallback() {
        let text = combine_output(Vec::new(), Vec::new(), false, 17);
        assert_eq!(text, "Exit code: 17");
    }

    #[test]
    fn test_spawn_capture_caps_large_stream_without_losing_raw_count() {
        let raw_len = DEFAULT_CAPTURE_CAP_BYTES + 1024;
        let input = vec![b'a'; raw_len];
        let result = join_capture(Some(spawn_capture(
            Cursor::new(input),
            None,
            "stdout",
            DEFAULT_CAPTURE_CAP_BYTES,
        )))
        .expect("capture");

        assert_eq!(result.raw_bytes, raw_len as u64);
        assert!(result.captured.len() < raw_len);
        let text = String::from_utf8_lossy(&result.captured);
        assert!(text.contains("stdout capture exceeded"));
    }

    #[test]
    fn test_spawn_capture_uses_configured_cap() {
        let input = vec![b'a'; 2048];
        let result = join_capture(Some(spawn_capture(
            Cursor::new(input),
            None,
            "stderr",
            1024,
        )))
        .expect("capture");

        assert_eq!(result.raw_bytes, 2048);
        assert!(result.captured.len() < 2048);
        assert!(String::from_utf8_lossy(&result.captured).contains("1024 bytes"));
    }

    #[test]
    fn test_runner_from_config_uses_capture_cap_and_timeout() {
        let mut config = Config::default();
        config.proxy.capture_cap_bytes = 1024;
        config.proxy.command_timeout_secs = 7;
        let runner = CommandRunner::from_config(&["echo".into(), "ok".into()], &config);

        assert_eq!(runner.capture_cap_bytes, 1024);
        assert_eq!(runner.command_timeout_secs, 7);
    }

    #[test]
    fn test_runner_from_config_zero_capture_cap_is_unlimited() {
        let mut config = Config::default();
        config.proxy.capture_cap_bytes = 0;
        let runner = CommandRunner::from_config(&["echo".into()], &config);

        assert_eq!(runner.capture_cap_bytes, usize::MAX);
    }

    #[tokio::test]
    async fn test_execute_times_out_long_running_command() {
        let runner = CommandRunner::with_limits(
            &["sh".into(), "-c".into(), "sleep 2".into()],
            DEFAULT_CAPTURE_CAP_BYTES,
            1,
        );
        let err = match runner.execute().await {
            Ok(_) => panic!("command should time out"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("timed out after 1s"));
    }
}
