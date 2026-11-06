//! Ordered OpenAI edit orchestration over the adapter's focused I/O owners.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use promptgen_core::compile_image_prompt;
use promptgen_core::diagnostic::CompilationOutcome;
use promptgen_core::image::{ImagePromptRequest, ImageReferenceRole, ImageTaskMode};
use promptgen_core::json::JsonValue;

use super::files::{
    TempWorkspace, lexical_absolute, preflight_output, read_bounded_utf8, write_private_file,
};
use super::limits::MAX_RESPONSE_BYTES;
use super::reference::{copy_reference, reject_reference_output_alias};
use super::request::{validate_compilation, validate_request};
use super::response::{decode_response, publish_image, read_request_id};
use super::support::io_error;
use super::transport::{CurlRequest, build_curl_config, read_api_key, run_curl};
use super::{OpenAiEditReceipt, OpenAiExecutionError, OpenAiRunConfig};
use promptgen_core::image_artifact::sha256::digest_hex;

/// Executes the deliberately narrow, one-base GPT Image 2 edit contract.
///
/// The reference path is copied into a private temporary directory before the
/// request. Curl receives its complete request configuration through stdin;
/// neither the API key nor compiled prompt is placed in argv.
pub fn execute_image_edit(
    compilation: &CompilationOutcome,
    request: &ImagePromptRequest,
    reference_path: &Path,
    config: &OpenAiRunConfig,
) -> Result<OpenAiEditReceipt, OpenAiExecutionError> {
    validate_request(request, compilation.prompt.as_deref().unwrap_or_default())?;
    execute(compilation, request, Some(reference_path), config)
}

/// Reference-free GPT Image 2.6 generation through the Image API, including native GIF.
pub fn execute_image_generate(
    compilation: &CompilationOutcome,
    request: &ImagePromptRequest,
    config: &OpenAiRunConfig,
) -> Result<OpenAiEditReceipt, OpenAiExecutionError> {
    if request.task_mode != ImageTaskMode::Generate
        || !request.references.is_empty()
        || !matches!(
            request.output.model.as_str(),
            "gpt-image-2.6-sunburst" | "gpt-image-2.6-flare"
        )
    {
        return Err(OpenAiExecutionError::new(
            "OPENAI_GENERATE_CONTRACT",
            "OpenAI generation requires reference-free generate with GPT Image 2.6",
        ));
    }
    execute(compilation, request, None, config)
}

fn execute(
    compilation: &CompilationOutcome,
    request: &ImagePromptRequest,
    reference_path: Option<&Path>,
    config: &OpenAiRunConfig,
) -> Result<OpenAiEditReceipt, OpenAiExecutionError> {
    validate_compilation(compilation)?;
    let rebound = compile_image_prompt(request);
    if !rebound.is_valid() || rebound.prompt != compilation.prompt {
        return Err(OpenAiExecutionError::new(
            "OPENAI_COMPILATION_MISMATCH",
            "compiled prompt does not match the supplied typed edit request",
        ));
    }
    if let Some(path) = reference_path {
        reject_reference_output_alias(path, &config.output_path)?;
    }
    preflight_output(&config.output_path, config.overwrite)?;
    let api_key = read_api_key()?;
    let started_at = SystemTime::now();
    let started_unix_ms = unix_millis(started_at);
    let compiled_prompt = compilation.prompt.as_deref().ok_or_else(|| {
        OpenAiExecutionError::new("OPENAI_MISSING_PROMPT", "valid compilation has no prompt")
    })?;
    let compiled_prompt_sha256 = digest_hex(compiled_prompt.as_bytes());
    let compiled_prompt_chars = compiled_prompt.chars().count() as u64;

    let workspace = TempWorkspace::create()?;
    let reference = reference_path
        .map(|path| copy_reference(path, &workspace.path))
        .transpose()?;
    let prompt_path = workspace.path.join("compiled-prompt.txt");
    let body = if reference.is_none() {
        let mut fields = vec![
            ("model", JsonValue::from(request.output.model.clone())),
            ("prompt", JsonValue::from(compiled_prompt)),
            (
                "size",
                JsonValue::from(format!(
                    "{}x{}",
                    request.output.width, request.output.height
                )),
            ),
            ("quality", JsonValue::from(request.output.quality.as_str())),
            (
                "background",
                JsonValue::from(request.output.background.as_str()),
            ),
            (
                "output_format",
                JsonValue::from(request.output.format.clone()),
            ),
            ("n", JsonValue::from(1_u64)),
        ];
        for (key, value) in [
            ("frame_count", request.output.frame_count),
            ("frame_duration_ms", request.output.frame_duration_ms),
        ] {
            if let Some(value) = value {
                fields.push((key, JsonValue::from(u64::from(value))));
            }
        }
        JsonValue::object(fields).to_compact_string()
    } else {
        compiled_prompt.to_owned()
    };
    write_private_file(&prompt_path, body.as_bytes())?;
    let response_path = workspace.path.join("response.json");
    let headers_path = workspace.path.join("response.headers");
    let stdout_path = workspace.path.join("curl.stdout");
    let stderr_path = workspace.path.join("curl.stderr");
    let curl_request = CurlRequest {
        api_key: &api_key,
        prompt_path: &prompt_path,
        reference_path: reference.as_ref().map(|reference| reference.path.as_path()),
        mime: reference
            .as_ref()
            .map_or(super::reference::ImageMime::Png, |reference| reference.mime),
        response_path: &response_path,
        headers_path: &headers_path,
        request,
        timeout: config.timeout,
    };
    let curl_config = build_curl_config(&curl_request);
    let http_status = run_curl(
        config,
        &workspace.path,
        &curl_config,
        &stdout_path,
        &stderr_path,
    )?;
    if http_status != 200 {
        return Err(OpenAiExecutionError::new(
            "OPENAI_HTTP_STATUS",
            format!("image endpoint returned HTTP status {http_status}"),
        ));
    }
    let response = read_bounded_utf8(&response_path, MAX_RESPONSE_BYTES, "response body")?;
    let request_id = read_request_id(&headers_path)?;
    let image = decode_response(&response, &request.output)?;
    let (output_sha256, info) = publish_image(
        &image,
        &config.output_path,
        config.overwrite,
        &request.output,
    )?;
    let finished_unix_ms = unix_millis(SystemTime::now());
    Ok(OpenAiEditReceipt {
        backend: "openai-images".to_owned(),
        provider: "openai".to_owned(),
        model: request.output.model.clone(),
        format: request.output.format.clone(),
        frame_count: request.output.frame_count,
        frame_duration_ms: request.output.frame_duration_ms,
        endpoint_contract: if reference.is_some() {
            "POST https://api.openai.com/v1/images/edits"
        } else {
            "POST https://api.openai.com/v1/images/generations"
        }
        .to_owned(),
        request_id,
        http_status,
        compiled_prompt_sha256,
        compiled_prompt_chars,
        reference_index: if reference.is_some() { 1 } else { 0 },
        reference_role: if reference.is_some() {
            ImageReferenceRole::Base.as_str()
        } else {
            "none"
        }
        .to_owned(),
        reference_sha256: reference
            .as_ref()
            .map_or_else(String::new, |r| r.sha256.clone()),
        reference_bytes: reference.as_ref().map_or(0, |r| r.bytes),
        output_sha256,
        output_path: lexical_absolute(&config.output_path)
            .map_err(|error| io_error("resolve output path", &config.output_path, error))?,
        width: info.width,
        height: info.height,
        started_unix_ms,
        finished_unix_ms,
    })
}

fn unix_millis(value: SystemTime) -> u64 {
    value
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
