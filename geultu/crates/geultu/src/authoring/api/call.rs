use crate::application::support::parse;
use crate::{Application, Error, Result};
use serde_json::{json, Value};

use super::super::{Action, Kind, Mutation};
use super::requests::{DocumentRef, Handoff, History, ItemRef, ProjectRef, Restore};

pub fn call(app: &Application, name: &str, arguments: Value) -> Result<Value> {
    match name {
        "authoring.history" => {
            let req: History = parse(arguments)?;
            app.workspace()
                .authoring_history(&req.project_id, req.kind, req.revision)
        }
        "authoring.restore" => {
            let req: Restore = parse(arguments)?;
            let history =
                app.workspace()
                    .authoring_history(&req.project_id, req.kind, Some(req.revision))?;
            let document = parse(history["document"].clone())?;
            let mutation = Mutation {
                command_id: req.command_id.clone(),
                action: Action::Restore {
                    document,
                    expected_revision: req.expected_revision,
                    actor: req.actor,
                },
            };
            let commit = app.workspace().transact(
                &req.project_id,
                &req.command_id,
                crate::domain::Command::Authoring { mutation },
            )?;
            Ok(
                json!({"commit":commit,"result":super::super::status(&app.workspace().load_project(&req.project_id)?)}),
            )
        }
        "authoring.bundle" => {
            let req: ProjectRef = parse(arguments)?;
            let project = app.workspace().load_project(&req.project_id)?;
            Ok(json!({"project_revision":project.revision,"documents":{
                "source":super::super::export(&project,Kind::Source)?,
                "story":super::super::export(&project,Kind::Story)?,
                "synopsis":super::super::export(&project,Kind::Synopsis)?
            }}))
        }
        "authoring.status" => {
            let req: ProjectRef = parse(arguments)?;
            Ok(super::super::status(
                &app.workspace().load_project(&req.project_id)?,
            ))
        }
        "authoring.document" | "authoring.export" => {
            let req: DocumentRef = parse(arguments)?;
            let project = app.workspace().load_project(&req.project_id)?;
            if name == "authoring.export" {
                return super::super::export(&project, req.kind);
            }
            let doc = project
                .authoring
                .documents
                .get(&req.kind)
                .ok_or_else(|| Error::NotFound("document not found".into()))?;
            Ok(
                json!({"document":doc,"markdown":doc.markdown(),"stale_reasons":super::super::stale_reasons(&project,doc)}),
            )
        }
        "authoring.task" | "authoring.candidate" => {
            let req: ItemRef = parse(arguments)?;
            let project = app.workspace().load_project(&req.project_id)?;
            if name == "authoring.task" {
                super::super::packet(&project, &req.id)
            } else {
                super::super::candidate_view(&project, &req.id)
            }
        }
        "authoring.handoff" => {
            let req: Handoff = parse(arguments)?;
            app.workspace().export_authoring_synopsis(
                &req.project_id,
                &req.actor,
                &req.language,
                req.expected_revision,
                &req.expected_markdown_sha256,
            )
        }
        _ => mutate(app, name, arguments),
    }
}

enum ResultView {
    Status,
    Task(String),
    Candidate(String),
}

fn mutate(app: &Application, name: &str, mut arguments: Value) -> Result<Value> {
    let action_name = match name {
        "authoring.start" => "start",
        "authoring.brief" => "set_brief",
        "authoring.prepare" => "prepare",
        "authoring.submit" => "submit",
        "authoring.edit" => "edit",
        "authoring.review" => "review",
        "authoring.cancel" => "cancel",
        _ => {
            return Err(Error::InvalidInput(format!(
                "unknown authoring operation: {name}"
            )))
        }
    };
    let object = arguments
        .as_object_mut()
        .ok_or_else(|| Error::InvalidInput("arguments must be an object".into()))?;
    let project_id: String = parse(
        object
            .remove("project_id")
            .ok_or_else(|| Error::InvalidInput("project_id is required".into()))?,
    )?;
    let command_id: String = parse(object.remove("command_id").ok_or_else(|| {
        Error::InvalidInput("command_id is required for durable authoring operations".into())
    })?)?;
    let expected_project_revision = if action_name == "prepare" {
        object
            .remove("expected_project_revision")
            .map(parse::<u64>)
            .transpose()?
    } else {
        None
    };
    if object.contains_key("action") {
        return Err(Error::InvalidInput(
            "action is determined by the public operation".into(),
        ));
    }
    object.insert("action".into(), json!(action_name));
    let action: Action = parse(arguments)?;
    let view = match &action {
        Action::Prepare { task_id, .. } => ResultView::Task(task_id.clone()),
        Action::Submit { candidate_id, .. } | Action::Review { candidate_id, .. } => {
            ResultView::Candidate(candidate_id.clone())
        }
        _ => ResultView::Status,
    };
    let mutation = Mutation {
        command_id: command_id.clone(),
        action,
    };
    let command = crate::domain::Command::Authoring { mutation };
    let commit = match expected_project_revision {
        Some(expected) => app.workspace().transact_if_project_revision(
            &project_id,
            expected,
            &command_id,
            command,
        )?,
        None => app
            .workspace()
            .transact(&project_id, &command_id, command)?,
    };
    let project = app.workspace().load_project(&project_id)?;
    let result = match view {
        ResultView::Task(task_id) => super::super::packet(&project, &task_id)?,
        ResultView::Candidate(candidate_id) => {
            super::super::candidate_view(&project, &candidate_id)?
        }
        ResultView::Status => super::super::status(&project),
    };
    Ok(json!({"commit":commit,"result":result}))
}
