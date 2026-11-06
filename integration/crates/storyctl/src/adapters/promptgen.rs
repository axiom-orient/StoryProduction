//! PromptGen process adapter. Candidate validation never adopts an owner artifact.
use crate::effects::{files, hash, json, process};
use crate::error::{Error, Result};
use crate::workflows::capability::{self, JOB_SCHEMA, REQUEST_SCHEMA, RESULT_SCHEMA};
use crate::{
    contracts::SchemaRegistry,
    roots::{CapabilityRoot, StoryResourceRoot},
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use story_integration_core::Outcome;

const MAX_DIAGNOSTIC_BYTES: usize = 64 * 1024;
const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;
const PAYLOAD_SCHEMA: &str = "ipc-image-prompt-v1.schema.json";
const COMPILATION_SCHEMA: &str = "ipc-compilation-v1.schema.json";
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 240;
pub const DEFAULT_ARTIFACT_WAIT_SECONDS: u64 = 30;
const CLEANUP_GRACE_SECONDS: u64 = 5;

pub struct PromptgenOptions {
    pub request: PathBuf,
    pub output_dir: PathBuf,
    pub promptgen_bin: PathBuf,
    pub codex_binary: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
    pub timeout_seconds: u64,
    pub artifact_wait_seconds: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Compile,
    Generate,
    CodexEdit,
    ApiGenerate,
    Edit,
}
impl Mode {
    fn argument(self) -> &'static str {
        match self {
            Self::Compile => "prompt-only",
            Self::Generate | Self::CodexEdit => "codex-imagegen",
            Self::ApiGenerate => "openai-image-generate",
            Self::Edit => "openai-image-edit",
        }
    }
    fn external(self) -> bool {
        self != Self::Compile
    }
}

fn mode(request: &Value) -> Result<Mode> {
    let (mode, kind, network) = match request["capability"].as_str() {
        Some("image.prompt.compile") => (Mode::Compile, "local_process", "denied"),
        Some("image.generate") if request["provider"]["kind"] == "openai" => {
            (Mode::ApiGenerate, "openai", "required")
        }
        Some("image.generate") => (Mode::Generate, "codex", "required"),
        Some("image.edit") if request["provider"]["kind"] == "codex" => {
            (Mode::CodexEdit, "codex", "required")
        }
        Some("image.edit") => (Mode::Edit, "openai", "required"),
        _ => {
            return Err(Error::not_applied(
                "PromptGen does not implement the requested capability",
            ))
        }
    };
    if request["provider"]["kind"] != kind || request["provider"]["networkPolicy"] != network {
        return Err(Error::not_applied(format!(
            "capability requires {kind} with networkPolicy={network}"
        )));
    }
    let references = request["references"].as_array().map_or(0, Vec::len);
    if (matches!(mode, Mode::Edit | Mode::CodexEdit) && references != 1)
        || (!matches!(mode, Mode::Edit | Mode::CodexEdit) && references != 0)
    {
        return Err(Error::not_applied(
            "image.edit requires one reference; other modes cannot consume references",
        ));
    }
    for output in request["expectedOutputs"]
        .as_array()
        .ok_or_else(|| Error::not_applied("expectedOutputs is missing"))?
    {
        let supported = if mode == Mode::Compile {
            output["kind"] == "prompt_compilation" && output["mediaType"] == "application/json"
        } else {
            (output["kind"] == "generation_receipt" && output["mediaType"] == "application/json")
                || (output["kind"] == "image_candidate"
                    && (output["mediaType"] == "image/png"
                        || (matches!(
                            mode,
                            Mode::Edit | Mode::ApiGenerate | Mode::Generate | Mode::CodexEdit
                        ) && output["mediaType"] == "image/gif")))
        };
        if !supported {
            return Err(Error::not_applied(
                "expectedOutputs contains an unsupported output",
            ));
        }
    }
    Ok(mode)
}

fn executable(selected: &Path) -> Result<PathBuf> {
    let absolute = if selected.is_absolute() {
        selected.to_owned()
    } else {
        std::env::current_dir()?.join(selected)
    };
    if let Ok(metadata) = std::fs::symlink_metadata(&absolute) {
        if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 {
            return files::non_symlink_path(&absolute);
        }
    }
    Err(Error::not_started(
        "--promptgen-bin must name an executable regular file",
    ))
}

