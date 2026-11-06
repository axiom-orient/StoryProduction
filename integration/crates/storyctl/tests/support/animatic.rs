//! Native Story -> NVC -> SAC -> SMC mechanical media qualification.
//! Generated PNGs and review actors are test fixtures, never artwork or human approval evidence.
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use storyctl::effects::{
    hash, json as codec,
    process::{invoke, ProcessSpec},
};

type Result<T> = std::result::Result<T, String>;
const BOARD: &str = "storyboards/boards/full.json";
const BOARD_REVIEW: &str = "storyboards/reviews/full.json";
const PLAN: &str = "animatic/story-native-plan.json";
const PLAN_REVIEW: &str = "animatic/story-native-review.json";
const ACTOR: &str = "SIMULATED_TEST_ONLY";
const ATTEMPT: &str = "story-native-media";

fn check(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing or invalid {key}: {value}"))
}
fn path(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| format!("non-UTF-8 test path: {}", path.display()))
}
fn read_json(path: &Path) -> Result<Value> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    codec::parse_object(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}
fn write_json(path: &Path, value: &Value) -> Result<()> {
    let parent = path.parent().ok_or("test artifact has no parent")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let bytes = codec::stable_bytes(value).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

fn selected(variable: &str, candidates: &[PathBuf]) -> Result<PathBuf> {
    let candidates = std::env::var_os(variable)
        .map(|v| vec![PathBuf::from(v)])
        .unwrap_or_else(|| candidates.to_vec());
    for candidate in candidates {
        if !candidate.is_absolute() {
            return Err(format!("{variable} must select an absolute path"));
        }
        if candidate.is_file() {
            let resolved = candidate.canonicalize().map_err(|e| e.to_string())?;
            check(
                std::fs::symlink_metadata(&resolved)
                    .map_err(|e| e.to_string())?
                    .is_file(),
                "media tool must be a regular file",
            )?;
            return Ok(resolved);
        }
    }
    Err(format!(
        "{variable} has no installed test tool; this required media test was not executed"
    ))
}

struct Flow<'a> {
    root: &'a Path,
    workspace: &'a Path,
    storyctl: &'a Path,
    geultu: &'a Path,
    nvc: &'a Path,
    environment: BTreeMap<String, String>,
    calls: usize,
}
impl Flow<'_> {
    fn call(&mut self, arguments: &[&str], timeout: u64) -> Result<Value> {
        self.calls += 1;
        let mut argv = vec![
            path(self.storyctl)?.into(),
            "--story-root".into(),
            path(self.root)?.into(),
            "text".into(),
            "--workspace".into(),
            path(self.workspace)?.into(),
            "--geultu-bin".into(),
            path(self.geultu)?.into(),
            "--nvc-bin".into(),
            path(self.nvc)?.into(),
            "animatic".into(),
        ];
        argv.extend(arguments.iter().map(|v| (*v).to_owned()));
        let output = invoke(&ProcessSpec {
            argv,
            input: Vec::new(),
            cwd: self.root.to_owned(),
            timeout: Duration::from_secs(timeout),
            env: self.environment.clone(),
            pass_fds: Vec::new(),
            max_output_bytes: 16 * 1024 * 1024,
        })
        .map_err(|e| format!("Story animatic {arguments:?}: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "Story animatic {arguments:?} exited {}: stdout={} stderr={}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        codec::parse_object(&output.stdout)
            .map_err(|e| format!("Story animatic {arguments:?}: {e}"))
    }
}

// One tiny deterministic PNG fixture encoder. This is test data, not a production pixel path.
fn png(width: u32, height: u32, cutout: bool) -> Vec<u8> {
    fn crc(bytes: &[u8]) -> u32 {
        let mut value = 0xffff_ffff_u32;
        for byte in bytes {
            value ^= *byte as u32;
            for _ in 0..8 {
                value = (value >> 1) ^ if value & 1 == 1 { 0xedb8_8320 } else { 0 };
            }
        }
        value ^ 0xffff_ffff
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], payload: &[u8]) {
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(payload);
        let mut bound = kind.to_vec();
        bound.extend_from_slice(payload);
        out.extend_from_slice(&crc(&bound).to_be_bytes());
    }
    let mut scanlines = Vec::new();
    for y in 0..height {
        scanlines.push(0);
        for x in 0..width {
            scanlines.extend_from_slice(&[
                if cutout { 230 } else { x as u8 },
                y as u8,
                if cutout { 210 } else { 60 },
                if cutout && x < width / 4 { 0 } else { 255 },
            ]);
        }
    }
    let mut compressed = vec![0x78, 0x01]; // zlib, stored DEFLATE blocks.
    let blocks = scanlines.chunks(u16::MAX as usize);
    let block_count = blocks.len();
    for (index, block) in blocks.enumerate() {
        compressed.push(if index + 1 == block_count { 1 } else { 0 });
        let length = block.len() as u16;
        compressed.extend_from_slice(&length.to_le_bytes());
        compressed.extend_from_slice(&(!length).to_le_bytes());
        compressed.extend_from_slice(block);
    }
    let (mut a, mut b) = (1_u32, 0_u32);
    for byte in &scanlines {
        a = (a + *byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    compressed.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &compressed);
    chunk(&mut out, b"IEND", &[]);
    out
}

pub fn verify(
    root: &Path,
    workspace: &Path,
    temp: &Path,
    storyctl: &Path,
    geultu: &Path,
    nvc: &Path,
) -> Result<()> {
    let sac = selected(
        "STORY_TEST_SAC",
        &[
            root.join("SceneAssetCompiler/target/release/sac-animatic"),
            root.join("SceneAssetCompiler/target/debug/sac-animatic"),
        ],
    )?;
    let smc = selected(
        "STORY_TEST_SMC",
        &[
            root.join("SceneMotionCompiler/target/release/smc-animatic"),
            root.join("SceneMotionCompiler/target/debug/smc-animatic"),
        ],
    )?;
    let ffmpeg = selected(
        "SMC_FFMPEG_BIN",
        &[
            PathBuf::from("/opt/homebrew/bin/ffmpeg"),
            PathBuf::from("/usr/local/bin/ffmpeg"),
            PathBuf::from("/usr/bin/ffmpeg"),
        ],
    )?;
    let ffprobe = selected(
        "SMC_FFPROBE_BIN",
        &[
            PathBuf::from("/opt/homebrew/bin/ffprobe"),
            PathBuf::from("/usr/local/bin/ffprobe"),
            PathBuf::from("/usr/bin/ffprobe"),
        ],
    )?;
    let chromium = selected(
        "SMC_CHROMIUM_PATH",
        &[
            PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
            PathBuf::from("/Applications/Chromium.app/Contents/MacOS/Chromium"),
            PathBuf::from("/usr/bin/chromium"),
            PathBuf::from("/usr/bin/google-chrome"),
        ],
    )?;
    let font = selected(
        "SMC_FONT_PATH",
        &[
            PathBuf::from("/System/Library/Fonts/AppleSDGothicNeo.ttc"),
            PathBuf::from("/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"),
        ],
    )?;
    let environment = BTreeMap::from([
        ("PATH".into(), "/story-test-no-interpreters".into()),
        ("SMC_FFMPEG_BIN".into(), path(&ffmpeg)?.into()),
        ("SMC_FFPROBE_BIN".into(), path(&ffprobe)?.into()),
        ("SMC_CHROMIUM_PATH".into(), path(&chromium)?.into()),
        ("SMC_FONT_PATH".into(), path(&font)?.into()),
    ]);
    let mut flow = Flow {
        root,
        workspace,
        storyctl,
        geultu,
        nvc,
        environment,
        calls: 0,
    };
    let descriptor = read_json(&workspace.join("story-project.json"))?;
    let project = workspace
        .join("narrative")
        .join(text(&descriptor, "projectId")?);
    let source = flow.call(
        &[
            "direction",
            "source",
            "--board",
            BOARD,
            "--board-review",
            BOARD_REVIEW,
        ],
        120,
    )?;
    let mut panels = Vec::new();
    let scenes = source["board"]["scenes"]
        .as_array()
        .ok_or("source board scenes absent")?;
    for scene in scenes {
        for panel in scene["panels"]
            .as_array()
            .ok_or("source board panels absent")?
        {
            let captions = panel["sourceRefs"]
                .as_array()
                .ok_or("sourceRefs absent")?
                .first()
                .map(|reference| reference["semanticId"].clone())
                .into_iter()
                .collect::<Vec<_>>();
            let camera = |frame, x| json!({"frame":frame,"x":x,"y":0,"scalePerMille":1000,"easing":"linear"});
            let motion = |frame, x, y| json!({"frame":frame,"x":x,"y":y,"scalePerMille":1000,"easing":"linear","rotationMilliDeg":0,"opacityPerMille":1000});
            panels.push(json!({"sceneId":scene["sceneId"],"panelId":panel["panelId"],"durationFrames":2,
                "camera":[camera(0,0),camera(1,2)],"layers":[
                    {"layerId":"background","depthPerMille":400,"poses":[{"frame":0,"assetId":"background"}],"motion":[motion(0,64,48),motion(1,64,48)]},
                    {"layerId":"actor","depthPerMille":1000,"poses":[{"frame":0,"assetId":"actor"}],"motion":[motion(0,40,24),motion(1,88,24)]}],"captions":captions}));
        }
    }
    check(
        !panels.is_empty(),
        "approved board must provide panels for media qualification",
    )?;
    let frame_count = panels.len() * 2;
    let plan = json!({"schemaVersion":"nvc.animatic-plan/1","source":source["source"],"boardHash":source["board"]["contentHash"],"boardReviewHash":source["boardReview"]["contentHash"],
        "fps":12,"canvas":{"width":640,"height":360,"background":"#111827"},
        "assets":[{"assetId":"background","width":128,"height":96,"alpha":"opaque","brief":"MECHANICAL_TEST_ONLY background"},
            {"assetId":"actor","width":32,"height":32,"alpha":"cutout","brief":"MECHANICAL_TEST_ONLY cutout"}],"panels":panels,"openQuestions":[]});
    write_json(&project.join("animatic/story-native-draft.json"), &plan)?;
    let submitted = flow.call(
        &[
            "direction",
            "submit",
            "--board",
            BOARD,
            "--board-review",
            BOARD_REVIEW,
            "--draft",
            "animatic/story-native-draft.json",
            "--output",
            PLAN,
        ],
        120,
    )?;
    check(
        submitted["status"] == "candidate",
        "direction submit must remain a candidate",
    )?;
    let plan_hash = text(&submitted, "contentHash")?.to_owned();
    let reviewed = flow.call(
        &[
            "direction",
            "review",
            "--board",
            BOARD,
            "--board-review",
            BOARD_REVIEW,
            "--plan",
            PLAN,
            "--expected-hash",
            &plan_hash,
            "--reviewer",
            ACTOR,
            "--decision",
            "approved",
            "--output",
            PLAN_REVIEW,
        ],
        120,
    )?;
    check(
        reviewed["decision"] == "approved",
        "synthetic direction review was not recorded",
    )?;
    let asset_root = temp.join("story-native-media");
    std::fs::create_dir_all(asset_root.join("animatic")).map_err(|e| e.to_string())?;
    let asset_root_arg = path(&asset_root)?;
    let packet = flow.call(
        &[
            "packet",
            "--asset-root",
            asset_root_arg,
            "--board",
            BOARD,
            "--board-review",
            BOARD_REVIEW,
            "--plan",
            PLAN,
            "--review",
            PLAN_REVIEW,
        ],
        120,
    )?;
    check(
        packet["status"] == "exported",
        "Story packet export was not confirmed",
    )?;
    let packet_path = text(&packet, "packet")?.to_owned();
    let captured_packet = read_json(&asset_root.join(&packet_path))?;
    check(
        captured_packet["contentHash"] == packet["packetHash"],
        "Story packet hash differs from producer packet",
    )?;
    let mut assets = Vec::new();
    for (id, width, height, cutout) in [("background", 128, 96, false), ("actor", 32, 32, true)] {
        let bytes = png(width, height, cutout);
        let relative = format!("animatic/{id}.png");
        std::fs::write(asset_root.join(&relative), &bytes).map_err(|e| e.to_string())?;
        assets.push(json!({"assetId":id,"path":relative,"sha256":hash::sha256_bytes(&bytes),"width":width,"height":height,
            "alpha":if cutout {"cutout"} else {"opaque"},"provenance":{"executor":"user-supplied","source":"MECHANICAL_TEST_ONLY; generated deterministic bytes are not user artwork"}}));
    }
    write_json(
        &asset_root.join("asset-draft.json"),
        &json!({"schemaVersion":"sac.animatic-assets/1","packetHash":packet["packetHash"],"assets":assets}),
    )?;
    let sac_arg = path(&sac)?;
    let pack = flow.call(
        &[
            "assets",
            "--asset-root",
            asset_root_arg,
            "--sac-bin",
            sac_arg,
            "submit",
            "--packet",
            &packet_path,
            "--draft",
            "asset-draft.json",
            "--output",
            "animatic/assets.json",
        ],
        120,
    )?;
    check(
        pack["status"] == "candidate",
        "asset submit must remain a candidate",
    )?;
    let inspected = flow.call(
        &[
            "assets",
            "--asset-root",
            asset_root_arg,
            "--sac-bin",
            sac_arg,
            "inspect",
            "--packet",
            &packet_path,
            "--pack",
            "animatic/assets.json",
        ],
        120,
    )?;
    check(
        inspected["status"] == "inspected" && inspected["packHash"] == pack["contentHash"],
        "asset inspection does not bind candidate bytes",
    )?;
    let pack_hash = text(&pack, "contentHash")?.to_owned();
    let asset_review = flow.call(
        &[
            "assets",
            "--asset-root",
            asset_root_arg,
            "--sac-bin",
            sac_arg,
            "review",
            "--packet",
            &packet_path,
            "--pack",
            "animatic/assets.json",
            "--expected-hash",
            &pack_hash,
            "--reviewer",
            ACTOR,
            "--decision",
            "approved",
            "--output",
            "animatic/assets-review.json",
        ],
        120,
    )?;
    check(
        asset_review["decision"] == "approved" && asset_review["packHash"] == pack["contentHash"],
        "synthetic asset review did not bind candidate bytes",
    )?;
    let smc_arg = path(&smc)?;
    let rendered = flow.call(
        &[
            "render",
            "--asset-root",
            asset_root_arg,
            "--smc-bin",
            smc_arg,
            "--board",
            BOARD,
            "--board-review",
            BOARD_REVIEW,
            "--plan",
            PLAN,
            "--review",
            PLAN_REVIEW,
            "--assets",
            "animatic/assets.json",
            "--asset-review",
            "animatic/assets-review.json",
            "--attempt",
            ATTEMPT,
        ],
        1200,
    )?;
    check(
        rendered["schemaVersion"] == "story.animatic-observation/1"
            && rendered["status"] == "candidate"
            && rendered["sourceStatus"] == "current",
        "Story did not observe a current native render candidate",
    )?;
    let receipt = &rendered["render"];
    check(
        receipt["schemaVersion"] == "smc.animatic-result/1"
            && receipt["status"] == "candidate"
            && receipt["finalApproval"] == false
            && receipt["audio"] == "none"
            && receipt["frameCount"].as_u64() == Some(frame_count as u64),
        "render candidate claimed approval or incorrect media",
    )?;
    let render_root = asset_root.join(format!("animatic/renders/{ATTEMPT}"));
    check(
        read_json(&render_root.join("result.json"))? == *receipt,
        "Story changed the SMC producer receipt",
    )?;
    let outputs = receipt["artifacts"]
        .as_array()
        .ok_or("SMC candidate artifacts absent")?;
    check(
        outputs.len() == 6
            && receipt["gifPreview"]["mediaType"] == "image/gif"
            && receipt["gifPreview"]["nativeGeneration"] == false
            && receipt["gifPreview"]["containsAudio"] == false,
        "SMC candidate must bind its six artifacts and an honest rendered GIF preview",
    )?;
    for artifact in outputs {
        let bytes =
            std::fs::read(asset_root.join(text(artifact, "path")?)).map_err(|e| e.to_string())?;
        check(
            artifact["sha256"] == hash::sha256_bytes(&bytes),
            "native media artifact hash changed",
        )?;
    }
    let request = read_json(&render_root.join("request.json"))?;
    for (field, selected_path) in [("encoder", &ffmpeg), ("probe", &ffprobe)] {
        check(
            request[field]["path"] == json!(selected_path)
                && request[field]["sha256"]
                    == hash::sha256_file(selected_path).map_err(|e| e.to_string())?,
            "media tool lock does not bind selected bytes",
        )?;
    }
    check(
        request["rendererLock"]["browser"]["binaryPath"] == json!(chromium)
            && request["rendererLock"]["browser"]["binarySha256"]
                == hash::sha256_file(&chromium).map_err(|e| e.to_string())?
            && request["rendererLock"]["font"]["path"] == json!(font)
            && request["rendererLock"]["font"]["sha256"]
                == hash::sha256_file(&font).map_err(|e| e.to_string())?,
        "renderer lock does not bind selected Chrome/font bytes",
    )?;
    let first = std::fs::read(render_root.join("frames/000000.png")).map_err(|e| e.to_string())?;
    let second = std::fs::read(render_root.join("frames/000001.png")).map_err(|e| e.to_string())?;
    check(
        hash::sha256_bytes(&first) != hash::sha256_bytes(&second),
        "native frame movement was not observed",
    )?;
    for operation in ["status", "recover"] {
        let observed = flow.call(
            &[
                operation,
                "--asset-root",
                asset_root_arg,
                "--smc-bin",
                smc_arg,
                "--attempt",
                ATTEMPT,
            ],
            120,
        )?;
        check(
            observed == *receipt,
            "status/recover changed the original render receipt",
        )?;
    }
    eprintln!("PASS: {} Story animatic calls; {frame_count} real Chromium frames; byte-locked font/FFmpeg/FFprobe; candidate-only VP9 WebM; no interpreter PATH", flow.calls);
    Ok(())
}
