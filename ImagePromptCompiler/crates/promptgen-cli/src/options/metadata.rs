use std::collections::BTreeSet;

use promptgen_core::catalog::CatalogFamily;

use crate::error::CliError;
use crate::help::{print_catalog_help, print_lut_help, print_schema_help};

use super::{ArgCursor, OutputFormat, OutputOptions, claim_option, parse_output_option};

#[derive(Debug)]
pub(crate) struct SchemaOptions {
    pub(crate) target: String,
    pub(crate) output: OutputOptions,
}

#[derive(Debug)]
pub(crate) struct CatalogOptions {
    pub(crate) family: Option<CatalogFamily>,
    pub(crate) format: OutputFormat,
    pub(crate) output: OutputOptions,
}

pub(crate) fn parse_schema_options(mut cursor: ArgCursor) -> Result<SchemaOptions, CliError> {
    let target = match cursor.next_string()? {
        Some(value) if matches!(value.as_str(), "--help" | "-h") => {
            print_schema_help();
            return Err(CliError::printed_help());
        }
        Some(value) => value,
        None => return Err(CliError::usage("schema requires a target")),
    };
    let mut output = OutputOptions::default();
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        if parse_output_option(&argument, &mut cursor, &mut seen, &mut output)? {
            continue;
        }
        match argument.as_str() {
            "--help" | "-h" => {
                print_schema_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    Ok(SchemaOptions { target, output })
}

pub(crate) fn parse_catalog_options(mut cursor: ArgCursor) -> Result<CatalogOptions, CliError> {
    let mut family = None;
    let mut format = OutputFormat::Json;
    let mut output = OutputOptions::default();
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        if parse_output_option(&argument, &mut cursor, &mut seen, &mut output)? {
            continue;
        }
        match argument.as_str() {
            "--family" => {
                claim_option(&mut seen, "--family")?;
                let value = cursor.required_value("--family")?;
                family = if value == "all" {
                    None
                } else {
                    Some(CatalogFamily::parse(&value).ok_or_else(|| {
                        CliError::usage(format!("unknown catalog family {value:?}"))
                    })?)
                };
            }
            "--format" => {
                claim_option(&mut seen, "--format")?;
                format = OutputFormat::parse(&cursor.required_value("--format")?)?;
            }
            "--help" | "-h" => {
                print_catalog_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    Ok(CatalogOptions {
        family,
        format,
        output,
    })
}

pub(crate) fn parse_lut_options(mut cursor: ArgCursor) -> Result<OutputOptions, CliError> {
    let mut output = OutputOptions::default();
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        if parse_output_option(&argument, &mut cursor, &mut seen, &mut output)? {
            continue;
        }
        match argument.as_str() {
            "--help" | "-h" => {
                print_lut_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    Ok(output)
}
