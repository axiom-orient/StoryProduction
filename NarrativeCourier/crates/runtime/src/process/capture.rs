//! Private capture files with live budgets, bounded reads and explicit cleanup.

use super::lifecycle::{admit, cancellation_requested, run_monitored};
use crate::{Error, Result};
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const MAX_CAPTURE_BYTES: u64 = 8 * 1024 * 1024;
static CAPTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct CaptureCleanup(Option<PathBuf>);
impl CaptureCleanup {
    fn finish(&mut self) -> Result<()> {
        if let Some(path) = &self.0 {
            match fs::remove_dir_all(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        self.0 = None;
        Ok(())
    }
}
impl Drop for CaptureCleanup {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

fn check_capture(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_CAPTURE_BYTES
    {
        return Err(Error::Executor(
            "process capture is non-regular or exceeded its byte budget".into(),
        ));
    }
    Ok(())
}
fn read_capture(path: &Path) -> Result<Vec<u8>> {
    check_capture(path)?;
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_CAPTURE_BYTES {
        return Err(Error::Executor(
            "opened process capture exceeds its regular-file budget".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_CAPTURE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != metadata.len() || bytes.len() as u64 > MAX_CAPTURE_BYTES {
        return Err(Error::Executor(
            "process capture changed size during bounded read".into(),
        ));
    }
    Ok(bytes)
}

pub fn run_piped_command_capture(
    command: &mut Command,
    stdin_bytes: &[u8],
    timeout: Duration,
    cancel_path: &Path,
    run_id: &str,
) -> Result<Output> {
    admit(timeout, run_id)?;
    if cancellation_requested(cancel_path, run_id)? {
        return Err(Error::Executor(
            "child process cancelled before capture setup".into(),
        ));
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::Executor(format!("capture clock is before Unix epoch: {error}")))?
        .as_nanos();
    let sequence = CAPTURE_SEQUENCE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| Error::Executor("process capture sequence exhausted".into()))?;
    let directory = std::env::temp_dir().join(format!(
        "narrative-courier-process-{}-{stamp}-{sequence}",
        std::process::id()
    ));
    fs::DirBuilder::new().mode(0o700).create(&directory)?;
    let mut cleanup = CaptureCleanup(Some(directory.clone()));
    let result = (|| -> Result<Output> {
        let stdout = directory.join("stdout");
        let stderr = directory.join("stderr");
        let open = |path: &Path| {
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(path)
        };
        command.stdout(Stdio::from(open(&stdout)?));
        command.stderr(Stdio::from(open(&stderr)?));
        let status = run_monitored(command, stdin_bytes, timeout, cancel_path, run_id, || {
            check_capture(&stdout)?;
            check_capture(&stderr)
        })?;
        Ok(Output {
            status,
            stdout: read_capture(&stdout)?,
            stderr: read_capture(&stderr)?,
        })
    })();
    let removed = cleanup.finish();
    match (result, removed) {
        (Ok(output), Ok(())) => Ok(output),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(cleanup)) => Err(Error::Executor(format!(
            "process completed but capture cleanup failed: {cleanup}"
        ))),
        (Err(error), Err(cleanup)) => Err(Error::Executor(format!(
            "{error}; capture cleanup failed: {cleanup}"
        ))),
    }
}
