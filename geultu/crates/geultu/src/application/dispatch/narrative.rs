//! Scene, fact, timeline, and thread routes over canonical project state.

use super::super::requests::common::ProjectRef;
use super::super::requests::narrative::{
    FactAdd, SceneAdd, SceneRef, SceneSetText, ThreadAdd, ThreadSetStatus, TimelineAdd,
};
use super::super::{commit, parse, Application};
use crate::domain::Command;
use crate::error::{Error, Result};
use crate::model::{Fact, Scene, StoryThread, ThreadStatus, TimelineEntry};
use serde_json::Value;

pub(in crate::application) fn call(
    app: &Application,
    name: &str,
    arguments: Value,
) -> Result<Value> {
    match name {
        "scene.add" => {
            let req: SceneAdd = parse(arguments)?;
            if !crate::contracts::validate::valid_scene_id(&req.scene_id) {
                return Err(Error::InvalidInput(format!(
                    "scene_id '{}' must match ^scene_[A-Za-z0-9_.-]{{1,120}}$; \
                     story.export rejects any other scene id",
                    req.scene_id
                )));
            }
            let scene = Scene {
                id: req.scene_id,
                title: req.title,
                text: req.text.unwrap_or_default(),
                order: req.order.unwrap_or(0),
                revision: 0,
            };
            commit(
                &app.workspace,
                &req.project_id,
                req.command_id,
                Command::AddScene { scene },
            )
        }
        "scene.list" => {
            let req: ProjectRef = parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            let mut scenes: Vec<_> = project.scenes.into_values().collect();
            scenes.sort_by_key(|scene| (scene.order, scene.id.clone()));
            Ok(serde_json::to_value(scenes)?)
        }
        "scene.get" => {
            let req: SceneRef = parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            Ok(serde_json::to_value(
                project.scenes.get(&req.scene_id).ok_or_else(|| {
                    Error::NotFound(format!("scene '{}' was not found", req.scene_id))
                })?,
            )?)
        }
        "scene.set_text" => {
            let req: SceneSetText = parse(arguments)?;
            commit(
                &app.workspace,
                &req.project_id,
                req.command_id,
                Command::SetSceneText {
                    scene_id: req.scene_id,
                    text: req.text,
                    expected_scene_revision: req.expected_scene_revision,
                    run_id: req.run_id,
                },
            )
        }
        "fact.add" => {
            let req: FactAdd = parse(arguments)?;
            let fact = Fact {
                id: req.fact_id,
                subject: req.subject,
                predicate: req.predicate,
                object: req.object,
                evidence: req.evidence,
            };
            commit(
                &app.workspace,
                &req.project_id,
                req.command_id,
                Command::AddFact { fact },
            )
        }
        "fact.list" => {
            let req: ProjectRef = parse(arguments)?;
            Ok(serde_json::to_value(
                app.workspace
                    .load_project(&req.project_id)?
                    .facts
                    .into_values()
                    .collect::<Vec<_>>(),
            )?)
        }
        "timeline.add" => {
            let req: TimelineAdd = parse(arguments)?;
            let entry = TimelineEntry {
                id: req.entry_id,
                order: req.order,
                label: req.label,
                description: req.description,
                scene_id: req.scene_id,
            };
            commit(
                &app.workspace,
                &req.project_id,
                req.command_id,
                Command::AddTimeline { entry },
            )
        }
        "timeline.list" => {
            let req: ProjectRef = parse(arguments)?;
            let mut entries: Vec<_> = app
                .workspace
                .load_project(&req.project_id)?
                .timeline
                .into_values()
                .collect();
            entries.sort_by_key(|entry| (entry.order, entry.id.clone()));
            Ok(serde_json::to_value(entries)?)
        }
        "thread.add" => {
            let req: ThreadAdd = parse(arguments)?;
            let thread = StoryThread {
                id: req.thread_id,
                title: req.title,
                status: req.status.unwrap_or(ThreadStatus::Open),
                evidence: req.evidence,
            };
            commit(
                &app.workspace,
                &req.project_id,
                req.command_id,
                Command::AddThread { thread },
            )
        }
        "thread.set_status" => {
            let req: ThreadSetStatus = parse(arguments)?;
            commit(
                &app.workspace,
                &req.project_id,
                req.command_id,
                Command::SetThreadStatus {
                    thread_id: req.thread_id,
                    status: req.status,
                },
            )
        }
        "thread.list" => {
            let req: ProjectRef = parse(arguments)?;
            Ok(serde_json::to_value(
                app.workspace
                    .load_project(&req.project_id)?
                    .threads
                    .into_values()
                    .collect::<Vec<_>>(),
            )?)
        }
        _ => Err(super::unknown_tool(name)),
    }
}
