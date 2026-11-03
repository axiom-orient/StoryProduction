use crate::io::{self, Result};
use serde_json::{json, Value};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct Group(pub Child);
impl Group {
    pub fn stop(&mut self) -> Result<()> {
        unsafe {
            libc::kill(-(self.0.id() as i32), libc::SIGKILL);
        }
        let _ = self.0.kill();
        io::io(self.0.wait())?;
        Ok(())
    }
}
impl Drop for Group {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
pub fn spawn(command: &mut Command) -> Result<Group> {
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.spawn().map(Group).map_err(|e| e.to_string())
}
enum Input<'a> {
    Empty,
    File(fs::File),
    Chunks(&'a mut dyn FnMut() -> Result<Option<Vec<u8>>>),
}
struct Streams<'a> {
    input: Input<'a>,
    output: Option<&'a fs::File>,
}
pub fn execute(
    path: &Path,
    args: &[String],
    timeout: Duration,
    allow_interrupt: bool,
    cancel: impl FnMut() -> Result<()>,
) -> Result<Vec<u8>> {
    execute_io(
        path,
        args,
        timeout,
        allow_interrupt,
        Streams {
            input: Input::Empty,
            output: None,
        },
        cancel,
    )
}
/// Frames come from held files in bounded chunks; encoded bytes go to one held file.
/// The encoder receives no project pathname or directory alias.
pub fn encode(
    path: &Path,
    args: &[String],
    output: &fs::File,
    input: &mut dyn FnMut() -> Result<Option<Vec<u8>>>,
    cancel: impl FnMut() -> Result<()>,
) -> Result<()> {
    execute_io(
        path,
        args,
        Duration::from_secs(600),
        true,
        Streams {
            input: Input::Chunks(input),
            output: Some(output),
        },
        cancel,
    )
    .map(|_| ())
}
pub fn inspect_file(
    path: &Path,
    args: &[String],
    input: fs::File,
    cancel: impl FnMut() -> Result<()>,
) -> Result<Vec<u8>> {
    execute_io(
        path,
        args,
        Duration::from_secs(600),
        true,
        Streams {
            input: Input::File(input),
            output: None,
        },
        cancel,
    )
}
fn nonblocking(fd: std::os::fd::RawFd) -> Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    io::check(
        flags >= 0 && unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } >= 0,
        "PROCESS_PIPE_SETUP",
    )
}
fn drain(stream: &mut dyn Read, target: &mut Vec<u8>, opened: &mut bool) -> Result<()> {
    if !*opened {
        return Ok(());
    }
    let mut buffer = [0u8; 65536];
    match stream.read(&mut buffer) {
        Ok(0) => *opened = false,
        Ok(n) => target.extend_from_slice(&buffer[..n]),
        Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {}
        Err(error) => return Err(error.to_string()),
    }
    Ok(())
}
fn execute_io(
    path: &Path,
    args: &[String],
    timeout: Duration,
    allow_interrupt: bool,
    mut streams: Streams<'_>,
    mut cancel: impl FnMut() -> Result<()>,
) -> Result<Vec<u8>> {
    let mut command = Command::new(path);
    let stdin = match &mut streams.input {
        Input::Empty => Stdio::null(),
        Input::File(file) => Stdio::from(io::io(file.try_clone())?),
        Input::Chunks(_) => Stdio::piped(),
    };
    let stdout = match streams.output {
        Some(file) => Stdio::from(io::io(file.try_clone())?),
        None => Stdio::piped(),
    };
    command
        .args(args)
        .env_remove("FFREPORT")
        .env("TZ", "UTC")
        .env("LC_ALL", "C")
        .stdin(stdin)
        .stdout(stdout)
        .stderr(Stdio::piped());
    let mut child = spawn(&mut command)?;
    let mut stdin = child.0.stdin.take();
    let mut stdout = child.0.stdout.take();
    let mut stderr = child.0.stderr.take().unwrap();
    if let Some(pipe) = &stdin {
        nonblocking(pipe.as_raw_fd())?;
    }
    if let Some(pipe) = &stdout {
        nonblocking(pipe.as_raw_fd())?;
    }
    nonblocking(stderr.as_raw_fd())?;
    let mut output = Vec::new();
    let mut error = Vec::new();
    let started = Instant::now();
    let mut out_open = stdout.is_some();
    let mut err_open = true;
    let mut status = None;
    let mut chunk = Vec::new();
    let mut offset = 0;
    let mut delivered = 0u64;
    while out_open || err_open || status.is_none() {
        if allow_interrupt && io::interrupted() {
            return Err("WORKER_INTERRUPTED".into());
        }
        cancel()?;
        io::check(started.elapsed() < timeout, "PROCESS_TIMEOUT")?;
        if status.is_none() {
            status = io::io(child.0.try_wait())?;
        }
        if stdin.is_some() && status.is_none() {
            if offset == chunk.len() {
                if let Input::Chunks(next) = &mut streams.input {
                    match next()? {
                        Some(bytes) => {
                            io::check(
                                !bytes.is_empty() && bytes.len() <= 16 * 1024 * 1024,
                                "PROCESS_INPUT_CHUNK_LIMIT",
                            )?;
                            chunk = bytes;
                            offset = 0;
                        }
                        None => {
                            stdin.take();
                        }
                    }
                }
            }
            if let Some(pipe) = &mut stdin {
                let end = (offset + 65536).min(chunk.len());
                match pipe.write(&chunk[offset..end]) {
                    Ok(0) => return Err("PROCESS_STDIN_CLOSED".into()),
                    Ok(n) => {
                        offset += n;
                        delivered += n as u64;
                        io::check(delivered <= 2 * 1024 * 1024 * 1024, "PROCESS_INPUT_LIMIT")?;
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            ErrorKind::WouldBlock | ErrorKind::Interrupted
                        ) => {}
                    Err(error) => return Err(format!("PROCESS_STDIN:{error}")),
                }
            }
        }
        if let Some(pipe) = &mut stdout {
            drain(pipe, &mut output, &mut out_open)?;
        }
        drain(&mut stderr, &mut error, &mut err_open)?;
        io::check(
            output.len() + error.len() <= 1024 * 1024,
            "PROCESS_OUTPUT_LIMIT",
        )?;
        if let Some(file) = streams.output {
            io::check(
                io::io(file.metadata())?.len() <= 2 * 1024 * 1024 * 1024,
                "MEDIA_SIZE",
            )?;
        }
        if out_open || err_open || status.is_none() {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    if !status.unwrap().success() {
        return Err(format!(
            "PROCESS_FAILED:{}",
            String::from_utf8_lossy(&error)
        ));
    }
    Ok(output)
}
pub fn plain(path: &Path, args: &[&str]) -> Result<Vec<u8>> {
    execute(
        path,
        &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        Duration::from_secs(60),
        false,
        || Ok(()),
    )
}
pub fn identity(pid: u64) -> Result<Option<String>> {
    io::check(pid > 0 && pid <= i32::MAX as u64, "INVALID_WORKER_PID")?;
    match plain(
        Path::new("/bin/ps"),
        &["-p", &pid.to_string(), "-o", "lstart="],
    ) {
        Ok(bytes) => {
            let value = String::from_utf8(bytes)
                .map_err(|e| e.to_string())?
                .trim()
                .to_string();
            Ok((!value.is_empty()).then_some(value))
        }
        Err(e) if e.starts_with("PROCESS_FAILED:") => Ok(None),
        Err(e) => Err(e),
    }
}
pub fn executable(configured: &str, label: &str) -> Result<PathBuf> {
    let candidates = if configured.contains('/') {
        vec![PathBuf::from(configured)]
    } else {
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|path| path.join(configured))
            .collect()
    };
    for candidate in candidates {
        if !candidate.exists() {
            continue;
        }
        let path = io::io(candidate.canonicalize())?;
        let meta = io::io(fs::metadata(&path))?;
        io::check(
            meta.is_file() && meta.permissions().mode() & 0o111 != 0,
            &format!("{label}_NOT_EXECUTABLE"),
        )?;
        return Ok(path);
    }
    Err(format!("{label}_UNAVAILABLE:{configured}"))
}
pub fn binding(path: &Path, version_option: &str) -> Result<Value> {
    let bytes = io::read(path, 512 * 1024 * 1024)?;
    let version = plain(path, &[version_option])?;
    let first = String::from_utf8(version)
        .map_err(|e| e.to_string())?
        .lines()
        .next()
        .unwrap_or("")
        .to_string();
    io::check(!first.is_empty(), "EXECUTABLE_VERSION_EMPTY")?;
    Ok(json!({"path":path,"sha256":io::sha(&bytes),"version":first}))
}
pub fn verify(value: &Value) -> Result<()> {
    let path = Path::new(value["path"].as_str().ok_or("EXECUTABLE_PATH")?);
    io::check(
        io::sha(&io::read(path, 512 * 1024 * 1024)?) == value["sha256"],
        "EXECUTABLE_CHANGED_AFTER_CLAIM",
    )
}
