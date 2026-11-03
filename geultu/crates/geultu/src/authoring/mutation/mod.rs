//! Replayable authoring mutations applied to a private next-state value.

mod candidates;
mod documents;
mod review;
mod tasks;

use crate::domain::validation::validate_id;
use crate::model::Project;
use crate::{Error, Result};

use super::{digest, Action, Mutation, State};

pub fn apply(project: &Project, mutation: &Mutation) -> Result<State> {
    validate_id("command id", &mutation.command_id)?;
    let fingerprint = digest(mutation);
    if let Some(previous) = project.authoring.requests.get(&mutation.command_id) {
        if previous != &fingerprint {
            return Err(Error::Conflict(
                "command_id already has a different authoring payload".into(),
            ));
        }
        return Ok(project.authoring.clone());
    }

    let mut state = project.authoring.clone();
    match &mutation.action {
        Action::Start { source, actor } => documents::start(&mut state, source, actor)?,
        Action::SetBrief {
            brief,
            expected_revision,
        } => tasks::set_brief(&mut state, brief, *expected_revision)?,
        Action::Prepare {
            task_id,
            target,
            instruction,
            selection,
        } => tasks::prepare(
            project,
            &mut state,
            task_id,
            *target,
            instruction,
            selection,
        )?,
        Action::Submit {
            candidate_id,
            task_id,
            draft,
            based_on,
        } => candidates::submit(
            &mut state,
            candidate_id,
            task_id,
            draft,
            based_on.as_deref(),
        )?,
        Action::Edit {
            kind,
            blocks,
            expected_revision,
            actor,
        } => documents::edit(
            project,
            &mut state,
            *kind,
            blocks,
            *expected_revision,
            actor,
        )?,
        Action::Restore {
            document,
            expected_revision,
            actor,
        } => documents::restore(&mut state, document, *expected_revision, actor)?,
        Action::Review {
            candidate_id,
            expected_candidate_hash,
            accept,
            actor,
            note,
        } => review::review(
            project,
            &mut state,
            candidate_id,
            expected_candidate_hash,
            *accept,
            actor,
            note,
        )?,
        Action::Cancel { task_id } => tasks::cancel(&mut state, task_id)?,
    }
    state
        .requests
        .insert(mutation.command_id.clone(), fingerprint);
    Ok(state)
}
