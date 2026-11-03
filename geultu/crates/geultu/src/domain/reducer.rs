//! Deterministic Event-to-Project reduction and projection helpers.

use super::validation::validate_operations;
use super::Event;
use crate::error::{Error, Result};
use crate::model::{CandidateStatus, ChangeOperation, ChangeSetStatus, Project};
use crate::voice::validate_profile;
use std::collections::{btree_map::Entry, BTreeMap};

pub fn evolve(project: Option<Project>, event: &Event) -> Result<Project> {
    match (project, event) {
        (
            None,
            Event::ProjectCreated {
                id,
                title,
                display_label,
                summary,
            },
        ) => {
            let normalized = crate::domain::validation::normalize_project_metadata(
                title,
                display_label.as_deref(),
                summary.as_deref(),
            )
            .map_err(|error| Error::Corrupt(format!("invalid project metadata: {error}")))?;
            if normalized != (title.clone(), display_label.clone(), summary.clone()) {
                return Err(Error::Corrupt(
                    "project creation metadata is not normalized".into(),
                ));
            }
            let mut project = Project::empty(id.clone(), title.clone());
            project.display_label = display_label.clone();
            project.summary = summary.clone();
            project.revision = 1;
            Ok(project)
        }
        (None, _) => Err(Error::Corrupt(
            "journal does not start with project_created".into(),
        )),
        (Some(_), Event::ProjectCreated { .. }) => Err(Error::Corrupt(
            "project_created appears more than once".into(),
        )),
        (Some(mut project), event) => {
            let next_revision = advance_revision(project.revision, "project")?;
            let next_content_revision = if event.affects_content_revision() {
                advance_revision(project.content_revision, "content")?
            } else {
                project.content_revision
            };
            match event {
                Event::AuthoringApplied { mutation } => {
                    project.authoring = crate::authoring::apply(&project, mutation)?;
                }
                Event::ProjectMetadataSet {
                    title,
                    display_label,
                    summary,
                } => {
                    let normalized = crate::domain::validation::normalize_project_metadata(
                        title,
                        display_label.as_deref(),
                        summary.as_deref(),
                    )
                    .map_err(|error| {
                        Error::Corrupt(format!("invalid project metadata event: {error}"))
                    })?;
                    if normalized != (title.clone(), display_label.clone(), summary.clone()) {
                        return Err(Error::Corrupt(
                            "project metadata event is not normalized".into(),
                        ));
                    }
                    project.title = title.clone();
                    project.display_label = display_label.clone();
                    project.summary = summary.clone();
                }
                Event::SceneAdded { scene } => {
                    insert_new(&mut project.scenes, scene.id.clone(), scene, "scene")?;
                }
                Event::SceneTextSet { scene_id, text } => {
                    let scene = project.scenes.get_mut(scene_id).ok_or_else(|| {
                        Error::Corrupt(format!("event references missing scene '{scene_id}'"))
                    })?;
                    scene.text = text.clone();
                    scene.revision = advance_revision(scene.revision, "scene")?;
                }
                Event::ScenesImported { scenes } => {
                    for scene in scenes {
                        insert_new(&mut project.scenes, scene.id.clone(), scene, "scene")?;
                    }
                }
                Event::FactAdded { fact } => {
                    insert_new(&mut project.facts, fact.id.clone(), fact, "fact")?;
                }
                Event::TimelineAdded { entry } => {
                    insert_new(
                        &mut project.timeline,
                        entry.id.clone(),
                        entry,
                        "timeline entry",
                    )?;
                }
                Event::ThreadAdded { thread } => {
                    insert_new(&mut project.threads, thread.id.clone(), thread, "thread")?;
                }
                Event::ThreadStatusSet { thread_id, status } => {
                    let thread = project.threads.get_mut(thread_id).ok_or_else(|| {
                        Error::Corrupt(format!("event references missing thread '{thread_id}'"))
                    })?;
                    thread.status = status.clone();
                }
                Event::CandidateAdded { candidate } => {
                    insert_new(
                        &mut project.candidates,
                        candidate.id.clone(),
                        candidate,
                        "candidate",
                    )?;
                }
                Event::CandidateRejected { candidate_id } => {
                    let candidate = project.candidates.get_mut(candidate_id).ok_or_else(|| {
                        Error::Corrupt(format!(
                            "event references missing candidate '{candidate_id}'"
                        ))
                    })?;
                    candidate.status = CandidateStatus::Rejected;
                }
                Event::ChangeSetAdded { changeset } => {
                    insert_new(
                        &mut project.changesets,
                        changeset.id.clone(),
                        changeset,
                        "changeset",
                    )?;
                }
                Event::ChangeSetApproved {
                    changeset_id,
                    candidate_id,
                    operations,
                } => {
                    validate_operations(&project, operations).map_err(|error| {
                        Error::Corrupt(format!("invalid approved changeset: {error}"))
                    })?;
                    apply_operations(&mut project, operations)?;
                    let changeset = project.changesets.get_mut(changeset_id).ok_or_else(|| {
                        Error::Corrupt(format!(
                            "event references missing changeset '{changeset_id}'"
                        ))
                    })?;
                    changeset.status = ChangeSetStatus::Approved;
                    let candidate = project.candidates.get_mut(candidate_id).ok_or_else(|| {
                        Error::Corrupt(format!(
                            "event references missing candidate '{candidate_id}'"
                        ))
                    })?;
                    candidate.status = CandidateStatus::Approved;
                }
                Event::ChangeSetRejected { changeset_id } => {
                    let changeset = project.changesets.get_mut(changeset_id).ok_or_else(|| {
                        Error::Corrupt(format!(
                            "event references missing changeset '{changeset_id}'"
                        ))
                    })?;
                    changeset.status = ChangeSetStatus::Rejected;
                }
                Event::VisualPlanAdded { plan } => {
                    insert_new(
                        &mut project.visual_plans,
                        plan.id.clone(),
                        plan,
                        "visual plan",
                    )?;
                }
                Event::VoiceProfileActivated { profile } => {
                    validate_profile(profile).map_err(|error| {
                        Error::Corrupt(format!("invalid voice profile event: {error}"))
                    })?;
                    project.voice_profile = Some(profile.clone());
                }
                Event::ValidationReceiptRecorded { receipt } => {
                    receipt.validate().map_err(|error| {
                        Error::Corrupt(format!("invalid validation receipt event: {error}"))
                    })?;
                    insert_or_verify_identical(
                        &mut project.validation_receipts,
                        receipt.receipt_id.clone(),
                        receipt,
                        "validation receipt",
                    )?;
                }
                Event::HumanDecisionRecorded { decision } => {
                    decision.validate().map_err(|error| {
                        Error::Corrupt(format!("invalid human decision event: {error}"))
                    })?;
                    insert_or_verify_identical(
                        &mut project.human_decisions,
                        decision.decision_id.clone(),
                        decision,
                        "human decision",
                    )?;
                }
                Event::TextRunOpened { run } => {
                    run.validate().map_err(|error| {
                        Error::Corrupt(format!("invalid text run event: {error}"))
                    })?;
                    if project.text_runs.contains_key(&run.run_id) {
                        return Err(Error::Corrupt(format!(
                            "text run '{}' opened more than once",
                            run.run_id
                        )));
                    }
                    project.text_runs.insert(run.run_id.clone(), run.clone());
                }
                Event::TextRunBudgetSpent {
                    run_id,
                    revision_units,
                } => {
                    let run = project.text_runs.get_mut(run_id).ok_or_else(|| {
                        Error::Corrupt(format!("event references missing text run '{run_id}'"))
                    })?;
                    if run.project_id != project.id {
                        return Err(Error::Corrupt(format!(
                            "text run '{run_id}' belongs to a different project"
                        )));
                    }
                    run.budgets.revision_remaining =
                        crate::admission::budget::revision_remaining_after_spend(
                            run,
                            *revision_units,
                        )
                        .map_err(|error| {
                            Error::Corrupt(format!("invalid text run budget spend: {error}"))
                        })?;
                }
                Event::VisualRunAdmitted { run } => {
                    run.validate().map_err(|error| {
                        Error::Corrupt(format!("invalid visual run event: {error}"))
                    })?;
                    if project.visual_runs.contains_key(&run.run_id) {
                        return Err(Error::Corrupt(format!(
                            "visual run '{}' admitted more than once",
                            run.run_id
                        )));
                    }
                    project.visual_runs.insert(run.run_id.clone(), run.clone());
                }
                Event::ProjectCreated { .. } => unreachable!(),
            }
            project.revision = next_revision;
            project.content_revision = next_content_revision;
            Ok(project)
        }
    }
}

