use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use crate::error::CliError;
use crate::help::print_image_help;

use super::{
    ArgCursor, CompileOptions, OutputFormat, OutputOptions, claim_option, parse_output_option,
    parse_seconds,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImageMode {
    PromptOnly,
    CodexImagegen,
    OpenAiImageEdit,
    OpenAiImageGenerate,
}

pub(crate) struct ImageOptions {
    pub(crate) compile: CompileOptions,
    pub(crate) mode: ImageMode,
    pub(crate) image_output: Option<PathBuf>,
    pub(crate) reference_image: Option<PathBuf>,
    pub(crate) codex_binary: PathBuf,
    pub(crate) codex_home: Option<PathBuf>,
    pub(crate) timeout: Duration,
    pub(crate) artifact_wait: Duration,
}

impl ImageMode {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "prompt-only" => Ok(Self::PromptOnly),
            "codex-imagegen" => Ok(Self::CodexImagegen),
            "openai-image-edit" => Ok(Self::OpenAiImageEdit),
            "openai-image-generate" => Ok(Self::OpenAiImageGenerate),
            _ => Err(CliError::usage(format!(
                "invalid --mode {value:?}; expected prompt-only, codex-imagegen, openai-image-edit, or openai-image-generate"
            ))),
        }
    }
}

pub(crate) fn parse_image_options(mut cursor: ArgCursor) -> Result<ImageOptions, CliError> {
    let mut compile = CompileOptions {
        input: PathBuf::from("-"),
        format: OutputFormat::Json,
        output: OutputOptions::default(),
    };
    let mut mode = ImageMode::PromptOnly;
    let mut image_output = None;
    let mut reference_image = None;
    let mut codex_binary = PathBuf::from("codex");
    let mut codex_home = None;
    let mut timeout = Duration::from_secs(240);
    let mut artifact_wait = Duration::from_secs(30);
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        if parse_output_option(&argument, &mut cursor, &mut seen, &mut compile.output)? {
            continue;
        }
        match argument.as_str() {
            "--input" => {
                claim_option(&mut seen, "--input")?;
                compile.input = PathBuf::from(cursor.required_value("--input")?);
            }
            "--format" => {
                claim_option(&mut seen, "--format")?;
                compile.format = OutputFormat::parse(&cursor.required_value("--format")?)?;
            }
            "--mode" => {
                claim_option(&mut seen, "--mode")?;
                mode = ImageMode::parse(&cursor.required_value("--mode")?)?;
            }
            "--image-output" => {
                claim_option(&mut seen, "--image-output")?;
                image_output = Some(PathBuf::from(cursor.required_value("--image-output")?));
            }
            "--reference-image" => {
                claim_option(&mut seen, "--reference-image")?;
                reference_image = Some(PathBuf::from(cursor.required_value("--reference-image")?));
            }
            "--codex-binary" => {
                claim_option(&mut seen, "--codex-binary")?;
                codex_binary = PathBuf::from(cursor.required_value("--codex-binary")?);
            }
            "--codex-home" => {
                claim_option(&mut seen, "--codex-home")?;
                codex_home = Some(PathBuf::from(cursor.required_value("--codex-home")?));
            }
            "--timeout-seconds" => {
                claim_option(&mut seen, "--timeout-seconds")?;
                timeout = Duration::from_secs(parse_seconds(
                    &cursor.required_value("--timeout-seconds")?,
                    "--timeout-seconds",
                    3_600,
                )?);
            }
            "--artifact-wait-seconds" => {
                claim_option(&mut seen, "--artifact-wait-seconds")?;
                artifact_wait = Duration::from_secs(parse_seconds(
                    &cursor.required_value("--artifact-wait-seconds")?,
                    "--artifact-wait-seconds",
                    600,
                )?);
            }
            "--help" | "-h" => {
                print_image_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    if mode != ImageMode::CodexImagegen
        && (seen.contains("--codex-binary") || seen.contains("--codex-home"))
    {
        return Err(CliError::usage(
            "--codex-binary and --codex-home require --mode codex-imagegen; openai-image-edit uses OPENAI_API_KEY",
        ));
    }
    Ok(ImageOptions {
        compile,
        mode,
        image_output,
        reference_image,
        codex_binary,
        codex_home,
        timeout,
        artifact_wait,
    })
}
