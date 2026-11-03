//! Project-level deterministic style, continuity, and staleness findings.

use super::finding;
use super::text::{
    normalize, normalize_sentence, split_paragraphs, split_sentences, style_metrics,
};
use crate::model::{Project, ReviewReport, Severity, ThreadStatus};
use std::collections::{BTreeMap, BTreeSet};

pub fn review_project(project: &Project) -> ReviewReport {
    let combined = project
        .scenes
        .values()
        .map(|scene| scene.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let metrics = style_metrics(&combined);
    let mut findings = Vec::new();
    if project.scenes.is_empty() {
        findings.push(finding(
            "project.no_scenes",
            Severity::Error,
            "Project has no scenes",
            None,
            None,
        ));
    }
    let mut titles: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    let mut sentences: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for scene in project.scenes.values() {
        titles
            .entry(scene.title.trim().to_lowercase())
            .or_default()
            .push(scene.id.as_str());
        if scene.text.trim().is_empty() {
            findings.push(finding(
                "scene.empty",
                Severity::Warning,
                "Scene text is empty",
                Some(scene.id.clone()),
                None,
            ));
        }
        for paragraph in split_paragraphs(&scene.text) {
            if paragraph.chars().count() > 1_200 {
                findings.push(finding(
                    "style.long_paragraph",
                    Severity::Warning,
                    "Paragraph exceeds 1,200 characters",
                    Some(scene.id.clone()),
                    Some(paragraph.chars().take(120).collect()),
                ));
            }
        }
        for sentence in split_sentences(&scene.text) {
            let normalized = normalize_sentence(sentence);
            if normalized.chars().count() >= 12 {
                sentences
                    .entry(normalized)
                    .or_default()
                    .push(scene.id.as_str());
            }
        }
    }
    for (title, scene_ids) in titles {
        if !title.is_empty() && scene_ids.len() > 1 {
            findings.push(finding(
                "scene.duplicate_title",
                Severity::Warning,
                format!("Duplicate scene title across {} scenes", scene_ids.len()),
                scene_ids.first().map(|id| (*id).to_owned()),
                Some(scene_ids.join(", ")),
            ));
        }
    }
    for (sentence, scene_ids) in sentences {
        if scene_ids.len() > 1 {
            findings.push(finding(
                "style.repeated_sentence",
                Severity::Warning,
                "The same sentence appears more than once",
                scene_ids.first().map(|id| (*id).to_owned()),
                Some(sentence),
            ));
        }
    }

    let mut facts: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for fact in project.facts.values() {
        facts
            .entry((normalize(&fact.subject), normalize(&fact.predicate)))
            .or_default()
            .insert(normalize(&fact.object));
        if fact.evidence.is_empty() {
            findings.push(finding(
                "fact.no_evidence",
                Severity::Info,
                format!("Fact '{}' has no scene evidence", fact.id),
                None,
                None,
            ));
        }
    }
    for ((subject, predicate), objects) in facts {
        if objects.len() > 1 {
            findings.push(finding(
                "continuity.fact_conflict",
                Severity::Error,
                format!("Conflicting values for '{subject} / {predicate}'"),
                None,
                Some(objects.into_iter().collect::<Vec<_>>().join(" | ")),
            ));
        }
    }

    let mut orders: BTreeMap<i64, Vec<&str>> = BTreeMap::new();
    for entry in project.timeline.values() {
        orders
            .entry(entry.order)
            .or_default()
            .push(entry.id.as_str());
    }
    for (order, ids) in orders {
        if ids.len() > 1 {
            findings.push(finding(
                "continuity.timeline_collision",
                Severity::Warning,
                format!("Multiple timeline entries use order {order}"),
                None,
                Some(ids.join(", ")),
            ));
        }
    }

    for thread in project.threads.values() {
        if thread.status == ThreadStatus::Open && thread.evidence.is_empty() {
            findings.push(finding(
                "thread.open_without_evidence",
                Severity::Info,
                format!("Open thread '{}' has no evidence", thread.title),
                None,
                None,
            ));
        }
    }

    for candidate in project.candidates.values() {
        if candidate.status == crate::model::CandidateStatus::Pending
            && candidate.source_revision != project.content_revision
        {
            findings.push(finding(
                "candidate.stale",
                Severity::Warning,
                format!(
                    "Pending candidate '{}' was created from revision {}",
                    candidate.id, candidate.source_revision
                ),
                None,
                Some(format!(
                    "current content revision: {}",
                    project.content_revision
                )),
            ));
        }
    }
    for plan in project.visual_plans.values() {
        if plan.source_revision != project.content_revision {
            findings.push(finding(
                "visual.stale",
                Severity::Info,
                format!(
                    "Visual plan '{}' was created from revision {}",
                    plan.id, plan.source_revision
                ),
                Some(plan.scene_id.clone()),
                Some(format!(
                    "current content revision: {}",
                    project.content_revision
                )),
            ));
        }
    }

    ReviewReport {
        project_id: project.id.clone(),
        source_revision: project.revision,
        metrics,
        findings,
    }
}
