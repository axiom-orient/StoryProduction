//! An executable transport selects one directory by handle before domain dispatch.
//! Relative domain paths remain attached to this process directory after rename.
use crate::{Error, Result};
use std::fs::File;
use std::path::{Component, Path};

pub struct SelectedWorkingDirectory {
    #[cfg(unix)]
    previous: File,
    #[cfg(unix)]
    _selected: File,
}
impl SelectedWorkingDirectory {
    /// Call before starting threads or domain effects. The executable owns this
    /// process-global selection; this is not a concurrent library workspace API.
    pub fn enter(path: &Path) -> Result<Self> {
        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::os::fd::{AsRawFd, FromRawFd};
            use std::os::unix::ffi::OsStrExt;
            if path.as_os_str().is_empty()
                || path
                    .components()
                    .any(|part| matches!(part, Component::ParentDir | Component::Prefix(_)))
            {
                return Err(Error::Contract(
                    "selected root must be nonempty and contain no parent traversal".into(),
                ));
            }
            let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
            let take = |fd| -> Result<File> {
                if fd < 0 {
                    return Err(std::io::Error::last_os_error().into());
                }
                Ok(unsafe { File::from_raw_fd(fd) })
            };
            let previous = take(unsafe { libc::open(c".".as_ptr(), flags) })?;
            let mut selected = if path.is_absolute() {
                take(unsafe { libc::open(c"/".as_ptr(), flags) })?
            } else {
                previous.try_clone()?
            };
            for part in path.components() {
                if let Component::Normal(part) = part {
                    let part = CString::new(part.as_bytes())
                        .map_err(|_| Error::Contract("selected root contains NUL".into()))?;
                    selected =
                        take(unsafe { libc::openat(selected.as_raw_fd(), part.as_ptr(), flags) })?;
                }
            }
            if unsafe { libc::fchdir(selected.as_raw_fd()) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(Self {
                previous,
                _selected: selected,
            })
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err(Error::Contract(
                "project-root selection requires POSIX directory handles".into(),
            ))
        }
    }
}
impl Drop for SelectedWorkingDirectory {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            unsafe {
                libc::fchdir(self.previous.as_raw_fd());
            }
        }
    }
}
