use crate::error::{Error, Result};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, OnceLock};

pub const WORKSPACE_FD_ENV: &str = "STORY_SEMI_WORKSPACE_FD";

pub(crate) fn absolute(path: &Path) -> Result<PathBuf> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(Error::not_applied("parent path traversal is not accepted"));
    }
    std::path::absolute(path).map_err(Error::from)
}

pub(crate) fn descriptor_binding(path: &Path) -> Result<Option<(PathBuf, i32)>> {
    let Some(raw) = std::env::var_os(WORKSPACE_FD_ENV) else {
        return Ok(None);
    };
    let raw = raw
        .to_str()
        .ok_or_else(|| Error::not_applied("invalid SEMI workspace descriptor"))?;
    let descriptor = raw
        .parse::<i32>()
        .ok()
        .filter(|value| *value >= 0 && value.to_string() == raw)
        .ok_or_else(|| Error::not_applied("invalid SEMI workspace descriptor"))?;
    let root = PathBuf::from("/dev/fd").join(raw);
    let absolute = absolute(path)?;
    if !absolute.starts_with(&root) {
        return Ok(None);
    }
    let mut opened: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(descriptor, &mut opened) } != 0 {
        return Err(Error::not_applied(
            "SEMI workspace descriptor is unavailable",
        ));
    }
    use std::os::unix::fs::MetadataExt;
    // Darwin's devfs stat("/dev/fd/N") exposes a synthetic device number.
    // Reopening the descriptor path and fstat-ing it observes the real directory.
    let current = std::fs::File::open(&root)
        .and_then(|file| file.metadata())
        .map_err(|_| Error::not_applied("SEMI workspace descriptor is unavailable"))?;
    if opened.st_mode & libc::S_IFMT != libc::S_IFDIR
        || !current.is_dir()
        || opened.st_dev as u64 != current.dev()
        || opened.st_ino as u64 != current.ino()
    {
        return Err(Error::not_applied(
            "SEMI workspace descriptor is not the selected directory",
        ));
    }
    Ok(Some((root, descriptor)))
}

pub fn workspace_descriptor_for_argument(path: &Path) -> Result<Option<i32>> {
    let absolute = absolute(path)?;
    let Some((root, descriptor)) = descriptor_binding(&absolute)? else {
        if std::env::var_os(WORKSPACE_FD_ENV).is_some() {
            return Err(Error::not_applied(
                "SEMI workspace descriptor does not match the workspace argument",
            ));
        }
        return Ok(None);
    };
    if root != absolute {
        return Err(Error::not_applied(
            "SEMI workspace argument must name the descriptor root",
        ));
    }
    Ok(Some(descriptor))
}

/// One invocation's selected workspace. Ordinary CLI roots and borrowed SEMI
/// descriptors become owned directory capabilities without process-global state.
pub(crate) struct WorkspaceBinding {
    display_path: PathBuf,
    supplied_descriptor: bool,
    directory: OnceLock<Arc<super::files::Directory>>,
}

impl WorkspaceBinding {
    pub(crate) fn new(path: &Path) -> Result<Self> {
        let descriptor = workspace_descriptor_for_argument(path)?;
        let display_path = super::files::non_symlink_path(path)?;
        let directory = OnceLock::new();
        if let Some(descriptor) = descriptor {
            let _ = directory.set(Arc::new(super::files::Directory::from_fd(descriptor)?));
        } else if super::files::directory_exists(&display_path)? {
            let _ = directory.set(Arc::new(super::files::Directory::open(
                &display_path,
                false,
            )?));
        }
        Ok(Self {
            display_path,
            supplied_descriptor: descriptor.is_some(),
            directory,
        })
    }

    pub(crate) fn display_path(&self) -> &Path {
        &self.display_path
    }

    pub(crate) fn directory(&self) -> Result<Arc<super::files::Directory>> {
        self.directory.get().cloned().ok_or_else(|| {
            Error::not_applied("workspace does not exist; create it from validated source first")
        })
    }

    /// Called only after create has admitted source bytes and its encoded limit.
    pub(crate) fn create(&self) -> Result<Arc<super::files::Directory>> {
        if self.directory.get().is_none() {
            let directory = Arc::new(super::files::Directory::open(&self.display_path, true)?);
            let _ = self.directory.set(directory);
        }
        self.ensure_current()?;
        self.directory()
    }

    pub(crate) fn ensure_current(&self) -> Result<()> {
        if !self.supplied_descriptor {
            self.directory()?.verify_path(&self.display_path)?;
        }
        Ok(())
    }
}
