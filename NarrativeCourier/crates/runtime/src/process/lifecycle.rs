//! Unix process-group custody with nonblocking stdin and a single execution deadline.

use crate::{Error, Result};
use std::{
    fs::{self, File},
    io::{Read, Write},
    os::{fd::AsRawFd, unix::process::CommandExt},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

pub(super) fn admit(timeout: Duration, run_id: &str) -> Result<()> {
    if timeout.is_zero()
        || run_id.trim().is_empty()
        || run_id.trim() != run_id
        || run_id.len() > 4094
    {
        return Err(Error::Executor(
            "process requires a positive timeout and bounded nonempty run id".into(),
        ));
    }
    Ok(())
}
pub(super) fn cancellation_requested(path: &Path, run_id: &str) -> Result<bool> {
    let entry = match fs::symlink_metadata(path) {
        Ok(entry) => entry,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if entry.file_type().is_symlink() || !entry.is_file() || entry.len() > 4096 {
        return Err(Error::Contract(
            "cancel marker must be a bounded regular non-symlink file".into(),
        ));
    }
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(Error::Contract("cancel marker changed type".into()));
    }
    let mut bytes = Vec::new();
    file.take(4097).read_to_end(&mut bytes)?;
    if bytes.len() > 4096 {
        return Err(Error::Contract(
            "cancel marker exceeded its byte limit".into(),
        ));
    }
    let marker = String::from_utf8(bytes)
        .map_err(|_| Error::Contract("cancel marker is not UTF-8".into()))?;
    Ok(marker.trim() == run_id)
}

struct Custody {
    child: Child,
    group: i32,
    finished: bool,
}
impl Custody {
    fn group_alive(&self) -> Result<bool> {
        if unsafe { libc::kill(-self.group, 0) } == 0 {
            return Ok(true);
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            Ok(false)
        } else {
            Err(error.into())
        }
    }
    fn signal(&self, signal: i32) -> Result<()> {
        if unsafe { libc::kill(-self.group, signal) } == 0 {
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            Ok(())
        } else {
            Err(error.into())
        }
    }
    fn terminate(&mut self) -> Result<()> {
        let mut failures = Vec::new();
        if let Err(error) = self.signal(libc::SIGTERM) {
            failures.push(error.to_string());
        }
        for _ in 0..20 {
            if let Err(error) = self.child.try_wait() {
                failures.push(error.to_string());
                break;
            }
            match self.group_alive() {
                Ok(false) => break,
                Ok(true) => thread::sleep(Duration::from_millis(50)),
                Err(error) => {
                    failures.push(error.to_string());
                    break;
                }
            }
        }
        if let Err(error) = self.signal(libc::SIGKILL) {
            failures.push(error.to_string());
        }
        if let Err(error) = self.child.wait() {
            failures.push(error.to_string());
        }
        self.finished = true;
        if failures.is_empty() {
            Ok(())
        } else {
            Err(Error::Executor(format!(
                "process cleanup failed; outcome unknown: {}",
                failures.join("; ")
            )))
        }
    }
}
impl Drop for Custody {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.terminate();
        }
    }
}

pub fn run_piped_command(
    command: &mut Command,
    stdin_bytes: &[u8],
    timeout: Duration,
    cancel_path: &Path,
    run_id: &str,
) -> Result<ExitStatus> {
    run_monitored(
        command,
        stdin_bytes,
        timeout,
        cancel_path,
        run_id,
        || Ok(()),
    )
}

pub(super) fn run_monitored<F: FnMut() -> Result<()>>(
    command: &mut Command,
    stdin_bytes: &[u8],
    timeout: Duration,
    cancel_path: &Path,
    run_id: &str,
    mut monitor: F,
) -> Result<ExitStatus> {
    admit(timeout, run_id)?;
    if cancellation_requested(cancel_path, run_id)? {
        return Err(Error::Executor(
            "child process cancelled before spawn".into(),
        ));
    }
    command.stdin(Stdio::piped());
    command.process_group(0);
    let started = Instant::now();
    let mut child = command
        .spawn()
        .map_err(|error| Error::Executor(format!("failed to start child process: {error}")))?;
    let group = match i32::try_from(child.id()) {
        Ok(group) if group > 0 => group,
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Executor(
                "child id cannot identify an owned Unix process group".into(),
            ));
        }
    };
    let mut process = Custody {
        child,
        group,
        finished: false,
    };
    let result = (|| -> Result<ExitStatus> {
        let mut stdin = Some(
            process
                .child
                .stdin
                .take()
                .ok_or_else(|| Error::Executor("child stdin unavailable".into()))?,
        );
        let descriptor = stdin
            .as_ref()
            .ok_or_else(|| Error::Executor("child stdin unavailable".into()))?
            .as_raw_fd();
        let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
        if flags == -1
            || unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1
        {
            return Err(std::io::Error::last_os_error().into());
        }
        let mut delivered = 0;
        let mut status = None;
        loop {
            if cancellation_requested(cancel_path, run_id)? {
                return Err(Error::Executor("child process cancelled".into()));
            }
            if started.elapsed() >= timeout {
                return Err(Error::Executor(format!(
                    "child process timed out after {} seconds",
                    timeout.as_secs()
                )));
            }
            monitor()?;
            if delivered == stdin_bytes.len() {
                stdin = None;
            }
            let mut progressed = false;
            if let Some(pipe) = stdin.as_mut() {
                let end = delivered.saturating_add(64 * 1024).min(stdin_bytes.len());
                match pipe.write(&stdin_bytes[delivered..end]) {
                    Ok(0) => return Err(Error::Executor("child stdin returned write-zero".into())),
                    Ok(length) => {
                        delivered += length;
                        progressed = true;
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                        ) => {}
                    Err(error) => return Err(error.into()),
                }
            }
            if status.is_none() {
                status = process.child.try_wait()?;
            }
            if let Some(status) = status {
                if stdin.is_none() {
                    if process.group_alive()? {
                        return Err(Error::Executor(
                            "child exited with descendants still in the owned group".into(),
                        ));
                    }
                    monitor()?;
                    if cancellation_requested(cancel_path, run_id)? {
                        return Err(Error::Executor(
                            "child process cancelled before completion".into(),
                        ));
                    }
                    process.finished = true;
                    return Ok(status);
                }
            }
            if !progressed {
                thread::sleep(Duration::from_millis(20));
            }
        }
    })();
    match result {
        Ok(status) => Ok(status),
        Err(error) => match process.terminate() {
            Ok(()) => Err(error),
            Err(cleanup) => Err(Error::Executor(format!("{error}; {cleanup}"))),
        },
    }
}
