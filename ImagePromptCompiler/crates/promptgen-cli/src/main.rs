#![forbid(unsafe_code)]

mod commands;
mod error;
mod help;
mod options;
mod output;
mod paths;

use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

use error::{CliError, EXIT_OK};

use help::{print_help, print_interview_help};
use options::{
    ArgCursor, parse_catalog_options, parse_compile_options, parse_image_options,
    parse_lut_options, parse_mcp_options, parse_schema_options, parse_serve_options,
};

fn main() -> ExitCode {
    match dispatch(env::args_os().skip(1).collect()) {
        Ok(code) => ExitCode::from(code),
        Err(error) if error.exit_code == EXIT_OK => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error[{}]: {}", error.code, error.message);
            ExitCode::from(error.exit_code)
        }
    }
}

fn dispatch(arguments: Vec<OsString>) -> Result<u8, CliError> {
    let mut cursor = ArgCursor::new(arguments);
    let Some(command) = cursor.next_string()? else {
        print_help();
        return Ok(EXIT_OK);
    };
    match command.as_str() {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(EXIT_OK)
        }
        "version" | "--version" | "-V" => {
            println!("promptgen {}", env!("CARGO_PKG_VERSION"));
            Ok(EXIT_OK)
        }
        "image" => commands::compile_image(parse_image_options(cursor)?),
        "interview" => {
            commands::run_guided_interview(parse_compile_options(cursor, print_interview_help)?)
        }
        "serve" => commands::serve_ui(parse_serve_options(cursor)?),
        "mcp" => commands::serve_mcp(parse_mcp_options(cursor)?),
        "schema" => commands::emit_schema(parse_schema_options(cursor)?),
        "catalog" => commands::emit_catalog(parse_catalog_options(cursor)?),
        "lut-presets" => commands::emit_lut_presets(parse_lut_options(cursor)?),
        other => Err(CliError::usage(format!("unknown command {other:?}"))),
    }
}
