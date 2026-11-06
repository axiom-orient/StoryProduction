//! One native animated-image effect through the existing ChatGPT-authenticated Codex CLI.
//! This lane publishes a mechanically checked candidate; it does not approve its motion.
use crate::{
    CodexExecutionError, CodexRunConfig,
    artifact::{
        discovery::discover_gif_artifact,
        publication::{lexical_absolute, parent_or_current, publish_file},
    },
    event_stream,
    preflight::{ensure_runtime_paths, validate_compilation},
    process::{
        OwnedChild, WorkingDirectory, create_new_file, exit_code, isolated_codex_exec,
        probe_codex_version, read_log_snippet,
    },
};
use promptgen_core::{
    compile_image_prompt,
    diagnostic::CompilationOutcome,
    image::{ImagePromptRequest, ImageReferenceRole, ImageTaskMode},
    image_artifact::{gif, png, sha256::digest_hex},
    json::JsonValue,
};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    process::Stdio,
    time::{Duration, SystemTime},
};

fn fail(code: &'static str, message: impl Into<String>) -> CodexExecutionError {
    CodexExecutionError::new(code, message)
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>, CodexExecutionError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|e| crate::io_error("inspect GIF input", path, e))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > limit as u64 {
        return Err(fail("CODEX_GIF_INPUT", "bounded regular file required"));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| crate::io_error("open GIF input", path, e))?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| crate::io_error("read GIF input", path, e))?;
    if bytes.len() > limit {
        return Err(fail("CODEX_GIF_INPUT", "input byte budget exceeded"));
    }
    Ok(bytes)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), CodexExecutionError> {
    let mut file = create_new_file(path, "reserve GIF evidence")?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| crate::io_error("write GIF evidence", path, e))
}
fn require_chatgpt_login(config: &CodexRunConfig) -> Result<(), CodexExecutionError> {
    let scratch = WorkingDirectory::create()?;
    let stdout = scratch.path.join("auth.stdout");
    let stderr = scratch.path.join("auth.stderr");
    let mut command = std::process::Command::new(&config.codex_binary);
    command
        .args(["login", "status"])
        .env("CODEX_HOME", &config.codex_home)
        .env_remove("OPENAI_API_KEY")
        .env_remove("CODEX_API_KEY")
        .current_dir(&scratch.path)
        .stdin(Stdio::null())
        .stdout(Stdio::from(create_new_file(
            &stdout,
            "capture account kind",
        )?))
        .stderr(Stdio::from(create_new_file(
            &stderr,
            "capture account kind",
        )?));
    let mut child = OwnedChild::spawn(&mut command, "CODEX_AUTH_PROBE", &config.codex_binary)?;
    let status = child.wait(Duration::from_secs(15))?;
    let observed = format!(
        "{}\n{}",
        read_log_snippet(&stdout),
        read_log_snippet(&stderr)
    );
    if !status.success()
        || !observed
            .lines()
            .any(|line| line.trim() == "Logged in using ChatGPT")
    {
        return Err(fail(
            "CODEX_CHATGPT_LOGIN_REQUIRED",
            "selected CODEX_HOME must be signed in with ChatGPT; no image request was sent",
        ));
    }
    Ok(())
}

