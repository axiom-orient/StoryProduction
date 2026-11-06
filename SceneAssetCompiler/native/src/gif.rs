//! Decode an existing GIF into reviewed-plan input. Never generate pixels or approve motion.
use crate::{
    check,
    contracts::{parse, seal, sha256},
    files::Root,
    png, Error, Options, Result,
};
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Result<Self> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("sac-gif-{}-{stamp}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn execute(binary: &str, args: &[&str], scratch: &Path) -> Result<Vec<u8>> {
    let out = fs::File::create(scratch.join("process.stdout"))?;
    let err = fs::File::create(scratch.join("process.stderr"))?;
    let mut child = Command::new(binary)
        .args(args)
        .current_dir(scratch)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .spawn()
        .map_err(|_| Error::refused("GIF_DECODER_UNAVAILABLE"))?;
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                check(status.success(), "GIF_DECODE_FAILED")?;
                break;
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::refused("GIF_DECODE_TIMEOUT_OR_WAIT_FAILED"));
            }
        }
        if fs::metadata(scratch.join("process.stdout"))?.len() > 4 * 1024 * 1024
            || fs::metadata(scratch.join("process.stderr"))?.len() > 1024 * 1024
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::refused("GIF_DECODER_OUTPUT_LIMIT"));
        }
    }
    read_bounded(&scratch.join("process.stdout"), 4 * 1024 * 1024)
}
fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    check(bytes.len() <= limit, "GIF_DECODER_OUTPUT_LIMIT")?;
    Ok(bytes)
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::refused(format!("GIF_REQUEST_FIELD:{key}")))
}

pub fn inspect(
    root: &Root,
    options: &Options,
    packet: &Value,
    contracts: &crate::contracts::Contracts,
) -> Result<Value> {
    let receipt = crate::read_json(root, crate::required(&options.pack)?)?;
    contracts.validate("sac", "gif-intake-v1", &receipt)?;
    check(
        receipt["schemaVersion"] == "sac.gif-intake/1"
            && receipt["candidateOnly"] == true
            && receipt["directionApproved"] == false
            && receipt["sourcePacketHash"] == packet["contentHash"]
            && seal(receipt.clone())? == receipt,
        "GIF_INTAKE_RECEIPT_CHANGED",
    )?;
    for field in ["input", "executionEvidence"] {
        let path = text(&receipt[field], "path")?;
        let bytes = root.read(path, 50 * 1024 * 1024)?;
        check(
            receipt[field]["sha256"] == format!("sha256:{}", sha256(&bytes)),
            "GIF_SOURCE_OR_EVIDENCE_CHANGED",
        )?;
    }
    let assets = receipt["assets"]
        .as_array()
        .ok_or_else(|| Error::refused("GIF_INTAKE_ASSETS_MISSING"))?;
    check((2..=16).contains(&assets.len()), "GIF_FRAME_COUNT_INVALID")?;
    for asset in assets {
        let bytes = root.read(text(asset, "path")?, 64 * 1024 * 1024)?;
        let info = png::inspect(&bytes)?;
        check(
            asset["sha256"] == format!("sha256:{}", sha256(&bytes))
                && asset["width"] == info.width
                && asset["height"] == info.height
                && !info.has_transparent_pixels,
            "GIF_DECODED_FRAME_CHANGED",
        )?;
    }
    Ok(
        json!({"status":"gif_inspected","candidateOnly":true,"directionApproved":false,
        "contentHash":receipt["contentHash"],"frameCount":assets.len()}),
    )
}

