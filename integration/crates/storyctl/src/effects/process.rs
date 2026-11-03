use crate::error::{Error, Result};
use std::collections::BTreeMap;
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::time::{Duration, Instant};

static CANCELLED: AtomicBool = AtomicBool::new(false);
static SIGNAL_INSTALLATION: Once = Once::new();

pub const MAX_PROCESS_TIMEOUT_SECONDS: u64 = 7200;
pub const DEFAULT_OWNER_TIMEOUT_SECONDS: u64 = 120;

extern "C" fn cancel_signal(_signal: libc::c_int) {
    CANCELLED.store(true, Ordering::SeqCst);
}

pub fn install_signal_handlers() {
    SIGNAL_INSTALLATION.call_once(|| unsafe {
        libc::signal(
            libc::SIGINT,
            cancel_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            cancel_signal as *const () as libc::sighandler_t,
        );
    });
}

pub fn cancellation_requested() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}

pub struct ProcessSpec {
    pub argv: Vec<String>,
    pub input: Vec<u8>,
    pub cwd: PathBuf,
    pub timeout: Duration,
    pub env: BTreeMap<String, String>,
    pub pass_fds: Vec<i32>,
    pub max_output_bytes: usize,
}

impl Default for ProcessSpec {
    fn default() -> Self {
        Self {
            argv: Vec::new(),
            input: Vec::new(),
            cwd: PathBuf::from("."),
            timeout: Duration::from_secs(DEFAULT_OWNER_TIMEOUT_SECONDS),
            env: BTreeMap::new(),
            pass_fds: Vec::new(),
            max_output_bytes: super::MAX_BYTES,
        }
    }
}

pub struct ProcessOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: ExitStatus,
}

struct ProcessGroup(Child);

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        // Killing a local group is cleanup, never proof of provider cancellation.
        unsafe {
            libc::kill(-(self.0.id() as i32), libc::SIGKILL);
        }
        let _ = self.0.wait();
    }
}

