use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::PathBuf;

use promptgen_mcp::MAX_BODY_BYTES;

use crate::error::CliError;

mod image;
mod interview;
mod metadata;
mod service;

pub(super) use self::image::{ImageMode, ImageOptions, parse_image_options};
pub(super) use self::interview::parse_compile_options;
pub(super) use self::metadata::{
    CatalogOptions, SchemaOptions, parse_catalog_options, parse_lut_options, parse_schema_options,
};
pub(super) use self::service::{
    McpOptions, McpTransport, ServeOptions, parse_mcp_options, parse_serve_options,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OutputFormat {
    Json,
    Text,
}

impl OutputFormat {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "json" => Ok(Self::Json),
            "text" => Ok(Self::Text),
            _ => Err(CliError::usage(format!(
                "invalid --format {value:?}; expected json or text"
            ))),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct OutputOptions {
    pub(crate) result: Option<PathBuf>,
    pub(crate) force: bool,
}

#[derive(Debug)]
pub(super) struct CompileOptions {
    pub(super) input: PathBuf,
    pub(super) format: OutputFormat,
    pub(super) output: OutputOptions,
}

fn parse_body_bytes(value: &str) -> Result<usize, CliError> {
    let bytes = value
        .parse::<usize>()
        .map_err(|_| CliError::usage("--max-body-bytes expects a positive integer"))?;
    if bytes == 0 || bytes > MAX_BODY_BYTES {
        return Err(CliError::usage(format!(
            "--max-body-bytes must be within 1..={MAX_BODY_BYTES}"
        )));
    }
    Ok(bytes)
}

fn claim_option(seen: &mut BTreeSet<&'static str>, option: &'static str) -> Result<(), CliError> {
    if seen.insert(option) {
        Ok(())
    } else {
        Err(CliError::usage(format!(
            "option {option} was provided more than once"
        )))
    }
}

fn parse_output_option(
    argument: &str,
    cursor: &mut ArgCursor,
    seen: &mut BTreeSet<&'static str>,
    output: &mut OutputOptions,
) -> Result<bool, CliError> {
    match argument {
        "--result" => {
            claim_option(seen, "--result")?;
            output.result = Some(PathBuf::from(cursor.required_value("--result")?));
            Ok(true)
        }
        "--force" => {
            claim_option(seen, "--force")?;
            output.force = true;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn parse_seconds(value: &str, option: &str, maximum: u64) -> Result<u64, CliError> {
    let seconds = value
        .parse::<u64>()
        .map_err(|_| CliError::usage(format!("{option} expects a positive integer")))?;
    if seconds == 0 || seconds > maximum {
        return Err(CliError::usage(format!(
            "{option} must be within 1..={maximum} seconds"
        )));
    }
    Ok(seconds)
}

pub(super) struct ArgCursor {
    values: Vec<OsString>,
    index: usize,
}

impl ArgCursor {
    pub(super) const fn new(values: Vec<OsString>) -> Self {
        Self { values, index: 0 }
    }

    pub(super) fn next_string(&mut self) -> Result<Option<String>, CliError> {
        let Some(value) = self.values.get(self.index) else {
            return Ok(None);
        };
        self.index += 1;
        value
            .clone()
            .into_string()
            .map(Some)
            .map_err(|_| CliError::usage("arguments must be valid UTF-8"))
    }

    fn required_value(&mut self, option: &str) -> Result<String, CliError> {
        self.next_string()?
            .ok_or_else(|| CliError::usage(format!("{option} requires a value")))
    }
}
