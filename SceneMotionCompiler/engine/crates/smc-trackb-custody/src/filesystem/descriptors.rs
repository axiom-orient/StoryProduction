//! POSIX descriptor-relative operations and stable file identity primitives.

use crate::error::{CustodyError, Result};
use std::ffi::CString;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileIdentity {
    pub(crate) dev: u64,
    pub(crate) ino: u64,
}

pub(crate) fn ensure_directory_at(parent: &File, name: &str) -> io::Result<File> {
    let c_name = CString::new(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let result = unsafe { libc::mkdirat(parent.as_raw_fd(), c_name.as_ptr(), 0o700) };
    if result != 0 {
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::AlreadyExists {
            return Err(error);
        }
    }
    open_directory_at(parent, name)
}

pub(crate) fn open_exclusive_at(parent: &File, name: &str, mode: u32) -> io::Result<File> {
    let c_name = CString::new(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let flags = libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let fd = unsafe { libc::openat(parent.as_raw_fd(), c_name.as_ptr(), flags, mode) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

pub(crate) fn open_directory(path: &Path) -> io::Result<File> {
    let mut components = path.components();
    if components.next() != Some(Component::RootDir) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "directory path must be absolute",
        ));
    }
    let mut directory = open_directory_root()?;
    for component in components {
        let Component::Normal(name) = component else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "directory path contains a non-normal component",
            ));
        };
        let name = name.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "directory path contains non-UTF-8 component",
            )
        })?;
        directory = open_directory_at(&directory, name)?;
    }
    Ok(directory)
}

pub(crate) fn open_directory_root() -> io::Result<File> {
    let c_path = CString::new("/").expect("literal root path has no NUL");
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let fd = unsafe { libc::open(c_path.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

pub(crate) fn open_directory_at(parent: &File, name: &str) -> io::Result<File> {
    let c_name = CString::new(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let fd = unsafe { libc::openat(parent.as_raw_fd(), c_name.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

pub(crate) fn open_existing_directory_at(
    parent: &File,
    name: &str,
    label: &str,
) -> Result<Option<File>> {
    match open_directory_at(parent, name) {
        Ok(directory) => Ok(Some(directory)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(CustodyError::io(label, &error)),
    }
}

pub(crate) fn file_identity(file: &File) -> io::Result<FileIdentity> {
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file is not regular",
        ));
    }
    Ok(metadata_identity(&metadata))
}

pub(crate) fn directory_identity(file: &File) -> io::Result<FileIdentity> {
    let metadata = file.metadata()?;
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file is not a directory",
        ));
    }
    Ok(metadata_identity(&metadata))
}

pub(crate) fn metadata_identity(metadata: &std::fs::Metadata) -> FileIdentity {
    FileIdentity {
        dev: metadata.dev(),
        ino: metadata.ino(),
    }
}

pub(crate) fn stat_is_directory(stat: &libc::stat) -> bool {
    (stat.st_mode & libc::S_IFMT) == libc::S_IFDIR
}

pub(crate) fn stat_is_regular(stat: &libc::stat) -> bool {
    (stat.st_mode & libc::S_IFMT) == libc::S_IFREG
}

pub(crate) fn stat_identity(stat: &libc::stat) -> FileIdentity {
    FileIdentity {
        dev: stat.st_dev as u64,
        ino: stat.st_ino,
    }
}

pub(crate) fn fstatat_no_follow(parent: &File, name: &str) -> io::Result<libc::stat> {
    let c_name = CString::new(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            c_name.as_ptr(),
            &mut stat,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(stat)
}

pub(crate) fn sync_directory(directory: &File) -> io::Result<()> {
    directory.sync_all()
}

pub(crate) fn unlink_exact(parent: &File, name: &str, expected: FileIdentity) -> io::Result<()> {
    let c_name = CString::new(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
    let current_stat = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            c_name.as_ptr(),
            &mut stat,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if current_stat != 0 {
        return Err(io::Error::last_os_error());
    }
    if (stat.st_mode & libc::S_IFMT) != libc::S_IFREG
        || stat.st_dev as u64 != expected.dev
        || stat.st_ino as u64 != expected.ino
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "directory entry identity changed",
        ));
    }
    let result = unsafe { libc::unlinkat(parent.as_raw_fd(), c_name.as_ptr(), 0) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn unlink_at(parent: &File, name: &str) -> io::Result<()> {
    let c_name = CString::new(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let result = unsafe { libc::unlinkat(parent.as_raw_fd(), c_name.as_ptr(), 0) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn symlink_at(parent: &File, target: &str, name: &str) -> io::Result<()> {
    let target = CString::new(target)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let name = CString::new(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let result = unsafe { libc::symlinkat(target.as_ptr(), parent.as_raw_fd(), name.as_ptr()) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(crate) fn link_at(parent: &File, from: &str, to: &str) -> io::Result<()> {
    let from = CString::new(from)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let to = CString::new(to)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let result = unsafe {
        libc::linkat(
            parent.as_raw_fd(),
            from.as_ptr(),
            parent.as_raw_fd(),
            to.as_ptr(),
            0,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(crate) fn rename_at(parent: &File, from: &str, to: &str) -> io::Result<()> {
    let from = CString::new(from)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let to = CString::new(to)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let result = unsafe {
        libc::renameat(
            parent.as_raw_fd(),
            from.as_ptr(),
            parent.as_raw_fd(),
            to.as_ptr(),
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
