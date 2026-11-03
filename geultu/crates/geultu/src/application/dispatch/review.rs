//! Routes review tool names to receipt, candidate, changeset, and run owners.

mod candidates;
mod changesets;
mod receipts;
mod runs;

use super::Application;
use crate::error::{Error, Result};
use crate::model::{Candidate, Project};
use serde_json::Value;

fn require_candidate<'a>(project: &'a Project, candidate_id: &str) -> Result<&'a Candidate> {
    project
        .candidates
        .get(candidate_id)
        .ok_or_else(|| Error::NotFound(format!("candidate '{}' was not found", candidate_id)))
}

pub(in crate::application) fn call(
    app: &Application,
    name: &str,
    arguments: Value,
) -> Result<Value> {
    match name {
        "review.run" | "proof.verify" => receipts::call(app, name, arguments),
        "candidate.create"
        | "scene.prepare_candidate"
        | "candidate.list"
        | "candidate.subject"
        | "candidate.reject" => candidates::call(app, name, arguments),
        "changeset.create" | "changeset.approve" | "changeset.reject" => {
            changesets::call(app, name, arguments)
        }
        "run.open" => runs::call(app, name, arguments),
        _ => Err(super::unknown_tool(name)),
    }
}