fn validate_payload_mode(mode: Mode, payload: &Value) -> Result<()> {
    if mode == Mode::Compile {
        return Ok(());
    }
    let task = payload["task_mode"].as_str().unwrap_or("generate");
    let references = payload["references"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let supported = match mode {
        Mode::Generate | Mode::ApiGenerate => task == "generate" && references.is_empty(),
        Mode::Edit | Mode::CodexEdit => {
            task == "edit"
                && references.len() == 1
                && references[0]["index"] == 1
                && references[0]["role"] == "base"
        }
        Mode::Compile => true,
    };
    let model = payload["output"]["model"].as_str().unwrap_or("");
    let format = payload["output"]["format"].as_str().unwrap_or("");
    let output_supported = match mode {
        Mode::Generate | Mode::CodexEdit => {
            (mode == Mode::Generate && model == "gpt-image-2" && format == "png")
                || (matches!(model, "gpt-image-2.6-sunburst" | "gpt-image-2.6-flare")
                    && format == "gif")
        }
        Mode::ApiGenerate => {
            matches!(model, "gpt-image-2.6-sunburst" | "gpt-image-2.6-flare")
                && matches!(format, "png" | "gif")
        }
        Mode::Edit => {
            (model == "gpt-image-2" && format == "png")
                || (matches!(model, "gpt-image-2.6-sunburst" | "gpt-image-2.6-flare")
                    && matches!(format, "png" | "gif"))
        }
        Mode::Compile => true,
    };
    if !supported
        || !output_supported
        || (matches!(mode, Mode::ApiGenerate | Mode::Edit)
            && payload["output"]["size_policy"] == "provider")
    {
        return Err(Error::not_applied("capability and typed payload disagree: Codex accepts GPT Image 2 PNG generate or GPT Image 2.6 GIF generate/single-base edit; Image API requires an exact-size supported generate/edit profile"));
    }
    Ok(())
}

fn output_directory(
    capability_root: &CapabilityRoot,
    target: &Path,
    request_hash: &str,
) -> Result<files::Directory> {
    let root = capability_root.path();
    let absolute = if target.is_absolute() {
        target.to_owned()
    } else {
        root.join(target)
    };
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| Error::not_applied("output_dir must be inside --capability-root"))?;
    if relative.as_os_str().is_empty() {
        return Err(Error::not_applied(
            "output_dir must be a dedicated execution directory",
        ));
    }
    let path = capability::safe_relative(root, &relative.to_string_lossy())?;
    let directory = files::Directory::open(&path, true)?;
    if !directory.is_empty()? {
        return Err(Error::not_applied(
            "output_dir is not empty; inspect the existing execution instead of overwriting it",
        ));
    }
    directory.exclusive(".execution-owner", format!("{request_hash}\n").as_bytes())?;
    directory.verify_identity()?;
    Ok(directory)
}

fn capture(source: &Path, reference: &Value, maximum: usize) -> Result<Vec<u8>> {
    let expected_length = reference["byteLength"]
        .as_u64()
        .ok_or_else(|| Error::not_applied("input byteLength is invalid"))?;
    let limit =
        usize::try_from(expected_length).map_err(|_| Error::not_applied("input is too large"))?;
    if limit > maximum {
        return Err(Error::not_applied(
            "input exceeds the adapter snapshot policy",
        ));
    }
    let bytes = files::read_bytes_limited(source, limit)?;
    if bytes.len() as u64 != expected_length
        || hash::sha256_bytes(&bytes) != reference["sha256"].as_str().unwrap_or("")
    {
        return Err(Error::not_applied("input changed while snapshotting"));
    }
    Ok(bytes)
}

fn verify_snapshots(
    directory: &files::Directory,
    request_hash: &str,
    snapshots: &BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    directory.verify_identity()?;
    if directory.read_bytes_limited(".execution-owner", 80)?
        != format!("{request_hash}\n").as_bytes()
    {
        return Err(Error::not_applied("execution claim changed"));
    }
    for (name, bytes) in snapshots {
        if directory.read_bytes_limited(name, bytes.len())? != *bytes {
            return Err(Error::not_applied(format!(
                "captured execution input changed: {name}"
            )));
        }
    }
    Ok(())
}

