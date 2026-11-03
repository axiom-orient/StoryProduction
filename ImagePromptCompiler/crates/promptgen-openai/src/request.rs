use promptgen_core::diagnostic::{CompilationOutcome, PromptKind};
use promptgen_core::image::{ImagePromptRequest, ImageReferenceRole, ImageTaskMode};

use super::{API_MODEL, OpenAiExecutionError};

const MAX_PROMPT_CHARS: usize = 32_000;

pub(super) fn validate_compilation(
    compilation: &CompilationOutcome,
) -> Result<(), OpenAiExecutionError> {
    if compilation.kind != PromptKind::Image || !compilation.is_valid() {
        return Err(OpenAiExecutionError::new(
            "OPENAI_INVALID_COMPILATION",
            "OpenAI image edit requires a valid image compilation",
        ));
    }
    Ok(())
}

pub(super) fn validate_request(
    request: &ImagePromptRequest,
    prompt: &str,
) -> Result<(), OpenAiExecutionError> {
    if request.task_mode != ImageTaskMode::Edit {
        return Err(OpenAiExecutionError::new(
            "OPENAI_MODE_UNSUPPORTED",
            "OpenAI adapter supports task_mode=edit only",
        ));
    }
    if request.references.len() != 1 {
        return Err(OpenAiExecutionError::new(
            "OPENAI_REFERENCE_COUNT",
            "OpenAI adapter requires exactly one declared reference",
        ));
    }
    let reference = &request.references[0];
    if reference.index != 1 || reference.role != ImageReferenceRole::Base {
        return Err(OpenAiExecutionError::new(
            "OPENAI_REFERENCE_CONTRACT",
            "OpenAI adapter requires reference index=1 with role=base",
        ));
    }
    if request.output.model != API_MODEL || request.output.format != "png" {
        return Err(OpenAiExecutionError::new(
            "OPENAI_OUTPUT_CONTRACT",
            "OpenAI adapter requires model=gpt-image-2 and format=png",
        ));
    }
    if prompt.trim().is_empty() {
        return Err(OpenAiExecutionError::new(
            "OPENAI_EMPTY_PROMPT",
            "compiled image prompt must not be blank",
        ));
    }
    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(OpenAiExecutionError::new(
            "OPENAI_PROMPT_LIMIT",
            format!("compiled prompt exceeds {MAX_PROMPT_CHARS} characters"),
        ));
    }
    Ok(())
}
