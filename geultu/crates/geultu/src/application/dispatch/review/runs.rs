//! Text-run initialization route.

use super::super::super::{commit, Application};
use crate::application::requests::review::RunOpen;
use crate::contracts::{TextRunState, TextRunStatus};
use crate::domain::Command;
use crate::error::{Error, Result};
use serde_json::Value;

pub(super) fn call(app: &Application, name: &str, arguments: Value) -> Result<Value> {
    match name {
        "run.open" => {
            let req: RunOpen = super::super::super::parse(arguments)?;
            let run = TextRunState {
                schema_id: crate::contracts::TEXT_RUN_STATE_SCHEMA_ID.into(),
                project_id: req.project_id.clone(),
                run_id: req.run_id,
                revision: 0,
                stage: req.stage,
                status: TextRunStatus::Ready,
                attempt: 0,
                budgets: req.budgets,
                frozen_inputs: req.frozen_inputs,
                accepted_outputs: req.accepted_outputs.unwrap_or_default(),
                pending_decision: None,
            };
            run.validate()?;
            let run_id = run.run_id.clone();
            let mut result = commit(
                &app.workspace,
                &req.project_id,
                req.command_id,
                Command::OpenTextRun { run },
            )?;
            let recorded = app.workspace.load_project(&req.project_id)?;
            let stored = recorded.text_runs.get(&run_id).ok_or_else(|| {
                Error::Conflict("run.open command did not record the requested run".into())
            })?;
            result
                .as_object_mut()
                .ok_or_else(|| Error::Corrupt("commit result is not an object".into()))?
                .insert("run".into(), serde_json::to_value(stored)?);
            Ok(result)
        }
        _ => Err(super::super::unknown_tool(name)),
    }
}
