//! NVC-owned derived direction: current released source, candidates, and separate review.
//! These projections never write STS authority or adopt assets/rendered media.

mod animatic;
mod arguments;
mod codec;
mod project;
mod schemas;
mod storyboard;

use crate::codec::{json, Value};
use std::fmt;
use std::io::Write;

pub const MAX_JSON_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug)]
pub struct Error {
    pub outcome: &'static str,
    pub message: String,
    io: bool,
}

impl Error {
    fn refusal(message: impl Into<String>) -> Self {
        Self {
            outcome: "not_applied",
            message: message.into(),
            io: false,
        }
    }
    fn unknown(message: impl Into<String>) -> Self {
        Self {
            outcome: "outcome_unknown",
            message: message.into(),
            io: false,
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self {
            outcome: "not_applied",
            message: error.to_string(),
            io: true,
        }
    }
}
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::refusal(error.to_string())
    }
}
impl From<narrative_courier_runtime::Error> for Error {
    fn from(error: narrative_courier_runtime::Error) -> Self {
        Self::refusal(error.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;

fn require(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::refusal(message))
    }
}

struct Publication {
    path: String,
    data: Vec<u8>,
    response: Value,
}
impl Publication {
    fn json(path: String, value: &Value, response: Value) -> Result<Self> {
        let data = codec::artifact_bytes(value)?;
        require(
            data.len() <= MAX_JSON_BYTES,
            "STORYBOARD_ARTIFACT_TOO_LARGE",
        )?;
        Ok(Self {
            path,
            data,
            response,
        })
    }
}
enum Operation {
    Observation(Value),
    Publication(Publication),
}

/// Public owner operation. Argument admission happens before project/filesystem effects.
/// Response bytes are bounded before any immutable publication is attempted.
pub fn execute(lane: &str, args: impl IntoIterator<Item = String>) -> Result<serde_json::Value> {
    execute_ordered(lane, args).map(|value| value.to_semantic())
}

fn execute_ordered(lane: &str, args: impl IntoIterator<Item = String>) -> Result<Value> {
    let invocation = arguments::parse(lane, args)?;
    let project = project::Project::open(&invocation.root)?;
    let _lock = project.lock()?;
    let operation = match lane {
        "storyboard" => storyboard::run(&project, &invocation)?,
        "animatic" => animatic::run(&project, &invocation)?,
        _ => return Err(Error::refusal("UNKNOWN_AUTHORING_LANE")),
    };
    let response = match &operation {
        Operation::Observation(response) => response,
        Operation::Publication(publication) => &publication.response,
    };
    let output = codec::artifact_bytes(response)?;
    require(
        output.len() <= MAX_JSON_BYTES,
        "AUTHORING_RESPONSE_TOO_LARGE",
    )?;
    if let Operation::Publication(publication) = &operation {
        project
            .publish(&publication.path, &publication.data)
            .map_err(|error| {
                if lane == "animatic" && error.io {
                    Error::unknown(format!(
                        "Publication not confirmed; inspect {} before retrying: {error}",
                        publication.path
                    ))
                } else {
                    error
                }
            })?;
    }
    Ok(response.clone())
}

/// Native CLI facade; stdout loss after a potentially publishing command stays unknown.
pub fn cli_main(lane: &str, args: Vec<String>) -> i32 {
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h" | "help") {
        println!("{}", arguments::help(lane));
        return 0;
    }
    match execute_ordered(lane, args) {
        Ok(response) => {
            let output = match codec::artifact_bytes(&response) {
                Ok(bytes) => bytes,
                Err(error) => return write_error(Error::unknown(error.message)),
            };
            let mut stdout = std::io::stdout().lock();
            if let Err(error) = stdout.write_all(&output).and_then(|()| stdout.flush()) {
                return write_error(Error::unknown(format!("RESPONSE_NOT_CONFIRMED:{error}")));
            }
            0
        }
        Err(error) => write_error(error),
    }
}
fn write_error(error: Error) -> i32 {
    let value = json!({"status": error.outcome, "error": error.message});
    let _ = writeln!(std::io::stderr().lock(), "{value}");
    if error.outcome == "outcome_unknown" {
        3
    } else {
        2
    }
}
