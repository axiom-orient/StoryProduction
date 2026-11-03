use crate::error::{Error, Result};
use serde_json::Value;
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn c_name(value: &std::ffi::OsStr) -> Result<CString> {
    CString::new(value.as_bytes()).map_err(|_| Error::not_applied("path contains a NUL byte"))
}

fn take_fd(descriptor: i32) -> Result<File> {
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub fn non_symlink_path(path: &Path) -> Result<PathBuf> {
    let absolute = super::workspace::absolute(path)?;
    let binding = super::workspace::descriptor_binding(&absolute)?;
    let (mut current, relative) = match binding {
        Some((root, _)) => (
            root.clone(),
            absolute.strip_prefix(root).unwrap().to_path_buf(),
        ),
        None => (
            PathBuf::from("/"),
            absolute.strip_prefix("/").unwrap().to_path_buf(),
        ),
    };
    for component in relative.components() {
        if let Component::Normal(part) = component {
            current.push(part);
            match std::fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(Error::not_applied(format!(
                        "symlink paths are not accepted: {}",
                        current.display()
                    )));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(absolute)
}

pub fn create_directories(path: &Path) -> Result<()> {
    let directory = directory_walk(path, true, false)?.unwrap();
    directory.sync_all()?;
    Ok(())
}

pub(crate) fn open_directory(path: &Path) -> Result<File> {
    Ok(directory_walk(path, false, false)?.unwrap())
}

pub fn directory_exists(path: &Path) -> Result<bool> {
    Ok(directory_walk(path, false, true)?.is_some())
}

pub fn path_exists(path: &Path) -> Result<bool> {
    let path = non_symlink_path(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| Error::not_applied("path has no parent"))?;
    let Some(directory) = directory_walk(parent, false, true)? else {
        return Ok(false);
    };
    let name = c_name(
        path.file_name()
            .ok_or_else(|| Error::not_applied("path has no leaf"))?,
    )?;
    let mut metadata: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            &mut metadata,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        let error = std::io::Error::last_os_error();
        return if error.kind() == std::io::ErrorKind::NotFound {
            Ok(false)
        } else {
            Err(error.into())
        };
    }
    if metadata.st_mode & libc::S_IFMT == libc::S_IFLNK {
        return Err(Error::not_applied("symlink path is not accepted"));
    }
    Ok(true)
}

pub fn list_json_files(path: &Path) -> Result<Vec<PathBuf>> {
    Directory::open(path, false)?
        .list_json_files()
        .map(|names| names.into_iter().map(|name| path.join(name)).collect())
}

pub fn remove_empty_directory(path: &Path) -> Result<()> {
    let (directory, name) = parent(path, false)?;
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    directory.sync_all()?;
    Ok(())
}

/// Atomically claim one fresh directory. An existing directory is not replay.
pub fn reserve_directory(path: &Path) -> Result<bool> {
    let (directory, name) = parent(path, true)?;
    if unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            let existing = take_fd(unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            })?;
            if !existing.metadata()?.is_dir() {
                return Err(Error::not_applied("reserved directory is not a directory"));
            }
            return Ok(false);
        }
        return Err(error.into());
    }
    directory.sync_all()?;
    Ok(true)
}

// Walk from an opened directory using no-follow openat. A renamed or replaced
// parent path cannot redirect a file read/publication after this anchor opens.
fn parent(path: &Path, create: bool) -> Result<(File, CString)> {
    let absolute = non_symlink_path(path)?;
    let file_name = absolute
        .file_name()
        .ok_or_else(|| Error::not_applied("file path has no leaf"))?;
    let parent_path = absolute
        .parent()
        .ok_or_else(|| Error::not_applied("file path has no parent"))?;
    Ok((
        directory_walk(parent_path, create, false)?.unwrap(),
        c_name(file_name)?,
    ))
}

