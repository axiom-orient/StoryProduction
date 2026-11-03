//! Command and event invariant checks owned by the geultu domain.

use crate::error::{Error, Result};
use crate::model::{
    ChangeOperation, EvidenceRef, Fact, Project, StoryThread, TimelineEntry, MAX_TEXT_BYTES,
};
use std::collections::BTreeSet;

pub(crate) fn validate_id(label: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 64 {
        return Err(Error::InvalidInput(format!(
            "{label} must contain 1..=64 characters"
        )));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(Error::InvalidInput(format!(
            "{label} may contain only ASCII letters, digits, '.', '_' and '-'"
        )));
    }
    Ok(())
}

pub(crate) fn normalize_project_metadata(
    title: &str,
    display_label: Option<&str>,
    summary: Option<&str>,
) -> Result<(String, Option<String>, Option<String>)> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(Error::InvalidInput(
            "project title must not be empty".into(),
        ));
    }
    let display_label = normalize_optional_line("project display label", display_label, 80)?;
    let summary = normalize_optional_line("project summary", summary, 280)?;
    Ok((title, display_label, summary))
}

fn normalize_optional_line(
    label: &str,
    value: Option<&str>,
    max_characters: usize,
) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Ok(None);
    }
    if normalized.chars().count() > max_characters {
        return Err(Error::InvalidInput(format!(
            "{label} must be at most {max_characters} characters"
        )));
    }
    Ok(Some(normalized))
}

pub(super) fn validate_text(text: &str) -> Result<()> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(Error::InvalidInput(format!(
            "text exceeds {MAX_TEXT_BYTES} bytes"
        )));
    }
    Ok(())
}

pub(super) fn require_non_empty(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(Error::InvalidInput(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

pub(super) fn validate_evidence(project: &Project, evidence: &[EvidenceRef]) -> Result<()> {
    for item in evidence {
        require_scene(project, &item.scene_id)?;
    }
    Ok(())
}

pub(super) fn require_scene(project: &Project, scene_id: &str) -> Result<()> {
    if project.scenes.contains_key(scene_id) {
        Ok(())
    } else {
        Err(Error::NotFound(format!("scene '{scene_id}' was not found")))
    }
}

pub(super) fn validate_fact(project: &Project, fact: &Fact) -> Result<()> {
    validate_id("fact id", &fact.id)?;
    require_non_empty("fact subject", &fact.subject)?;
    require_non_empty("fact predicate", &fact.predicate)?;
    require_non_empty("fact object", &fact.object)?;
    if project.facts.contains_key(&fact.id) {
        return Err(Error::Conflict(format!(
            "fact '{}' already exists",
            fact.id
        )));
    }
    validate_evidence(project, &fact.evidence)
}

pub(super) fn validate_timeline_entry(project: &Project, entry: &TimelineEntry) -> Result<()> {
    validate_id("timeline id", &entry.id)?;
    require_non_empty("timeline label", &entry.label)?;
    if project.timeline.contains_key(&entry.id) {
        return Err(Error::Conflict(format!(
            "timeline entry '{}' already exists",
            entry.id
        )));
    }
    if let Some(scene_id) = &entry.scene_id {
        require_scene(project, scene_id)?;
    }
    Ok(())
}

pub(super) fn validate_thread(project: &Project, thread: &StoryThread) -> Result<()> {
    validate_id("thread id", &thread.id)?;
    require_non_empty("thread title", &thread.title)?;
    if project.threads.contains_key(&thread.id) {
        return Err(Error::Conflict(format!(
            "thread '{}' already exists",
            thread.id
        )));
    }
    validate_evidence(project, &thread.evidence)
}

pub(super) fn validate_operations(project: &Project, operations: &[ChangeOperation]) -> Result<()> {
    let mut scene_ids = BTreeSet::new();
    let mut fact_ids = BTreeSet::new();
    let mut timeline_ids = BTreeSet::new();
    let mut thread_ids = BTreeSet::new();
    for operation in operations {
        match operation {
            ChangeOperation::ReplaceSceneText {
                scene_id,
                expected_scene_revision,
                text,
            } => {
                validate_text(text)?;
                let scene = project
                    .scenes
                    .get(scene_id)
                    .ok_or_else(|| Error::NotFound(format!("scene '{scene_id}' was not found")))?;
                if scene.revision != *expected_scene_revision {
                    return Err(Error::Conflict(format!("scene '{scene_id}' revision mismatch: expected {expected_scene_revision}, actual {}", scene.revision)));
                }
                if !scene_ids.insert(scene_id.as_str()) {
                    return Err(Error::Conflict(format!(
                        "scene '{scene_id}' appears more than once in changeset"
                    )));
                }
            }
            ChangeOperation::AddFact { fact } => {
                validate_fact(project, fact)?;
                if !fact_ids.insert(fact.id.as_str()) {
                    return Err(Error::Conflict(format!(
                        "fact '{}' appears more than once in changeset",
                        fact.id
                    )));
                }
            }
            ChangeOperation::AddTimeline { entry } => {
                validate_timeline_entry(project, entry)?;
                if !timeline_ids.insert(entry.id.as_str()) {
                    return Err(Error::Conflict(format!(
                        "timeline entry '{}' appears more than once in changeset",
                        entry.id
                    )));
                }
            }
            ChangeOperation::AddThread { thread } => {
                validate_thread(project, thread)?;
                if !thread_ids.insert(thread.id.as_str()) {
                    return Err(Error::Conflict(format!(
                        "thread '{}' appears more than once in changeset",
                        thread.id
                    )));
                }
            }
        }
    }
    Ok(())
}