fn nonblocking(descriptor: RawFd) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Execute one explicit command. Domain exit codes and receipts are adapter policy.
pub fn invoke(spec: &ProcessSpec) -> Result<ProcessOutput> {
    if spec.argv.is_empty()
        || spec.input.len() > super::MAX_BYTES
        || !(Duration::from_secs(1)..=Duration::from_secs(MAX_PROCESS_TIMEOUT_SECONDS))
            .contains(&spec.timeout)
        || spec.max_output_bytes == 0
        || spec.max_output_bytes > super::MAX_BYTES
    {
        return Err(Error::not_started(
            "process input/argv/output limit or timeout is outside its allowed boundary",
        ));
    }
    if cancellation_requested() {
        return Err(Error::not_started("process cancelled before dispatch"));
    }
    let started = Instant::now();
    let explicit_cwd = spec
        .pass_fds
        .iter()
        .copied()
        .find(|descriptor| *descriptor >= 0 && spec.cwd == format!("/dev/fd/{descriptor}"));
    let cwd_anchor = if let Some(descriptor) = explicit_cwd {
        Some(
            super::files::Directory::from_fd(descriptor)
                .map_err(|error| Error::not_started(error.message))?,
        )
    } else if super::workspace::descriptor_binding(&spec.cwd)
        .map_err(|error| Error::not_started(error.message))?
        .is_some()
    {
        Some(
            super::files::Directory::open(&spec.cwd, false)
                .map_err(|error| Error::not_started(error.message))?,
        )
    } else {
        None
    };
    let cwd_fd = cwd_anchor.as_ref().map(super::files::Directory::raw_fd);
    let mut command = Command::new(&spec.argv[0]);
    command
        .args(&spec.argv[1..])
        .envs(&spec.env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if cwd_fd.is_none() {
        command.current_dir(&spec.cwd);
    }
    let pass_fds = spec.pass_fds.clone();
    unsafe {
        command.pre_exec(move || {
            if libc::setsid() < 0 {
                return Err(io::Error::last_os_error());
            }
            if let Some(descriptor) = cwd_fd {
                if libc::fchdir(descriptor) != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            for descriptor in &pass_fds {
                let flags = libc::fcntl(*descriptor, libc::F_GETFD);
                if flags < 0
                    || libc::fcntl(*descriptor, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0
                {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    let child = command
        .spawn()
        .map_err(|error| Error::not_started(format!("process could not start: {error}")))?;
    let mut child = ProcessGroup(child);
    let mut stdin = child.0.stdin.take();
    let mut stdout = child.0.stdout.take();
    let mut stderr = child.0.stderr.take();
    if spec.input.is_empty() {
        stdin.take();
    }
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = (|| -> Result<ExitStatus> {
        for descriptor in [
            stdin.as_ref().map(AsRawFd::as_raw_fd),
            stdout.as_ref().map(AsRawFd::as_raw_fd),
            stderr.as_ref().map(AsRawFd::as_raw_fd),
        ]
        .into_iter()
        .flatten()
        {
            nonblocking(descriptor)
                .map_err(|error| Error::unknown(format!("process pipe setup failed: {error}")))?;
        }
        let mut sent = 0;
        let mut status = None;
        loop {
            if cancellation_requested() {
                return Err(Error::unknown(
                    "process cancelled by local signal; external effect remains unknown",
                ));
            }
            if started.elapsed() >= spec.timeout {
                return Err(Error::unknown(
                    "process timeout exceeded; inspect existing operation before retrying",
                ));
            }
            if status.is_none() {
                status = child
                    .0
                    .try_wait()
                    .map_err(|error| Error::unknown(format!("process wait failed: {error}")))?;
            }
            if stdin.is_none() && stdout.is_none() && stderr.is_none() && status.is_some() {
                break;
            }
            let mut descriptors = Vec::with_capacity(3);
            if let Some(stream) = &stdin {
                descriptors.push(libc::pollfd {
                    fd: stream.as_raw_fd(),
                    events: libc::POLLOUT,
                    revents: 0,
                });
            }
            if let Some(stream) = &stdout {
                descriptors.push(libc::pollfd {
                    fd: stream.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                });
            }
            if let Some(stream) = &stderr {
                descriptors.push(libc::pollfd {
                    fd: stream.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                });
            }
            let remaining = spec
                .timeout
                .saturating_sub(started.elapsed())
                .as_millis()
                .min(100) as i32;
            let count = unsafe {
                libc::poll(
                    descriptors.as_mut_ptr(),
                    descriptors.len() as libc::nfds_t,
                    remaining,
                )
            };
            if count < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(Error::unknown(format!("process poll failed: {error}")));
            }
            for descriptor in &descriptors {
                if descriptor.revents == 0 {
                    continue;
                }
                if stdin
                    .as_ref()
                    .is_some_and(|stream| stream.as_raw_fd() == descriptor.fd)
                {
                    let remaining = &spec.input[sent..];
                    let count = unsafe {
                        libc::write(
                            descriptor.fd,
                            remaining.as_ptr().cast(),
                            remaining.len().min(65_536),
                        )
                    };
                    if count < 0 {
                        let error = io::Error::last_os_error();
                        if matches!(
                            error.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) {
                            continue;
                        }
                        return Err(Error::unknown(format!(
                            "process closed input before receiving the whole request: {error}"
                        )));
                    }
                    sent += count as usize;
                    if sent == spec.input.len() {
                        stdin.take();
                    }
                    continue;
                }
                let is_stdout = stdout
                    .as_ref()
                    .is_some_and(|stream| stream.as_raw_fd() == descriptor.fd);
                let mut buffer = [0u8; 65_536];
                let count =
                    unsafe { libc::read(descriptor.fd, buffer.as_mut_ptr().cast(), buffer.len()) };
                if count < 0 {
                    let error = io::Error::last_os_error();
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) {
                        continue;
                    }
                    return Err(Error::unknown(format!(
                        "process output read failed: {error}"
                    )));
                }
                if count == 0 {
                    if is_stdout {
                        stdout.take();
                    } else {
                        stderr.take();
                    }
                } else {
                    let length = count as usize;
                    let capacity = spec.max_output_bytes.saturating_sub(out.len() + err.len());
                    let target = if is_stdout { &mut out } else { &mut err };
                    target.extend_from_slice(&buffer[..length.min(capacity)]);
                    if length > capacity {
                        return Err(Error::unknown("process output limit exceeded"));
                    }
                }
            }
        }
        std::str::from_utf8(&out).map_err(|_| Error::unknown("process did not return UTF-8"))?;
        let group_exists = unsafe { libc::kill(-(child.0.id() as i32), 0) } == 0;
        if group_exists {
            return Err(Error::unknown(
                "process exited while descendants remain; inspect the operation before retrying",
            ));
        }
        if cancellation_requested() {
            return Err(Error::unknown(
                "process cancelled by local signal; external effect remains unknown",
            ));
        }
        Ok(status.unwrap())
    })();
    match result {
        Ok(status) => Ok(ProcessOutput {
            stdout: out,
            stderr: err,
            status,
        }),
        Err(mut error) => {
            error.spawned = true;
            error.stdout = out;
            error.stderr = err;
            Err(error)
        }
    }
}
