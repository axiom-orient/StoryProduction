use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use promptgen_core::image_artifact::{png, sha256::digest_hex};

use super::process::OwnedChild;
use super::{CodexExecutionError, exit_code, io_error};

#[derive(Debug)]
pub(super) struct PreparedCandidate {
    pub(super) path: PathBuf,
    pub(super) source_width: u32,
    pub(super) source_height: u32,
    pub(super) dimensions_normalized: bool,
}

pub(super) fn prepare_candidate(
    source: &Path,
    expected_dimensions: (u32, u32),
    working: &Path,
    attempt: u8,
) -> Result<PreparedCandidate, CodexExecutionError> {
    let source_info = png::inspect_file(source).map_err(|error| {
        CodexExecutionError::new(
            "CODEX_INVALID_PNG",
            format!("{}: {error}", source.display()),
        )
    })?;
    if (source_info.width, source_info.height) == expected_dimensions {
        return Ok(PreparedCandidate {
            path: source.to_path_buf(),
            source_width: source_info.width,
            source_height: source_info.height,
            dimensions_normalized: false,
        });
    }
    let same_aspect = u64::from(source_info.width) * u64::from(expected_dimensions.1)
        == u64::from(source_info.height) * u64::from(expected_dimensions.0);
    let can_downscale =
        source_info.width >= expected_dimensions.0 && source_info.height >= expected_dimensions.1;
    if !same_aspect || !can_downscale {
        return Err(CodexExecutionError::new(
            "CODEX_INVALID_PNG",
            format!(
                "{}: dimension mismatch: expected {}x{}, actual {}x{}; only exact-aspect downscaling is allowed",
                source.display(),
                expected_dimensions.0,
                expected_dimensions.1,
                source_info.width,
                source_info.height
            ),
        ));
    }
    let normalized = working.join(format!("candidate-{attempt}.normalized.png"));
    normalize_png_dimensions(source, &normalized, expected_dimensions)?;
    png::validate_file(&normalized, expected_dimensions).map_err(|error| {
        CodexExecutionError::new(
            "CODEX_INVALID_PNG",
            format!("{}: {error}", normalized.display()),
        )
    })?;
    Ok(PreparedCandidate {
        path: normalized,
        source_width: source_info.width,
        source_height: source_info.height,
        dimensions_normalized: true,
    })
}

pub(super) fn sha256_file(path: &Path) -> Result<String, CodexExecutionError> {
    let bytes = fs::read(path)
        .map_err(|error| io_error("read generated PNG for fidelity gate", path, error))?;
    Ok(digest_hex(&bytes))
}

#[cfg(target_os = "macos")]
fn normalize_png_dimensions(
    source: &Path,
    destination: &Path,
    expected_dimensions: (u32, u32),
) -> Result<(), CodexExecutionError> {
    let binary = Path::new("/usr/bin/sips");
    if !binary.is_file() {
        return Err(CodexExecutionError::new(
            "CODEX_DIMENSION_NORMALIZER_UNAVAILABLE",
            "macOS image normalizer is unavailable at /usr/bin/sips",
        ));
    }
    if destination.exists() {
        return Err(CodexExecutionError::new(
            "CODEX_DIMENSION_NORMALIZER_OUTPUT",
            format!(
                "dimension normalizer destination already exists: {}",
                destination.display()
            ),
        ));
    }
    let mut command = Command::new(binary);
    let mut child = OwnedChild::spawn(
        command
            .arg("-z")
            .arg(expected_dimensions.1.to_string())
            .arg(expected_dimensions.0.to_string())
            .arg(source)
            .arg("--out")
            .arg(destination)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
        "CODEX_DIMENSION_NORMALIZER_SPAWN",
        binary,
    )?;
    let status = child.wait(Duration::from_secs(30))?;
    if !status.success() {
        return Err(CodexExecutionError::new(
            "CODEX_DIMENSION_NORMALIZER_FAILED",
            format!(
                "macOS image normalizer exited with code {}",
                exit_code(status)
            ),
        ));
    }
    let metadata = fs::symlink_metadata(destination)
        .map_err(|error| io_error("inspect normalized PNG", destination, error))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(CodexExecutionError::new(
            "CODEX_DIMENSION_NORMALIZER_OUTPUT",
            format!(
                "dimension normalizer did not produce a regular file: {}",
                destination.display()
            ),
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn normalize_png_dimensions(
    _source: &Path,
    _destination: &Path,
    _expected_dimensions: (u32, u32),
) -> Result<(), CodexExecutionError> {
    Err(CodexExecutionError::new(
        "CODEX_DIMENSION_NORMALIZER_UNAVAILABLE",
        "provider-native PNG downscaling is currently available only on macOS",
    ))
}