fn directory_walk(path: &Path, create: bool, missing_ok: bool) -> Result<Option<File>> {
    let absolute = non_symlink_path(path)?;
    let binding = super::workspace::descriptor_binding(&absolute)?;
    let (directory, relative) = match binding {
        Some((root, descriptor)) => (
            take_fd(unsafe { libc::fcntl(descriptor, libc::F_DUPFD_CLOEXEC, 0) })?,
            absolute
                .strip_prefix(root)
                .map_err(|_| Error::not_applied("file escaped workspace descriptor"))?
                .to_path_buf(),
        ),
        None => (
            take_fd(unsafe {
                libc::open(
                    c"/".as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
                )
            })?,
            absolute.strip_prefix("/").unwrap().to_path_buf(),
        ),
    };
    walk_from(directory, &relative, create, missing_ok)
}

fn walk_from(
    mut directory: File,
    relative: &Path,
    create: bool,
    missing_ok: bool,
) -> Result<Option<File>> {
    for component in relative.components() {
        if let Component::Normal(part) = component {
            let name = c_name(part)?;
            let flags = libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK;
            let mut descriptor =
                unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
            if descriptor < 0
                && create
                && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound
            {
                let status = unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o755) };
                if status != 0
                    && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err(std::io::Error::last_os_error().into());
                }
                directory.sync_all()?;
                descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
            }
            if descriptor < 0
                && missing_ok
                && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound
            {
                return Ok(None);
            }
            directory = take_fd(descriptor)?;
        }
    }
    Ok(Some(directory))
}

/// One selected directory for the entire transport operation. Children are
/// opened relative to this FD; replacing its public pathname cannot redirect I/O.
pub struct Directory {
    file: File,
    path: PathBuf,
    original_path: bool,
}

impl Directory {
    pub fn open(path: &Path, create: bool) -> Result<Self> {
        let path = non_symlink_path(path)?;
        let file = directory_walk(&path, create, false)?.unwrap();
        Ok(Self {
            file,
            path,
            original_path: true,
        })
    }

    pub fn from_fd(descriptor: i32) -> Result<Self> {
        let file = take_fd(unsafe { libc::fcntl(descriptor, libc::F_DUPFD_CLOEXEC, 0) })?;
        if !file.metadata()?.is_dir() {
            return Err(Error::not_applied("descriptor is not a directory"));
        }
        let path = PathBuf::from(format!("/dev/fd/{}", file.as_raw_fd()));
        Ok(Self {
            file,
            path,
            original_path: false,
        })
    }

    pub fn raw_fd(&self) -> i32 {
        self.file.as_raw_fd()
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn identity(&self) -> Result<(u64, u64)> {
        let metadata = self.file.metadata()?;
        Ok((metadata.dev(), metadata.ino()))
    }

    fn relative(path: &Path, empty_ok: bool) -> Result<()> {
        if (path.as_os_str().is_empty() && !empty_ok)
            || path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(Error::not_applied(
                "directory-relative path must contain only normal components",
            ));
        }
        Ok(())
    }

    pub fn child(&self, relative: impl AsRef<Path>, create: bool) -> Result<Self> {
        self.child_impl(relative.as_ref(), create, false)?
            .ok_or_else(|| Error::not_applied("directory is missing"))
    }
    pub fn child_optional(&self, relative: impl AsRef<Path>) -> Result<Option<Self>> {
        self.child_impl(relative.as_ref(), false, true)
    }
    fn child_impl(&self, relative: &Path, create: bool, missing_ok: bool) -> Result<Option<Self>> {
        Self::relative(relative, true)?;
        Ok(
            walk_from(self.file.try_clone()?, relative, create, missing_ok)?.map(|file| Self {
                file,
                path: self.path.join(relative),
                original_path: self.original_path,
            }),
        )
    }

    fn parent(
        &self,
        relative: &Path,
        create: bool,
        missing_ok: bool,
    ) -> Result<Option<(Self, CString)>> {
        Self::relative(relative, false)?;
        let name = c_name(
            relative
                .file_name()
                .ok_or_else(|| Error::not_applied("path has no leaf"))?,
        )?;
        Ok(self
            .child_impl(
                relative.parent().unwrap_or(Path::new("")),
                create,
                missing_ok,
            )?
            .map(|parent| (parent, name)))
    }

