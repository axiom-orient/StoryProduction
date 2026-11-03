//! Closed CLI admission and domain-owner dispatch; stdout failure remains an error.

mod arguments;
mod delivery;
mod sts;

pub(super) use arguments::parse;
use arguments::{Command, Invocation};
use std::io::Write;
use sts_workflow::Engine;
pub(super) type CliResult<T> = Result<T, Box<dyn std::error::Error>>;

fn write_text(text: &str) -> CliResult<()> {
    let mut output = std::io::stdout().lock();
    output.write_all(text.as_bytes())?;
    output.flush()?;
    Ok(())
}
fn write_json(value: &impl serde::Serialize) -> CliResult<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    let mut output = std::io::stdout().lock();
    output.write_all(&bytes)?;
    output.flush()?;
    Ok(())
}

pub(super) fn dispatch(invocation: &Invocation, engine: &Engine) -> CliResult<()> {
    match invocation.command {
        Command::DeliverContext
        | Command::Deliver
        | Command::DeliverReview
        | Command::DeliverList
        | Command::DeliverCancel => delivery::dispatch(invocation, engine),
        _ => sts::dispatch(invocation, engine),
    }
}