fn compilation<'a>(registry: &SchemaRegistry, value: &'a Value) -> Result<&'a str> {
    registry.validate(COMPILATION_SCHEMA, value)?;
    let prompt = value["prompt"].as_str().filter(|v| !v.trim().is_empty());
    let diagnostics = value["diagnostics"].as_array();
    if value["kind"] != "image"
        || !matches!(
            value["status"].as_str(),
            Some("valid" | "valid_with_warnings")
        )
        || prompt.is_none()
        || diagnostics.is_none()
        || diagnostics
            .unwrap()
            .iter()
            .any(|v| !v.is_object() || v["severity"] == "error")
    {
        return Err(Error::not_applied(
            "PromptGen returned an invalid image compilation",
        ));
    }
    Ok(prompt.unwrap())
}

fn artifact(
    capability_root: &CapabilityRoot,
    output: &files::Directory,
    name: &str,
    bytes: &[u8],
    kind: &str,
    media_type: &str,
) -> Result<Value> {
    let path = output.path().join(name);
    let relative = path
        .strip_prefix(capability_root.path())
        .map_err(|_| Error::not_applied("candidate is outside --capability-root"))?
        .to_string_lossy();
    let digest = hash::sha256_bytes(bytes);
    let mut value = json!({"kind":kind,"identity":format!("artifact.{}",&digest[7..31]),"path":relative,"sha256":digest,"mediaType":media_type,"byteLength":bytes.len()});
    if media_type == "image/png" {
        if bytes.len() < 33
            || &bytes[..8] != b"\x89PNG\r\n\x1a\n"
            || &bytes[8..16] != b"\0\0\0\rIHDR"
        {
            return Err(Error::not_applied(
                "candidate image has no supported PNG dimensions",
            ));
        }
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        if width == 0 || height == 0 {
            return Err(Error::not_applied("candidate PNG has empty dimensions"));
        }
        value["width"] = json!(width);
        value["height"] = json!(height);
    }
    if media_type == "image/gif" {
        if bytes.len() < 13 || !matches!(&bytes[..6], b"GIF87a" | b"GIF89a") {
            return Err(Error::not_applied("candidate has no GIF header"));
        }
        value["width"] = json!(u16::from_le_bytes(bytes[6..8].try_into().unwrap()));
        value["height"] = json!(u16::from_le_bytes(bytes[8..10].try_into().unwrap()));
    }
    Ok(value)
}