    pub fn verify_identity(&self) -> Result<()> {
        if self.original_path {
            self.verify_path(&self.path)?;
        }
        Ok(())
    }
    pub fn verify_path(&self, path: &Path) -> Result<()> {
        let current = open_directory(path)?.metadata()?;
        if self.identity()? != (current.dev(), current.ino()) {
            return Err(Error::not_applied("selected directory identity changed"));
        }
        Ok(())
    }
    pub fn verify_child(&self, name: &str, expected: &Self) -> Result<()> {
        Self::relative(Path::new(name), false)?;
        if Path::new(name).components().count() != 1 {
            return Err(Error::not_applied(
                "child identity name must be one component",
            ));
        }
        if self.child(name, false)?.identity()? != expected.identity()? {
            return Err(Error::not_applied(
                "selected child directory identity changed",
            ));
        }
        Ok(())
    }
    pub fn reserve_child(&self, name: &str) -> Result<Option<Self>> {
        Self::relative(Path::new(name), false)?;
        if Path::new(name).components().count() != 1 {
            return Err(Error::not_applied(
                "directory reservation name must be one component",
            ));
        }
        let name_c = c_name(std::ffi::OsStr::new(name))?;
        if unsafe { libc::mkdirat(self.raw_fd(), name_c.as_ptr(), 0o700) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                self.child(name, false)?;
                return Ok(None);
            }
            return Err(error.into());
        }
        self.file.sync_all()?;
        let child = self.child(name, false)?;
        self.verify_child(name, &child)?;
        Ok(Some(child))
    }

    pub fn exists(&self, relative: impl AsRef<Path>) -> Result<bool> {
        let Some((parent, name)) = self.parent(relative.as_ref(), false, true)? else {
            return Ok(false);
        };
        let Some(metadata) = metadata_at(&parent.file, &name)? else {
            return Ok(false);
        };
        if metadata.st_mode & libc::S_IFMT == libc::S_IFLNK {
            return Err(Error::not_applied("symlink path is not accepted"));
        }
        Ok(true)
    }
    pub fn read(&self, relative: impl AsRef<Path>) -> Result<Vec<u8>> {
        self.read_bytes_limited(relative, super::MAX_BYTES)
    }
    pub fn read_bytes_limited(
        &self,
        relative: impl AsRef<Path>,
        max_bytes: usize,
    ) -> Result<Vec<u8>> {
        let (parent, name) = self.parent(relative.as_ref(), false, false)?.unwrap();
        read_opened(opened_at(&parent.file, &name)?, max_bytes)
    }
    pub fn immutable(&self, relative: impl AsRef<Path>, data: &[u8]) -> Result<()> {
        let (parent, name) = self.parent(relative.as_ref(), true, false)?.unwrap();
        immutable_in(&parent.file, &name, data)
    }
    pub fn exclusive(&self, relative: impl AsRef<Path>, data: &[u8]) -> Result<File> {
        let (parent, name) = self.parent(relative.as_ref(), true, false)?.unwrap();
        exclusive_in(&parent.file, &name, data)
    }
    pub fn atomic_write_json(&self, relative: impl AsRef<Path>, value: &Value) -> Result<()> {
        let data = super::json::stable_bytes(value)?;
        let (parent, name) = self.parent(relative.as_ref(), true, false)?.unwrap();
        atomic_write_in(&parent.file, &name, &data)
    }
    pub fn lock(&self, relative: &str) -> Result<SubmissionLock> {
        let (parent, name) = self.parent(Path::new(relative), true, false)?.unwrap();
        submission_lock_in(&parent.file, &name)
    }
    pub fn submission_lock(&self) -> Result<SubmissionLock> {
        self.lock("submission.lock")
    }

    pub fn remove_file(&self, relative: impl AsRef<Path>, expected: &File) -> Result<()> {
        let (parent, name) = self.parent(relative.as_ref(), false, false)?.unwrap();
        let current = metadata_at(&parent.file, &name)?
            .ok_or_else(|| Error::not_applied("claimed file disappeared"))?;
        let held = expected.metadata()?;
        if current.st_mode & libc::S_IFMT != libc::S_IFREG
            || (current.st_dev as u64, current.st_ino as u64) != (held.dev(), held.ino())
        {
            return Err(Error::not_applied(
                "claimed file identity changed; cleanup refused",
            ));
        }
        if unsafe { libc::unlinkat(parent.raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        parent.file.sync_all()?;
        Ok(())
    }
    pub fn remove_empty_child(&self, name: &str, expected: &Self) -> Result<()> {
        self.verify_child(name, expected)?;
        let name = c_name(std::ffi::OsStr::new(name))?;
        if unsafe { libc::unlinkat(self.raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        self.file.sync_all()?;
        Ok(())
    }

    pub fn is_empty(&self) -> Result<bool> {
        self.names(true).map(|names| names.is_empty())
    }
    pub fn list_json_files(&self) -> Result<Vec<PathBuf>> {
        let mut result = vec![];
        for name in self.names(false)? {
            if !name.as_bytes().ends_with(b".json") {
                continue;
            }
            let name_c = c_name(&name)?;
            let metadata = metadata_at(&self.file, &name_c)?
                .ok_or_else(|| Error::not_applied("directory entry changed"))?;
            if metadata.st_mode & libc::S_IFMT != libc::S_IFREG {
                return Err(Error::not_applied(
                    "JSON directory entry is not a regular file",
                ));
            }
            result.push(PathBuf::from(name));
        }
        result.sort();
        Ok(result)
    }
    fn names(&self, first_only: bool) -> Result<Vec<std::ffi::OsString>> {
        let duplicate = unsafe { libc::fcntl(self.raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
        if duplicate < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let stream = unsafe { libc::fdopendir(duplicate) };
        if stream.is_null() {
            unsafe {
                libc::close(duplicate);
            }
            return Err(std::io::Error::last_os_error().into());
        }
        struct Stream(*mut libc::DIR);
        impl Drop for Stream {
            fn drop(&mut self) {
                unsafe {
                    libc::closedir(self.0);
                }
            }
        }
        let stream = Stream(stream);
        unsafe {
            libc::rewinddir(stream.0);
        }
        let mut names = vec![];
        let mut bytes = 0usize;
        loop {
            #[cfg(target_os = "macos")]
            unsafe {
                *libc::__error() = 0;
            }
            #[cfg(target_os = "linux")]
            unsafe {
                *libc::__errno_location() = 0;
            }
            let entry = unsafe { libc::readdir(stream.0) };
            if entry.is_null() {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() != Some(0) {
                    return Err(error.into());
                }
                break;
            }
            let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
            if !matches!(name.to_bytes(), b"." | b"..") {
                bytes += name.to_bytes().len();
                if bytes > super::MAX_BYTES {
                    return Err(Error::not_applied(
                        "directory listing exceeds its byte limit",
                    ));
                }
                names.push(std::ffi::OsStr::from_bytes(name.to_bytes()).to_owned());
                if first_only {
                    break;
                }
            }
        }
        Ok(names)
    }
}

fn metadata_at(directory: &File, name: &CString) -> Result<Option<libc::stat>> {
    let mut metadata = unsafe { std::mem::zeroed() };
    if unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            &mut metadata,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        let error = std::io::Error::last_os_error();
        return if error.kind() == std::io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error.into())
        };
    }
    Ok(Some(metadata))
}

fn opened_at(directory: &File, name: &CString) -> Result<File> {
    let mut before: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            &mut before,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    if before.st_mode & libc::S_IFMT != libc::S_IFREG {
        return Err(Error::not_applied("input is not a regular file"));
    }
    let file = take_fd(unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    })?;
    let opened = file.metadata()?;
    if !opened.is_file()
        || opened.dev() != before.st_dev as u64
        || opened.ino() != before.st_ino as u64
        || opened.len() != before.st_size as u64
    {
        return Err(Error::not_applied("input changed or is not a regular file"));
    }
    Ok(file)
}

pub(crate) fn open_verified(path: &Path) -> Result<File> {
    let (directory, name) = parent(path, false)?;
    opened_at(&directory, &name)
}

fn read_opened(file: File, max_bytes: usize) -> Result<Vec<u8>> {
    let before = file.metadata()?;
    if before.len() > max_bytes as u64 {
        return Err(Error::not_applied("input exceeds its byte limit"));
    }
    let mut data = Vec::with_capacity((before.len() as usize).min(max_bytes));
    (&file).take(max_bytes as u64 + 1).read_to_end(&mut data)?;
    let after = file.metadata()?;
    if data.len() > max_bytes {
        return Err(Error::not_applied("input exceeds its byte limit"));
    }
    if data.len() as u64 != before.len()
        || after.len() != before.len()
        || after.dev() != before.dev()
        || after.ino() != before.ino()
        || after.mtime() != before.mtime()
        || after.mtime_nsec() != before.mtime_nsec()
        || after.ctime() != before.ctime()
        || after.ctime_nsec() != before.ctime_nsec()
    {
        return Err(Error::not_applied("input changed while reading"));
    }
    Ok(data)
}

pub fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    read_bytes_limited(path, super::MAX_BYTES)
}

