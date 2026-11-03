//! Delivery CLI effects consume an already-admitted invocation.

use super::{write_json, write_text, CliResult, Command, Invocation};
use narrative_courier_contracts::DeliveryRequest;
use narrative_courier_delivery::DeliveryEngine;
use sts_workflow::Engine;

pub(super) fn dispatch(invocation: &Invocation, engine: &Engine) -> CliResult<()> {
    let name = &invocation.project;
    let positional = &invocation.positional;
    match invocation.command {
        Command::DeliverContext => {
            let package = engine.narrative_package(name)?;
            write_json(&DeliveryEngine::context(
                &engine.project_dir(name)?,
                &package,
            )?)
        }
        Command::Deliver => {
            let request: DeliveryRequest = serde_json::from_slice(&std::fs::read(&positional[0])?)?;
            let package = engine.narrative_package(name)?;
            write_json(&DeliveryEngine::run(
                &engine.project_dir(name)?,
                &package,
                request,
            )?)
        }
        Command::DeliverReview => {
            let package = engine.narrative_package(name)?;
            write_json(&DeliveryEngine::review(
                &engine.project_dir(name)?,
                &package,
                &positional[0],
                &positional[1],
                positional[2] == "approve",
                positional.get(3).map(String::as_str),
            )?)
        }
        Command::DeliverList => {
            let package = engine.narrative_package(name)?;
            write_json(&DeliveryEngine::list(&engine.project_dir(name)?, &package)?)
        }
        Command::DeliverCancel => {
            DeliveryEngine::cancel(&engine.project_dir(name)?)?;
            write_text(&format!("delivery cancellation requested for {name}\n"))
        }
        _ => Err("STS command reached Delivery CLI owner".into()),
    }
}