fn candidates(
    capability_root: &CapabilityRoot,
    output: &files::Directory,
    mode: Mode,
    request: &Value,
    payload: &Value,
    registry: &SchemaRegistry,
) -> Result<(Vec<Value>, Value)> {
    let result_bytes =
        output.read_bytes_limited("promptgen-result.json", crate::effects::MAX_BYTES)?;
    let value = json::parse_object(&result_bytes)?;
    let mut artifacts = vec![artifact(
        capability_root,
        output,
        "promptgen-result.json",
        &result_bytes,
        if mode == Mode::Compile {
            "prompt_compilation"
        } else {
            "generation_receipt"
        },
        "application/json",
    )?];
    let mut receipt = json!({});
    if mode == Mode::Compile {
        compilation(registry, &value)?;
    } else {
        receipt = if matches!(mode, Mode::Generate | Mode::CodexEdit) {
            value["execution"].clone()
        } else {
            value.clone()
        };
        if !receipt.is_object()
            || receipt["backend"]
                != if matches!(mode, Mode::Generate | Mode::CodexEdit) {
                    "codex-imagegen"
                } else {
                    "openai-images"
                }
        {
            return Err(Error::not_applied("provider receipt backend mismatch"));
        }
        if matches!(mode, Mode::Generate | Mode::CodexEdit) {
            let prompt = compilation(registry, &value["compilation"])?;
            if receipt["compiled_prompt_sha256"] != hash::sha256_hex_bytes(prompt.as_bytes())
                || receipt["thread_id"].as_str().is_none_or(str::is_empty)
                || receipt["image_call_id"].as_str().is_none_or(str::is_empty)
                || receipt["process_exit_code"].as_i64() != Some(0)
            {
                return Err(Error::not_applied(
                    "receipt has no successful bound image call",
                ));
            }
        } else if receipt["provider"] != "openai"
            || receipt["http_status"].as_u64() != Some(200)
            || (mode == Mode::Edit
                && receipt["reference_sha256"].as_str()
                    != request["references"][0]["sha256"].as_str().map(|v| &v[7..]))
            || (mode == Mode::ApiGenerate
                && (receipt["reference_index"] != 0
                    || receipt["reference_bytes"] != 0
                    || receipt["endpoint_contract"]
                        != "POST https://api.openai.com/v1/images/generations"))
        {
            return Err(Error::not_applied(
                "edit receipt has no successful bound provider/reference response",
            ));
        }
        let gif = payload["output"]["format"] == "gif";
        let name = if gif { "image.gif" } else { "image.png" };
        let media_type = if gif { "image/gif" } else { "image/png" };
        if matches!(mode, Mode::Edit | Mode::ApiGenerate)
            && (receipt["model"] != payload["output"]["model"]
                || (gif
                    && ["format", "frame_count", "frame_duration_ms"]
                        .iter()
                        .any(|key| receipt[key] != payload["output"][key])))
        {
            return Err(Error::not_applied(
                "edit receipt model/animation profile mismatch",
            ));
        }
        if gif
            && matches!(mode, Mode::Generate | Mode::CodexEdit)
            && (["format", "frame_count", "frame_duration_ms", "size_policy"]
                .iter()
                .any(|key| {
                    let expected = if *key == "size_policy" && payload["output"][key].is_null() {
                        json!("exact")
                    } else {
                        payload["output"][key].clone()
                    };
                    receipt[key] != expected
                })
                || receipt["requested_model"] != payload["output"]["model"]
                || receipt["auth_mode"] != "chatgpt_subscription"
                || (mode == Mode::CodexEdit
                    && receipt["reference_sha256"].as_str()
                        != request["references"][0]["sha256"].as_str().map(|v| &v[7..])))
        {
            return Err(Error::not_applied(
                "Codex GIF receipt profile/reference differs from request",
            ));
        }
        let image_path = output.path().join(name);
        let image_bytes = output.read_bytes_limited(name, MAX_IMAGE_BYTES)?;
        let image = artifact(
            capability_root,
            output,
            name,
            &image_bytes,
            "image_candidate",
            media_type,
        )?;
        let hash_key = if matches!(mode, Mode::Generate | Mode::CodexEdit) {
            "sha256"
        } else {
            "output_sha256"
        };
        let path_key = if matches!(mode, Mode::Generate | Mode::CodexEdit) {
            "output_artifact"
        } else {
            "output_path"
        };
        let reported_path = receipt[path_key]
            .as_str()
            .map(PathBuf::from)
            .ok_or_else(|| Error::not_applied("receipt output path is missing"))?;
        let reported_path = if reported_path.is_absolute() {
            reported_path
        } else {
            output.path().join(reported_path)
        };
        if receipt[hash_key].as_str() != image["sha256"].as_str().map(|v| &v[7..])
            || reported_path != image_path
        {
            return Err(Error::not_applied("receipt image hash/path mismatch"));
        }
        for key in ["width", "height"] {
            if receipt[key].as_u64().is_none()
                || receipt[key] != image[key]
                || (!(gif && payload["output"]["size_policy"] == "provider")
                    && image[key] != payload["output"][key])
            {
                return Err(Error::not_applied(
                    "requested, receipt, and image dimensions differ",
                ));
            }
        }
        if output.read_bytes_limited(name, MAX_IMAGE_BYTES)? != image_bytes {
            return Err(Error::not_applied(
                "candidate image changed during validation",
            ));
        }
        artifacts.push(image);
    }
    for expected in request["expectedOutputs"].as_array().unwrap() {
        if !artifacts
            .iter()
            .any(|a| a["kind"] == expected["kind"] && a["mediaType"] == expected["mediaType"])
        {
            return Err(Error::not_applied(
                "candidate does not fulfill expectedOutputs",
            ));
        }
    }
    if output.read_bytes_limited("promptgen-result.json", crate::effects::MAX_BYTES)?
        != result_bytes
    {
        return Err(Error::not_applied(
            "provider receipt changed during validation",
        ));
    }
    let allowed = [
        "backend",
        "provider",
        "model",
        "request_id",
        "auth_mode",
        "requested_model",
        "provider_model",
        "model_identity_verified",
        "semantic_review_required",
        "reference_sha256",
        "size_policy",
        "format",
        "frame_count",
        "frame_duration_ms",
        "thread_id",
        "image_call_id",
        "sha256",
        "output_sha256",
        "width",
        "height",
    ];
    receipt
        .as_object_mut()
        .unwrap()
        .retain(|key, _| allowed.contains(&key.as_str()));
    Ok((artifacts, receipt))
}

