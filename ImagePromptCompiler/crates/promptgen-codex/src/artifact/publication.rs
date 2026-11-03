//! Output path admission, verified byte copy, and atomic publication.

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

use super::super::{CodexExecutionError, NEXT_NONCE, io_error, millis};
use promptgen_core::image_artifact::{png, sha256};

pub(crate) fn validate_existing_output(
    path: &Path,
    overwrite: bool,
) -> Result<(), CodexExecutionError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                return Err(CodexExecutionError::new(
                    "CODEX_OUTPUT_NOT_REGULAR",
                    format!("output target is not a regular file: {}", path.display()),
                ));
            }
            if !overwrite {
                return Err(CodexExecutionError::new(
                    "CODEX_OUTPUT_EXISTS",
                    format!("output already exists: {}", path.display()),
                ));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error("inspect output target", path, error)),
    }
    Ok(())
}

pub(crate) fn parent_or_current(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

pub(crate) fn lexical_absolute(path: &Path) -> io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(value) => normalized.push(value),
        }
    }
    Ok(normalized)
}

fn unique_sibling(parent: &Path, name: &str, role: &str) -> PathBuf {
    parent.join(format!(
        ".{name}.promptgen-{role}-{}-{}-{}",
        std::process::id(),
        millis(SystemTime::now()),
        NEXT_NONCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ))
}

pub(crate) fn validate_copy_publish(
    source: &Path,
    destination: &Path,
    overwrite: bool,
    expected_dimensions: (u32, u32),
) -> Result<(String, png::PngInfo), CodexExecutionError> {
    validate_existing_output(destination, overwrite)?;
    let parent = parent_or_current(destination);
    fs::create_dir_all(parent)
        .map_err(|error| io_error("create output directory", parent, error))?;
    let name = destination
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("image.png");
    let temporary = unique_sibling(parent, name, "partial");

    let operation = (|| {
        let mut input =
            File::open(source).map_err(|error| io_error("open generated PNG", source, error))?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| io_error("create temporary output", &temporary, error))?;
        let mut hasher = sha256::Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = input
                .read(&mut buffer)
                .map_err(|error| io_error("read generated PNG", source, error))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            output
                .write_all(&buffer[..read])
                .map_err(|error| io_error("write temporary output", &temporary, error))?;
        }
        output
            .sync_all()
            .map_err(|error| io_error("sync temporary output", &temporary, error))?;
        drop(output);
        let info = png::validate_file(&temporary, expected_dimensions).map_err(|error| {
            CodexExecutionError::new(
                "CODEX_INVALID_PNG",
                format!("{}: {error}", temporary.display()),
            )
        })?;
        publish_file(&temporary, destination, overwrite)?;
        sync_parent(parent)?;
        Ok((sha256::hex(&hasher.finalize()), info))
    })();

    if operation.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    operation
}

pub(crate) fn publish_file(
    temporary: &Path,
    destination: &Path,
    overwrite: bool,
) -> Result<(), CodexExecutionError> {
    if !destination.exists() {
        return publish_new(temporary, destination);
    }
    if !overwrite {
        return Err(CodexExecutionError::new(
            "CODEX_OUTPUT_EXISTS",
            format!("output already exists: {}", destination.display()),
        ));
    }
    // `fs::rename` replaces an existing destination in one step, so the published path is
    // never transiently absent. Moving the previous output aside first would leave a window
    // where a crash destroys the visible
    // output and leaves the only copy under a name nothing here would ever recover.
    fs::rename(temporary, destination)
        .map_err(|error| io_error("publish validated output", destination, error))
}

fn publish_new(temporary: &Path, destination: &Path) -> Result<(), CodexExecutionError> {
    match fs::hard_link(temporary, destination) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Err(CodexExecutionError::new(
                "CODEX_OUTPUT_EXISTS",
                format!("output already exists: {}", destination.display()),
            ));
        }
        Err(error) => {
            return Err(io_error(
                "publish output with create-new hard link",
                destination,
                error,
            ));
        }
    }
    fs::remove_file(temporary)
        .map_err(|error| io_error("remove published temporary link", temporary, error))?;
    Ok(())
}

fn sync_parent(parent: &Path) -> Result<(), CodexExecutionError> {
    #[cfg(unix)]
    {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| io_error("sync output directory", parent, error))?;
    }
    Ok(())
}
