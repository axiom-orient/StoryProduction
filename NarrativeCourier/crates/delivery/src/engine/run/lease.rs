//! Delivery active-run, cancellation, and publish-ownership lifecycle.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::DeliveryEngine;
use crate::project_owned_path;
use crate::render::process::RunControl;
use narrative_courier_runtime::store::{atomic_write_text, lock_project, ProjectLock};
use narrative_courier_runtime::{Error, Result};

const MAX_RUN_MARKER_BYTES: usize = 128;

pub(super) struct ActiveRunGuard {
    active: PathBuf,
    cancel: PathBuf,
    publishing: PathBuf,
    run_id: String,
    project: PathBuf,
    finished: bool,
}

impl Drop for ActiveRunGuard {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if let Ok(_lock) = lock_control(&self.project) {
            for marker in [&self.active, &self.cancel, &self.publishing] {
                let _ = remove_owned_marker(marker, &self.run_id);
            }
        }
    }
}

impl ActiveRunGuard {
    pub(super) fn finish(mut self) -> Result<()> {
        let lock = lock_control(&self.project)?;
        let cleanup = (|| -> Result<()> {
            for marker in [&self.active, &self.cancel, &self.publishing] {
                remove_owned_marker(marker, &self.run_id)?;
            }
            Ok(())
        })();
        self.finished = true;
        drop(lock);
        cleanup
    }
}

pub(super) fn begin(project: &Path) -> Result<(RunControl, ActiveRunGuard)> {
    let _lock = lock_control(project)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Contract("system clock is before the Unix epoch".into()))?
        .as_nanos();
    let run_id = format!("delivery-{}-{stamp}", std::process::id());
    let active = project_owned_path(project, ".delivery-active")?;
    let cancel = project_owned_path(project, ".delivery-cancel")?;
    let publishing = project_owned_path(project, ".delivery-publishing")?;
    read_marker(&active)?;
    read_marker(&cancel)?;
    read_marker(&publishing)?;
    remove_if_exists(&cancel)?;
    remove_if_exists(&publishing)?;
    atomic_write_text(&active, &run_id)?;
    let guard = ActiveRunGuard {
        active,
        cancel: cancel.clone(),
        publishing,
        run_id: run_id.clone(),
        project: project.to_path_buf(),
        finished: false,
    };
    let control = RunControl {
        run_id,
        cancel_path: cancel,
    };
    Ok((control, guard))
}

pub(super) fn claim_publish_ownership(project: &Path, control: &RunControl) -> Result<()> {
    let _lock = lock_control(project)?;
    let active = project_owned_path(project, ".delivery-active")?;
    let publishing = project_owned_path(project, ".delivery-publishing")?;
    let owner = match read_marker(&active)? {
        Some(owner) => owner,
        None => {
            return Err(Error::Executor(
                "RENDER_CANCELED: delivery no longer owns publish lease".into(),
            ));
        }
    };
    if owner.trim() != control.run_id {
        return Err(Error::Executor(
            "RENDER_CANCELED: stale delivery publish lease".into(),
        ));
    }
    if let Err(error) = fs::rename(&active, &publishing) {
        if error.kind() == std::io::ErrorKind::NotFound {
            return Err(Error::Executor(
                "RENDER_CANCELED: cancellation won publish race".into(),
            ));
        }
        return Err(error.into());
    }
    let claimed = read_marker(&project_owned_path(project, ".delivery-publishing")?)?
        .ok_or_else(|| Error::Executor("RENDER_CANCELED: publish lease disappeared".into()))?;
    if claimed.trim() != control.run_id {
        return Err(Error::Executor(
            "RENDER_CANCELED: invalid publish lease".into(),
        ));
    }
    Ok(())
}

impl DeliveryEngine {
    pub fn cancel(project: &Path) -> Result<()> {
        let _lock = lock_control(project)?;
        let active = project_owned_path(project, ".delivery-active")?;
        let cancel = project_owned_path(project, ".delivery-cancel")?;
        let run_id = match read_marker(&active)? {
            Some(run_id) => run_id,
            None => {
                return Err(Error::Contract("no cancellable delivery run".into()));
            }
        };
        if run_id.trim().is_empty() {
            return Err(Error::Contract("no cancellable delivery run".into()));
        }
        if let Err(error) = fs::rename(&active, &cancel) {
            if error.kind() == std::io::ErrorKind::NotFound {
                return Err(Error::Contract(
                    "delivery run reached publish commit before cancellation".into(),
                ));
            }
            return Err(error.into());
        }
        Ok(())
    }
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn remove_owned_marker(path: &Path, run_id: &str) -> Result<()> {
    match read_marker(path)? {
        None => Ok(()),
        Some(value) if value.trim() == run_id => remove_if_exists(path),
        Some(_) => Err(Error::Contract(
            "delivery run marker belongs to a different run".into(),
        )),
    }
}

fn read_marker(path: &Path) -> Result<Option<String>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::Contract(
            "delivery run marker is not a regular file".into(),
        ));
    }
    if metadata.len() > MAX_RUN_MARKER_BYTES as u64 {
        return Err(Error::Contract(
            "delivery run marker exceeds its size limit".into(),
        ));
    }
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(Error::Contract(
            "delivery run marker is not a regular file".into(),
        ));
    }
    let mut bytes = Vec::with_capacity(MAX_RUN_MARKER_BYTES);
    file.take((MAX_RUN_MARKER_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RUN_MARKER_BYTES {
        return Err(Error::Contract(
            "delivery run marker exceeds its size limit".into(),
        ));
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| Error::Contract("delivery run marker is not UTF-8".into()))
}

/// Separate from the long-running project lock so cancellation can coordinate promptly.
fn lock_control(project: &Path) -> Result<ProjectLock> {
    let directory = project_owned_path(project, ".delivery-control")?;
    match fs::create_dir(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let metadata = fs::symlink_metadata(&directory)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::Contract(
            "delivery control root is not a regular directory".into(),
        ));
    }
    project_owned_path(project, ".delivery-control/.workflow.lock")?;
    lock_project(&directory)
}
