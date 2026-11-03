//! Thread-bound Codex PNG discovery and identity admission.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::super::{CodexExecutionError, io_error};

const STABLE_POLLS_REQUIRED: u8 = 3;
const CLOCK_TOLERANCE: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(150);

pub(crate) fn discover_artifact(
    generated_root: &Path,
    thread_id: &str,
    started_at: SystemTime,
    timeout: Duration,
) -> Result<(String, PathBuf), CodexExecutionError> {
    let root = fs::canonicalize(generated_root)
        .map_err(|error| io_error("canonicalize generated root", generated_root, error))?;
    let thread_dir = root.join(sanitize_component(thread_id));
    let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
        CodexExecutionError::new(
            "CODEX_ARTIFACT_TIMEOUT_RANGE",
            format!(
                "artifact wait timeout is too large: {} seconds",
                timeout.as_secs()
            ),
        )
    })?;
    let mut last_state: Option<(PathBuf, u64, SystemTime)> = None;
    let mut stable = 0_u8;

    loop {
        let files = list_thread_pngs(&thread_dir)?;
        if files.len() > 1 {
            return Err(CodexExecutionError::new(
                "CODEX_ARTIFACT_COUNT",
                format!(
                    "thread directory must contain exactly one PNG: dir={}, files={files:?}",
                    thread_dir.display()
                ),
            ));
        }
        if let Some(path) = files.into_iter().next() {
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| io_error("inspect generated artifact", &path, error))?;
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                return Err(CodexExecutionError::new(
                    "CODEX_ARTIFACT_TYPE",
                    format!(
                        "generated artifact is not a regular file: {}",
                        path.display()
                    ),
                ));
            }
            let modified = metadata
                .modified()
                .map_err(|error| io_error("read artifact modification time", &path, error))?;
            let earliest = started_at
                .checked_sub(CLOCK_TOLERANCE)
                .unwrap_or(UNIX_EPOCH);
            if modified < earliest {
                return Err(CodexExecutionError::new(
                    "CODEX_ARTIFACT_STALE",
                    format!("artifact predates this execution: {}", path.display()),
                ));
            }
            let state = (path.clone(), metadata.len(), modified);
            if last_state.as_ref() == Some(&state) {
                stable += 1;
            } else {
                stable = 1;
                last_state = Some(state);
            }
            if stable >= STABLE_POLLS_REQUIRED {
                let actual = fs::canonicalize(&path)
                    .map_err(|error| io_error("canonicalize generated artifact", &path, error))?;
                let canonical_thread = fs::canonicalize(&thread_dir).map_err(|error| {
                    io_error("canonicalize thread artifact directory", &thread_dir, error)
                })?;
                if !actual.starts_with(&canonical_thread) || !actual.starts_with(&root) {
                    return Err(CodexExecutionError::new(
                        "CODEX_ARTIFACT_ESCAPE",
                        format!("artifact escapes authority directory: {}", actual.display()),
                    ));
                }
                let call_id = actual
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        CodexExecutionError::new(
                            "CODEX_CALL_ID",
                            format!("artifact has no valid call id: {}", actual.display()),
                        )
                    })?
                    .to_owned();
                if actual.file_name().and_then(OsStr::to_str)
                    != Some(&format!("{}.png", sanitize_component(&call_id)))
                {
                    return Err(CodexExecutionError::new(
                        "CODEX_ARTIFACT_IDENTITY",
                        format!("artifact name is not call-id based: {}", actual.display()),
                    ));
                }
                return Ok((call_id, actual));
            }
        }
        if Instant::now() >= deadline {
            return Err(CodexExecutionError::new(
                "CODEX_ARTIFACT_TIMEOUT",
                format!(
                    "no stable, unique PNG appeared in {} within {} seconds",
                    thread_dir.display(),
                    timeout.as_secs()
                ),
            ));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn list_thread_pngs(thread_dir: &Path) -> Result<Vec<PathBuf>, CodexExecutionError> {
    let entries = match fs::read_dir(thread_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(io_error(
                "read thread artifact directory",
                thread_dir,
                error,
            ));
        }
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| io_error("read thread artifact entry", thread_dir, error))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| io_error("inspect thread artifact entry", &path, error))?;
        if metadata.file_type().is_symlink() {
            return Err(CodexExecutionError::new(
                "CODEX_ARTIFACT_SYMLINK",
                format!("symlink found in thread directory: {}", path.display()),
            ));
        }
        if metadata.is_dir() {
            return Err(CodexExecutionError::new(
                "CODEX_ARTIFACT_LAYOUT",
                format!(
                    "unexpected nested directory in thread artifact root: {}",
                    path.display()
                ),
            ));
        }
        if metadata.is_file()
            && path
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn sanitize_component(value: &str) -> String {
    let output = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if output.is_empty() {
        "generated_image".to_owned()
    } else {
        output
    }
}
