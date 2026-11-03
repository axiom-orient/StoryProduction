use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::OpenAiExecutionError;
use super::support::io_error;

pub(super) static NEXT_NONCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn preflight_output(path: &Path, overwrite: bool) -> Result<(), OpenAiExecutionError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(OpenAiExecutionError::new(
                    "OPENAI_OUTPUT_INVALID",
                    "image output must be a regular non-symlink file",
                ));
            }
            if !overwrite {
                return Err(OpenAiExecutionError::new(
                    "OPENAI_OUTPUT_EXISTS",
                    "image output already exists; use --force to replace it",
                ));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error("inspect image output", path, error)),
    }
    Ok(())
}

pub(super) fn read_bounded_utf8(
    path: &Path,
    maximum: usize,
    label: &str,
) -> Result<String, OpenAiExecutionError> {
    let bytes = read_bounded_bytes(path, maximum, label)?;
    String::from_utf8(bytes).map_err(|_| {
        OpenAiExecutionError::new("OPENAI_RESPONSE_ENCODING", format!("{label} is not UTF-8"))
    })
}

pub(super) fn read_bounded_bytes(
    path: &Path,
    maximum: usize,
    label: &str,
) -> Result<Vec<u8>, OpenAiExecutionError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        OpenAiExecutionError::new("OPENAI_RESPONSE_IO", format!("{label} is missing"))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > maximum as u64 {
        return Err(OpenAiExecutionError::new(
            "OPENAI_RESPONSE_LIMIT",
            format!("{label} exceeds its bounded size"),
        ));
    }
    let file = File::open(path).map_err(|_| {
        OpenAiExecutionError::new("OPENAI_RESPONSE_IO", format!("{label} could not be opened"))
    })?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| {
            OpenAiExecutionError::new("OPENAI_RESPONSE_IO", format!("{label} could not be read"))
        })?;
    if bytes.len() > maximum {
        return Err(OpenAiExecutionError::new(
            "OPENAI_RESPONSE_LIMIT",
            format!("{label} exceeds its bounded size"),
        ));
    }
    Ok(bytes)
}

pub(super) fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), OpenAiExecutionError> {
    let mut file = create_private_file(path)?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| io_error("write private file", path, error))
}

pub(super) fn create_private_file(path: &Path) -> Result<File, OpenAiExecutionError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| io_error("create private file", path, error))
}

pub(super) fn lexical_absolute(path: &Path) -> io::Result<PathBuf> {
    let mut output = if path.is_absolute() {
        PathBuf::new()
    } else {
        env::current_dir()?
    };
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => output.push(prefix.as_os_str()),
            Component::RootDir => output.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                output.pop();
            }
            Component::Normal(value) => output.push(value),
        }
    }
    Ok(output)
}

#[derive(Debug)]
pub(super) struct TempWorkspace {
    pub(super) path: PathBuf,
}

impl TempWorkspace {
    pub(super) fn create() -> Result<Self, OpenAiExecutionError> {
        let base = env::temp_dir();
        for _ in 0..32 {
            let path = base.join(format!(
                ".promptgen-openai-{}-{}",
                std::process::id(),
                NEXT_NONCE.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        if let Err(error) =
                            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                        {
                            let _ = fs::remove_dir(&path);
                            return Err(io_error("protect temporary directory", &path, error));
                        }
                    }
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(io_error("create temporary directory", &base, error)),
            }
        }
        Err(OpenAiExecutionError::new(
            "OPENAI_TEMP",
            "could not allocate a unique temporary directory",
        ))
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