pub fn read_bytes_limited(path: &Path, max_bytes: usize) -> Result<Vec<u8>> {
    read_opened(open_verified(path)?, max_bytes)
}

pub fn read_text(path: &Path) -> Result<String> {
    String::from_utf8(read_bytes(path)?).map_err(|_| Error::not_applied("input is not UTF-8"))
}

pub fn read_at(directory_fd: i32, relative_path: &str) -> Result<Vec<u8>> {
    if relative_path.is_empty()
        || relative_path.contains('/')
        || matches!(relative_path, "." | "..")
    {
        return Err(Error::not_applied(
            "descriptor-relative file names must be one path component",
        ));
    }
    let directory = take_fd(unsafe { libc::fcntl(directory_fd, libc::F_DUPFD_CLOEXEC, 0) })?;
    if !directory.metadata()?.is_dir() {
        return Err(Error::not_applied(
            "workspace descriptor is not a directory",
        ));
    }
    let name = CString::new(relative_path)
        .map_err(|_| Error::not_applied("file name contains a NUL byte"))?;
    read_opened(opened_at(&directory, &name)?, super::MAX_BYTES)
}

struct Temporary<'a> {
    directory: &'a File,
    name: CString,
    file: File,
}
impl Drop for Temporary<'_> {
    fn drop(&mut self) {
        unsafe {
            libc::unlinkat(self.directory.as_raw_fd(), self.name.as_ptr(), 0);
        }
    }
}

