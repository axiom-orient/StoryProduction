mod openai;

use promptgen_codex::{CodexRunConfig, execute_image_generation};
use promptgen_core::image::ImagePromptRequest;
use promptgen_core::json::JsonValue;
use promptgen_core::{CompilationOutcome, CompilationStatus, compile_image_prompt};

use self::openai::{preflight_paths, run as run_openai_edit};
use super::super::error::{CliError, EXIT_OK, EXIT_VALIDATION};
use super::super::options::{CompileOptions, ImageMode, ImageOptions, OutputFormat};
use super::super::output::{preflight_output, read_json, write_output};
use super::super::paths::{resolve_codex_home, same_lexical_path};

pub(crate) fn compile_image(options: ImageOptions) -> Result<u8, CliError> {
    let request = ImagePromptRequest::from_json(read_json(&options.compile.input)?)
        .map_err(|error| CliError::validation(error.to_string()))?;
    let openai_edit_preflight = if options.mode == ImageMode::OpenAiImageEdit {
        Some(preflight_paths(&options)?)
    } else {
        None
    };
    let compilation = compile_image_prompt(&request);
    if compilation.status == CompilationStatus::Invalid {
        return emit_compilation(compilation, options.compile);
    }
    match options.mode {
        ImageMode::PromptOnly => {
            if options.image_output.is_some() || options.reference_image.is_some() {
                return Err(CliError::usage(
                    "--image-output and --reference-image require an image execution mode",
                ));
            }
            emit_compilation(compilation, options.compile)
        }
        ImageMode::CodexImagegen => {
            if options.reference_image.is_some() {
                return Err(CliError::usage(
                    "--reference-image is valid only with --mode openai-image-edit",
                ));
            }
            let image_output = options
                .image_output
                .ok_or_else(|| CliError::usage("--mode codex-imagegen requires --image-output"))?;
            if let Some(result) = options.compile.output.result.as_deref()
                && same_lexical_path(result, &image_output)?
            {
                return Err(CliError::usage(
                    "--result and --image-output must be different paths",
                ));
            }
            if let Some(result) = options.compile.output.result.as_deref() {
                preflight_output(result, options.compile.output.force)?;
            }
            let codex_home = resolve_codex_home(options.codex_home)?;
            let mut config = CodexRunConfig::new(options.codex_binary, codex_home, image_output);
            config.timeout = options.timeout;
            config.artifact_wait_timeout = options.artifact_wait;
            config.overwrite = options.compile.output.force;
            let receipt = execute_image_generation(&compilation, &request, &config)
                .map_err(|error| CliError::execution(error.to_string()))?;
            let value = JsonValue::object([
                ("compilation", compilation.to_json()),
                ("execution", receipt.to_json()),
            ]);
            let text = match options.compile.format {
                OutputFormat::Json => value.to_pretty_string(),
                OutputFormat::Text => format!(
                    "{}\n\n[execution]\nbackend={}\nthread_id={}\ncall_id={}\noutput={}\nsha256={}\nsize={}x{}\n",
                    compilation.to_text().trim_end(),
                    receipt.backend,
                    receipt.thread_id,
                    receipt.image_call_id,
                    receipt.output_artifact.display(),
                    receipt.sha256,
                    receipt.width,
                    receipt.height
                ),
            };
            write_output(
                &text,
                options.compile.output.result.as_deref(),
                options.compile.output.force,
            )?;
            Ok(EXIT_OK)
        }
        ImageMode::OpenAiImageEdit => {
            let preflight = openai_edit_preflight
                .ok_or_else(|| CliError::execution("OpenAI edit paths were not preflighted"))?;
            run_openai_edit(
                &compilation,
                &request,
                options.compile,
                options.timeout,
                preflight,
            )
        }
    }
}

fn emit_compilation(
    compilation: CompilationOutcome,
    options: CompileOptions,
) -> Result<u8, CliError> {
    let text = match options.format {
        OutputFormat::Json => compilation.to_json().to_pretty_string(),
        OutputFormat::Text => compilation.to_text(),
    };
    write_output(
        &text,
        options.output.result.as_deref(),
        options.output.force,
    )?;
    Ok(if compilation.status == CompilationStatus::Invalid {
        EXIT_VALIDATION
    } else {
        EXIT_OK
    })
}
