//! Exclusive lock acquisition and RAII release for workspace mutations.

use super::now_ms;
use crate::error::{Error, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub(super) struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(super) fn acquire_lock(path: &Path) -> Result<LockGuard> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            // A failed lock record write must release the lock we just created.
            let guard = LockGuard {
                path: path.to_path_buf(),
            };
            writeln!(
                file,
                "pid={} created_at_ms={}",
                std::process::id(),
                now_ms()
            )?;
            file.sync_all()?;
            Ok(guard)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(Error::Busy(format!(
                "workspace is locked: {}. Inspect the owner before explicit recovery",
                path.display()
            )))
        }
        Err(error) => Err(Error::Io(error)),
    }
}
