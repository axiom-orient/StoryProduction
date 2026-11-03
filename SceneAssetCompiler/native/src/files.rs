//! SAC filesystem boundary. All project paths are opened below one directory FD.
use crate::{check, Error, Result};
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
struct Temporary<'a> {
    directory: &'a File,
    name: CString,
    file: File,
}
impl Drop for Temporary<'_> {
    fn drop(&mut self) {
        let Ok(owned) = self.file.metadata() else {
            return;
        };
        let mut observed: libc::stat = unsafe { std::mem::zeroed() };
        let present = unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                self.name.as_ptr(),
                &mut observed,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if present == 0 && observed.st_dev as u64 == owned.dev() && observed.st_ino == owned.ino() {
            unsafe { libc::unlinkat(self.directory.as_raw_fd(), self.name.as_ptr(), 0) };
        }
    }
}

pub struct Root {
    directory: File,
}
impl Root {
    pub fn new(path: &Path) -> Result<Self> {
        let path = absolute(path)?;
        let info = std::fs::symlink_metadata(&path)?;
        check(
            info.is_dir() && !info.file_type().is_symlink(),
            "PATH_UNSAFE:project_root",
        )?;
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)?;
        check(
            directory.metadata()?.dev() == info.dev() && directory.metadata()?.ino() == info.ino(),
            "PROJECT_ROOT_CHANGED",
        )?;
        Ok(Self { directory })
    }
    fn parent(&self, relative: &str, create: bool) -> Result<(File, CString)> {
        let parts = parts(relative)?;
        let mut current = self.directory.try_clone()?;
        for part in &parts[..parts.len() - 1] {
            let name = CString::new(*part).map_err(|_| Error::refused("PATH_UNSAFE"))?;
            if create {
                let status = unsafe { libc::mkdirat(current.as_raw_fd(), name.as_ptr(), 0o755) };
                if status < 0
                    && std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST)
                {
                    return Err(std::io::Error::last_os_error().into());
                }
                if status == 0 {
                    current.sync_all()?;
                }
            }
            let fd = unsafe {
                libc::openat(
                    current.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(Error::refused(format!(
                    "PATH_UNSAFE:{}",
                    std::io::Error::last_os_error()
                )));
            }
            current = unsafe { File::from_raw_fd(fd) };
        }
        Ok((
            current,
            CString::new(*parts.last().unwrap()).map_err(|_| Error::refused("PATH_UNSAFE"))?,
        ))
    }
    pub fn read(&self, relative: &str, limit: usize) -> Result<Vec<u8>> {
        let (parent, name) = self.parent(relative, false)?;
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(Error::refused(format!(
                "INPUT_UNREADABLE:{}",
                std::io::Error::last_os_error()
            )));
        }
        capture(unsafe { File::from_raw_fd(fd) }, limit)
    }
    pub fn exists(&self, relative: &str) -> Result<bool> {
        let parts = parts(relative)?;
        let mut parent = self.directory.try_clone()?;
        for part in &parts[..parts.len() - 1] {
            let name = CString::new(*part).map_err(|_| Error::refused("PATH_UNSAFE"))?;
            let fd = unsafe {
                libc::openat(
                    parent.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::NotFound {
                    return Ok(false);
                }
                return Err(error.into());
            }
            parent = unsafe { File::from_raw_fd(fd) };
        }
        let name =
            CString::new(*parts.last().unwrap()).map_err(|_| Error::refused("PATH_UNSAFE"))?;
        let mut metadata: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe {
            libc::fstatat(
                parent.as_raw_fd(),
                name.as_ptr(),
                &mut metadata,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } < 0
        {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::NotFound {
                return Ok(false);
            }
            return Err(error.into());
        }
        check(
            metadata.st_mode & libc::S_IFMT == libc::S_IFREG,
            "ARTIFACT_NOT_REGULAR",
        )?;
        self.read(relative, 16 * 1024 * 1024)?;
        Ok(true)
    }
    pub fn publish(&self, relative: &str, raw: &[u8]) -> Result<()> {
        check(
            relative.starts_with("animatic/"),
            "OUTPUT_MUST_BE_UNDER_ANIMATIC",
        )?;
        let (parent, name) = self.parent(relative, true)?;
        let mut temporary = None;
        for _ in 0..100 {
            let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let temporary_name = CString::new(format!(
                ".sac-animatic-{}-{sequence}.tmp",
                std::process::id()
            ))
            .unwrap();
            let fd = unsafe {
                libc::openat(
                    parent.as_raw_fd(),
                    temporary_name.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o644,
                )
            };
            if fd < 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    continue;
                }
                return Err(Error::unknown(format!(
                    "Publication not confirmed; inspect {relative} before retrying: {error}"
                )));
            }
            temporary = Some(Temporary {
                directory: &parent,
                name: temporary_name,
                file: unsafe { File::from_raw_fd(fd) },
            });
            break;
        }
        let mut temporary =
            temporary.ok_or_else(|| Error::unknown("Could not reserve publication temporary"))?;
        temporary
            .file
            .write_all(raw)
            .and_then(|_| temporary.file.sync_all())
            .map_err(|e| {
                Error::unknown(format!(
                    "Publication not confirmed; inspect {relative} before retrying: {e}"
                ))
            })?;
        let linked = unsafe {
            libc::linkat(
                parent.as_raw_fd(),
                temporary.name.as_ptr(),
                parent.as_raw_fd(),
                name.as_ptr(),
                0,
            )
        };
        if linked < 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::EEXIST) {
                check(
                    self.read(relative, raw.len())? == raw,
                    "IMMUTABLE_ARTIFACT_CONFLICT",
                )?;
            } else {
                return Err(Error::unknown(format!(
                    "Publication not confirmed; inspect {relative} before retrying: {error}"
                )));
            }
        }
        parent.sync_all().map_err(|e| {
            Error::unknown(format!(
                "Publication not confirmed; inspect {relative} before retrying: {e}"
            ))
        })
    }
}

