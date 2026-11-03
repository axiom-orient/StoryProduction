//! Declared implementation capabilities and explicit executable-version discovery.

use super::types::{fingerprint, validate, BackendDescriptor, BackendLimits, BackendOperation};
use narrative_courier_contracts::OutputFormat;
use narrative_courier_runtime::process::run_piped_command_capture;
use narrative_courier_runtime::{Error, Result};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn descriptor(
    id: &str,
    version: String,
    formats: Vec<OutputFormat>,
    limits: BackendLimits,
) -> Result<BackendDescriptor> {
    let mut descriptor = BackendDescriptor {
        backend_id: id.into(),
        version,
        fingerprint: String::new(),
        operations: vec![BackendOperation::Render],
        formats,
        limits,
    };
    descriptor.fingerprint = fingerprint(&descriptor)?;
    validate(&descriptor)?;
    Ok(descriptor)
}

fn command_version(exe: &str, arg: &str) -> Result<Option<String>> {
    let mut executable = None;
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            let candidate = directory.join(exe);
            match std::fs::metadata(&candidate) {
                Ok(metadata) if metadata.is_file() => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        if metadata.permissions().mode() & 0o111 == 0 {
                            continue;
                        }
                    }
                    executable = Some(candidate.canonicalize()?);
                    break;
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    let Some(executable) = executable else {
        return Ok(None);
    };
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Contract("system clock is before the Unix epoch".into()))?
        .as_nanos();
    let run_id = format!("backend-probe-{}-{stamp}", std::process::id());
    let cancel_path = std::env::temp_dir().join(format!(".{run_id}.cancel"));
    let mut command = Command::new(executable);
    command.arg(arg);
    let output = run_piped_command_capture(
        &mut command,
        b"",
        Duration::from_secs(5),
        &cancel_path,
        &run_id,
    )?;
    if !output.status.success() {
        return Err(Error::Executor(format!(
            "BACKEND_PROBE_FAILED: {exe} exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = String::from_utf8(output.stdout).map_err(|error| {
        Error::Executor(format!(
            "BACKEND_PROBE_FAILED: {exe} version is not UTF-8: {error}"
        ))
    })?;
    let version = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_owned())
        .ok_or_else(|| {
            Error::Executor(format!(
                "BACKEND_PROBE_FAILED: {exe} returned an empty version"
            ))
        })?;
    Ok(Some(version))
}

pub fn describe_backends() -> Result<Vec<BackendDescriptor>> {
    let mut out = vec![
        descriptor(
            "internal-html",
            env!("CARGO_PKG_VERSION").into(),
            vec![OutputFormat::Html],
            BackendLimits::default(),
        )?,
        descriptor(
            "internal-graph",
            env!("CARGO_PKG_VERSION").into(),
            vec![OutputFormat::Json],
            BackendLimits::default(),
        )?,
    ];
    // External renderers use Runtime-owned process-tree cancellation. Do not advertise
    // capabilities on platforms where that lifecycle contract is not implemented.
    #[cfg(unix)]
    {
        let magick_version = command_version("magick", "--version")?;
        if let Some(version) = &magick_version {
            out.push(descriptor(
                "imagemagick",
                version.clone(),
                vec![OutputFormat::Png, OutputFormat::Webp],
                BackendLimits::default(),
            )?);
        }
        if let Some(version) = command_version("weasyprint", "--version")? {
            out.push(descriptor(
                "weasyprint",
                version,
                vec![OutputFormat::Pdf],
                BackendLimits::default(),
            )?);
        }
        if let (Some(ffmpeg), Some(ffprobe), Some(magick)) = (
            command_version("ffmpeg", "-version")?,
            command_version("ffprobe", "-version")?,
            magick_version,
        ) {
            out.push(descriptor(
                "ffmpeg-text-video",
                format!("{ffmpeg}; {ffprobe}; {magick}"),
                vec![OutputFormat::Mp4],
                BackendLimits {
                    width_multiple: Some(2),
                    height_multiple: Some(2),
                },
            )?);
        }
    }
    Ok(out)
}
