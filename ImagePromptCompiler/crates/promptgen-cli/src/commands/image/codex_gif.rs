use crate::{
    error::{CliError, EXIT_OK},
    options::{ImageOptions, OutputFormat},
    output::{preflight_output, write_output},
    paths::{resolve_codex_home, same_lexical_path},
};
use promptgen_codex::{CodexRunConfig, execute_image_gif};
use promptgen_core::{CompilationOutcome, image::ImagePromptRequest, json::JsonValue};

pub(super) fn run(
    compilation: &CompilationOutcome,
    request: &ImagePromptRequest,
    options: ImageOptions,
) -> Result<u8, CliError> {
    let output = options
        .image_output
        .as_ref()
        .ok_or_else(|| CliError::usage("codex-imagegen GIF requires --image-output"))?;
    if let Some(result) = options.compile.output.result.as_deref() {
        if same_lexical_path(result, output)?
            || options
                .reference_image
                .as_ref()
                .is_some_and(|reference| result == reference)
        {
            return Err(CliError::usage(
                "receipt, reference and image output must have different paths",
            ));
        }
        preflight_output(result, options.compile.output.force)?;
    }
    let home = resolve_codex_home(options.codex_home)?;
    let mut config = CodexRunConfig::new(options.codex_binary, home, output);
    config.timeout = options.timeout;
    config.artifact_wait_timeout = options.artifact_wait;
    config.overwrite = options.compile.output.force;
    let receipt = execute_image_gif(
        compilation,
        request,
        options.reference_image.as_deref(),
        &config,
    )
    .map_err(|error| {
        if error.code == "CODEX_GIF_SERVICE_REJECTED" {
            CliError {
                exit_code: crate::error::EXIT_PROVIDER_REJECTED,
                code: "PROVIDER_REJECTED",
                message: error.to_string(),
            }
        } else {
            CliError::execution(error.to_string())
        }
    })?;
    let result = JsonValue::object([
        ("compilation", compilation.to_json()),
        ("execution", receipt),
    ]);
    let text = match options.compile.format {
        OutputFormat::Json | OutputFormat::Text => result.to_pretty_string(),
    };
    write_output(
        &text,
        options.compile.output.result.as_deref(),
        options.compile.output.force,
    )?;
    Ok(EXIT_OK)
}
