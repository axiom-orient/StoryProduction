//! Changeset creation and human-decision routes.

use super::super::super::{commit, commit_decision, decision_input, Application};
use super::require_candidate;
use crate::application::requests::review::{ChangeSetApprove, ChangeSetCreate, ChangeSetRef};
use crate::contracts::DecisionKind;
use crate::domain::Command;
use crate::error::Result;
use crate::model::{ChangeSet, ChangeSetStatus};
use serde_json::Value;

pub(super) fn call(app: &Application, name: &str, arguments: Value) -> Result<Value> {
    match name {
        "changeset.create" => {
            let req: ChangeSetCreate = super::super::super::parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            let candidate = require_candidate(&project, &req.candidate_id)?;
            let changeset = ChangeSet {
                id: req
                    .changeset_id
                    .unwrap_or_else(|| crate::application::support::generated_id("changeset")),
                candidate_id: req.candidate_id,
                source_revision: candidate.source_revision,
                operations: req.operations,
                status: ChangeSetStatus::Pending,
            };
            commit(
                &app.workspace,
                &req.project_id,
                req.command_id,
                Command::AddChangeSet { changeset },
            )
        }
        "changeset.approve" => {
            let req: ChangeSetApprove = super::super::super::parse(arguments)?;
            let claims = decision_input(
                req.actor_id,
                req.reason,
                req.expected_artifact_revision,
                req.expected_artifact_digest,
            );
            let decision = app.bind_changeset_decision(
                &req.project_id,
                &req.changeset_id,
                DecisionKind::Approve,
                &claims,
            )?;
            commit_decision(
                &app.workspace,
                &req.project_id,
                req.command_id,
                decision,
                |decision| Command::ApproveChangeSet {
                    changeset_id: req.changeset_id,
                    expected_project_revision: req.expected_project_revision,
                    run_id: req.run_id,
                    decision,
                },
            )
        }
        "changeset.reject" => {
            let req: ChangeSetRef = super::super::super::parse(arguments)?;
            let claims = decision_input(
                req.actor_id,
                req.reason,
                req.expected_artifact_revision,
                req.expected_artifact_digest,
            );
            let decision = app.bind_changeset_decision(
                &req.project_id,
                &req.changeset_id,
                DecisionKind::Reject,
                &claims,
            )?;
            commit_decision(
                &app.workspace,
                &req.project_id,
                req.command_id,
                decision,
                |decision| Command::RejectChangeSet {
                    changeset_id: req.changeset_id,
                    decision,
                },
            )
        }
        _ => Err(super::super::unknown_tool(name)),
    }
}
