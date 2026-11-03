//! Visual planning and fail-closed package admission routes.

use super::super::requests::common::ProjectRef;
use super::super::requests::visual::{
    VisualAdmitPackage, VisualAdmitPackageResponse, VisualPlanRequest,
};
use super::super::support::generated_id;
use super::super::{command_id, commit, parse, Application};
use super::unknown_tool;
use crate::domain::Command;
use crate::error::{Error, Result};
use crate::visual::plan_scene;
use serde_json::Value;

pub(in crate::application) fn call(
    app: &Application,
    name: &str,
    arguments: Value,
) -> Result<Value> {
    match name {
        "visual.plan" => {
            let req: VisualPlanRequest = parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            let plan = plan_scene(
                &project,
                &req.scene_id,
                req.plan_id.unwrap_or_else(|| generated_id("visual")),
            )?;
            commit(
                &app.workspace,
                &req.project_id,
                req.command_id,
                Command::AddVisualPlan { plan },
            )
        }
        "visual.list" => {
            let req: ProjectRef = parse(arguments)?;
            Ok(serde_json::to_value(
                app.workspace
                    .load_project(&req.project_id)?
                    .visual_plans
                    .into_values()
                    .collect::<Vec<_>>(),
            )?)
        }
        "visual.admit_package" => {
            let req: VisualAdmitPackage = parse(arguments)?;
            let project = app.workspace.load_project(&req.input.project_id)?;
            let outcome = crate::visual_run::admit_package(&project, &req.input)?;
            let result = app.workspace.transact_if_project_revision(
                &req.input.project_id,
                project.revision,
                &command_id(req.command_id),
                Command::AdmitVisualPackage {
                    run: outcome.run.clone(),
                    receipt: outcome.receipt.clone(),
                },
            )?;
            let recorded = app.workspace.load_project(&req.input.project_id)?;
            if recorded.visual_runs.get(&outcome.run.run_id) != Some(&outcome.run)
                || recorded
                    .validation_receipts
                    .get(&outcome.receipt.receipt_id)
                    != Some(&outcome.receipt)
            {
                return Err(Error::Conflict(
                    "visual admission command did not record the requested run/receipt".into(),
                ));
            }
            Ok(serde_json::to_value(VisualAdmitPackageResponse {
                project_id: req.input.project_id,
                run_id: outcome.run.run_id.clone(),
                preflight: outcome.preflight,
                run: outcome.run,
                receipt: outcome.receipt,
                revision: result.revision,
                event_count: result.event_count,
                idempotent: result.idempotent,
            })?)
        }
        _ => Err(unknown_tool(name)),
    }
}
