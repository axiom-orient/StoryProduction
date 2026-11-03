use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use promptgen_core::json::{JsonValue, parse};

use super::CliError;

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
static NEXT_NONCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn read_json(path: &Path) -> Result<JsonValue, CliError> {
    let text = read_input(path)?;
    parse(&text).map_err(|error| CliError::validation(format!("JSON parse failed: {error}")))
}

fn read_input(path: &Path) -> Result<String, CliError> {
    let mut bytes = Vec::new();
    if path == Path::new("-") {
        io::stdin()
            .take((MAX_INPUT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| CliError::io("read stdin", path, error))?;
    } else {
        let metadata =
            fs::metadata(path).map_err(|error| CliError::io("stat input", path, error))?;
        if !metadata.is_file() {
            return Err(CliError::usage(format!(
                "input is not a regular file: {}",
                path.display()
            )));
        }
        if metadata.len() > MAX_INPUT_BYTES as u64 {
            return Err(CliError::usage(format!(
                "input exceeds {} bytes: {}",
                MAX_INPUT_BYTES,
                path.display()
            )));
        }
        File::open(path)
            .and_then(|file| {
                file.take((MAX_INPUT_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)
            })
            .map_err(|error| CliError::io("read input", path, error))?;
    }
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(CliError::usage(format!(
            "input exceeds {MAX_INPUT_BYTES} bytes"
        )));
    }
    String::from_utf8(bytes)
        .map_err(|error| CliError::validation(format!("input is not UTF-8: {error}")))
}

pub(super) fn write_output(text: &str, path: Option<&Path>, force: bool) -> Result<(), CliError> {
    let Some(path) = path else {
        let mut stdout = io::stdout().lock();
        stdout
            .write_all(text.as_bytes())
            .and_then(|()| {
                if text.ends_with('\n') {
                    Ok(())
                } else {
                    stdout.write_all(b"\n")
                }
            })
            .and_then(|()| stdout.flush())
            .map_err(|error| CliError::io("write stdout", Path::new("<stdout>"), error))?;
        return Ok(());
    };
    preflight_output(path, force)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| CliError::io("create output directory", parent, error))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("result");
    let temporary = parent.join(format!(
        ".{name}.promptgen-partial-{}-{}-{}",
        std::process::id(),
        unix_millis(),
        NEXT_NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    let operation = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| CliError::io("create temporary output", &temporary, error))?;
        file.write_all(text.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| CliError::io("write temporary output", &temporary, error))?;
        drop(file);
        publish_output(&temporary, path, force)
    })();
    if operation.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    operation
}

pub(super) fn preflight_output(path: &Path, force: bool) -> Result<(), CliError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(CliError::usage(format!(
                    "output is not a regular file: {}",
                    path.display()
                )));
            }
            if !force {
                return Err(CliError::usage(format!(
                    "output already exists; use --force to replace it: {}",
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(CliError::io("inspect output", path, error)),
    }
    Ok(())
}

fn publish_output(temporary: &Path, destination: &Path, force: bool) -> Result<(), CliError> {
    if !destination.exists() {
        fs::hard_link(temporary, destination)
            .map_err(|error| CliError::io("publish new output", destination, error))?;
        fs::remove_file(temporary)
            .map_err(|error| CliError::io("remove temporary output link", temporary, error))?;
        return Ok(());
    }
    if !force {
        return Err(CliError::usage(format!(
            "output already exists: {}",
            destination.display()
        )));
    }
    // `fs::rename` replaces an existing destination in one step, so the published path is
    // never transiently absent. Moving the previous output aside first would leave a window
    // where a crash destroys the visible output and leaves the only copy under a name nothing
    // here would ever recover.
    fs::rename(temporary, destination)
        .map_err(|error| CliError::io("publish replacement output", destination, error))
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