fn advance_revision(revision: u64, label: &str) -> Result<u64> {
    revision
        .checked_add(1)
        .ok_or_else(|| Error::Corrupt(format!("{label} revision exceeds u64 capacity")))
}

fn insert_new<T: Clone>(
    map: &mut BTreeMap<String, T>,
    key: String,
    value: &T,
    label: &str,
) -> Result<()> {
    match map.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(value.clone());
            Ok(())
        }
        Entry::Occupied(entry) => Err(Error::Corrupt(format!(
            "{label} '{}' is already recorded",
            entry.key()
        ))),
    }
}

fn insert_or_verify_identical<T: PartialEq + Clone>(
    map: &mut BTreeMap<String, T>,
    key: String,
    value: &T,
    label: &str,
) -> Result<()> {
    match map.get(&key) {
        Some(existing) if existing == value => Ok(()),
        Some(_) => Err(Error::Corrupt(format!(
            "{label} '{key}' is already recorded with different content"
        ))),
        None => {
            map.insert(key, value.clone());
            Ok(())
        }
    }
}

fn apply_operations(project: &mut Project, operations: &[ChangeOperation]) -> Result<()> {
    for operation in operations {
        match operation {
            ChangeOperation::ReplaceSceneText { scene_id, text, .. } => {
                let scene = project.scenes.get_mut(scene_id).ok_or_else(|| {
                    Error::Corrupt(format!(
                        "approved operation references missing scene '{scene_id}'"
                    ))
                })?;
                scene.text = text.clone();
                scene.revision = advance_revision(scene.revision, "scene")?;
            }
            ChangeOperation::AddFact { fact } => {
                insert_new(&mut project.facts, fact.id.clone(), fact, "fact")?;
            }
            ChangeOperation::AddTimeline { entry } => {
                insert_new(
                    &mut project.timeline,
                    entry.id.clone(),
                    entry,
                    "timeline entry",
                )?;
            }
            ChangeOperation::AddThread { thread } => {
                insert_new(&mut project.threads, thread.id.clone(), thread, "thread")?;
            }
        }
    }
    Ok(())
}
