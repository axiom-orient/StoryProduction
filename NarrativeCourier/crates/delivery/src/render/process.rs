//! Render process cancellation, cleanup, and bounded media inspection.

use narrative_courier_runtime::process::run_piped_command_capture;
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct RunControl {
    pub(crate) run_id: String,
    pub(crate) cancel_path: PathBuf,
}

pub(super) struct StageCleanup {
    stage: Option<PathBuf>,
    run_stage: Option<PathBuf>,
}

impl StageCleanup {
    pub(super) fn new(run_stage: PathBuf) -> Self {
        Self {
            stage: None,
            run_stage: Some(run_stage),
        }
    }

    pub(super) fn own_stage(&mut self, stage: PathBuf) {
        self.stage = Some(stage);
    }

    pub(super) fn finish(mut self) -> Result<()> {
        self.cleanup()
    }

    fn cleanup(&mut self) -> Result<()> {
        if let Some(stage) = self.stage.clone() {
            match fs::remove_dir_all(stage) {
                Ok(()) => self.stage = None,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => self.stage = None,
                Err(error) => return Err(error.into()),
            }
        }
        if let Some(run_stage) = self.run_stage.clone() {
            match fs::remove_dir(run_stage) {
                Ok(()) => self.run_stage = None,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => self.run_stage = None,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
}

impl Drop for StageCleanup {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn cancellation_requested(control: &RunControl) -> Result<bool> {
    match fs::read_to_string(&control.cancel_path) {
        Ok(value) => Ok(value.trim() == control.run_id),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn check_cancel(control: &RunControl) -> Result<()> {
    if cancellation_requested(control)? {
        Err(Error::Executor("RENDER_CANCELED".into()))
    } else {
        Ok(())
    }
}

pub(super) fn run_external(
    mut command: Command,
    timeout_secs: u64,
    control: &RunControl,
) -> Result<()> {
    check_cancel(control)?;
    let output = run_piped_command_capture(
        &mut command,
        b"",
        Duration::from_secs(timeout_secs),
        &control.cancel_path,
        &control.run_id,
    )?;
    if !output.status.success() {
        return Err(Error::Executor(format!(
            "RENDER_FAILED: process exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    check_cancel(control)
}

pub(super) fn identify_dimensions(path: &Path, control: &RunControl) -> Result<(u32, u32, String)> {
    let mut command = Command::new("magick");
    command.args(["identify", "-format", "%w %h %m"]).arg(path);
    let out = run_piped_command_capture(
        &mut command,
        b"",
        Duration::from_secs(30),
        &control.cancel_path,
        &control.run_id,
    )?;
    if !out.status.success() {
        return Err(Error::Executor(format!(
            "TECHNICAL_FAIL: magick identify failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let text = String::from_utf8(out.stdout)
        .map_err(|_| Error::Contract("TECHNICAL_FAIL: image dimensions are not UTF-8".into()))?;
    let mut it = text.split_whitespace();
    let w = it
        .next()
        .ok_or_else(|| Error::Contract("TECHNICAL_FAIL: image width missing".into()))?
        .parse::<u32>()
        .map_err(|_| Error::Contract("TECHNICAL_FAIL: invalid image width".into()))?;
    let h = it
        .next()
        .ok_or_else(|| Error::Contract("TECHNICAL_FAIL: image height missing".into()))?
        .parse::<u32>()
        .map_err(|_| Error::Contract("TECHNICAL_FAIL: invalid image height".into()))?;
    let format = it
        .next()
        .ok_or_else(|| Error::Contract("TECHNICAL_FAIL: image format missing".into()))?
        .to_owned();
    if w == 0 || h == 0 || it.next().is_some() {
        return Err(Error::Contract(
            "TECHNICAL_FAIL: invalid image dimension response".into(),
        ));
    }
    check_cancel(control)?;
    Ok((w, h, format))
}

pub(super) struct VideoProbe {
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    pub fps_num: u32,
    pub fps_den: u32,
}

pub(super) fn ffprobe_video(path: &Path, control: &RunControl) -> Result<VideoProbe> {
    let mut command = Command::new("ffprobe");
    command
        .args([
            "-v",
            "error",
            "-count_frames",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,nb_read_frames,avg_frame_rate,codec_name:format=format_name",
            "-of",
            "json",
        ])
        .arg(path);
    let out = run_piped_command_capture(
        &mut command,
        b"",
        Duration::from_secs(30),
        &control.cancel_path,
        &control.run_id,
    )?;
    if !out.status.success() {
        return Err(Error::Executor(format!(
            "TECHNICAL_FAIL: ffprobe failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    let streams = v
        .get("streams")
        .and_then(|x| x.as_array())
        .ok_or_else(|| Error::Contract("TECHNICAL_FAIL: video stream missing".into()))?;
    if streams.len() != 1 {
        return Err(Error::Contract(
            "TECHNICAL_FAIL: expected exactly one selected video stream".into(),
        ));
    }
    let s = &streams[0];
    let container = v
        .get("format")
        .and_then(|v| v.get("format_name"))
        .and_then(|v| v.as_str());
    if !container.is_some_and(|value| value.split(',').any(|name| name == "mp4"))
        || s.get("codec_name").and_then(|v| v.as_str()) != Some("h264")
    {
        return Err(Error::Contract(
            "TECHNICAL_FAIL: expected MP4/H.264 output".into(),
        ));
    }
    let parse = |name: &str| -> Result<u32> {
        let value = s
            .get(name)
            .ok_or_else(|| Error::Contract(format!("TECHNICAL_FAIL: {name} missing")))?;
        if let Some(n) = value.as_u64() {
            return u32::try_from(n)
                .map_err(|_| Error::Contract(format!("TECHNICAL_FAIL: {name} exceeds u32")));
        }
        value
            .as_str()
            .ok_or_else(|| Error::Contract(format!("TECHNICAL_FAIL: {name} invalid")))?
            .parse::<u32>()
            .map_err(|_| Error::Contract(format!("TECHNICAL_FAIL: {name} invalid")))
    };
    let rate = s
        .get("avg_frame_rate")
        .and_then(|v| v.as_str())
        .and_then(|value| value.split_once('/'))
        .ok_or_else(|| Error::Contract("TECHNICAL_FAIL: video frame rate missing".into()))?;
    let fps_num = rate
        .0
        .parse::<u32>()
        .map_err(|_| Error::Contract("TECHNICAL_FAIL: invalid video frame rate".into()))?;
    let fps_den = rate
        .1
        .parse::<u32>()
        .map_err(|_| Error::Contract("TECHNICAL_FAIL: invalid video frame rate".into()))?;
    let probe = VideoProbe {
        width: parse("width")?,
        height: parse("height")?,
        frames: parse("nb_read_frames")?,
        fps_num,
        fps_den,
    };
    if probe.width == 0 || probe.height == 0 || probe.frames == 0 || fps_num == 0 || fps_den == 0 {
        return Err(Error::Contract(
            "TECHNICAL_FAIL: zero video measurement".into(),
        ));
    }
    check_cancel(control)?;
    Ok(probe)
}