pub fn import(
    root: &Root,
    options: &Options,
    packet: &Value,
    contracts: &crate::contracts::Contracts,
) -> Result<Value> {
    let output = crate::required(&options.output)?;
    check(!root.exists(output)?, "GIF_IMPORT_ALREADY_EXISTS")?;
    let draft_path = crate::required(&options.draft)?;
    let draft_raw = root.read(draft_path, 1024 * 1024)?;
    let draft = parse(&draft_raw)?;
    check(
        draft.as_object().unwrap().keys().all(|k| {
            [
                "input",
                "assetPrefix",
                "fps",
                "playback",
                "executionEvidence",
            ]
            .contains(&k.as_str())
        }),
        "GIF_REQUEST_UNKNOWN_FIELD",
    )?;
    let input = text(&draft, "input")?;
    let prefix = text(&draft, "assetPrefix")?;
    check(
        prefix.len() <= 100
            && prefix
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            && prefix.as_bytes()[0].is_ascii_alphanumeric(),
        "GIF_ASSET_PREFIX_INVALID",
    )?;
    check(
        draft["playback"] == "once-hold",
        "GIF_PLAYBACK_MUST_BE_ONCE_HOLD",
    )?;
    let fps = draft["fps"]
        .as_u64()
        .ok_or_else(|| Error::refused("GIF_FPS_REQUIRED"))?;
    check([12, 24, 30, 60].contains(&fps), "GIF_FPS_INVALID")?;
    let evidence = text(&draft, "executionEvidence")?;
    let evidence_raw = root.read(evidence, 1024 * 1024)?;
    let execution = parse(&evidence_raw)?; // Declaration does not authenticate a provider.
    check(
        matches!(
            execution["status"].as_str(),
            Some("succeeded" | "succeeded_gif_generation" | "user-supplied")
        ),
        "GIF_EXECUTION_NOT_RESOLVED",
    )?;
    let raw = root.read(input, 50 * 1024 * 1024)?;
    if execution["schemaVersion"] == "story.capability-result/1" {
        let artifact = execution["artifacts"]
            .as_array()
            .and_then(|assets| assets.iter().find(|a| a["path"] == input))
            .ok_or_else(|| Error::refused("GIF_EXECUTION_ARTIFACT_MISSING"))?;
        check(
            execution["candidateOnly"] == true
                && artifact["mediaType"] == "image/gif"
                && artifact["sha256"] == format!("sha256:{}", sha256(&raw))
                && artifact["byteLength"] == raw.len() as u64
                && execution["source"]["kind"] == "nvc.animatic-packet"
                && execution["source"]["sha256"] == packet["contentHash"],
            "GIF_EXECUTION_BINDING_CHANGED",
        )?;
    }
    check(
        raw.len() >= 13 && matches!(&raw[..6], b"GIF87a" | b"GIF89a"),
        "ANIMATED_GIF_SIGNATURE_REQUIRED",
    )?;
    let scratch = Scratch::new()?;
    fs::write(scratch.0.join("source.gif"), &raw)?;
    let probe = execute(
        &std::env::var("SAC_FFPROBE_BIN").unwrap_or_else(|_| "ffprobe".into()),
        &[
            "-v",
            "error",
            "-max_alloc",
            "67108864",
            "-ignore_loop",
            "1",
            "-min_delay",
            "0",
            "-read_intervals",
            "%+#17",
            "-select_streams",
            "v:0",
            "-show_frames",
            "-show_streams",
            "-show_entries",
            "stream=codec_name,width,height:frame=width,height,pkt_duration_time,duration_time",
            "-of",
            "json",
            "source.gif",
        ],
        &scratch.0,
    )?;
    let probe: Value =
        serde_json::from_slice(&probe).map_err(|_| Error::refused("GIF_PROBE_INVALID"))?;
    let stream = &probe["streams"][0];
    check(stream["codec_name"] == "gif", "GIF_CODEC_REQUIRED")?;
    let width = stream["width"]
        .as_u64()
        .ok_or_else(|| Error::refused("GIF_WIDTH_REQUIRED"))?;
    let height = stream["height"]
        .as_u64()
        .ok_or_else(|| Error::refused("GIF_HEIGHT_REQUIRED"))?;
    let frames = probe["frames"]
        .as_array()
        .ok_or_else(|| Error::refused("GIF_FRAMES_REQUIRED"))?;
    check(
        (2..=16).contains(&frames.len())
            && width > 0
            && height > 0
            && width <= 8192
            && height <= 8192
            && width * height * frames.len() as u64 <= 16_777_216,
        "GIF_FRAME_OR_PIXEL_BUDGET",
    )?;
    let mut elapsed_ms = 0_u64;
    let mut timings = Vec::new();
    let mut previous_frame = None;
    for frame in frames {
        check(
            frame["width"] == width && frame["height"] == height,
            "GIF_FRAME_CANVAS_CHANGED",
        )?;
        let seconds = frame["duration_time"]
            .as_str()
            .or_else(|| frame["pkt_duration_time"].as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|n| n.is_finite() && *n > 0.0)
            .ok_or_else(|| Error::refused("GIF_FRAME_DURATION_REQUIRED"))?;
        let ms = (seconds * 1000.0).round() as u64;
        check(
            (20..=2000).contains(&ms) && (seconds * 1000.0 - ms as f64).abs() < 0.01,
            "GIF_FRAME_DURATION_INVALID",
        )?;
        // Sample the source at each destination-frame timestamp: ceil prevents early actions.
        let at = (elapsed_ms * fps).div_ceil(1000);
        check(
            previous_frame.is_none_or(|old| at > old),
            "GIF_FPS_TOO_LOW: a source frame would be lost",
        )?;
        timings.push((at, ms));
        previous_frame = Some(at);
        elapsed_ms += ms;
    }
    check(
        previous_frame.is_some_and(|last| last < (elapsed_ms * fps).div_ceil(1000)),
        "GIF_FPS_TOO_LOW: final source frame would be lost",
    )?;
    execute(
        &std::env::var("SAC_FFMPEG_BIN").unwrap_or_else(|_| "ffmpeg".into()),
        &[
            "-nostdin",
            "-v",
            "error",
            "-xerror",
            "-max_alloc",
            "67108864",
            "-ignore_loop",
            "1",
            "-min_delay",
            "0",
            "-i",
            "source.gif",
            "-map",
            "0:v:0",
            "-an",
            "-fps_mode",
            "passthrough",
            "-frames:v",
            "17",
            "-pix_fmt",
            "rgba",
            "frame-%02d.png",
        ],
        &scratch.0,
    )?;
    let output_dir = Path::new(output)
        .parent()
        .and_then(Path::to_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::refused("GIF_OUTPUT_PARENT_REQUIRED"))?;
    let mut candidates = Vec::new();
    for (index, (at, ms)) in timings.iter().enumerate() {
        let bytes = read_bounded(
            &scratch.0.join(format!("frame-{:02}.png", index + 1)),
            64 * 1024 * 1024,
        )?;
        let info = png::inspect(&bytes)?;
        check(
            u64::from(info.width) == width
                && u64::from(info.height) == height
                && !info.has_transparent_pixels,
            "GIF_DECODED_FRAME_MUST_BE_OPAQUE",
        )?;
        candidates.push((
            format!("{output_dir}/{prefix}-{:02}.png", index + 1),
            bytes,
            *at,
            *ms,
        ));
    }
    check(
        !scratch
            .0
            .join(format!("frame-{:02}.png", frames.len() + 1))
            .exists(),
        "GIF_FRAME_COUNT_CHANGED",
    )?;
    check(
        root.read(input, 50 * 1024 * 1024)? == raw
            && root.read(draft_path, 1024 * 1024)? == draft_raw
            && root.read(evidence, 1024 * 1024)? == evidence_raw
            && crate::read_json(root, &options.packet)? == *packet,
        "GIF_INPUT_CHANGED",
    )?;
    let mut assets = Vec::new();
    let mut requirements = Vec::new();
    let mut poses = Vec::new();
    for (index, (path, bytes, at, ms)) in candidates.iter().enumerate() {
        let id = format!("{prefix}-{:02}", index + 1);
        assets.push(json!({"assetId":id,"path":path,"sha256":format!("sha256:{}",sha256(bytes)),"width":width,"height":height,"alpha":"opaque",
            "provenance":{"executor":"user-supplied","source":format!("decoded GIF {}; execution declaration {} (not provider proof)", sha256(&raw), sha256(&evidence_raw))}}));
        requirements.push(json!({"assetId":id,"width":width,"height":height,"alpha":"opaque","brief":format!("GIF source frame {} at {}ms; delay {}ms; motion requires visual review",index+1,timings[..index].iter().map(|(_,ms)| ms).sum::<u64>(),ms)}));
        poses.push(json!({"frame":at,"assetId":id}));
    }
    let receipt = seal(
        json!({"schemaVersion":"sac.gif-intake/1","candidateOnly":true,"directionApproved":false,
        "sourcePacketHash":packet["contentHash"],"source":packet["plan"]["source"],"boardHash":packet["plan"]["boardHash"],
        "input":{"path":input,"sha256":format!("sha256:{}",sha256(&raw))},"executionEvidence":{"path":evidence,"sha256":format!("sha256:{}",sha256(&evidence_raw)),"authenticated":false},
        "fps":fps,"playback":"once-hold","sourceDurationMs":elapsed_ms,"durationFrames":(elapsed_ms*fps).div_ceil(1000),
        "assets":assets,"planFragment":{"assets":requirements,"poses":poses},"timingPolicy":"ceil cumulative source time; hold last frame; no looping or time stretching"}),
    )?;
    contracts.validate("sac", "gif-intake-v1", &receipt)?;
    // Preflight every destination before the first publication. A later I/O fault is unknown.
    for (path, _, _, _) in &candidates {
        check(!root.exists(path)?, "GIF_FRAME_DESTINATION_EXISTS")?;
    }
    for (path, bytes, _, _) in &candidates {
        root.publish(path, bytes).map_err(|e| {
            Error::unknown(format!("GIF_IMPORT_PARTIAL: inspect {output_dir}: {e}"))
        })?;
    }
    crate::publish(root, output, &receipt)
        .map_err(|e| Error::unknown(format!("GIF_IMPORT_PARTIAL: inspect {output_dir}: {e}")))?;
    Ok(
        json!({"status":"gif_candidate","candidateOnly":true,"path":output,"contentHash":receipt["contentHash"],"frameCount":frames.len(),"directionApproved":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_decoder_preserves_frames_timing_and_never_approves_direction() {
        // Local GIF codec test only. No provider or production success claim.
        let temp = tempfile::tempdir().unwrap();
        let root = Root::new(temp.path()).unwrap();
        let contracts =
            crate::contracts::Contracts::new(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let packet = json!({"contentHash":format!("sha256:{}", "a".repeat(64)),"plan":{"source":{"narrativePackageSha256":format!("sha256:{}","c".repeat(64)),"releaseBundleSha256":format!("sha256:{}","d".repeat(64)),"sceneMapSha256":format!("sha256:{}","e".repeat(64))},"boardHash":format!("sha256:{}", "b".repeat(64))}});
        fs::write(
            temp.path().join("packet.json"),
            serde_json::to_vec(&packet).unwrap(),
        )
        .unwrap();
        let mut raw = b"GIF89a\x01\0\x01\0\x80\0\0\0\0\0\xff\xff\xff".to_vec();
        for pixel in [0x44, 0x4c] {
            raw.extend_from_slice(b"\x21\xf9\x04\0\x14\0\0\0\x2c\0\0\0\0\x01\0\x01\0\0\x02\x02");
            raw.extend_from_slice(&[pixel, 1, 0]);
        }
        raw.push(0x3b);
        fs::write(temp.path().join("source.gif"), &raw).unwrap();
        fs::write(
            temp.path().join("evidence.json"),
            b"{\"kind\":\"codec-test-fixture\",\"status\":\"user-supplied\"}",
        )
        .unwrap();
        let mut draft = json!({"input":"source.gif","assetPrefix":"test","fps":12,"playback":"once-hold","executionEvidence":"evidence.json"});
        fs::write(
            temp.path().join("draft.json"),
            serde_json::to_vec(&draft).unwrap(),
        )
        .unwrap();
        let options = Options {
            operation: "import-gif".into(),
            project_root: temp.path().into(),
            owner_root: PathBuf::new(),
            packet: "packet.json".into(),
            draft: Some("draft.json".into()),
            output: Some("animatic/gif-test/receipt.json".into()),
            pack: None,
            expected_hash: None,
            reviewer: None,
            decision: None,
            note: String::new(),
        };
        let result = import(&root, &options, &packet, &contracts).unwrap();
        assert_eq!(result["frameCount"], 2);
        let receipt = crate::read_json(&root, "animatic/gif-test/receipt.json").unwrap();
        assert_eq!(receipt["directionApproved"], false);
        assert_eq!(receipt["sourceDurationMs"], 400);
        assert_eq!(receipt["durationFrames"], 5);
        assert_eq!(receipt["planFragment"]["poses"][1]["frame"], 3);
        assert_ne!(
            receipt["assets"][0]["sha256"],
            receipt["assets"][1]["sha256"]
        );
        let mut inspect_options = Options {
            operation: "inspect-gif".into(),
            project_root: temp.path().into(),
            owner_root: PathBuf::new(),
            packet: "packet.json".into(),
            pack: Some("animatic/gif-test/receipt.json".into()),
            draft: None,
            output: None,
            expected_hash: None,
            reviewer: None,
            decision: None,
            note: String::new(),
        };
        assert_eq!(
            inspect(&root, &inspect_options, &packet, &contracts).unwrap()["status"],
            "gif_inspected"
        );
        fs::write(
            temp.path().join("evidence.json"),
            b"{\"status\":\"outcome_unknown\"}",
        )
        .unwrap();
        assert!(inspect(&root, &inspect_options, &packet, &contracts).is_err());
        inspect_options.output = Some("animatic/unknown/receipt.json".into());
        inspect_options.draft = Some("draft.json".into());
        assert!(import(&root, &inspect_options, &packet, &contracts).is_err());
        assert!(!temp.path().join("animatic/unknown").exists());
        fs::write(
            temp.path().join("evidence.json"),
            b"{\"kind\":\"codec-test-fixture\",\"status\":\"user-supplied\"}",
        )
        .unwrap();
        assert!(import(&root, &options, &packet, &contracts).is_err());
        // Low destination FPS must refuse, rather than silently dropping a source frame.
        let mut fast = raw.clone();
        fast[23] = 2;
        fast[46] = 2;
        fs::write(temp.path().join("source.gif"), &fast).unwrap();
        let mut second = options;
        second.output = Some("animatic/fast/receipt.json".into());
        assert!(import(&root, &second, &packet, &contracts).is_err());
        assert!(!temp.path().join("animatic/fast").exists());
        draft["playback"] = json!("loop");
        fs::write(
            temp.path().join("draft.json"),
            serde_json::to_vec(&draft).unwrap(),
        )
        .unwrap();
        assert!(import(&root, &second, &packet, &contracts).is_err());
    }
}
