//! STS project-root path resolution and immutable publish target preflight.

use narrative_courier_runtime::paths::validate_relative_path;
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn path_parts(relative: &str) -> Result<Vec<&str>> {
    validate_relative_path(relative)?;
    Ok(relative.split('/').collect())
}

pub(super) fn resolve_regular_file_within_root(root: &Path, relative: &str) -> Result<PathBuf> {
    let parts = path_parts(relative)?;
    let root_meta = fs::symlink_metadata(root)?;
    if root_meta.file_type().is_symlink() || !root_meta.is_dir() {
        return Err(Error::Contract(format!(
            "declared input root is not a regular directory: {}",
            root.display()
        )));
    }
    let real_root = fs::canonicalize(root)?;
    let mut candidate = root.to_path_buf();
    for (index, part) in parts.iter().enumerate() {
        candidate.push(part);
        let metadata = fs::symlink_metadata(&candidate)?;
        if metadata.file_type().is_symlink() {
            return Err(Error::Contract(format!(
                "input contains a symlink component: {relative}"
            )));
        }
        if index + 1 == parts.len() {
            if !metadata.is_file() {
                return Err(Error::Contract(format!(
                    "input is not a regular non-symlink file: {}",
                    relative
                )));
            }
        } else if !metadata.is_dir() {
            return Err(Error::Contract(format!(
                "input parent is not a directory: {relative}"
            )));
        }
    }
    let real_candidate = fs::canonicalize(&candidate)?;
    if !real_candidate.starts_with(&real_root) {
        return Err(Error::Contract(format!(
            "input escapes declared root: {relative}"
        )));
    }
    let real_meta = fs::symlink_metadata(&real_candidate)?;
    if real_meta.file_type().is_symlink() || !real_meta.is_file() {
        return Err(Error::Contract(format!(
            "input is not a regular file: {relative}"
        )));
    }
    // A selected executable cwd is the root capability. Canonical names are
    // only containment observations and must not replace relative I/O paths.
    Ok(if root.is_relative() {
        candidate
    } else {
        real_candidate
    })
}

pub(super) fn resolve_directory_within_root(root: &Path, relative: &str) -> Result<PathBuf> {
    let parts = path_parts(relative)?;
    let root_meta = fs::symlink_metadata(root)?;
    if root_meta.file_type().is_symlink() || !root_meta.is_dir() {
        return Err(Error::Contract(format!(
            "declared project root is not a regular directory: {}",
            root.display()
        )));
    }
    let real_root = fs::canonicalize(root)?;
    let mut candidate = root.to_path_buf();
    for part in parts {
        candidate.push(part);
        let metadata = fs::symlink_metadata(&candidate)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(Error::Contract(format!(
                "project directory is not a regular non-symlink directory: {relative}"
            )));
        }
    }
    let real_candidate = fs::canonicalize(&candidate)?;
    if !real_candidate.starts_with(&real_root) {
        return Err(Error::Contract(format!(
            "project directory escapes declared root: {relative}"
        )));
    }
    let real_meta = fs::symlink_metadata(&real_candidate)?;
    if real_meta.file_type().is_symlink() || !real_meta.is_dir() {
        return Err(Error::Contract(format!(
            "project directory is not a regular directory: {relative}"
        )));
    }
    // A selected executable cwd is the root capability. Canonical names are
    // only containment observations and must not replace relative I/O paths.
    Ok(if root.is_relative() {
        candidate
    } else {
        real_candidate
    })
}

pub(crate) fn ensure_atomic_file_target(path: &Path, label: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(Error::Contract(format!(
                "{label} target is not a regular non-symlink file: {}",
                path.display()
            )))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::Io(error)),
    }
}
