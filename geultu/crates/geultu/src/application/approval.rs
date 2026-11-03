//! Human-decision claim validation and changeset subject binding.

use super::support::commit;
use super::Application;
use crate::contracts::{DecisionKind, HumanDecision};
use crate::domain::Command;
use crate::error::{Error, Result};
use crate::model::DecisionInput;
use crate::store::Workspace;
use serde_json::Value;

impl Application {
    /// Resolves a changeset's candidate and binds the required human decision
    /// claims to that exact artifact before the domain command is created.
    pub(in crate::application) fn bind_changeset_decision(
        &self,
        project_id: &str,
        changeset_id: &str,
        kind: DecisionKind,
        claims: &DecisionInput,
    ) -> Result<crate::contracts::HumanDecision> {
        let project = self.workspace.load_project(project_id)?;
        let changeset = project
            .changesets
            .get(changeset_id)
            .ok_or_else(|| Error::NotFound(format!("changeset '{changeset_id}' was not found")))?;
        let candidate = project
            .candidates
            .get(&changeset.candidate_id)
            .ok_or_else(|| Error::Corrupt("changeset references a missing candidate".into()))?;
        crate::receipts::bind_decision(
            candidate,
            format!("project:{project_id}/changeset:{changeset_id}"),
            kind,
            claims,
        )
    }
}

/// Converts the required public decision claims into the owner-level input.
pub(super) fn decision_input(
    actor_id: String,
    reason: Option<String>,
    expected_artifact_revision: u64,
    expected_artifact_digest: String,
) -> DecisionInput {
    DecisionInput {
        actor_id,
        reason,
        expected_artifact_revision,
        expected_artifact_digest,
    }
}

/// Serialize the response decision before dispatch, then commit the same owned decision.
pub(super) fn commit_decision(
    workspace: &Workspace,
    project_id: &str,
    command_id: Option<String>,
    decision: HumanDecision,
    make_command: impl FnOnce(HumanDecision) -> Command,
) -> Result<Value> {
    let response_decision = serde_json::to_value(&decision)?;
    let mut result = commit(workspace, project_id, command_id, make_command(decision))?;
    result
        .as_object_mut()
        .ok_or_else(|| Error::Corrupt("commit result is not an object".into()))?
        .insert("decision".into(), response_decision);
    Ok(result)
}
