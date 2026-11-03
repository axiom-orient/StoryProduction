use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::error::CliError;

use super::{
    ArgCursor, CompileOptions, OutputFormat, OutputOptions, claim_option, parse_output_option,
};

pub(crate) fn parse_compile_options(
    mut cursor: ArgCursor,
    print_help: fn(),
) -> Result<CompileOptions, CliError> {
    let mut input = PathBuf::from("-");
    let mut format = OutputFormat::Json;
    let mut output = OutputOptions::default();
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        if parse_output_option(&argument, &mut cursor, &mut seen, &mut output)? {
            continue;
        }
        match argument.as_str() {
            "--input" => {
                claim_option(&mut seen, "--input")?;
                input = PathBuf::from(cursor.required_value("--input")?);
            }
            "--format" => {
                claim_option(&mut seen, "--format")?;
                format = OutputFormat::parse(&cursor.required_value("--format")?)?;
            }
            "--help" | "-h" => {
                print_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    Ok(CompileOptions {
        input,
        format,
        output,
    })
}
