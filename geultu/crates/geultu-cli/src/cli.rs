use geultu::Error;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

pub(super) mod commands;

pub(super) enum CliError {
    Usage(String),
    Engine(Error),
    Other(String),
}
impl From<Error> for CliError {
    fn from(value: Error) -> Self {
        Self::Engine(value)
    }
}
impl From<io::Error> for CliError {
    fn from(value: io::Error) -> Self {
        Self::Engine(Error::Io(value))
    }
}
impl From<serde_json::Error> for CliError {
    fn from(value: serde_json::Error) -> Self {
        Self::Engine(Error::Json(value))
    }
}

struct Options {
    workspace: PathBuf,
    writer_program: Option<PathBuf>,
    writer_args: Vec<String>,
    pretty: bool,
    command: String,
    args: Vec<String>,
}

fn parse_args(args: Vec<String>) -> Result<Options, CliError> {
    let mut workspace = PathBuf::from(".");
    let mut writer_program = None;
    let mut writer_args = Vec::new();
    let mut pretty = false;
    let mut command = None;
    let mut rest = Vec::new();
    let mut literal = false;
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        if command.is_some() {
            if !literal && argument == "--" {
                literal = true;
            } else if !literal && argument == "--pretty" {
                pretty = true;
            } else {
                rest.push(argument);
            }
            continue;
        }
        if literal {
            command = Some(argument);
            continue;
        }
        match argument.as_str() {
            "--workspace" => {
                workspace = PathBuf::from(
                    args.next()
                        .ok_or_else(|| CliError::Usage("--workspace requires a path".into()))?,
                );
                if workspace.as_os_str().is_empty() {
                    return Err(CliError::Usage("--workspace must not be empty".into()));
                }
            }
            "--writer-program" => {
                let path = args
                    .next()
                    .ok_or_else(|| CliError::Usage("--writer-program requires a path".into()))?;
                if path.is_empty() {
                    return Err(CliError::Usage("--writer-program must not be empty".into()));
                }
                writer_program = Some(PathBuf::from(path));
            }
            "--writer-arg" => writer_args.push(
                args.next()
                    .ok_or_else(|| CliError::Usage("--writer-arg requires a value".into()))?,
            ),
            "--pretty" => pretty = true,
            "--version" | "-V" => command = Some("version".into()),
            "--help" | "-h" => command = Some("help".into()),
            "--" => literal = true,
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!("unknown option '{value}'")));
            }
            _ => command = Some(argument),
        }
    }
    if writer_program.is_none() && !writer_args.is_empty() {
        return Err(CliError::Usage(
            "--writer-arg requires --writer-program".into(),
        ));
    }
    Ok(Options {
        workspace,
        writer_program,
        writer_args,
        pretty,
        command: command.unwrap_or_else(|| "help".into()),
        args: rest,
    })
}

fn read_json_argument(value: &str) -> Result<Value, CliError> {
    let content = if value == "-" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else if let Some(path) = value.strip_prefix('@') {
        fs::read_to_string(path)?
    } else {
        value.into()
    };
    serde_json::from_str(&content)
        .map_err(|error| CliError::Usage(format!("invalid JSON arguments: {error}")))
}

fn read_text(value: &str) -> Result<String, CliError> {
    if value == "-" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        Ok(buffer)
    } else {
        Ok(fs::read_to_string(value)?)
    }
}

fn require_len(
    args: &[String],
    minimum: usize,
    maximum: usize,
    usage: &str,
) -> Result<(), CliError> {
    if !(minimum..=maximum).contains(&args.len()) {
        Err(CliError::Usage(format!("usage: geultu {usage}")))
    } else {
        Ok(())
    }
}

fn print_result(result: geultu::Result<Value>, pretty: bool) -> Result<(), CliError> {
    match result {
        Ok(value) => print_value(json!({"ok":true,"result":value}), pretty),
        Err(error) => Err(CliError::Engine(error)),
    }
}

fn print_value(value: Value, pretty: bool) -> Result<(), CliError> {
    let text = if pretty {
        serde_json::to_string_pretty(&value)?
    } else {
        serde_json::to_string(&value)?
    };
    use std::io::Write;
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{text}")?;
    Ok(())
}

pub(super) fn usage() -> &'static str {
    "geultu — canonical writing engine\n\n\
Usage:\n  geultu [--workspace PATH] init\n  geultu [--workspace PATH] tools [--pretty]\n  geultu [--workspace PATH] call TOOL_NAME [JSON|@FILE|-] [--pretty]\n  geultu [--workspace PATH] mcp\n  geultu [--workspace PATH] doctor\n  geultu [--workspace PATH] project-create PROJECT_ID TITLE\n  geultu [--workspace PATH] project-list\n  geultu [--workspace PATH] scene-add PROJECT_ID SCENE_ID TITLE [TEXT_FILE|-]\n  geultu [--workspace PATH] scene-set PROJECT_ID SCENE_ID TEXT_FILE|- [EXPECTED_SCENE_REVISION]\n  geultu [--workspace PATH] review PROJECT_ID\n  geultu [--workspace PATH] export PROJECT_ID [OUTPUT_FILE]\n  geultu [--workspace PATH] export-reviewed-synopsis PROJECT_ID ACTOR_ID REVISION SHA256 LANGUAGE [REASON]\n\n\
Options:\n  Put workspace/writer options before the command. --pretty is also accepted after it.\n  Use -- to preserve following arguments literally, including --pretty.\n  --writer-program PATH accepts repeated --writer-arg ARG.\n  The writer receives GenerationPlan JSON and returns GeneratedDraft JSON.\n\n\
Use command_id when the tool schema permits it for idempotent retries."
}
