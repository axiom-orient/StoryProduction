//! Atomic JSON and file operations shared by workspace persistence and handoff export.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static STAGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    atomic_write(path, &bytes)
}

pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    atomic_replace(path, |file| {
        file.write_all(bytes)?;
        Ok(())
    })
}

pub(super) fn atomic_replace(
    path: &Path,
    write: impl FnOnce(&mut File) -> Result<()>,
) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::InvalidInput("path has no parent".into()))?;
    fs::create_dir_all(parent)?;
    let sequence = STAGE_SEQUENCE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| Error::Conflict("file staging sequence exhausted".into()))?;
    let temp = parent.join(format!(
        ".{}.{}.{}.{}.tmp",
        path.file_name().and_then(|v| v.to_str()).unwrap_or("state"),
        std::process::id(),
        super::now_ms(),
        sequence,
    ));
    // Never remove a pre-existing temporary file owned by another operation.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let mut published = false;
    let result = (|| -> Result<()> {
        write(&mut file)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)?;
        published = true;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if !published {
        if let Err(error) = fs::remove_file(&temp) {
            return Err(Error::Io(std::io::Error::new(
                error.kind(),
                format!(
                "file replacement failed and staging cleanup failed: {error}; operation: {result:?}"
            ),
            )));
        }
    }
    match result {
        Err(error) if published => Err(Error::Io(std::io::Error::other(format!(
            "file replacement is visible but durability was not confirmed at {}: {error}",
            path.display()
        )))),
        result => result,
    }
}

pub(super) fn ensure_directory(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::InvalidInput(format!(
            "directory is not a regular directory: {}",
            path.display()
        )));
    }
    Ok(())
}

pub(super) fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes)
        .map_err(|error| Error::Corrupt(format!("invalid JSON at {}: {error}", path.display())))
}

pub(super) fn absolutize(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}
