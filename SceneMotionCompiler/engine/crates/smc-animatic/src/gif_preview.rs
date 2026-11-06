//! A rendered preview of the exact captured frames, never an image-generation receipt.
use crate::{io, process};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::{io::Read, path::Path};

fn input_args(fps: u64) -> Vec<String> {
    [
        "-nostdin",
        "-v",
        "error",
        "-f",
        "image2pipe",
        "-vcodec",
        "png",
        "-protocol_whitelist",
        "pipe",
        "-framerate",
        &fps.to_string(),
        "-i",
        "pipe:0",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn encode_frames(
    encoder: &Path,
    args: &[String],
    target: &std::fs::File,
    frames: &io::Directory,
    count: u64,
    cancel: &mut impl FnMut() -> io::Result<()>,
) -> io::Result<()> {
    let mut next = 0;
    process::encode(
        encoder,
        args,
        target,
        &mut || {
            if next == count {
                return Ok(None);
            }
            let bytes = io::read_at(frames, &format!("{next:06}.png"), 16 * 1024 * 1024)?;
            next += 1;
            Ok(Some(bytes))
        },
        cancel,
    )
}

pub fn render(
    directory: &io::Directory,
    frames: &io::Directory,
    plan: &Value,
    encoder: &Value,
    probe: &Value,
    count: u64,
    mut cancel: impl FnMut() -> io::Result<()>,
) -> io::Result<Value> {
    let fps = plan["fps"].as_u64().ok_or("GIF_SOURCE_FPS")?;
    let width = plan["canvas"]["width"]
        .as_u64()
        .ok_or("GIF_SOURCE_WIDTH")?
        .min(640);
    let output_fps = 6_u64;
    let output_frames = (count * output_fps).div_ceil(fps).max(1);
    let filter =
        format!("fps=fps={output_fps}:round=up:eof_action=pass,scale={width}:-2:flags=lanczos");
    let binary = Path::new(encoder["path"].as_str().ok_or("GIF_ENCODER")?);
    process::verify(encoder)?;
    let mut palette = directory.partial_media("palette.partial.png")?;
    let mut args = input_args(fps);
    args.extend([
        "-vf".into(),
        format!("{filter},palettegen=stats_mode=diff"),
        "-frames:v".into(),
        "1".into(),
        "-vcodec".into(),
        "png".into(),
        "-f".into(),
        "image2".into(),
        "pipe:1".into(),
    ]);
    encode_frames(binary, &args, palette.file(), frames, count, &mut cancel)?;
    let mut palette_bytes = Vec::new();
    io::io(
        palette
            .reader()?
            .take(16385)
            .read_to_end(&mut palette_bytes),
    )?;
    io::check(
        !palette_bytes.is_empty() && palette_bytes.len() <= 16384,
        "GIF_PALETTE_LIMIT",
    )?;
    let palette_uri = format!("data:image/png;base64,{}", STANDARD.encode(palette_bytes));
    let duration = count as f64 / fps as f64;
    let final_delay = ((duration - (output_frames - 1) as f64 / output_fps as f64) * 100.0)
        .round()
        .max(1.0) as u64;
    let mut gif = directory.partial_media("preview.partial.gif")?;
    let mut args = input_args(fps);
    args.extend([
        "-protocol_whitelist".into(),
        "data".into(),
        "-f".into(),
        "image2pipe".into(),
        "-vcodec".into(),
        "png".into(),
        "-i".into(),
        palette_uri,
        "-filter_complex".into(),
        format!(
            "[0:v]{filter}[v];[v][1:v]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle"
        ),
        "-frames:v".into(),
        output_frames.to_string(),
        "-an".into(),
        "-loop".into(),
        "0".into(),
        "-final_delay".into(),
        final_delay.to_string(),
        "-threads".into(),
        "1".into(),
        "-f".into(),
        "gif".into(),
        "pipe:1".into(),
    ]);
    process::verify(encoder)?;
    encode_frames(binary, &args, gif.file(), frames, count, &mut cancel)?;
    io::check(
        io::io(gif.file().metadata())?.len() <= 128 * 1024 * 1024,
        "GIF_PREVIEW_BYTE_LIMIT",
    )?;
    process::verify(probe)?;
    let observed = io::parse(&process::inspect_file(
        Path::new(probe["path"].as_str().ok_or("GIF_PROBE")?),
        &[
            "-v",
            "error",
            "-protocol_whitelist",
            "pipe",
            "-ignore_loop",
            "1",
            "-min_delay",
            "0",
            "-f",
            "gif",
            "-count_frames",
            "-show_entries",
            "stream=codec_name,width,height,nb_read_frames:frame=duration_time,best_effort_timestamp_time",
            "-of",
            "json",
            "pipe:0",
        ]
        .map(str::to_owned),
        gif.reader()?,
        &mut cancel,
    )?)?;
    let streams = observed["streams"].as_array().ok_or("GIF_OBSERVATION")?;
    io::check(streams.len() == 1, "GIF_UNEXPECTED_STREAM")?;
    let stream = &streams[0];
    // A non-seekable GIF input reports stream.duration=0. Read the decoded
    // frame delays instead; retaining the held descriptor avoids pathname races.
    let decoded = observed["frames"].as_array().ok_or("GIF_FRAMES_MISSING")?;
    io::check(decoded.len() as u64 == output_frames, "GIF_FRAME_COUNT")?;
    let mut observed_seconds = 0.0;
    for frame in decoded {
        let time = frame["best_effort_timestamp_time"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or("GIF_FRAME_TIME")?;
        let delay = frame["duration_time"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or("GIF_FRAME_DELAY")?;
        io::check(
            time.is_finite()
                && delay.is_finite()
                && delay > 0.0
                && (time - observed_seconds).abs() <= 0.011,
            "GIF_FRAME_TIMING",
        )?;
        observed_seconds += delay;
    }
    io::check(
        stream["codec_name"] == "gif"
            && stream["width"] == width
            && stream["nb_read_frames"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                == Some(output_frames)
            && (observed_seconds - duration).abs() <= 0.021,
        "GIF_OBSERVATION_MISMATCH",
    )?;
    cancel()?;
    gif.publish(directory, "preview.gif")?;
    Ok(
        json!({"kind":"rendered-preview", "mediaType":"image/gif", "nativeGeneration":false,
        "frameCount":output_frames, "frameRate":output_fps, "width":stream["width"],
        "height":stream["height"], "durationMs":(observed_seconds*1000.0).round() as u64,
        "containsAudio":false, "sha256":io::hash_file(directory,"preview.gif")?}),
    )
}
