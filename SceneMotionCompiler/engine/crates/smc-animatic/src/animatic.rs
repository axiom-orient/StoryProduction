use crate::{
    browser, gif_preview,
    io::{self, Result},
    process, producer,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use jsonschema::{Draft, Retrieve, Uri, Validator};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

const NAMES: &[&str] = &[
    "preview.webm",
    "composition.html",
    "captions.vtt",
    "source.json",
    "request.json",
];
pub fn policy() -> Value {
    json!({"maxFrames":36000,"maxImageBytes":64*1024*1024,"maxTotalImageBytes":128*1024*1024,"maxFrameBytes":16*1024*1024,"maxFrameSequenceBytes":2u64*1024*1024*1024,"renderTimeoutMs":900000,"processTimeoutMs":600000,"processOutputBytes":1048576,"crf":24,"cpuUsed":2,"gifPreview":{"frameRate":6,"maxWidth":640,"maxBytes":134217728}})
}
#[derive(Clone)]
struct Local(Arc<BTreeMap<String, Value>>);
impl Retrieve for Local {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> std::result::Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.0
            .get(uri.as_str())
            .cloned()
            .ok_or_else(|| format!("UNREGISTERED_LOCAL_SCHEMA:{uri}").into())
    }
}
pub struct Context {
    pub root: io::Directory,
    validators: BTreeMap<String, Validator>,
    runtime: String,
    guard: String,
}
impl Context {
    pub fn new(root: &Path, owner_root: Option<&Path>) -> Result<Self> {
        let root = io::root(root)?;
        let owner_root = owner_root.map(io::root).transpose()?;
        let pinned_resource = |relative: &str, embedded: &str| -> Result<String> {
            let Some(owner) = &owner_root else {
                return Ok(embedded.to_string());
            };
            let bytes = io::read_at(owner, relative, 8 * 1024 * 1024)
                .map_err(|error| format!("OWNER_RESOURCE_UNAVAILABLE:{relative}:{error}"))?;
            io::check(
                bytes == embedded.as_bytes(),
                "OWNER_RESOURCE_BYTE_PIN_CHANGED",
            )?;
            String::from_utf8(bytes).map_err(|error| error.to_string())
        };
        let schemas = [
            include_str!("../../../../contracts/schemas/animatic/animatic-plan-v1.schema.json"),
            include_str!("../../../../contracts/schemas/animatic/animatic-review-v1.schema.json"),
            include_str!("../../../../contracts/schemas/animatic/animatic-packet-v1.schema.json"),
            include_str!("../../../../contracts/schemas/animatic/animatic-assets-v1.schema.json"),
            include_str!(
                "../../../../contracts/schemas/animatic/animatic-assets-review-v1.schema.json"
            ),
            include_str!("../../../../contracts/schemas/animatic/animatic-result-v1.schema.json"),
        ];
        let mut resources = BTreeMap::new();
        for (source, name) in schemas.into_iter().zip([
            "animatic-plan-v1",
            "animatic-review-v1",
            "animatic-packet-v1",
            "animatic-assets-v1",
            "animatic-assets-review-v1",
            "animatic-result-v1",
        ]) {
            let source = pinned_resource(
                &format!("contracts/schemas/animatic/{name}.schema.json"),
                source,
            )?;
            let schema = io::parse(source.as_bytes())?;
            resources.insert(
                schema["$id"].as_str().ok_or("SCHEMA_ID")?.to_string(),
                schema,
            );
        }
        let local = Local(Arc::new(resources.clone()));
        let mut validators = BTreeMap::new();
        for (id, schema) in resources {
            validators.insert(
                id,
                jsonschema::options()
                    .with_draft(Draft::Draft202012)
                    .should_validate_formats(true)
                    .with_retriever(local.clone())
                    .build(&schema)
                    .map_err(|e| e.to_string())?,
            );
        }
        let runtime = pinned_resource(
            "packages/production/src/animatic-runtime.js",
            include_str!("../../../../packages/production/src/animatic-runtime.js"),
        )?;
        let guard = pinned_resource(
            "packages/contracts/src/trackb-navigation-guard.js",
            include_str!("../../../../packages/contracts/src/trackb-navigation-guard.js"),
        )?;
        Ok(Self {
            root,
            validators,
            runtime,
            guard,
        })
    }
    fn validate(&self, name: &str, value: &Value) -> Result<()> {
        let id = format!("https://story-production.local/contracts/{name}");
        self.validators
            .get(&id)
            .ok_or("UNKNOWN_SCHEMA")?
            .validate(value)
            .map_err(|e| format!("SCHEMA_{name}:{e}"))?;
        producer::validate_hash(name, value)
    }
    fn load(&self, input: &Input) -> Result<Source> {
        let packet = io::read_json(&self.root, &input.packet)?;
        let pack = io::read_json(&self.root, &input.assets)?;
        let review = io::read_json(&self.root, &input.review)?;
        for (name, value) in [
            ("animatic-packet-v1", &packet),
            ("animatic-plan-v1", &packet["plan"]),
            ("animatic-review-v1", &packet["review"]),
            ("animatic-assets-v1", &pack),
            ("animatic-assets-review-v1", &review),
        ] {
            self.validate(name, value)?;
        }
        io::check(
            packet["review"]["decision"] == "approved"
                && packet["review"]["planHash"] == packet["plan"]["contentHash"]
                && io::canonical_hash(&packet["review"]["source"])?
                    == io::canonical_hash(&packet["plan"]["source"])?,
            "DIRECTION_NOT_APPROVED",
        )?;
        io::check(
            pack["packetHash"] == packet["contentHash"]
                && review["packetHash"] == packet["contentHash"]
                && review["packHash"] == pack["contentHash"]
                && review["decision"] == "approved",
            "ASSETS_NOT_APPROVED",
        )?;
        let plan = &packet["plan"];
        io::check(total_frames(plan) <= 36000, "RENDER_FRAME_LIMIT_EXCEEDED")?;
        io::check(
            number(&plan["canvas"]["width"])? % 2 == 0
                && number(&plan["canvas"]["height"])? % 2 == 0,
            "VP9_REQUIRES_EVEN_CANVAS",
        )?;
        let panels = array(&plan["panels"])?;
        let captions = array(&packet["captions"])?;
        io::check(panels.len() == captions.len(), "CAPTION_BINDING")?;
        for (panel, caption) in panels.iter().zip(captions) {
            let semantic: Vec<_> = array(&caption["lines"])?
                .iter()
                .map(|line| line["semanticId"].clone())
                .collect();
            io::check(
                caption["panelId"] == panel["panelId"] && json!(semantic) == panel["captions"],
                "CAPTION_BINDING",
            )?;
        }
        let mut requirements = BTreeMap::new();
        for asset in array(&plan["assets"])? {
            io::check(
                requirements
                    .insert(text(&asset["assetId"])?.to_string(), asset)
                    .is_none(),
                "ASSET_COVERAGE",
            )?;
        }
        io::check(
            array(&pack["assets"])?.len() == requirements.len(),
            "ASSET_COVERAGE",
        )?;
        let mut used = BTreeSet::new();
        for panel in panels {
            let duration = number(&panel["durationFrames"])?;
            let mut layer_ids = BTreeSet::new();
            let mut tracks = vec![&panel["camera"]];
            for layer in array(&panel["layers"])? {
                tracks.push(&layer["motion"]);
                io::check(
                    layer_ids.insert(text(&layer["layerId"])?),
                    "DUPLICATE_LAYER_ID",
                )?;
                let poses = array(&layer["poses"])?;
                io::check(poses[0]["frame"] == 0, "POSE_FRAME_ORDER")?;
                let mut previous = None;
                for pose in poses {
                    let frame = number(&pose["frame"])?;
                    io::check(
                        frame < duration && previous.is_none_or(|p| frame > p),
                        "POSE_FRAME_ORDER",
                    )?;
                    previous = Some(frame);
                    used.insert(text(&pose["assetId"])?);
                }
            }
            for track in tracks {
                let keys = array(track)?;
                io::check(
                    keys[0]["frame"] == 0
                        && number(&keys.last().unwrap()["frame"])? == duration - 1,
                    "KEYFRAME_ENDPOINTS_ORDER",
                )?;
                for pair in keys.windows(2) {
                    io::check(
                        number(&pair[0]["frame"])? < number(&pair[1]["frame"])?,
                        "KEYFRAME_ENDPOINTS_ORDER",
                    )?;
                }
            }
        }
        io::check(
            used.len() == requirements.len()
                && used.iter().all(|id| requirements.contains_key(*id)),
            "POSE_ASSET_COVERAGE",
        )?;
        let mut images = BTreeMap::new();
        let mut size = 0;
        for asset in array(&pack["assets"])? {
            let id = text(&asset["assetId"])?;
            let required = requirements.get(id).ok_or("ASSET_BINDING")?;
            io::check(
                !images.contains_key(id)
                    && ["width", "height"]
                        .iter()
                        .all(|key| asset[key].as_f64() == required[key].as_f64())
                    && asset["alpha"] == required["alpha"],
                "ASSET_BINDING",
            )?;
            let bytes = io::read_at(&self.root, text(&asset["path"])?, 64 * 1024 * 1024)?;
            size += bytes.len();
            io::check(size <= 128 * 1024 * 1024, "TOTAL_IMAGE_BYTES_EXCEEDED")?;
            io::check(
                io::sha(&bytes) == asset["sha256"]
                    && bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]),
                "PNG_HASH_CHANGED",
            )?;
            images.insert(
                id.to_string(),
                format!("data:image/png;base64,{}", STANDARD.encode(&bytes)),
            );
        }
        Ok(Source {
            packet,
            pack,
            review,
            images,
        })
    }
    pub fn inspect(&self, input: &Input) -> Result<Value> {
        self.load(input)?;
        Ok(json!({"status":"validated"}))
    }
    pub fn status(&self, attempt: &str) -> Result<Value> {
        attempt_path(attempt)?;
        self.status_inner(attempt)
            .map_err(|error| format!("OUTCOME_UNKNOWN:{error}"))
    }
    fn status_inner(&self, attempt: &str) -> Result<Value> {
        let dir = attempt_path(attempt)?;
        let directory = match self.root.directory(&dir, false) {
            Ok(directory) => directory,
            Err(error) if error.starts_with("NOT_FOUND:") => {
                return Ok(json!({"status":"not_started","attemptId":attempt}))
            }
            Err(error) => return Err(error),
        };
        self.status_at(attempt, &directory)
    }
    fn status_at(&self, attempt: &str, directory: &io::Directory) -> Result<Value> {
        let dir = attempt_path(attempt)?;
        let Some(request) = io::optional(directory, "request.json")? else {
            return Ok(
                json!({"status":"outcome_unknown","attemptId":attempt,"detail":"Torn worker claim; preserve this directory and use an explicit new attempt"}),
            );
        };
        io::check(
            request["contentHash"] == io::seal(request.clone())?["contentHash"],
            "REQUEST_HASH_CHANGED",
        )?;
        io::check(
            request["schemaVersion"] == "smc.animatic-request/1"
                && request["attemptId"] == attempt
                && request["workerIdentity"]
                    .as_str()
                    .is_some_and(|v| !v.is_empty())
                && request["workerPid"].as_u64().is_some_and(|v| v > 0),
            "INVALID_WORKER_CLAIM",
        )?;
        if let Some(result) = io::optional(directory, "result.json")? {
            self.validate("animatic-result-v1", &result)?;
            io::check(
                result["requestHash"] == request["contentHash"] && result["attemptId"] == attempt,
                "RESULT_BINDING_CHANGED",
            )?;
            if result["status"] == "candidate" {
                io::check(
                    result["finalApproval"] == false,
                    "CANDIDATE_IS_NOT_APPROVAL",
                )?;
                let artifacts = array(&result["artifacts"])?;
                let mut names = NAMES.to_vec();
                if request["policy"].get("gifPreview").is_some() {
                    io::check(result["gifPreview"].is_object(), "GIF_PREVIEW_MISSING")?;
                    names.push("preview.gif");
                }
                io::check(artifacts.len() == names.len(), "OUTPUT_ARTIFACT_COVERAGE")?;
                for (artifact, name) in artifacts.iter().zip(&names) {
                    io::check(
                        artifact["path"] == format!("{dir}/{name}"),
                        "OUTPUT_ARTIFACT_COVERAGE",
                    )?;
                    io::check(
                        io::hash_file(directory, name)? == artifact["sha256"],
                        "OUTPUT_HASH_CHANGED",
                    )?;
                }
                if result["gifPreview"].is_object() {
                    io::check(
                        result["gifPreview"]["sha256"] == io::hash_file(directory, "preview.gif")?,
                        "GIF_HASH_CHANGED",
                    )?;
                }
            }
            return Ok(result);
        }
        let live = process::identity(number(&request["workerPid"])?)?
            .is_some_and(|identity| Some(identity.as_str()) == request["workerIdentity"].as_str());
        Ok(
            json!({"status":if live{"running"}else{"outcome_unknown"},"attemptId":attempt,"requestHash":request["contentHash"],"detail":if live{"Worker has no terminal receipt yet"}else{"Worker disappeared; inspect partial files and clean up orphan processes before an explicit new attempt"}}),
        )
    }
    fn current_attempt(&self, attempt: &str, directory: &io::Directory) -> Result<()> {
        let current = self
            .root
            .directory(&attempt_path(attempt)?, false)
            .map_err(|error| format!("OUTCOME_UNKNOWN:ATTEMPT_DIRECTORY_CHANGED:{error}"))?;
        io::check(
            current.same_directory(directory)?,
            "OUTCOME_UNKNOWN:ATTEMPT_DIRECTORY_CHANGED",
        )
    }
    fn commit(&self, attempt: &str, result: Value) -> Result<Value> {
        let directory = self
            .root
            .directory(&attempt_path(attempt)?, false)
            .map_err(|error| format!("OUTCOME_UNKNOWN:{error}"))?;
        self.commit_at(attempt, &directory, result)
    }
    fn commit_at(&self, attempt: &str, directory: &io::Directory, result: Value) -> Result<Value> {
        self.commit_inner(attempt, directory, result)
            .map_err(|error| format!("OUTCOME_UNKNOWN:{error}"))
    }
    fn commit_inner(
        &self,
        attempt: &str,
        directory: &io::Directory,
        result: Value,
    ) -> Result<Value> {
        self.current_attempt(attempt, directory)?;
        let result = io::seal(result)?;
        self.validate("animatic-result-v1", &result)?;
        if let Err(error) = io::publish_json(directory, "result.json", &result) {
            if error == "IMMUTABLE_OUTPUT_CONFLICT" {
                if io::optional(directory, "result.json")?.is_none() {
                    return Err(error);
                }
            } else {
                return Ok(
                    json!({"status":"outcome_unknown","attemptId":attempt,"requestHash":result["requestHash"],"detail":format!("Terminal publication not confirmed; inspect this attempt before retrying: {error}")}),
                );
            }
        }
        self.current_attempt(attempt, directory)?;
        self.status_at(attempt, directory)
    }
    pub fn cancel(&self, attempt: &str) -> Result<Value> {
        let dir = attempt_path(attempt)?;
        let request = io::optional(&self.root, &format!("{dir}/request.json"))
            .map_err(|error| format!("OUTCOME_UNKNOWN:{error}"))?
            .ok_or("ATTEMPT_NOT_STARTED")?;
        self.status(attempt)?;
        self.commit(attempt,json!({"schemaVersion":"smc.animatic-result/1","status":"cancelled","attemptId":attempt,"requestHash":request["contentHash"]}))
    }
    pub fn recover(&self, attempt: &str) -> Result<Value> {
        let result = self.status(attempt)?;
        if result["status"] != "outcome_unknown" || result["requestHash"].is_null() {
            return Ok(result);
        }
        self.commit(attempt,json!({"schemaVersion":"smc.animatic-result/1","status":"outcome_unknown","attemptId":attempt,"requestHash":result["requestHash"],"detail":result["detail"]}))
    }
    pub fn render(&self, input: &Input) -> Result<Value> {
        let attempt = &input.attempt;
        let dir = attempt_path(attempt)?;
        let source = self.load(input)?;
        let binding = json!({"packetHash":source.packet["contentHash"],"assetPackHash":source.pack["contentHash"],"assetReviewHash":source.review["contentHash"],"reducedMotion":input.reduced});
        if let Some(previous) = io::optional(&self.root, &format!("{dir}/request.json"))
            .map_err(|error| format!("OUTCOME_UNKNOWN:{error}"))?
        {
            io::check(
                io::canonical_hash(&previous["sourceBinding"])? == io::canonical_hash(&binding)?,
                "ATTEMPT_INPUT_CHANGED",
            )?;
            return self.status(attempt);
        }
        let (lock, font) = browser::renderer_lock(input.reduced)?;
        let encoder = process::binding(
            &process::executable(
                &std::env::var("SMC_FFMPEG_BIN").unwrap_or("ffmpeg".into()),
                "FFMPEG",
            )?,
            "-version",
        )?;
        let probe = process::binding(
            &process::executable(
                &std::env::var("SMC_FFPROBE_BIN").unwrap_or("ffprobe".into()),
                "FFPROBE",
            )?,
            "-version",
        )?;
        let executable = io::io(std::env::current_exe())?;
        let implementation = io::sha(&io::read(&executable, 512 * 1024 * 1024)?);
        let request = io::seal(
            json!({"schemaVersion":"smc.animatic-request/1","attemptId":attempt,"sourceBinding":binding,"workerPid":std::process::id(),"workerIdentity":process::identity(std::process::id()as u64)?.ok_or("WORKER_IDENTITY_UNAVAILABLE")?,"rendererLock":lock,"encoder":encoder,"probe":probe,"policy":policy(),"implementationHash":implementation}),
        )?;
        let attempt_root = match self
            .root
            .create_exclusive_directory(&dir)
            .map_err(|error| {
                if error.starts_with("PUBLICATION_NOT_CONFIRMED:") {
                    format!("OUTCOME_UNKNOWN:{error}")
                } else {
                    error
                }
            })? {
            Some(directory) => directory,
            None => {
                if let Some(existing) = io::optional(&self.root, &format!("{dir}/request.json"))? {
                    io::check(
                        io::canonical_hash(&existing["sourceBinding"])?
                            == io::canonical_hash(&binding)?,
                        "ATTEMPT_INPUT_CHANGED",
                    )?;
                }
                return self.status(attempt);
            }
        };
        let claim = (|| -> Result<io::Directory> {
            io::publish_json(&attempt_root, "request.json", &request)?;
            attempt_root.directory("frames", true)
        })();
        let frames = match claim {
            Ok(directory) => directory,
            Err(error) => {
                return Ok(
                    json!({"status":"outcome_unknown","attemptId":attempt,"requestHash":request["contentHash"],"detail":format!("Worker claim publication not confirmed; inspect this attempt before retrying: {error}")}),
                )
            }
        };
        let started = Instant::now();
        let cancellation = || -> Result<()> {
            io::check(!io::interrupted(), "WORKER_INTERRUPTED")?;
            io::check(
                started.elapsed() < Duration::from_secs(900),
                "RENDER_TIMEOUT",
            )?;
            io::check(
                io::optional(&attempt_root, "result.json")?.is_none(),
                "TERMINAL_RESULT_EXISTS",
            )
        };
        let effect = (|| -> Result<Value> {
            let html = composition(&source, &font, input.reduced, &self.runtime)?;
            io::publish(&attempt_root, "composition.html", html.as_bytes())?;
            io::publish_json(
                &attempt_root,
                "source.json",
                &json!({"packet":source.packet,"assetPack":source.pack,"assetReview":source.review}),
            )?;
            let mut browser = browser::Browser::launch(&lock)?;
            let capture = (|| -> Result<()> {
                let mut client = browser::Cdp::connect(browser.port)?;
                let plan = &source.packet["plan"];
                client.configure(&lock, plan, &self.guard, &cancellation)?;
                let tree = client.send("Page.getFrameTree", json!({}), &cancellation)?;
                client.send(
                    "Page.setDocumentContent",
                    json!({"frameId":tree["frameTree"]["frame"]["id"],"html":html}),
                    &cancellation,
                )?;
                let ready=client.send("Runtime.evaluate",json!({"expression":"globalThis.animaticReady","awaitPromise":true,"returnByValue":true}),&cancellation)?;
                io::check(
                    ready["exceptionDetails"].is_null() && ready["result"]["value"] == true,
                    &format!("COMPOSITION_FAILED:{}", ready["exceptionDetails"]),
                )?;
                let cues = client.send(
                    "Runtime.evaluate",
                    json!({"expression":"globalThis.animatic.captions","returnByValue":true}),
                    &cancellation,
                )?;
                io::check(cues["exceptionDetails"].is_null(), "CAPTION_EXPORT_FAILED")?;
                io::publish(
                    &attempt_root,
                    "captions.vtt",
                    captions_vtt(&cues["result"]["value"], number(&plan["fps"])?)?.as_bytes(),
                )?;
                let mut total = 0u64;
                for frame in 0..total_frames(plan) {
                    cancellation()?;
                    let evaluated=client.send("Runtime.evaluate",json!({"expression":format!("globalThis.animatic.seekFrame({frame})"),"returnByValue":true}),&cancellation)?;
                    io::check(
                        evaluated["exceptionDetails"].is_null(),
                        "FRAME_COMPOSITION_FAILED",
                    )?;
                    let shot=client.send("Page.captureScreenshot",json!({"format":"png","fromSurface":true,"clip":{"x":0,"y":0,"width":plan["canvas"]["width"],"height":plan["canvas"]["height"],"scale":1}}),&cancellation)?;
                    let bytes = STANDARD
                        .decode(text(&shot["data"])?)
                        .map_err(|e| e.to_string())?;
                    total += bytes.len() as u64;
                    io::check(
                        bytes.len() <= 16 * 1024 * 1024 && total <= 2 * 1024 * 1024 * 1024,
                        "FRAME_STORAGE_LIMIT_EXCEEDED",
                    )?;
                    io::publish(&frames, &format!("{frame:06}.png"), &bytes)?;
                }
                client.send(
                    "Runtime.evaluate",
                    json!({"expression":"true","returnByValue":true}),
                    &cancellation,
                )?;
                io::check(
                    client.guard_ready && client.violations.is_empty(),
                    &format!("RENDER_RUNTIME_VIOLATION:{:?}", client.violations),
                )?;
                Ok(())
            })();
            let cleanup = browser.close();
            cleanup?;
            capture?;
            cancellation()?;
            let plan = &source.packet["plan"];
            let mut output_media = attempt_root.partial_media("preview.partial.webm")?;
            process::verify(&encoder)?;
            let args = vec![
                "-nostdin".into(),
                "-v".into(),
                "error".into(),
                "-f".into(),
                "image2pipe".into(),
                "-vcodec".into(),
                "png".into(),
                "-protocol_whitelist".into(),
                "pipe".into(),
                "-framerate".into(),
                number(&plan["fps"])?.to_string(),
                "-i".into(),
                "pipe:0".into(),
                "-frames:v".into(),
                total_frames(plan).to_string(),
                "-an".into(),
                "-c:v".into(),
                "libvpx-vp9".into(),
                "-b:v".into(),
                "0".into(),
                "-crf".into(),
                "24".into(),
                "-cpu-used".into(),
                "2".into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                "-f".into(),
                "webm".into(),
                "pipe:1".into(),
            ];
            let mut next_frame = 0;
            let mut frame_input = || -> Result<Option<Vec<u8>>> {
                if next_frame == total_frames(plan) {
                    return Ok(None);
                }
                let bytes =
                    io::read_at(&frames, &format!("{next_frame:06}.png"), 16 * 1024 * 1024)?;
                next_frame += 1;
                Ok(Some(bytes))
            };
            process::encode(
                Path::new(text(&encoder["path"])?),
                &args,
                output_media.file(),
                &mut frame_input,
                &cancellation,
            )?;
            process::verify(&probe)?;
            let args = [
                "-v",
                "error",
                "-protocol_whitelist",
                "pipe",
                "-f",
                "matroska",
                "-count_frames",
                "-show_streams",
                "-show_format",
                "-of",
                "json",
            ]
            .iter()
            .map(|v| v.to_string())
            .chain(std::iter::once("pipe:0".into()))
            .collect::<Vec<_>>();
            let observed = io::parse(&process::inspect_file(
                Path::new(text(&probe["path"])?),
                &args,
                output_media.reader()?,
                &cancellation,
            )?)?;
            let streams = array(&observed["streams"])?;
            io::check(streams.len() == 1, "WEBM_OBSERVATION_MISMATCH")?;
            let video = &streams[0];
            io::check(
                video["codec_type"] == "video"
                    && video["codec_name"] == "vp9"
                    && video["width"] == plan["canvas"]["width"]
                    && video["height"] == plan["canvas"]["height"]
                    && text(&video["nb_read_frames"])?.parse::<u64>().ok()
                        == Some(total_frames(plan))
                    && video["avg_frame_rate"] == format!("{}/1", number(&plan["fps"])?)
                    && text(&observed["format"]["format_name"])?.contains("webm"),
                "WEBM_OBSERVATION_MISMATCH",
            )?;
            let gif = gif_preview::render(
                &attempt_root,
                &frames,
                plan,
                &encoder,
                &probe,
                total_frames(plan),
                &cancellation,
            )?;
            let latest = self.load(input)?;
            io::check(
                io::canonical_hash(&json!([latest.packet, latest.pack, latest.review]))?
                    == io::canonical_hash(&json!([source.packet, source.pack, source.review]))?,
                "SOURCE_CHANGED_DURING_RENDER",
            )?;
            cancellation()?;
            output_media.publish(&attempt_root, "preview.webm")?;
            self.current_attempt(attempt, &attempt_root)?;
            let mut artifacts = Vec::new();
            for name in NAMES {
                let path = format!("{dir}/{name}");
                artifacts.push(json!({"sha256":io::hash_file(&attempt_root,name)?,"path":path}));
            }
            artifacts.push(json!({"sha256":gif["sha256"],"path":format!("{dir}/preview.gif")}));
            self.commit_at(attempt, &attempt_root, json!({"schemaVersion":"smc.animatic-result/1","status":"candidate","attemptId":attempt,"requestHash":request["contentHash"],"finalApproval":false,"audio":"none","frameCount":total_frames(plan),"fps":plan["fps"],"width":plan["canvas"]["width"],"height":plan["canvas"]["height"],"artifacts":artifacts,"gifPreview":gif}))
        })();
        match effect {
            Ok(result) => Ok(result),
            Err(error) => {
                if error.starts_with("OUTCOME_UNKNOWN:")
                    || error.starts_with("PUBLICATION_NOT_CONFIRMED:")
                {
                    return Ok(
                        json!({"status":"outcome_unknown","attemptId":attempt,"requestHash":request["contentHash"],"detail":error}),
                    );
                }
                self.current_attempt(attempt, &attempt_root)?;
                if io::optional(&attempt_root, "result.json")
                    .map_err(|error| format!("OUTCOME_UNKNOWN:{error}"))?
                    .is_some()
                {
                    return self.status(attempt);
                }
                self.commit_at(attempt, &attempt_root, json!({"schemaVersion":"smc.animatic-result/1","status":if error=="WORKER_INTERRUPTED"{"cancelled"}else{"failed"},"attemptId":attempt,"requestHash":request["contentHash"],"detail":error.chars().take(16000).collect::<String>()}))
            }
        }
    }
}