pub fn execute_image_gif(
    compilation: &CompilationOutcome,
    request: &ImagePromptRequest,
    reference: Option<&Path>,
    config: &CodexRunConfig,
) -> Result<JsonValue, CodexExecutionError> {
    validate_compilation(compilation)?;
    if !matches!(
        request.output.model.as_str(),
        "gpt-image-2.6-sunburst" | "gpt-image-2.6-flare"
    ) || request.output.format != "gif"
    {
        return Err(fail(
            "CODEX_GIF_PROFILE",
            "native GIF requires the GPT Image 2.6 GIF profile",
        ));
    }
    let rebound = compile_image_prompt(request);
    if !rebound.is_valid() || rebound.prompt != compilation.prompt {
        return Err(fail(
            "CODEX_COMPILATION_MISMATCH",
            "typed GIF request and prompt differ",
        ));
    }
    let edit = request.task_mode == ImageTaskMode::Edit
        && request.references.len() == 1
        && request.references[0].index == 1
        && request.references[0].role == ImageReferenceRole::Base
        && reference.is_some();
    let generate = request.task_mode == ImageTaskMode::Generate
        && request.references.is_empty()
        && reference.is_none();
    if !edit && !generate {
        return Err(fail(
            "CODEX_REFERENCE_INPUTS_REQUIRED",
            "GIF accepts reference-free generate or one bound base-image edit",
        ));
    }
    let reference_bytes = reference
        .map(|path| -> Result<Vec<u8>, CodexExecutionError> {
            if lexical_absolute(path).ok() == lexical_absolute(&config.output_path).ok()
                || path.canonicalize().ok()
                    == config
                        .output_path
                        .canonicalize()
                        .ok()
                        .filter(|_| config.output_path.exists())
            {
                return Err(fail(
                    "CODEX_REFERENCE_OUTPUT_ALIAS",
                    "reference cannot be the GIF output",
                ));
            }
            let bytes = read(path, 50 * 1024 * 1024)?;
            png::validate_bytes(
                &bytes,
                (
                    u32::from_be_bytes(
                        bytes
                            .get(16..20)
                            .ok_or_else(|| fail("CODEX_REFERENCE_PNG", "PNG reference required"))?
                            .try_into()
                            .unwrap(),
                    ),
                    u32::from_be_bytes(
                        bytes
                            .get(20..24)
                            .ok_or_else(|| fail("CODEX_REFERENCE_PNG", "PNG reference required"))?
                            .try_into()
                            .unwrap(),
                    ),
                ),
            )
            .map_err(|e| fail("CODEX_REFERENCE_PNG", e.to_string()))?;
            Ok(bytes)
        })
        .transpose()?;
    ensure_runtime_paths(config)?;
    let evidence = parent_or_current(&config.output_path).join("codex-gif-evidence");
    fs::create_dir(&evidence)
        .map_err(|e| crate::io_error("reserve immutable GIF attempt evidence", &evidence, e))?;
    let evidence = evidence
        .canonicalize()
        .map_err(|e| crate::io_error("resolve GIF evidence directory", &evidence, e))?;
    let version = probe_codex_version(&config.codex_binary, &evidence, Duration::from_secs(15))?;
    require_chatgpt_login(config)?;
    let prompt = compilation
        .prompt
        .as_deref()
        .ok_or_else(|| fail("CODEX_MISSING_PROMPT", "compiled prompt missing"))?;
    let frames = request
        .output
        .frame_count
        .ok_or_else(|| fail("CODEX_GIF_TIMING", "frame_count required"))?;
    let duration = request
        .output
        .frame_duration_ms
        .ok_or_else(|| fail("CODEX_GIF_TIMING", "frame_duration_ms required"))?;
    let reference_snapshot = evidence.join("reference.png");
    if let Some(bytes) = &reference_bytes {
        write_new(&reference_snapshot, bytes)?;
    }
    let instruction = format!(
        "Use the built-in image_generation tool exactly ONCE. Its gif argument MUST be {{\"frame_count\":{frames},\"frame_duration_ms\":{duration}}}. The image_generation_animation feature is enabled. Return an actual native animated GIF; never substitute PNG, a contact sheet, code-generated animation or a local API client. The attached image, when present, is reference index 1 and the exact base image for this edit. Follow the compiled visual contract below. Preserve its start/end action order; never reverse a one-way event. Do not run shell commands, edit files, browse or call other tools. After the image tool returns, end the turn. If GIF is unavailable, stop and state that fact.\n\nIMAGE_REQUEST_JSON\n{}",
        JsonValue::object([
            ("compiled_prompt", JsonValue::from(prompt)),
            ("generation_parameters", request.output.to_json())
        ])
        .to_compact_string()
    );
    write_new(&evidence.join("instruction.txt"), instruction.as_bytes())?;
    let started = SystemTime::now();
    write_new(
        &evidence.join("dispatch.json"),
        JsonValue::object([
            ("status", JsonValue::from("running")),
            (
                "auth",
                JsonValue::from("existing ChatGPT login; API key environment removed"),
            ),
            ("codex_version", JsonValue::from(version.clone())),
            ("started_unix_ms", JsonValue::from(crate::millis(started))),
            (
                "compiled_prompt_sha256",
                JsonValue::from(digest_hex(prompt.as_bytes())),
            ),
        ])
        .to_pretty_string()
        .as_bytes(),
    )?;
    let stdout = evidence.join("events.jsonl");
    let stderr = evidence.join("stderr.log");
    let mut command = isolated_codex_exec(&config.codex_binary);
    command.args([
        "--enable",
        "image_generation",
        "--enable",
        "image_generation_animation",
        "-c",
        "suppress_unstable_features_warning=true",
    ]);
    if reference.is_some() {
        command.arg("--image").arg(&reference_snapshot);
    }
    command
        .arg("--json")
        .arg("--")
        .arg(&instruction)
        .env("CODEX_HOME", &config.codex_home)
        .env_remove("OPENAI_API_KEY")
        .env_remove("CODEX_API_KEY")
        .current_dir(&evidence)
        .stdin(Stdio::null())
        .stdout(Stdio::from(create_new_file(
            &stdout,
            "create GIF event log",
        )?))
        .stderr(Stdio::from(create_new_file(
            &stderr,
            "create GIF error log",
        )?));
    let mut child = OwnedChild::spawn(&mut command, "CODEX_SPAWN", &config.codex_binary)?;
    let status = child.wait(config.timeout)?;
    write_new(
        &evidence.join("process.json"),
        JsonValue::object([
            (
                "process_exit_code",
                JsonValue::from(i64::from(exit_code(status))),
            ),
            (
                "finished_unix_ms",
                JsonValue::from(crate::millis(SystemTime::now())),
            ),
        ])
        .to_pretty_string()
        .as_bytes(),
    )?;
    if !status.success() {
        return Err(fail(
            "CODEX_PROCESS_FAILED",
            format!(
                "GIF worker exited {}; evidence={}; stderr={}",
                exit_code(status),
                evidence.display(),
                read_log_snippet(&stderr)
            ),
        ));
    }
    let turn = event_stream::parse_and_validate(
        &stdout,
        Some(&config.codex_home.join("skills/.system/imagegen/SKILL.md")),
    )?;
    let diagnostic = read(&stderr, 1024 * 1024)?;
    let diagnostic = String::from_utf8_lossy(&diagnostic);
    if diagnostic.lines().filter(|line| line.contains("ERROR codex_core::tools::router: error=image generation failed: http 400 Bad Request:") && line.contains("GIF output is not enabled for this request")).count() == 1 {
        write_new(&evidence.join("provider-rejection.json"), JsonValue::object([
            ("status",JsonValue::from("failed")),("http_status",JsonValue::from(400_u64)),
            ("reason",JsonValue::from("GIF output is not enabled for this request")),
            ("thread_id",JsonValue::from(turn.thread_id)),("stderr_sha256",JsonValue::from(digest_hex(diagnostic.as_bytes()))),
        ]).to_pretty_string().as_bytes())?;
        return Err(fail("CODEX_GIF_SERVICE_REJECTED",format!("HTTP 400: GIF output is not enabled for this request; evidence={}",evidence.display())));
    }
    let (call, source) = discover_gif_artifact(
        &config.generated_root(),
        &turn.thread_id,
        started,
        config.artifact_wait_timeout,
    )?;
    let bytes = read(&source, 50 * 1024 * 1024)?;
    let info = gif::inspect(&bytes).map_err(|e| fail("CODEX_INVALID_GIF", e.to_string()))?;
    if info.delays_ms.len() != frames as usize || info.delays_ms.iter().any(|ms| *ms != duration) {
        return Err(fail(
            "CODEX_GIF_TIMING",
            "actual GIF frame count/delays differ from request",
        ));
    }
    let requested = (request.output.width, request.output.height);
    if request.output.size_policy.as_deref() != Some("provider")
        && (info.width, info.height) != requested
    {
        return Err(fail(
            "CODEX_GIF_DIMENSIONS",
            format!(
                "expected {}x{}, observed {}x{}; evidence={}",
                requested.0,
                requested.1,
                info.width,
                info.height,
                evidence.display()
            ),
        ));
    }
    let hash = digest_hex(&bytes);
    let receipt = JsonValue::object([
        ("backend", JsonValue::from("codex-imagegen")),
        ("auth_mode", JsonValue::from("chatgpt_subscription")),
        ("codex_version", JsonValue::from(version)),
        ("event_contract", JsonValue::from(turn.event_contract)),
        ("thread_id", JsonValue::from(turn.thread_id)),
        ("image_call_id", JsonValue::from(call)),
        (
            "source_artifact",
            JsonValue::from(source.display().to_string()),
        ),
        (
            "output_artifact",
            JsonValue::from(
                lexical_absolute(&config.output_path)
                    .map_err(|e| crate::io_error("resolve GIF output", &config.output_path, e))?
                    .display()
                    .to_string(),
            ),
        ),
        ("sha256", JsonValue::from(hash)),
        ("width", JsonValue::from(u64::from(info.width))),
        ("height", JsonValue::from(u64::from(info.height))),
        ("requested_width", JsonValue::from(u64::from(requested.0))),
        ("requested_height", JsonValue::from(u64::from(requested.1))),
        ("format", JsonValue::from("gif")),
        ("frame_count", JsonValue::from(u64::from(frames))),
        ("frame_duration_ms", JsonValue::from(u64::from(duration))),
        (
            "size_policy",
            JsonValue::from(request.output.size_policy.as_deref().unwrap_or("exact")),
        ),
        (
            "requested_model",
            JsonValue::from(request.output.model.clone()),
        ),
        ("provider_model", JsonValue::from("UNKNOWN")),
        ("model_identity_verified", JsonValue::from(false)),
        ("semantic_review_required", JsonValue::from(true)),
        (
            "reference_sha256",
            reference_bytes
                .as_ref()
                .map_or(JsonValue::Null, |b| JsonValue::from(digest_hex(b))),
        ),
        (
            "reference_index",
            JsonValue::from(if edit { 1_u64 } else { 0_u64 }),
        ),
        ("process_exit_code", JsonValue::from(0_i64)),
        (
            "compiled_prompt_sha256",
            JsonValue::from(digest_hex(prompt.as_bytes())),
        ),
        (
            "compiled_prompt_chars",
            JsonValue::from(prompt.chars().count() as u64),
        ),
        ("started_unix_ms", JsonValue::from(crate::millis(started))),
        (
            "finished_unix_ms",
            JsonValue::from(crate::millis(SystemTime::now())),
        ),
        (
            "evidence_directory",
            JsonValue::from(evidence.display().to_string()),
        ),
    ]);
    write_new(
        &evidence.join("generated-candidate.json"),
        receipt.to_pretty_string().as_bytes(),
    )?;
    let temporary = evidence.join("validated.gif");
    write_new(&temporary, &bytes)?;
    let written = read(&temporary, 50 * 1024 * 1024)?;
    if written != bytes || read(&source, 50 * 1024 * 1024)? != bytes {
        return Err(fail(
            "CODEX_GIF_CHANGED",
            "GIF bytes changed before publication",
        ));
    }
    publish_file(&temporary, &config.output_path, config.overwrite)?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::Ordering;
    fn fixture(
        reject: bool,
        exact: bool,
    ) -> (std::path::PathBuf, ImagePromptRequest, CodexRunConfig) {
        let root = std::env::temp_dir().join(format!(
            "promptgen-subscription-gif-{}-{}",
            std::process::id(),
            crate::NEXT_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let mut request = ImagePromptRequest::from_json(
            promptgen_core::json::parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-animated-coffee.json"
            )))
            .unwrap(),
        )
        .unwrap();
        if !exact {
            request.output.size_policy = Some("provider".into());
        }
        let binary = root.join("provider-double");
        let home = root.join("home");
        let target = home.join("generated_images/fixture-thread/fixture-call.gif");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        let source = root.join("codec.gif");
        fs::write(
            &source,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/fixtures/codec-test-nineframes.gif"
            )),
        )
        .unwrap();
        // Scripted process seam plus real GIF codec bytes; never production/provider proof.
        let effect = if reject {
            "printf '%s\\n' 'ERROR codex_core::tools::router: error=image generation failed: http 400 Bad Request: GIF output is not enabled for this request' >&2".to_owned()
        } else {
            format!("/bin/cp '{}' '{}'", source.display(), target.display())
        };
        let body = format!(
            "#!/bin/sh\nset -eu\nif [ \"${{1-}}\" = --version ]; then printf 'codex-cli 0.168.0-alpha.7\\n'; exit 0; fi\nif [ \"${{1-}}\" = login ]; then printf 'Logged in using ChatGPT\\n'; exit 0; fi\nif [ \"${{OPENAI_API_KEY+x}}\" = x ] || [ \"${{CODEX_API_KEY+x}}\" = x ]; then exit 9; fi\nprintf '%s\\n' \"$@\" > '{}'/argv.txt\n{effect}\nprintf '%s\\n' '{{\"type\":\"thread.started\",\"thread_id\":\"fixture-thread\"}}' '{{\"type\":\"turn.started\"}}' '{{\"type\":\"turn.completed\"}}'\n",
            root.display()
        );
        fs::write(&binary, body).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        let mut config = CodexRunConfig::new(binary, home, root.join("out/image.gif"));
        config.timeout = Duration::from_secs(5);
        config.artifact_wait_timeout = Duration::from_secs(1);
        (root, request, config)
    }
    #[test]
    fn subscription_gif_binds_native_bytes_flags_and_explicit_provider_size() {
        let (root, request, config) = fixture(false, false);
        let receipt =
            execute_image_gif(&compile_image_prompt(&request), &request, None, &config).unwrap();
        let info = gif::inspect(&fs::read(&config.output_path).unwrap()).unwrap();
        assert_eq!((info.width, info.height), (32, 32));
        assert_eq!(info.delays_ms, vec![200; 9]);
        let object = receipt.as_object().unwrap();
        assert_eq!(object["auth_mode"].as_str(), Some("chatgpt_subscription"));
        assert_eq!(object["provider_model"].as_str(), Some("UNKNOWN"));
        let argv = fs::read_to_string(root.join("argv.txt")).unwrap();
        assert!(argv.contains("image_generation_animation"));
        assert!(argv.contains("gif argument MUST"));
        assert!(
            root.join("out/codex-gif-evidence/generated-candidate.json")
                .is_file()
        );
        assert!(
            execute_image_gif(&compile_image_prompt(&request), &request, None, &config).is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn exact_size_and_service_rejection_never_publish_a_gif() {
        for reject in [false, true] {
            let (root, request, config) = fixture(reject, true);
            let error = execute_image_gif(&compile_image_prompt(&request), &request, None, &config)
                .unwrap_err();
            assert_eq!(
                error.code,
                if reject {
                    "CODEX_GIF_SERVICE_REJECTED"
                } else {
                    "CODEX_GIF_DIMENSIONS"
                }
            );
            assert!(!config.output_path.exists());
            assert_eq!(
                root.join("out/codex-gif-evidence/provider-rejection.json")
                    .exists(),
                reject
            );
            fs::remove_dir_all(root).unwrap();
        }
    }
    #[test]
    fn api_key_login_cannot_dispatch_a_subscription_image_request() {
        let (root, request, config) = fixture(false, false);
        let script = fs::read_to_string(&config.codex_binary)
            .unwrap()
            .replace("Logged in using ChatGPT", "Logged in using an API key");
        fs::write(&config.codex_binary, script).unwrap();
        let error = execute_image_gif(&compile_image_prompt(&request), &request, None, &config)
            .unwrap_err();
        assert_eq!(error.code, "CODEX_CHATGPT_LOGIN_REQUIRED");
        assert!(!root.join("argv.txt").exists());
        assert!(!root.join("out/codex-gif-evidence/dispatch.json").exists());
        assert!(!config.output_path.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