pub fn run_promptgen(root: &Path, options: &PromptgenOptions) -> Result<Value> {
    let resource_root = StoryResourceRoot::select(root)?;
    let capability_root = resource_root.capability_root(None)?;
    run_promptgen_in_roots(&resource_root, &capability_root, options)
}

pub(crate) fn run_promptgen_in_roots(
    resource_root: &StoryResourceRoot,
    capability_root: &CapabilityRoot,
    options: &PromptgenOptions,
) -> Result<Value> {
    let process_timeout_seconds = options
        .timeout_seconds
        .checked_add(options.artifact_wait_seconds)
        .and_then(|seconds| seconds.checked_add(CLEANUP_GRACE_SECONDS))
        .ok_or_else(|| Error::not_applied("PromptGen timeout calculation overflowed"))?;
    if options.timeout_seconds == 0
        || options.timeout_seconds > process::MAX_PROCESS_TIMEOUT_SECONDS
        || options.artifact_wait_seconds > process::MAX_PROCESS_TIMEOUT_SECONDS
        || process_timeout_seconds > process::MAX_PROCESS_TIMEOUT_SECONDS
    {
        return Err(Error::not_applied(format!("timeout_seconds must be positive and the timeout plus artifact wait and cleanup grace must be at most {} seconds", process::MAX_PROCESS_TIMEOUT_SECONDS)));
    }
    let root = files::non_symlink_path(resource_root.path())?;
    let registry = SchemaRegistry::new(&root.join("integration/contracts"))?;
    let raw = files::read_bytes(&options.request)?;
    let request = json::parse_object(&raw)?;
    registry.validate(REQUEST_SCHEMA, &request)?;
    let mode = mode(&request)?;
    let payload_path = capability::safe_relative(
        capability_root.path(),
        request["payload"]["path"]
            .as_str()
            .ok_or_else(|| Error::not_applied("payload path is missing"))?,
    )?;
    let payload_bytes = capture(
        &payload_path,
        &request["payload"],
        crate::effects::MAX_BYTES,
    )?;
    let payload = json::parse_object(&payload_bytes)?;
    registry.validate(PAYLOAD_SCHEMA, &payload)?;
    validate_payload_mode(mode, &payload)?;
    if mode.external() {
        let media_type = if payload["output"]["format"] == "gif" {
            "image/gif"
        } else {
            "image/png"
        };
        if request["expectedOutputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["kind"] == "image_candidate" && a["mediaType"] != media_type)
            || request["provider"]["model"]
                .as_str()
                .is_some_and(|model| payload["output"]["model"] != model)
        {
            return Err(Error::not_applied(
                "requested provider model or image media type differs from typed payload",
            ));
        }
    }
    if matches!(mode, Mode::Edit | Mode::ApiGenerate)
        && std::env::var("OPENAI_API_KEY")
            .ok()
            .is_none_or(|key| key.trim().is_empty() || key.chars().any(char::is_control))
    {
        return Err(Error::not_started(
            "OPENAI_API_KEY missing or invalid; no provider request dispatched",
        ));
    }
    if !matches!(mode, Mode::Generate | Mode::CodexEdit)
        && (options.codex_binary.is_some() || options.codex_home.is_some())
    {
        return Err(Error::not_applied("Codex options require image.generate"));
    }
    let codex_binary = options
        .codex_binary
        .as_deref()
        .map(executable)
        .transpose()?;
    let codex_home = options
        .codex_home
        .as_ref()
        .map(|path| -> Result<PathBuf> {
            if path.is_absolute() {
                Ok(path.clone())
            } else {
                Ok(std::env::current_dir()?.join(path))
            }
        })
        .transpose()?;
    let mut snapshots = BTreeMap::from([
        ("capability-request.json".to_owned(), raw.clone()),
        ("input.json".to_owned(), payload_bytes.clone()),
    ]);
    let mut reference_name = None;
    if let Some(references) = request["references"].as_array() {
        for reference in references {
            let path = capability::safe_relative(
                capability_root.path(),
                reference["path"]
                    .as_str()
                    .ok_or_else(|| Error::not_applied("reference path is missing"))?,
            )?;
            let leaf = format!(
                "reference{}",
                path.extension()
                    .map(|v| format!(".{}", v.to_string_lossy()))
                    .unwrap_or_default()
            );
            snapshots.insert(leaf.clone(), capture(&path, reference, MAX_IMAGE_BYTES)?);
            reference_name = Some(leaf);
        }
    }
    let executable = executable(&options.promptgen_bin)?;
    let request_hash = hash::sha256_bytes(&raw);
    let output = output_directory(capability_root, &options.output_dir, &request_hash)?;
    let job_path = output.path().join("capability-job.json");
    let result_path = output.path().join("capability-result.json");
    let mut job = capability::create_job(&request, &request_hash)?;
    registry.validate(JOB_SCHEMA, &job)?;
    output.atomic_write_json("capability-job.json", &job)?;
    let mut command = vec![
        executable.to_string_lossy().into_owned(),
        "image".into(),
        "--input".into(),
        "-".into(),
        "--format".into(),
        "json".into(),
        "--result".into(),
        "promptgen-result.json".into(),
        "--mode".into(),
        mode.argument().into(),
    ];
    if mode.external() {
        command.extend([
            "--image-output".into(),
            if payload["output"]["format"] == "gif" {
                "image.gif".into()
            } else {
                "image.png".into()
            },
            "--timeout-seconds".into(),
            options.timeout_seconds.to_string(),
        ]);
    }
    if matches!(mode, Mode::Generate | Mode::CodexEdit) {
        command.extend([
            "--artifact-wait-seconds".into(),
            options.artifact_wait_seconds.to_string(),
        ]);
        if let Some(binary) = codex_binary {
            command.extend([
                "--codex-binary".into(),
                binary.to_string_lossy().into_owned(),
            ]);
        }
        if let Some(home) = codex_home {
            command.extend(["--codex-home".into(), home.to_string_lossy().into_owned()]);
        }
    }
    if let Some(name) = &reference_name {
        command.extend(["--reference-image".into(), name.clone()]);
    }
    let mut started = false;
    let mut binding_unknown = false;
    let mut definite_provider_rejection = false;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let execution: Result<(Vec<Value>, Value)> = (|| {
        for (name, bytes) in &snapshots {
            output.immutable(name, bytes)?;
        }
        verify_snapshots(&output, &request_hash, &snapshots)?;
        job = capability::transition_job(&job, "running", "promptgen-execution", None, None)?;
        registry.validate(JOB_SCHEMA, &job)?;
        output.atomic_write_json("capability-job.json", &job)?;
        let invocation = process::invoke(&process::ProcessSpec {
            argv: command.clone(),
            input: payload_bytes.clone(),
            cwd: PathBuf::from(format!("/dev/fd/{}", output.raw_fd())),
            timeout: Duration::from_secs(process_timeout_seconds),
            env: BTreeMap::new(),
            pass_fds: vec![output.raw_fd()],
            max_output_bytes: MAX_DIAGNOSTIC_BYTES,
        });
        let result = match invocation {
            Ok(value) => {
                started = true;
                value
            }
            Err(error) => {
                started = error.spawned;
                stdout = error.stdout.clone();
                stderr = error.stderr.clone();
                if started && verify_snapshots(&output, &request_hash, &snapshots).is_err() {
                    binding_unknown = true;
                }
                return Err(error);
            }
        };
        stdout = result.stdout;
        stderr = result.stderr;
        if let Err(error) = verify_snapshots(&output, &request_hash, &snapshots) {
            binding_unknown = true;
            return Err(error);
        }
        if !result.status.success() {
            if matches!(mode, Mode::Generate | Mode::CodexEdit)
                && payload["output"]["format"] == "gif"
                && result.status.code() == Some(5)
                && String::from_utf8_lossy(&stderr)
                    .starts_with("error[PROVIDER_REJECTED]: CODEX_GIF_SERVICE_REJECTED:")
            {
                definite_provider_rejection = true;
                return Err(Error::not_applied("Codex subscription provider rejected native GIF: HTTP 400 GIF output is not enabled for this request"));
            }
            return Err(Error::not_applied(format!(
                "PromptGen exited with status {}",
                result.status
            )));
        }
        if process::cancellation_requested() {
            return Err(Error::unknown(
                "process cancelled before candidate acceptance",
            ));
        }
        let candidate = candidates(
            capability_root,
            &output,
            mode,
            &request,
            &payload,
            &registry,
        )?;
        if process::cancellation_requested() {
            return Err(Error::unknown(
                "process cancelled before candidate acceptance",
            ));
        }
        if let Err(error) = verify_snapshots(&output, &request_hash, &snapshots) {
            binding_unknown = true;
            return Err(error);
        }
        Ok(candidate)
    })();
    let (result, stage) = match execution {
        Ok((artifacts, receipt)) => {
            let mut result =
                capability::terminal_result(&request, &request_hash, "succeeded", None);
            result["artifacts"] = json!(artifacts);
            result["providerReceipt"] = receipt;
            (result, "candidate-validated")
        }
        Err(error) => {
            let cancelled = process::cancellation_requested();
            if cancelled {
                job = capability::request_cancellation(&job)?;
            }
            let unknown = binding_unknown
                || (mode.external() && started && !definite_provider_rejection)
                || (error.outcome == Outcome::Unknown && (started || error.spawned));
            let status = if unknown {
                "outcome_unknown"
            } else if definite_provider_rejection {
                "failed"
            } else if cancelled {
                "cancelled"
            } else {
                "failed"
            };
            let stage = if cancelled {
                "cancelled"
            } else {
                "execution-or-candidate-validation"
            };
            let code = if definite_provider_rejection {
                "PROMPTGEN_PROVIDER_REJECTED"
            } else if cancelled {
                "PROMPTGEN_CANCELLED"
            } else if error.message.contains("output limit") {
                "PROMPTGEN_OUTPUT_LIMIT"
            } else if error.message.contains("timeout") {
                "PROMPTGEN_TIMEOUT"
            } else if error.message.starts_with("PromptGen exited") {
                "PROMPTGEN_FAILED"
            } else {
                "PROMPTGEN_INVALID_EXECUTION"
            };
            let message: String = error.message.chars().take(4000).collect();
            (
                capability::terminal_result(
                    &request,
                    &request_hash,
                    status,
                    Some(json!({"code":code,"message":message,"retryable":false,"stage":stage})),
                ),
                stage,
            )
        }
    };
    let status = result["status"].as_str().unwrap();
    let terminal = capability::transition_job(
        &job,
        status,
        stage,
        if status == "succeeded" {
            result["resultId"].as_str()
        } else {
            None
        },
        result.get("failure"),
    )?;
    registry.validate(RESULT_SCHEMA, &result)?;
    registry.validate(JOB_SCHEMA, &terminal)?;
    // All publication uses the held claim directory, including uncertain terminal evidence.
    output.atomic_write_json("capability-result.json",&result).and_then(|_|output.atomic_write_json("capability-job.json",&terminal))
        .map_err(|e|Error::unknown(format!("terminal persistence incomplete; inspect {} and {}; do not automatically retry: {e}",result_path.display(),job_path.display())))?;
    output.immutable("provider.stdout.log",&stdout).and_then(|_|output.immutable("provider.stderr.log",&stderr))
        .map_err(|e|Error::unknown(format!("diagnostic persistence incomplete; inspect {} and {}; do not automatically retry: {e}",result_path.display(),job_path.display())))?;
    Ok(
        json!({"job":terminal,"result":result,"command":command,"stdout":String::from_utf8_lossy(&stdout),"stderr":String::from_utf8_lossy(&stderr)}),
    )
}
