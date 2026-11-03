//! Pure candidate and changeset decisions with human-decision binding.

use crate::admission::budget::{admit_revision_spend, REVISION_UNITS_PER_TEXT_CHANGE};

use super::super::evolve;
use super::super::validation::{
    require_non_empty, validate_id, validate_operations, validate_text,
};
use super::super::Event;
use crate::contracts::{DecisionKind, HumanDecision};
use crate::error::{Error, Result};
use crate::model::{
    Candidate, CandidateStatus, ChangeOperation, ChangeSet, ChangeSetStatus, Project,
};

pub(super) fn add_candidate(project: &Project, candidate: Candidate) -> Result<Vec<Event>> {
    validate_id("candidate id", &candidate.id)?;
    require_non_empty("candidate kind", &candidate.kind)?;
    if let Some(writing_type) = &candidate.writing_type {
        validate_id("candidate writing type", writing_type)?;
    }
    if let Some(stage) = &candidate.stage {
        validate_id("candidate writing stage", stage)?;
    }
    validate_text(&candidate.content)?;
    if candidate.source_revision > project.content_revision {
        return Err(Error::InvalidInput(
            "candidate source content revision is newer than the project".into(),
        ));
    }
    if project.candidates.contains_key(&candidate.id) {
        return Err(Error::Conflict(format!(
            "candidate '{}' already exists",
            candidate.id
        )));
    }
    Ok(vec![Event::CandidateAdded { candidate }])
}

pub(super) fn prepare_scene_candidate(
    project: &Project,
    candidate: Candidate,
    scene_id: String,
    expected_scene_revision: u64,
    changeset_id: String,
) -> Result<Vec<Event>> {
    if candidate.source_revision != project.content_revision {
        return Err(Error::Conflict(format!(
            "candidate source content revision mismatch: expected {}, actual {}",
            project.content_revision, candidate.source_revision
        )));
    }
    let candidate_events = add_candidate(project, candidate.clone())?;
    let staged = evolve(Some(project.clone()), &candidate_events[0])?;
    let changeset = ChangeSet {
        id: changeset_id,
        candidate_id: candidate.id,
        source_revision: candidate.source_revision,
        operations: vec![ChangeOperation::ReplaceSceneText {
            scene_id,
            expected_scene_revision,
            text: candidate.content,
        }],
        status: ChangeSetStatus::Pending,
    };
    let changeset_events = add_changeset(&staged, changeset)?;
    Ok(candidate_events
        .into_iter()
        .chain(changeset_events)
        .collect())
}

pub(super) fn reject_candidate(
    project: &Project,
    candidate_id: String,
    decision: HumanDecision,
) -> Result<Vec<Event>> {
    let candidate = project
        .candidates
        .get(&candidate_id)
        .ok_or_else(|| Error::NotFound(format!("candidate '{candidate_id}' was not found")))?;
    if candidate.status != CandidateStatus::Pending {
        return Err(Error::Conflict(
            "only pending candidates can be rejected".into(),
        ));
    }
    verify_decision_binding(
        candidate,
        &decision,
        DecisionKind::Reject,
        &format!("project:{}/candidate:{}", project.id, candidate_id),
    )?;
    Ok(vec![
        Event::CandidateRejected { candidate_id },
        Event::HumanDecisionRecorded { decision },
    ])
}

pub(super) fn add_changeset(project: &Project, changeset: ChangeSet) -> Result<Vec<Event>> {
    validate_id("changeset id", &changeset.id)?;
    if changeset.operations.is_empty() {
        return Err(Error::InvalidInput(
            "changeset must contain at least one operation".into(),
        ));
    }
    if project.changesets.contains_key(&changeset.id) {
        return Err(Error::Conflict(format!(
            "changeset '{}' already exists",
            changeset.id
        )));
    }
    let candidate = project
        .candidates
        .get(&changeset.candidate_id)
        .ok_or_else(|| {
            Error::NotFound(format!(
                "candidate '{}' was not found",
                changeset.candidate_id
            ))
        })?;
    if candidate.status != CandidateStatus::Pending {
        return Err(Error::Conflict(
            "changesets require a pending candidate".into(),
        ));
    }
    if candidate.source_revision != changeset.source_revision {
        return Err(Error::InvalidInput(
            "candidate and changeset source revisions differ".into(),
        ));
    }
    validate_operations(project, &changeset.operations)?;
    Ok(vec![Event::ChangeSetAdded { changeset }])
}

