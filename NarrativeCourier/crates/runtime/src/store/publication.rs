//! Exclusive staging and explicit post-rename durability outcomes.

use crate::{Error, Result};
use std::path::Path;

#[cfg(not(windows))]
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(not(windows))]
static STAGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(not(windows))]
fn target_parent(path: &Path) -> Result<&Path> {
    if path.file_name().is_none() {
        return Err(Error::Contract(
            "publication target must name a file".into(),
        ));
    }
    Ok(path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new(".")))
}
#[cfg(not(windows))]
fn admit_target(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(Error::Contract(
            "publication target must be regular or absent".into(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
#[cfg(not(windows))]
fn confirm_directory(parent: &Path) -> Result<()> {
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            Error::Contract(format!(
                "file was published but directory durability is unconfirmed: {error}"
            ))
        })
}

#[cfg(not(windows))]
struct OwnedStage(Option<PathBuf>);
#[cfg(not(windows))]
impl OwnedStage {
    fn cleanup(&mut self) -> Result<()> {
        if let Some(path) = &self.0 {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        self.0 = None;
        Ok(())
    }
}
#[cfg(not(windows))]
impl Drop for OwnedStage {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    #[cfg(windows)]
    {
        let _ = (path, bytes);
        return Err(Error::Contract("atomic replacement is not implemented on Windows; canonical writes require Unix rename semantics".into()));
    }
    #[cfg(not(windows))]
    {
        let parent = target_parent(path)?;
        admit_target(path)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                Error::Contract(format!("publication clock is before Unix epoch: {error}"))
            })?
            .as_nanos();
        let sequence = STAGE_SEQUENCE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| Error::Contract("publication stage sequence exhausted".into()))?;
        let mut name = OsString::from(".");
        name.push(
            path.file_name()
                .ok_or_else(|| Error::Contract("publication filename missing".into()))?,
        );
        name.push(format!(".{}.{stamp}.{sequence}.tmp", std::process::id()));
        fs::create_dir_all(parent)?;
        let stage_path = parent.join(name);
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&stage_path)?;
        let mut stage = OwnedStage(Some(stage_path.clone()));
        let result = (|| -> Result<()> {
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            admit_target(path)?;
            fs::rename(&stage_path, path)?;
            stage.0 = None;
            confirm_directory(parent)
        })();
        let cleanup = stage.cleanup();
        match (result, cleanup) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) => Err(error),
            (Ok(()), Err(error)) => Err(error),
            (Err(error), Err(cleanup)) => Err(Error::Contract(format!(
                "{error}; owned stage cleanup failed: {cleanup}"
            ))),
        }
    }
}

pub fn atomic_write_text(path: &Path, text: &str) -> Result<()> {
    atomic_write(path, text.as_bytes())
}

/// Source is caller-owned. Failed preflight/publication never deletes it.
pub fn atomic_publish_file(source: &Path, target: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        let _ = (source, target);
        return Err(Error::Contract("atomic file publication is not implemented on Windows; canonical writes require Unix rename semantics".into()));
    }
    #[cfg(not(windows))]
    {
        let parent = target_parent(target)?;
        let source_parent = target_parent(source)?;
        let source_metadata = fs::symlink_metadata(source)?;
        if source_metadata.file_type().is_symlink() || !source_metadata.is_file() {
            return Err(Error::Contract(
                "publication source must be a regular non-symlink file".into(),
            ));
        }
        admit_target(target)?;
        if source == target {
            return Err(Error::Contract(
                "publication source and target are the same path".into(),
            ));
        }
        #[cfg(unix)]
        {
            match fs::symlink_metadata(target) {
                Ok(metadata) if super::same_file(&source_metadata, &metadata) => {
                    return Err(Error::Contract(
                        "publication source and target identify the same file".into(),
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        let file = File::open(source)?;
        let held_metadata = file.metadata()?;
        if !held_metadata.is_file() {
            return Err(Error::Contract(
                "opened publication source changed type".into(),
            ));
        }
        #[cfg(unix)]
        if !super::same_file(&source_metadata, &held_metadata) {
            return Err(Error::Contract(
                "publication source changed identity during open".into(),
            ));
        }
        file.sync_all()?;
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        if !super::same_file(&held_metadata, &fs::symlink_metadata(source)?) {
            return Err(Error::Contract(
                "publication source changed identity before rename".into(),
            ));
        }
        admit_target(target)?;
        fs::rename(source, target)?;
        confirm_directory(parent)?;
        if source_parent != parent {
            confirm_directory(source_parent)?;
        }
        Ok(())
    }
}
