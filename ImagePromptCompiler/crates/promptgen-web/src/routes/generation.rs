use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};

use promptgen_codex::{CodexRunConfig, execute_image_generation};
use promptgen_core::interview::InterviewRequestValue;
use promptgen_core::json::JsonValue;
use promptgen_core::{InterviewStatus, run_interview};

use super::super::default_output_name;
use super::super::http::{HttpRequest, Response};
use super::super::request::{GenerateRequest, GenerationMode, decode_interview};
use super::super::security::validate_output_name;
use super::super::server::ServerState;

const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;

pub(super) fn handle_interview(request: &HttpRequest, state: &ServerState) -> Response {
    match decode_interview(&request.body, state.config.max_body_bytes) {
        Ok(interview) => {
            let outcome = run_interview(&interview);
            Response::json(
                200,
                JsonValue::object([
                    (
                        "ok",
                        JsonValue::from(outcome.status != InterviewStatus::Invalid),
                    ),
                    ("interview", outcome.to_json()),
                ]),
            )
        }
        Err(error) => Response::json_error(400, "INTERVIEW_DECODE", error.to_string()),
    }
}

pub(super) fn handle_generate(request: &HttpRequest, state: &ServerState) -> Response {
    let generation = match GenerateRequest::decode(&request.body, state.config.max_body_bytes) {
        Ok(value) => value,
        Err(error) => return Response::json_error(400, "GENERATE_DECODE", error.to_string()),
    };
    let outcome = run_interview(&generation.interview);
    if outcome.status == InterviewStatus::NeedsInput {
        return Response::json(
            200,
            JsonValue::object([
                ("ok", JsonValue::from(false)),
                ("interview", outcome.to_json()),
                ("reason", JsonValue::from("needs_input")),
            ]),
        );
    }
    if outcome.status == InterviewStatus::Invalid {
        return Response::json(
            422,
            JsonValue::object([
                ("ok", JsonValue::from(false)),
                ("interview", outcome.to_json()),
                ("reason", JsonValue::from("invalid_compilation")),
            ]),
        );
    }
    if generation.mode == GenerationMode::PromptOnly {
        return Response::json(
            200,
            JsonValue::object([
                ("ok", JsonValue::from(true)),
                ("interview", outcome.to_json()),
                ("mode", JsonValue::from(generation.mode.as_str())),
            ]),
        );
    }
    // The interview already decoded and compiled this request; both are taken from
    // it together, so "ready with nothing to run" is not a state this route can see.
    let Some(ready) = outcome.ready() else {
        return Response::json_error(
            500,
            "GENERATE_NOT_READY",
            "interview is not in a ready state",
        );
    };
    let InterviewRequestValue::Image(image_request) = ready.request;
    let compilation = ready.compilation;
    let output_name = generation.output_name.unwrap_or_else(default_output_name);
    if let Err(message) = validate_output_name(&output_name) {
        return Response::json_error(400, "GENERATE_OUTPUT_NAME", message);
    }
    let _guard = match GenerationGuard::acquire(&state.generation_busy) {
        Some(guard) => guard,
        None => {
            return Response::json_error(
                409,
                "GENERATE_BUSY",
                "another image generation is already running",
            );
        }
    };
    let output_path = state.config.output_dir.join(&output_name);
    let config = CodexRunConfig {
        codex_binary: state.config.codex_binary.clone(),
        codex_home: state.config.codex_home.clone(),
        output_path,
        timeout: state.config.request_timeout,
        artifact_wait_timeout: state.config.artifact_wait_timeout,
        max_fidelity_attempts: 4,
        overwrite: false,
    };
    match execute_image_generation(compilation, image_request, &config) {
        Ok(receipt) => Response::json(
            200,
            JsonValue::object([
                (
                    "artifact_url",
                    JsonValue::from(format!("/artifacts/{output_name}")),
                ),
                ("interview", outcome.to_json()),
                ("mode", JsonValue::from(generation.mode.as_str())),
                ("ok", JsonValue::from(true)),
                ("receipt", receipt.to_json()),
            ]),
        ),
        Err(error) => Response::json(
            502,
            JsonValue::object([
                (
                    "error",
                    JsonValue::object([
                        ("code", JsonValue::from(error.code)),
                        ("message", JsonValue::from(error.message)),
                    ]),
                ),
                ("interview", outcome.to_json()),
                ("ok", JsonValue::from(false)),
            ]),
        ),
    }
}

pub(crate) fn serve_artifact(path: &str, state: &ServerState) -> Response {
    let name = path.trim_start_matches("/artifacts/");
    if validate_output_name(name).is_err() {
        return Response::json_error(404, "ARTIFACT_NOT_FOUND", "artifact not found");
    }
    let path = state.config.output_dir.join(name);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(value) if value.file_type().is_file() && !value.file_type().is_symlink() => value,
        _ => return Response::json_error(404, "ARTIFACT_NOT_FOUND", "artifact not found"),
    };
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Response::json_error(413, "ARTIFACT_TOO_LARGE", "artifact exceeds 64 MiB");
    }
    match fs::read(&path) {
        Ok(bytes) => Response::bytes(200, "image/png", bytes),
        Err(error) => Response::json_error(500, "ARTIFACT_READ", error.to_string()),
    }
}

pub(crate) struct GenerationGuard<'a> {
    busy: &'a AtomicBool,
}

impl<'a> GenerationGuard<'a> {
    pub(crate) fn acquire(busy: &'a AtomicBool) -> Option<Self> {
        busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self { busy })
    }
}

impl Drop for GenerationGuard<'_> {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Release);
    }
}
