//! Pure decisions for canonical scene, fact, timeline, and thread commands.

use crate::admission::budget::{admit_revision_spend, REVISION_UNITS_PER_TEXT_CHANGE};

use super::super::validation::{
    require_non_empty, validate_fact, validate_id, validate_text, validate_thread,
    validate_timeline_entry,
};
use super::super::Event;
use crate::error::{Error, Result};
use crate::model::{Fact, Project, Scene, StoryThread, ThreadStatus, TimelineEntry};
use std::collections::BTreeSet;

pub(super) fn add_scene(project: &Project, scene: Scene) -> Result<Vec<Event>> {
    validate_id("scene id", &scene.id)?;
    require_non_empty("scene title", &scene.title)?;
    validate_text(&scene.text)?;
    if project.scenes.contains_key(&scene.id) {
        return Err(Error::Conflict(format!(
            "scene '{}' already exists",
            scene.id
        )));
    }
    Ok(vec![Event::SceneAdded { scene }])
}

pub(super) fn set_scene_text(
    project: &Project,
    scene_id: String,
    text: String,
    expected_scene_revision: Option<u64>,
    run_id: Option<String>,
) -> Result<Vec<Event>> {
    validate_text(&text)?;
    let scene = project
        .scenes
        .get(&scene_id)
        .ok_or_else(|| Error::NotFound(format!("scene '{scene_id}' was not found")))?;
    if let Some(expected) = expected_scene_revision {
        if scene.revision != expected {
            return Err(Error::Conflict(format!(
                "scene revision mismatch: expected {expected}, actual {}",
                scene.revision
            )));
        }
    }
    if scene.text == text {
        return Ok(Vec::new());
    }
    let mut events = vec![Event::SceneTextSet { scene_id, text }];
    if let Some(run_id) = run_id {
        admit_revision_spend(project, &run_id, REVISION_UNITS_PER_TEXT_CHANGE)?;
        events.push(Event::TextRunBudgetSpent {
            run_id,
            revision_units: REVISION_UNITS_PER_TEXT_CHANGE,
        });
    }
    Ok(events)
}

pub(super) fn import_scenes(project: &Project, scenes: Vec<Scene>) -> Result<Vec<Event>> {
    if scenes.is_empty() {
        return Err(Error::InvalidInput(
            "markdown did not contain importable text".into(),
        ));
    }
    let mut scene_ids = BTreeSet::new();
    for scene in &scenes {
        validate_id("scene id", &scene.id)?;
        if !scene_ids.insert(scene.id.as_str()) {
            return Err(Error::Conflict(format!(
                "scene '{}' appears more than once in import",
                scene.id
            )));
        }
        require_non_empty("scene title", &scene.title)?;
        validate_text(&scene.text)?;
        if project.scenes.contains_key(&scene.id) {
            return Err(Error::Conflict(format!(
                "scene '{}' already exists",
                scene.id
            )));
        }
    }
    Ok(vec![Event::ScenesImported { scenes }])
}

pub(super) fn add_fact(project: &Project, fact: Fact) -> Result<Vec<Event>> {
    validate_fact(project, &fact)?;
    Ok(vec![Event::FactAdded { fact }])
}

pub(super) fn add_timeline(project: &Project, entry: TimelineEntry) -> Result<Vec<Event>> {
    validate_timeline_entry(project, &entry)?;
    Ok(vec![Event::TimelineAdded { entry }])
}

pub(super) fn add_thread(project: &Project, thread: StoryThread) -> Result<Vec<Event>> {
    validate_thread(project, &thread)?;
    Ok(vec![Event::ThreadAdded { thread }])
}

pub(super) fn set_thread_status(
    project: &Project,
    thread_id: String,
    status: ThreadStatus,
) -> Result<Vec<Event>> {
    let current = project
        .threads
        .get(&thread_id)
        .ok_or_else(|| Error::NotFound(format!("thread '{thread_id}' was not found")))?;
    if current.status == status {
        return Ok(Vec::new());
    }
    Ok(vec![Event::ThreadStatusSet { thread_id, status }])
}
