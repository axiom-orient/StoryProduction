//! Workspace and project lifecycle, import/export, and synopsis handoff routes.

use super::super::requests::common::ProjectRef;
use super::super::requests::workspace::{
    GeultuStoryExportRequest, ImportMarkdown, ProjectCreate, ProjectMetadataUpdate, RecoverProject,
    ReviewedSynopsisExportRequest,
};
use super::super::{command_id, parse, Application};
use crate::domain::Command;
use crate::error::{Error, Result};
use crate::markdown::{export_markdown, import_markdown};
use serde_json::{json, Value};

pub(in crate::application) fn call(
    app: &Application,
    name: &str,
    arguments: Value,
) -> Result<Value> {
    match name {
        "workspace.status" => Ok(serde_json::to_value(app.workspace.metadata()?)?),
        "workspace.recover_index" => Ok(serde_json::to_value(app.workspace.recover_index()?)?),
        "project.create" => {
            let req: ProjectCreate = parse(arguments)?;
            let command_id = command_id(req.command_id);
            Ok(serde_json::to_value(
                app.workspace.create_project_with_metadata(
                    &req.project_id,
                    &req.title,
                    req.display_label.as_deref(),
                    req.summary.as_deref(),
                    &command_id,
                )?,
            )?)
        }
        "project.update_metadata" => {
            let req: ProjectMetadataUpdate = parse(arguments)?;
            let command_id = command_id(req.command_id);
            Ok(serde_json::to_value(
                app.workspace.transact_if_project_revision(
                    &req.project_id,
                    req.expected_project_revision,
                    &command_id,
                    Command::SetProjectMetadata {
                        title: req.title,
                        display_label: Some(req.display_label),
                        summary: Some(req.summary),
                    },
                )?,
            )?)
        }
        "project.list" => Ok(serde_json::to_value(app.workspace.list_projects()?)?),
        "project.get" => {
            let req: ProjectRef = parse(arguments)?;
            Ok(serde_json::to_value(
                app.workspace.load_project(&req.project_id)?,
            )?)
        }
        "project.recover" => {
            let req: RecoverProject = parse(arguments)?;
            Ok(serde_json::to_value(
                app.workspace
                    .recover_project(&req.project_id, req.remove_lock)?,
            )?)
        }
        "project.import_markdown" => {
            let req: ImportMarkdown = parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            let start = project
                .scenes
                .values()
                .map(|scene| scene.order)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| Error::Conflict("scene order exceeds i64 capacity".into()))?;
            let scenes = import_markdown(&req.markdown, start)?;
            Ok(serde_json::to_value(
                app.workspace.transact_if_project_revision(
                    &req.project_id,
                    project.revision,
                    &command_id(req.command_id),
                    Command::ImportScenes { scenes },
                )?,
            )?)
        }

        "project.export_markdown" => {
            let req: ProjectRef = parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            Ok(
                json!({"project_id": project.id, "revision": project.revision, "markdown": export_markdown(&project)}),
            )
        }
        "synopsis.export_reviewed_handoff" => {
            let req: ReviewedSynopsisExportRequest = parse(arguments)?;
            let handoff = app.workspace.export_reviewed_synopsis(
                &req.project_id,
                &req.actor_id,
                &req.language,
                req.reason.as_deref(),
                req.expected_source_revision,
                &req.expected_synopsis_sha256,
            )?;
            Ok(serde_json::to_value(handoff)?)
        }
        "story.export" => {
            let req: GeultuStoryExportRequest = parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            let input = crate::story_package::GeultuStoryExportInput {
                language: req.language,
                logline: req.logline,
                global_constraints: req.global_constraints,
            };
            let story_package =
                crate::story_package::export_geultu_story_package(&project, &input)?;
            Ok(json!({
                "project_id": project.id,
                "source_project_revision": project.revision,
                "story_package_id": story_package.package_id,
                "story_package": story_package,
            }))
        }
        _ => Err(super::unknown_tool(name)),
    }
}
