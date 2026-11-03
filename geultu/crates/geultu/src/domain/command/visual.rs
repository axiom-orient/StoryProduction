//! Pure visual-plan and visual-run admission decisions.

use super::super::validation::{require_scene, validate_id};
use super::super::Event;
use crate::contracts::{ReceiptOutcome, ValidationReceipt, VisualRunState, VisualRunStatus};
use crate::error::{Error, Result};
use crate::model::{Project, VisualPlan};

pub(super) fn add_plan(project: &Project, plan: VisualPlan) -> Result<Vec<Event>> {
    validate_id("visual plan id", &plan.id)?;
    require_scene(project, &plan.scene_id)?;
    if plan.source_revision != project.content_revision {
        return Err(Error::Conflict(format!(
            "visual plan source revision mismatch: expected {}, actual {}",
            project.content_revision, plan.source_revision
        )));
    }
    if project.visual_plans.contains_key(&plan.id) {
        return Err(Error::Conflict(format!(
            "visual plan '{}' already exists",
            plan.id
        )));
    }
    Ok(vec![Event::VisualPlanAdded { plan }])
}

pub(super) fn admit_package(
    project: &Project,
    run: VisualRunState,
    receipt: ValidationReceipt,
) -> Result<Vec<Event>> {
    run.validate()?;
    receipt.validate()?;
    if run.project_id != project.id {
        return Err(Error::InvalidInput(format!(
            "visual run project_id '{}' does not match project '{}'",
            run.project_id, project.id
        )));
    }
    if run.status != VisualRunStatus::Ready
        || run.revision != 0
        || !run.accepted_outputs.is_empty()
        || run.pending_effect.is_some()
        || receipt.outcome != ReceiptOutcome::Pass
        || receipt.validator.id != crate::visual_run::ADMISSION_VALIDATOR_ID
    {
        return Err(Error::InvalidInput(
            "visual admission requires an initial READY run and an admission PASS receipt".into(),
        ));
    }
    let source_revision = project.content_revision.checked_add(1).ok_or_else(|| {
        Error::Conflict("visual admission source revision exceeds u64 capacity".into())
    })?;
    if run.narrative_package.revision != source_revision {
        return Err(Error::Conflict(
            "visual admission package source revision is stale".into(),
        ));
    }
    if receipt.subject != run.narrative_package {
        return Err(Error::Corrupt(
            "admission receipt subject does not match the admitted package reference".into(),
        ));
    }
    match project.visual_runs.get(&run.run_id) {
        Some(existing) if existing == &run => Ok(Vec::new()),
        Some(_) => Err(Error::Conflict(format!(
            "visual run '{}' is already admitted with different content",
            run.run_id
        ))),
        None => Ok(vec![
            Event::VisualRunAdmitted { run },
            Event::ValidationReceiptRecorded { receipt },
        ]),
    }
}