pub(super) fn approve_changeset(
    project: &Project,
    changeset_id: String,
    expected_project_revision: u64,
    run_id: Option<String>,
    decision: HumanDecision,
) -> Result<Vec<Event>> {
    if project.revision != expected_project_revision {
        return Err(Error::Conflict(format!(
            "project revision mismatch: expected {expected_project_revision}, actual {}",
            project.revision
        )));
    }
    let changeset = project
        .changesets
        .get(&changeset_id)
        .ok_or_else(|| Error::NotFound(format!("changeset '{changeset_id}' was not found")))?;
    if changeset.status != ChangeSetStatus::Pending {
        return Err(Error::Conflict(
            "only pending changesets can be approved".into(),
        ));
    }
    let candidate = project
        .candidates
        .get(&changeset.candidate_id)
        .ok_or_else(|| Error::Corrupt("changeset references a missing candidate".into()))?;
    if candidate.status != CandidateStatus::Pending {
        return Err(Error::Conflict("changeset candidate is not pending".into()));
    }
    if candidate.source_revision != project.content_revision {
        return Err(Error::Conflict(format!(
            "candidate source content revision mismatch: expected {}, actual {}",
            project.content_revision, candidate.source_revision
        )));
    }
    if changeset.source_revision != project.content_revision {
        return Err(Error::Conflict(format!(
            "changeset source content revision mismatch: expected {}, actual {}",
            project.content_revision, changeset.source_revision
        )));
    }
    validate_operations(project, &changeset.operations)?;
    verify_decision_binding(
        candidate,
        &decision,
        DecisionKind::Approve,
        &format!("project:{}/changeset:{}", project.id, changeset_id),
    )?;
    let mut events = vec![Event::ChangeSetApproved {
        changeset_id,
        candidate_id: changeset.candidate_id.clone(),
        operations: changeset.operations.clone(),
    }];
    if let Some(run_id) = run_id {
        admit_revision_spend(project, &run_id, REVISION_UNITS_PER_TEXT_CHANGE)?;
        events.push(Event::TextRunBudgetSpent {
            run_id,
            revision_units: REVISION_UNITS_PER_TEXT_CHANGE,
        });
    }
    events.push(Event::HumanDecisionRecorded { decision });
    Ok(events)
}

pub(super) fn reject_changeset(
    project: &Project,
    changeset_id: String,
    decision: HumanDecision,
) -> Result<Vec<Event>> {
    let changeset = project
        .changesets
        .get(&changeset_id)
        .ok_or_else(|| Error::NotFound(format!("changeset '{changeset_id}' was not found")))?;
    if changeset.status != ChangeSetStatus::Pending {
        return Err(Error::Conflict(
            "only pending changesets can be rejected".into(),
        ));
    }
    let candidate = project
        .candidates
        .get(&changeset.candidate_id)
        .ok_or_else(|| Error::Corrupt("changeset references a missing candidate".into()))?;
    verify_decision_binding(
        candidate,
        &decision,
        DecisionKind::Reject,
        &format!("project:{}/changeset:{}", project.id, changeset_id),
    )?;
    Ok(vec![
        Event::ChangeSetRejected { changeset_id },
        Event::HumanDecisionRecorded { decision },
    ])
}

/// Verifies that a caller-built decision is bound to the exact engine-computed
/// candidate artifact and matches the command semantics. Mismatches are
/// corrupt-state errors: the application layer must have produced the decision
/// through [`crate::receipts::bind_decision`].
fn verify_decision_binding(
    candidate: &Candidate,
    decision: &HumanDecision,
    expected_kind: DecisionKind,
    scope: &str,
) -> Result<()> {
    let subject = crate::receipts::candidate_subject(candidate)?;
    if decision.subject != subject {
        return Err(Error::Corrupt(
            "decision subject does not match the engine-computed artifact reference".into(),
        ));
    }
    if decision.decision != expected_kind {
        return Err(Error::Corrupt(
            "decision kind does not match the command semantics".into(),
        ));
    }
    if decision.scope != scope {
        return Err(Error::Corrupt(format!(
            "decision scope '{}' does not match '{scope}'",
            decision.scope
        )));
    }
    decision.validate()
}
