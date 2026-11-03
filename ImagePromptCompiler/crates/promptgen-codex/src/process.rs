use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use super::{CodexExecutionError, NEXT_NONCE, io_error, millis};

/// Owns one spawned process for the length of a run.
///
/// Between spawn and reap every early return — a failed stdin write, a status query
/// that errors — would otherwise leave a live child holding the ephemeral
/// `CODEX_HOME` and the temporary tree that is about to be deleted. Ownership makes
/// the kill-and-reap unconditional instead of a rule each error path must remember.
pub(super) struct OwnedChild {
    child: Child,
    reaped: bool,
}

const POLL_INTERVAL: Duration = Duration::from_millis(150);
const LOG_LIMIT: usize = 16 * 1024;

impl OwnedChild {
    pub(super) fn spawn(
        command: &mut Command,
        code: &'static str,
        binary: &Path,
    ) -> Result<Self, CodexExecutionError> {
        let child = command.spawn().map_err(|error| {
            CodexExecutionError::new(code, format!("failed to start {binary:?}: {error}"))
        })?;
        Ok(Self {
            child,
            reaped: false,
        })
    }

    pub(super) fn write_stdin(
        &mut self,
        bytes: &[u8],
        code: &'static str,
    ) -> Result<(), CodexExecutionError> {
        let mut stdin = self
            .child
            .stdin
            .take()
            .ok_or_else(|| CodexExecutionError::new(code, "stdin pipe is missing"))?;
        match stdin.write_all(bytes) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                // The child may close stdin before the parent finishes its write.
                // Accept only this transport-level EOF; wait(), status, event, result,
                // and artifact checks remain authoritative immediately afterwards.
                Ok(())
            }
            Err(error) => Err(CodexExecutionError::new(
                code,
                format!("failed to write instruction: {error}"),
            )),
        }
    }

    pub(super) fn wait(&mut self, timeout: Duration) -> Result<ExitStatus, CodexExecutionError> {
        let status = wait_for_process(&mut self.child, timeout);
        if status.is_ok() {
            self.reaped = true;
        }
        status
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Waits for exit, or reports why it did not.
///
/// Termination is deliberately not performed here: the child is owned by
/// [`OwnedChild`], whose `Drop` kills and reaps it exactly once. Killing in both
/// places meant the timeout path reaped twice.
fn wait_for_process(
    child: &mut Child,
    timeout: Duration,
) -> Result<ExitStatus, CodexExecutionError> {
    let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
        CodexExecutionError::new(
            "CODEX_TIMEOUT_RANGE",
            format!("timeout is too large: {} seconds", timeout.as_secs()),
        )
    })?;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
            Ok(None) => {
                return Err(CodexExecutionError::new(
                    "CODEX_TIMEOUT",
                    format!("process exceeded {} seconds", timeout.as_secs()),
                ));
            }
            Err(error) => {
                return Err(CodexExecutionError::new(
                    "CODEX_WAIT",
                    format!("failed to query child status: {error}"),
                ));
            }
        }
    }
}

pub(super) fn isolated_codex_exec(binary: &Path) -> Command {
    let mut command = Command::new(binary);
    command
        .arg("exec")
        .arg("--skip-git-repo-check")
        .arg("--ephemeral")
        .arg("--ignore-user-config")
        .arg("--ignore-rules")
        .arg("--sandbox")
        .arg("read-only")
        .arg("--color")
        .arg("never");
    command
}
pub(super) fn probe_codex_version(
    binary: &Path,
    working_directory: &Path,
    timeout: Duration,
) -> Result<String, CodexExecutionError> {
    let stdout_path = working_directory.join("version.stdout");
    let stderr_path = working_directory.join("version.stderr");
    let stdout_file = create_new_file(&stdout_path, "create version stdout log")?;
    let stderr_file = create_new_file(&stderr_path, "create version stderr log")?;
    let mut child = OwnedChild::spawn(
        Command::new(binary)
            .arg("--version")
            .current_dir(working_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file)),
        "CODEX_VERSION_PROBE",
        binary,
    )?;
    let status = child.wait(timeout).map_err(|error| {
        CodexExecutionError::new(
            "CODEX_VERSION_PROBE",
            format!("version probe did not complete: {error}"),
        )
    })?;
    let stdout = read_log_snippet(&stdout_path);
    let stderr = read_log_snippet(&stderr_path);
    if !status.success() {
        return Err(CodexExecutionError::new(
            "CODEX_VERSION_PROBE",
            format!(
                "version probe exit {}; stderr={stderr:?}",
                exit_code(status)
            ),
        ));
    }
    let version = stdout.trim().to_owned();
    if version.is_empty() {
        return Err(CodexExecutionError::new(
            "CODEX_VERSION_PROBE",
            "version probe returned an empty string",
        ));
    }
    Ok(version)
}
pub(super) fn create_new_file(
    path: &Path,
    operation: &'static str,
) -> Result<File, CodexExecutionError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| io_error(operation, path, error))
}

pub(super) fn read_log_snippet(path: &Path) -> String {
    let Ok(mut file) = File::open(path) else {
        return String::new();
    };
    let mut bytes = Vec::new();
    let _ = std::io::Read::by_ref(&mut file)
        .take(LOG_LIMIT as u64)
        .read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}
pub(super) fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(-1)
}

pub(super) struct WorkingDirectory {
    pub(super) path: PathBuf,
}

impl WorkingDirectory {
    pub(super) fn create() -> Result<Self, CodexExecutionError> {
        let path = std::env::temp_dir().join(format!(
            "promptgen-codex-{}-{}-{}",
            std::process::id(),
            millis(SystemTime::now()),
            NEXT_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)
            .map_err(|error| io_error("create Codex working directory", &path, error))?;
        Ok(Self { path })
    }
}

impl Drop for WorkingDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
