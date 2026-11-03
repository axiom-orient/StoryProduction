//! Fail-closed conformance checks for snapshot-bound Project state.

use crate::error::{Error, Result};
use crate::model::Project;

/// Enforces the project-state contract over a snapshot-bound project state.
///
/// Predicates mirror `project-state.schema.json` where the engine owns a
/// truthful counterpart: identity length, schema identity const, revision
/// minimum and map identity. Engine-owned invariants additionally bind revision to
/// event count, run project ownership, source lineage, and embedded contract shapes.
/// This is an internal Project admission gate, not a full ProjectState wire document.
///
/// # Errors
/// Returns [`Error::Corrupt`] naming the first violated predicate; callers
/// must never repair or ignore it.
pub(crate) fn ensure_snapshot_conformance(project: &Project, event_count: u64) -> Result<()> {
    crate::authoring::ensure_conformance(&project.authoring)?;
    if event_count < 1 {
        return Err(corrupt("event_count must be at least 1".into()));
    }
    if project.id.is_empty() || project.id.chars().take(257).count() > 256 {
        return Err(corrupt("project_id must contain 1..=256 characters".into()));
    }
    if project.schema_version != crate::model::SCHEMA_VERSION {
        return Err(corrupt(format!(
            "schema identity {} does not match {}",
            project.schema_version,
            crate::model::SCHEMA_VERSION
        )));
    }
    if project.revision < 1 {
        return Err(corrupt("revision must be an integer of at least 1".into()));
    }
    if project.revision != event_count {
        return Err(corrupt(format!(
            "revision {} does not match event_count {event_count}",
            project.revision
        )));
    }
    if project.content_revision > project.revision {
        return Err(corrupt(format!(
            "content_revision {} exceeds revision {}",
            project.content_revision, project.revision
        )));
    }
    ensure_key_identity(
        "scenes",
        project.scenes.iter().map(|(key, scene)| (key, &scene.id)),
    )?;
    ensure_key_identity(
        "facts",
        project.facts.iter().map(|(key, fact)| (key, &fact.id)),
    )?;
    ensure_key_identity(
        "timeline",
        project.timeline.iter().map(|(key, entry)| (key, &entry.id)),
    )?;
    ensure_key_identity(
        "threads",
        project
            .threads
            .iter()
            .map(|(key, thread)| (key, &thread.id)),
    )?;
    ensure_key_identity(
        "candidates",
        project
            .candidates
            .iter()
            .map(|(key, candidate)| (key, &candidate.id)),
    )?;
    ensure_key_identity(
        "changesets",
        project
            .changesets
            .iter()
            .map(|(key, changeset)| (key, &changeset.id)),
    )?;
    ensure_key_identity(
        "visual_plans",
        project
            .visual_plans
            .iter()
            .map(|(key, plan)| (key, &plan.id)),
    )?;
    for (key, receipt) in &project.validation_receipts {
        ensure_entity_identity("validation_receipts", key, &receipt.receipt_id)?;
        receipt
            .validate()
            .map_err(|error| corrupt(format!("embedded validation receipt rejected: {error}")))?;
    }
    for (key, decision) in &project.human_decisions {
        ensure_entity_identity("human_decisions", key, &decision.decision_id)?;
        decision
            .validate()
            .map_err(|error| corrupt(format!("embedded human decision rejected: {error}")))?;
    }
    for (key, run) in &project.text_runs {
        ensure_entity_identity("text_runs", key, &run.run_id)?;
        ensure_run_owner(&project.id, &run.project_id, key)?;
        run.validate()
            .map_err(|error| corrupt(format!("embedded text run rejected: {error}")))?;
    }
    for (key, run) in &project.visual_runs {
        ensure_entity_identity("visual_runs", key, &run.run_id)?;
        ensure_run_owner(&project.id, &run.project_id, key)?;
        run.validate()
            .map_err(|error| corrupt(format!("embedded visual run rejected: {error}")))?;
    }
    for (command_id, revision) in &project.applied_commands {
        if *revision == 0 || *revision > project.revision {
            return Err(corrupt(format!(
                "applied command '{command_id}' has invalid revision {revision}"
            )));
        }
    }
    for candidate in project.candidates.values() {
        if candidate.source_revision > project.content_revision {
            return Err(corrupt(format!(
                "candidate '{}' references a future content revision",
                candidate.id
            )));
        }
    }
    for changeset in project.changesets.values() {
        let candidate = project
            .candidates
            .get(&changeset.candidate_id)
            .ok_or_else(|| {
                corrupt(format!(
                    "changeset '{}' references a missing candidate",
                    changeset.id
                ))
            })?;
        if changeset.source_revision != candidate.source_revision {
            return Err(corrupt(format!(
                "changeset '{}' and its candidate have different source revisions",
                changeset.id
            )));
        }
    }
    for plan in project.visual_plans.values() {
        if !project.scenes.contains_key(&plan.scene_id)
            || plan.source_revision > project.content_revision
        {
            return Err(corrupt(format!(
                "visual plan '{}' has a missing scene or future source revision",
                plan.id
            )));
        }
    }
    if let Some(profile) = &project.voice_profile {
        crate::voice::validate_profile(profile)
            .map_err(|error| corrupt(format!("embedded voice profile rejected: {error}")))?;
    }
    Ok(())
}

fn ensure_key_identity<'a>(
    label: &str,
    pairs: impl Iterator<Item = (&'a String, &'a String)>,
) -> Result<()> {
    for (key, id) in pairs {
        ensure_entity_identity(label, key, id)?;
    }
    Ok(())
}

fn ensure_entity_identity(label: &str, key: &str, id: &str) -> Result<()> {
    if key != id {
        return Err(corrupt(format!(
            "{label} entry keyed '{key}' carries mismatched entity id '{id}'"
        )));
    }
    Ok(())
}

fn ensure_run_owner(project_id: &str, run_project_id: &str, run_id: &str) -> Result<()> {
    if run_project_id != project_id {
        return Err(corrupt(format!(
            "run '{run_id}' belongs to project '{run_project_id}', expected '{project_id}'"
        )));
    }
    Ok(())
}

fn corrupt(message: String) -> Error {
    Error::Corrupt(format!(
        "snapshot violates project-state contract: {message}"
    ))
}