fn parts(relative: &str) -> Result<Vec<&str>> {
    check(
        !relative.is_empty()
            && !relative.contains(['\\', ':'])
            && Path::new(relative)
                .components()
                .all(|p| matches!(p, Component::Normal(_)))
            && relative
                .split('/')
                .all(|p| !p.is_empty() && p != "." && p != ".."),
        "PATH_UNSAFE",
    )?;
    Ok(relative.split('/').collect())
}
fn absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}
fn capture(mut file: File, limit: usize) -> Result<Vec<u8>> {
    let before = file.metadata()?;
    check(before.is_file(), "ARTIFACT_NOT_REGULAR")?;
    check(before.len() <= limit as u64, "INPUT_TOO_LARGE")?;
    let mut raw = Vec::new();
    Read::by_ref(&mut file)
        .take(limit as u64 + 1)
        .read_to_end(&mut raw)?;
    let after = file.metadata()?;
    check(
        raw.len() <= limit
            && raw.len() as u64 == before.len()
            && before.len() == after.len()
            && before.mtime() == after.mtime()
            && before.mtime_nsec() == after.mtime_nsec()
            && before.ctime() == after.ctime()
            && before.ctime_nsec() == after.ctime_nsec(),
        "INPUT_CHANGED_DURING_READ",
    )?;
    Ok(raw)
}
pub fn read_absolute(path: &Path, limit: usize) -> Result<Vec<u8>> {
    capture(
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(path)?,
        limit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existence_and_replay_follow_the_selected_root_after_path_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let selected = temp.path().join("selected");
        let moved = temp.path().join("moved");
        std::fs::create_dir(&selected).unwrap();
        let root = Root::new(&selected).unwrap();
        root.publish("animatic/review.json", b"existing receipt")
            .unwrap();
        std::fs::rename(&selected, &moved).unwrap();
        std::fs::create_dir(&selected).unwrap();
        assert!(root.exists("animatic/review.json").unwrap());
        assert!(!root.exists("animatic/absent.json").unwrap());
        root.publish("animatic/review.json", b"existing receipt")
            .unwrap();
        assert!(!selected.join("animatic").exists());
        std::fs::create_dir_all(selected.join("animatic")).unwrap();
        std::fs::write(selected.join("animatic/absent.json"), b"replacement").unwrap();
        assert!(!root.exists("animatic/absent.json").unwrap());
    }
    #[test]
    fn concurrent_same_bytes_publish_once_and_conflicting_bytes_preserve_the_winner() {
        let temp = tempfile::tempdir().unwrap();
        let root = std::sync::Arc::new(Root::new(&temp.path().canonicalize().unwrap()).unwrap());
        let workers = (0..8)
            .map(|_| {
                let root = root.clone();
                std::thread::spawn(move || {
                    root.publish("animatic/nested/pack.json", b"same bytes\n")
                })
            })
            .collect::<Vec<_>>();
        for worker in workers {
            worker.join().unwrap().unwrap();
        }
        assert_eq!(
            root.read("animatic/nested/pack.json", 100).unwrap(),
            b"same bytes\n"
        );
        assert!(root
            .publish("animatic/nested/pack.json", b"different bytes\n")
            .is_err());
        assert_eq!(
            root.read("animatic/nested/pack.json", 100).unwrap(),
            b"same bytes\n"
        );
        assert!(std::fs::read_dir(temp.path().join("animatic/nested"))
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")));
    }
}
