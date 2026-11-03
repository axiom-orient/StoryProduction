use std::fs;

use promptgen_core::diagnostic::{CompilationOutcome, CompilationStatus, PromptKind};
use promptgen_core::image::{ImagePromptRequest, ImageTaskMode};

use super::artifact::publication::{parent_or_current, validate_existing_output};
use super::{CodexExecutionError, CodexRunConfig, io_error};

pub(super) fn validate_compilation(
    compilation: &CompilationOutcome,
) -> Result<(), CodexExecutionError> {
    if compilation.kind != PromptKind::Image {
        return Err(CodexExecutionError::new(
            "CODEX_WRONG_PROMPT_KIND",
            format!("expected image, found {}", compilation.kind.as_str()),
        ));
    }
    if compilation.status == CompilationStatus::Invalid {
        return Err(CodexExecutionError::new(
            "CODEX_INVALID_COMPILATION",
            "image compilation is invalid",
        ));
    }
    Ok(())
}

pub(super) fn validate_backend_contract(
    request: &ImagePromptRequest,
) -> Result<(), CodexExecutionError> {
    if request.output.model != "gpt-image-2" || request.output.format != "png" {
        return Err(CodexExecutionError::new(
            "CODEX_BACKEND_CONTRACT",
            "codex-imagegen supports the gpt-image-2 PNG profile only",
        ));
    }
    if request.task_mode != ImageTaskMode::Generate || !request.references.is_empty() {
        return Err(CodexExecutionError::new(
            "CODEX_REFERENCE_INPUTS_REQUIRED",
            "the prompt compiler supports reference-guided generation, edit, and composite contracts, but this Codex adapter currently has no verified image-input transport; use a backend that binds the declared images by index",
        ));
    }
    if request
        .subjects
        .iter()
        .any(|subject| super::fidelity::expected_instances(request, subject.count).is_none())
    {
        return Err(CodexExecutionError::new(
            "CODEX_SUBJECT_COUNT_RANGE",
            "total visible subject count across storyboard panels exceeds the receipt count range",
        ));
    }
    Ok(())
}

pub(super) fn ensure_runtime_paths(config: &CodexRunConfig) -> Result<(), CodexExecutionError> {
    if !(1..=4).contains(&config.max_fidelity_attempts) {
        return Err(CodexExecutionError::new(
            "CODEX_FIDELITY_ATTEMPTS",
            "max_fidelity_attempts must be between 1 and 4",
        ));
    }
    fs::create_dir_all(&config.codex_home)
        .map_err(|error| io_error("create CODEX_HOME", &config.codex_home, error))?;
    let generated_root = config.generated_root();
    fs::create_dir_all(&generated_root)
        .map_err(|error| io_error("create generated image root", &generated_root, error))?;
    let output_parent = parent_or_current(&config.output_path);
    fs::create_dir_all(output_parent)
        .map_err(|error| io_error("create output directory", output_parent, error))?;

    let root = fs::canonicalize(&generated_root)
        .map_err(|error| io_error("canonicalize generated root", &generated_root, error))?;
    let output_parent_canonical = fs::canonicalize(output_parent)
        .map_err(|error| io_error("canonicalize output parent", output_parent, error))?;
    let output_name = config.output_path.file_name().ok_or_else(|| {
        CodexExecutionError::new("CODEX_OUTPUT_PATH", "output path has no file name")
    })?;
    let output = output_parent_canonical.join(output_name);
    if output.starts_with(&root) {
        return Err(CodexExecutionError::new(
            "CODEX_OUTPUT_AUTHORITY",
            format!(
                "output must be outside generated root: output={}, root={}",
                output.display(),
                root.display()
            ),
        ));
    }
    validate_existing_output(&output, config.overwrite)
}