fn temporary<'a>(directory: &'a File, data: &[u8]) -> Result<Temporary<'a>> {
    for _ in 0..100 {
        let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name =
            CString::new(format!(".story-text-{}-{sequence}.tmp", std::process::id())).unwrap();
        let descriptor = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if descriptor < 0
            && std::io::Error::last_os_error().kind() == std::io::ErrorKind::AlreadyExists
        {
            continue;
        }
        let mut temporary = Temporary {
            directory,
            name,
            file: take_fd(descriptor)?,
        };
        temporary.file.write_all(data)?;
        temporary.file.sync_all()?;
        return Ok(temporary);
    }
    Err(Error::not_applied(
        "could not reserve a temporary publication file",
    ))
}

pub fn immutable(path: &Path, data: &[u8]) -> Result<()> {
    let (directory, name) = parent(path, true)?;
    immutable_in(&directory, &name, data)
}

fn immutable_in(directory: &File, name: &CString, data: &[u8]) -> Result<()> {
    let temporary = temporary(directory, data)?;
    if unsafe {
        libc::linkat(
            directory.as_raw_fd(),
            temporary.name.as_ptr(),
            directory.as_raw_fd(),
            name.as_ptr(),
            0,
        )
    } != 0
    {
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(error.into());
        }
        if read_opened(opened_at(directory, name)?, data.len())? != data {
            return Err(Error::not_applied(format!(
                "immutable output already differs: {}",
                name.to_string_lossy()
            )));
        }
    }
    directory.sync_all()?;
    Ok(())
}

