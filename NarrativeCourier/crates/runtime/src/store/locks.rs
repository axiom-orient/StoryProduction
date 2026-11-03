//! Stable lock-file acquisition and read-only active-run observation.

use crate::{Error, Result};
use fs2::FileExt;
use std::fs::{self, File, OpenOptions};
use std::path::Path;

const WORKFLOW_LOCK_FILE: &str = ".workflow.lock";
const AGENT_RUN_LOCK_FILE: &str = ".agent-run.lock";

pub struct ProjectLock {
    file: File,
}
impl Drop for ProjectLock {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.file);
    }
}

fn admit_directory(project: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(project)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::Contract(
            "lock root must be an existing regular directory".into(),
        ));
    }
    Ok(())
}
fn admit_file(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::Contract(
            "lock marker must be a regular non-symlink file".into(),
        ));
    }
    Ok(())
}
fn open_lock_file(project: &Path, filename: &str) -> Result<File> {
    admit_directory(project)?;
    let path = project.join(filename);
    match fs::symlink_metadata(&path) {
        Ok(_) => admit_file(&path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)?;
    admit_file(&path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(Error::Contract("opened lock marker is not regular".into()));
    }
    #[cfg(unix)]
    if !super::same_file(&metadata, &fs::symlink_metadata(&path)?) {
        return Err(Error::Contract(
            "lock marker changed identity during open".into(),
        ));
    }
    Ok(file)
}
pub fn lock_project(project: &Path) -> Result<ProjectLock> {
    let file = open_lock_file(project, WORKFLOW_LOCK_FILE)?;
    file.lock_exclusive()?;
    Ok(ProjectLock { file })
}
pub fn try_lock_agent_run(project: &Path) -> Result<ProjectLock> {
    let file = open_lock_file(project, AGENT_RUN_LOCK_FILE)?;
    match file.try_lock_exclusive() {
        Ok(()) => Ok(ProjectLock { file }),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Err(Error::Contract(
            "an Agent run is already active for this project".into(),
        )),
        Err(error) => Err(error.into()),
    }
}
pub fn agent_run_is_active(project: &Path) -> Result<bool> {
    match fs::symlink_metadata(project) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
        Ok(_) => admit_directory(project)?,
    }
    let path = project.join(AGENT_RUN_LOCK_FILE);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
        Ok(_) => admit_file(&path)?,
    }
    let file = OpenOptions::new().read(true).write(true).open(&path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(Error::Contract("opened lock marker is not regular".into()));
    }
    #[cfg(unix)]
    if !super::same_file(&metadata, &fs::symlink_metadata(&path)?) {
        return Err(Error::Contract(
            "lock marker changed identity during open".into(),
        ));
    }
    match file.try_lock_exclusive() {
        Ok(()) => {
            fs2::FileExt::unlock(&file)?;
            Ok(false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(true),
        Err(error) => Err(error.into()),
    }
}
