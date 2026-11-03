//! Candidate creation, listing, and rejection routes.

use super::super::super::support::generated_id;
use super::super::super::{commit, commit_decision, decision_input, Application};
use super::require_candidate;
use crate::application::requests::common::ProjectRef;
use crate::application::requests::review::{
    CandidateCreate, CandidateRef, CandidateSubjectRef, ScenePrepareCandidate,
};
use crate::domain::Command;
use crate::error::{Error, Result};
use crate::model::{Candidate, CandidateStatus, Receipt};
use crate::store::now_ms;
use serde_json::Value;

pub(super) fn call(app: &Application, name: &str, arguments: Value) -> Result<Value> {
    match name {
        "candidate.create" => {
            let req: CandidateCreate = super::super::super::parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            let candidate = Candidate {
                id: req
                    .candidate_id
                    .unwrap_or_else(|| generated_id("candidate")),
                kind: req.kind,
                source_revision: req.source_revision.unwrap_or(project.content_revision),
                writing_type: req.writing_type.map(|value| value.as_str().to_string()),
                stage: req.stage.map(|value| value.as_str().to_string()),
                content: req.content,
                receipt: Receipt {
                    provider: req.provider,
                    model: req.model,
                    request_id: req.request_id,
                    input_hash: req.input_hash,
                    created_at_ms: now_ms(),
                },
                status: CandidateStatus::Pending,
            };
            commit(
                &app.workspace,
                &req.project_id,
                req.command_id,
                Command::AddCandidate { candidate },
            )
        }
        "scene.prepare_candidate" => {
            let req: ScenePrepareCandidate = super::super::super::parse(arguments)?;
            let candidate = Candidate {
                id: req.candidate_id,
                kind: req.kind,
                source_revision: req.expected_content_revision,
                writing_type: None,
                stage: None,
                content: req.content,
                receipt: Receipt {
                    provider: req.provider,
                    model: req.model,
                    request_id: req.request_id,
                    input_hash: req.input_hash,
                    created_at_ms: now_ms(),
                },
                status: CandidateStatus::Pending,
            };
            commit(
                &app.workspace,
                &req.project_id,
                req.command_id,
                Command::PrepareSceneCandidate {
                    candidate,
                    scene_id: req.scene_id,
                    expected_scene_revision: req.expected_scene_revision,
                    changeset_id: req.changeset_id,
                },
            )
        }
        "candidate.list" => {
            let req: ProjectRef = super::super::super::parse(arguments)?;
            Ok(serde_json::to_value(
                app.workspace
                    .load_project(&req.project_id)?
                    .candidates
                    .into_values()
                    .collect::<Vec<_>>(),
            )?)
        }
        "candidate.subject" => {
            let req: CandidateSubjectRef = super::super::super::parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            let candidate = require_candidate(&project, &req.candidate_id)?;
            if candidate.status != CandidateStatus::Pending {
                return Err(Error::Conflict(
                    "only pending candidates have a review subject".into(),
                ));
            }
            Ok(serde_json::to_value(crate::receipts::candidate_subject(
                candidate,
            )?)?)
        }
        "candidate.reject" => {
            let req: CandidateRef = super::super::super::parse(arguments)?;
            let claims = decision_input(
                req.actor_id,
                req.reason,
                req.expected_artifact_revision,
                req.expected_artifact_digest,
            );
            let project = app.workspace.load_project(&req.project_id)?;
            let candidate = require_candidate(&project, &req.candidate_id)?;
            let decision = crate::receipts::bind_decision(
                candidate,
                format!("project:{}/candidate:{}", req.project_id, req.candidate_id),
                crate::contracts::DecisionKind::Reject,
                &claims,
            )?;
            commit_decision(
                &app.workspace,
                &req.project_id,
                req.command_id,
                decision,
                |decision| Command::RejectCandidate {
                    candidate_id: req.candidate_id,
                    decision,
                },
            )
        }
        _ => Err(super::super::unknown_tool(name)),
    }
}