/// Hold one edition directory across staging, re-observation and its final marker.
/// An edition must never continue publication into a replacement of that path.
pub(crate) struct PublicationDirectory(Directory);

impl PublicationDirectory {
    pub(crate) fn new(path: &Path) -> Result<Self> {
        Ok(Self(Directory::open(path, true)?))
    }

    pub(crate) fn immutable(&self, name: &str, data: &[u8]) -> Result<()> {
        self.0.immutable(name, data)
    }

    pub(crate) fn verify(
        &self,
        captured: &std::collections::BTreeMap<String, Vec<u8>>,
    ) -> Result<()> {
        self.0.verify_identity()?;
        for (name, expected) in captured {
            let actual = self.0.read(name)?;
            if actual != *expected {
                return Err(Error::not_applied(format!(
                    "published artifact changed: {name}; edition not committed"
                )));
            }
        }
        Ok(())
    }
}

/// Claim a single execution marker. Even an equal existing marker is a conflict.
pub fn exclusive(path: &Path, data: &[u8]) -> Result<()> {
    let (directory, name) = parent(path, true)?;
    exclusive_in(&directory, &name, data).map(|_| ())
}

fn exclusive_in(directory: &File, name: &CString, data: &[u8]) -> Result<File> {
    let temporary = temporary(directory, data)?;
    if unsafe {
        libc::linkat(
            directory.as_raw_fd(),
            temporary.name.as_ptr(),
            directory.as_raw_fd(),
            name.as_ptr(),
            0,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    directory
        .sync_all()
        .map_err(|e| Error::unknown(format!("exclusive claim durability not confirmed: {e}")))?;
    let current = metadata_at(directory, name)?
        .ok_or_else(|| Error::unknown("exclusive claim disappeared"))?;
    let held = temporary.file.metadata()?;
    if (current.st_dev as u64, current.st_ino as u64) != (held.dev(), held.ino()) {
        return Err(Error::unknown("exclusive claim identity changed"));
    }
    temporary.file.try_clone().map_err(Error::from)
}

pub fn atomic_write_json(path: &Path, value: &Value) -> Result<()> {
    let data = super::json::stable_bytes(value)?;
    let (directory, name) = parent(path, true)?;
    atomic_write_in(&directory, &name, &data)
}

fn atomic_write_in(directory: &File, name: &CString, data: &[u8]) -> Result<()> {
    // Avoid replacing a FIFO, directory, or symlink by mutable job state.
    let mut existing: libc::stat = unsafe { std::mem::zeroed() };
    let status = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            &mut existing,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if status == 0 && existing.st_mode & libc::S_IFMT != libc::S_IFREG {
        return Err(Error::not_applied(
            "mutable JSON output is not a regular file",
        ));
    }
    if status != 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::NotFound {
        return Err(std::io::Error::last_os_error().into());
    }
    let temporary = temporary(directory, data)?;
    if unsafe {
        libc::renameat(
            directory.as_raw_fd(),
            temporary.name.as_ptr(),
            directory.as_raw_fd(),
            name.as_ptr(),
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    directory.sync_all()?;
    Ok(())
}

pub struct SubmissionLock {
    _file: File,
}

impl Drop for SubmissionLock {
    fn drop(&mut self) {
        // Another thread's fork can briefly retain this CLOEXEC descriptor.
        // Release ownership explicitly instead of waiting for that child to exec.
        unsafe {
            libc::flock(self._file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

pub fn submission_lock(directory: &Path) -> Result<SubmissionLock> {
    let (parent, name) = parent(&directory.join("submission.lock"), false)?;
    submission_lock_in(&parent, &name)
}

fn submission_lock_in(parent: &File, name: &CString) -> Result<SubmissionLock> {
    let file = take_fd(unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            0o600,
        )
    })?;
    if !file.metadata()?.is_file() {
        return Err(Error::not_applied(
            "attempt submission lock is not a regular file",
        ));
    }
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let mut error =
            Error::unknown("attempt submission is already in progress; inspect it before retrying");
        error.spawned = false;
        return Err(error);
    }
    Ok(SubmissionLock { _file: file })
}