pub struct Input {
    pub packet: String,
    pub assets: String,
    pub review: String,
    pub attempt: String,
    pub reduced: bool,
}
struct Source {
    packet: Value,
    pack: Value,
    review: Value,
    images: BTreeMap<String, String>,
}
fn array(value: &Value) -> Result<&Vec<Value>> {
    value.as_array().ok_or("ARRAY_REQUIRED".into())
}
fn text(value: &Value) -> Result<&str> {
    value.as_str().ok_or("STRING_REQUIRED".into())
}
fn number(value: &Value) -> Result<u64> {
    value.as_u64().ok_or("INTEGER_REQUIRED".into())
}
fn total_frames(plan: &Value) -> u64 {
    plan["panels"]
        .as_array()
        .map(|panels| {
            panels
                .iter()
                .map(|p| p["durationFrames"].as_u64().unwrap_or(0))
                .sum()
        })
        .unwrap_or(0)
}
fn attempt_path(id: &str) -> Result<String> {
    io::check(
        !id.is_empty()
            && id.len() <= 64
            && id.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric())
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
        "INVALID_ATTEMPT_ID",
    )?;
    Ok(format!("animatic/renders/{id}"))
}
fn composition(source: &Source, font: &[u8], reduced: bool, runtime: &str) -> Result<String> {
    fn literal(value: &Value) -> String {
        value
            .to_string()
            .replace('<', "\\u003c")
            .replace('\u{2028}', "\\u2028")
            .replace('\u{2029}', "\\u2029")
    }
    let plan = &source.packet["plan"];
    let runtime = runtime
        .replace("export function", "function")
        .replace("export async function", "async function");
    Ok(format!("<!doctype html><html lang=\"ko\"><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src data:; font-src data:; style-src 'unsafe-inline'; script-src 'unsafe-inline'\"><style>@font-face{{font-family:AnimaticLocked;src:url(data:font/ttf;base64,{})}}html,body{{margin:0;width:100%;height:100%;overflow:hidden;background:{}}}canvas{{display:block}}p{{position:absolute;clip-path:inset(50%);width:1px;height:1px;overflow:hidden}}</style><canvas width=\"{}\" height=\"{}\" role=\"img\"></canvas><p></p><script>\n{runtime}\nglobalThis.animaticReady=installComposition({},{},{});</script></html>",STANDARD.encode(font),text(&plan["canvas"]["background"])?,number(&plan["canvas"]["width"])?,number(&plan["canvas"]["height"])?,literal(&source.packet),literal(&json!(source.images)),reduced))
}
fn captions_vtt(cues: &Value, fps: u64) -> Result<String> {
    fn time(frame: u64, fps: u64) -> String {
        let ms = (frame * 1000 + fps / 2) / fps;
        let sec = ms / 1000;
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            sec / 3600,
            (sec / 60) % 60,
            sec % 60,
            ms % 1000
        )
    }
    let mut result = String::from("WEBVTT\n\n");
    for cue in array(cues)? {
        let start = number(&cue["startFrame"])?;
        let end = number(&cue["endFrame"])?;
        io::check(end > start && end <= 36000, "CAPTION_EXPORT_TIME")?;
        let copy = text(&cue["text"])?
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        result.push_str(&format!(
            "{}\n{} --> {}\n{copy}\n\n",
            text(&cue["cueId"])?,
            time(start, fps),
            time(end, fps)
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hashes_match_js_safe_integer_canonical() {
        assert_eq!(
            io::canonical_hash(&json!({"a":-0.0,"z":1.0})).unwrap(),
            io::sha(br#"{"a":0,"z":1}"#)
        );
    }
    #[test]
    fn strict_duplicate_rejected() {
        assert!(io::parse(br#"{"a":1,"a":2}"#).is_err());
    }
    #[test]
    fn attempts_never_traverse() {
        for id in ["", "../x", "x/y", "-bad"] {
            assert!(attempt_path(id).is_err());
        }
    }
    #[test]
    fn vtt_has_exact_frame_rounding() {
        let packet = json!({"plan":{"fps":24,"panels":[{"durationFrames":1}]},"captions":[{"lines":[{"text":"<a>&"}]}]});
        assert_eq!(
            captions_vtt(&packet).unwrap(),
            "WEBVTT\n\n1\n00:00:00.000 --> 00:00:00.042\n&lt;a&gt;&amp;\n\n"
        );
    }
}
