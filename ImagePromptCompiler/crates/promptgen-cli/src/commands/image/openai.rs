use std::path::{Path, PathBuf};
use std::time::Duration;

use promptgen_core::diagnostic::CompilationOutcome;
use promptgen_core::image::ImagePromptRequest;
use promptgen_core::json::JsonValue;
use promptgen_openai::{
    OpenAiEditReceipt, OpenAiRunConfig, execute_image_edit, openai_paths_alias,
};

use crate::error::{CliError, EXIT_OK};
use crate::options::{CompileOptions, ImageOptions, OutputFormat};
use crate::output::{preflight_output, write_output};

#[derive(Debug)]
pub(super) struct OpenAiEditPreflight {
    image_output: PathBuf,
    reference_image: PathBuf,
}

fn paths_alias(left: &Path, right: &Path) -> Result<bool, CliError> {
    openai_paths_alias(left, right)
        .map_err(|error| CliError::io("resolve OpenAI edit path alias", left, error))
}

pub(super) fn preflight_paths(options: &ImageOptions) -> Result<OpenAiEditPreflight, CliError> {
    let image_output = options
        .image_output
        .clone()
        .ok_or_else(|| CliError::usage("--mode openai-image-edit requires --image-output"))?;
    let reference_image = options
        .reference_image
        .clone()
        .ok_or_else(|| CliError::usage("--mode openai-image-edit requires --reference-image"))?;
    if paths_alias(&reference_image, &image_output)? {
        return Err(CliError::usage(
            "--reference-image and --image-output must be different paths",
        ));
    }
    if let Some(result) = options.compile.output.result.as_deref()
        && paths_alias(result, &reference_image)?
    {
        return Err(CliError::usage(
            "--result and --reference-image must be different paths",
        ));
    }
    if let Some(result) = options.compile.output.result.as_deref()
        && paths_alias(result, &image_output)?
    {
        return Err(CliError::usage(
            "--result and --image-output must be different paths",
        ));
    }
    Ok(OpenAiEditPreflight {
        image_output,
        reference_image,
    })
}

pub(super) fn run(
    compilation: &CompilationOutcome,
    request: &ImagePromptRequest,
    compile: CompileOptions,
    timeout: Duration,
    preflight: OpenAiEditPreflight,
) -> Result<u8, CliError> {
    if let Some(result) = compile.output.result.as_deref() {
        preflight_output(result, compile.output.force)?;
    }
    let mut config = OpenAiRunConfig::new("curl", preflight.image_output);
    config.timeout = timeout;
    config.overwrite = compile.output.force;
    let receipt = execute_image_edit(compilation, request, &preflight.reference_image, &config)
        .map_err(|error| CliError::execution(error.to_string()))?;
    let text = if compile.output.result.is_some() {
        match compile.format {
            OutputFormat::Json => receipt.to_json().to_pretty_string(),
            OutputFormat::Text => receipt_text(&receipt),
        }
    } else {
        let value = JsonValue::object([
            ("compilation", compilation.to_json()),
            ("execution", receipt.to_json()),
        ]);
        match compile.format {
            OutputFormat::Json => value.to_pretty_string(),
            OutputFormat::Text => format!(
                "{}\n\n[execution]\nbackend={}\nprovider={}\nmodel={}\nrequest_id={}\noutput={}\nsha256={}\nsize={}x{}\n",
                compilation.to_text().trim_end(),
                receipt.backend,
                receipt.provider,
                receipt.model,
                receipt.request_id.as_deref().unwrap_or(""),
                receipt.output_path.display(),
                receipt.output_sha256,
                receipt.width,
                receipt.height
            ),
        }
    };
    write_output(
        &text,
        compile.output.result.as_deref(),
        compile.output.force,
    )?;
    Ok(EXIT_OK)
}

fn receipt_text(receipt: &OpenAiEditReceipt) -> String {
    format!(
        "backend={}\nprovider={}\nmodel={}\nrequest_id={}\nhttp_status={}\ncompiled_prompt_sha256={}\ncompiled_prompt_chars={}\nreference_index={}\nreference_role={}\nreference_sha256={}\nreference_bytes={}\noutput={}\noutput_sha256={}\nsize={}x{}\nstarted_unix_ms={}\nfinished_unix_ms={}\n",
        receipt.backend,
        receipt.provider,
        receipt.model,
        receipt.request_id.as_deref().unwrap_or(""),
        receipt.http_status,
        receipt.compiled_prompt_sha256,
        receipt.compiled_prompt_chars,
        receipt.reference_index,
        receipt.reference_role,
        receipt.reference_sha256,
        receipt.reference_bytes,
        receipt.output_path.display(),
        receipt.output_sha256,
        receipt.width,
        receipt.height,
        receipt.started_unix_ms,
        receipt.finished_unix_ms,
    )
}
