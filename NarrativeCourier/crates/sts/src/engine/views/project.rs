//! Read-only review, final-screenplay, and stage projections for STS UI clients.

use super::super::Engine;
use crate::model::views::{FinalView, ProjectView, ReviewView, UiStage};
use crate::model::workflow::{Manifest, Node, NodeKind, NodeStatus, ProjectStatus, State};
use narrative_courier_runtime::{Error, Result};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

impl Engine {
    pub fn project_view(&self, name: &str) -> Result<ProjectView> {
        let manifest = self.load_manifest()?;
        let project = self.project_dir(name)?;
        let state = self.load_state(&project, &manifest)?;
        let config = self.load_config(&project)?;
        let doctor = self.doctor_at(&project, &state, &manifest)?;
        if state.status == ProjectStatus::Completed && !doctor.ok {
            return Err(Error::Contract(format!(
                "completed project integrity check failed: {}",
                doctor.errors.join("; ")
            )));
        }
        let next = self.next_node(&state, &manifest);
        let review = if let Some(node) = next.filter(|n| {
            n.kind == NodeKind::Human && state.status != ProjectStatus::Paused && doctor.ok
        }) {
            Some(self.review_view(&project, &state, &manifest, node)?)
        } else {
            None
        };
        let final_view = if state.status == ProjectStatus::Completed {
            let content =
                self.read_durable_text(&project, &state, &manifest, "output/final_screenplay.md")?;
            let version = manifest
                .nodes
                .iter()
                .find(|n| n.output.as_deref() == Some("output/final_screenplay.md"))
                .and_then(|node| state.nodes.get(&node.id))
                .map(|record| record.version)
                .ok_or_else(|| {
                    Error::Contract(
                        "completed final screenplay version is missing from manifest/state".into(),
                    )
                })?;
            let revision_summary = self.read_durable_text(
                &project,
                &state,
                &manifest,
                "output/step_08_verification.md",
            )?;
            Some(FinalView {
                word_count: content.split_whitespace().count(),
                content,
                revision_summary,
                revisions: state.revision_log.len(),
                version,
            })
        } else {
            None
        };
        let mut metadata = HashMap::new();
        metadata.insert("source_format".into(), config.source_format.clone());
        metadata.insert("target_duration".into(), config.target_duration.to_string());
        metadata.insert("revisions".into(), state.revision_log.len().to_string());
        Ok(ProjectView {
            name: name.into(),
            title: config.title,
            status: state.status,
            updated_at: state.updated_at.clone(),
            next_node: next.map(|n| n.id.clone()),
            stages: self.ui_stages(&state, &manifest),
            review,
            final_view,
            doctor,
            metadata,
        })
    }

    fn review_view(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
        node: &Node,
    ) -> Result<ReviewView> {
        let gate_type = node
            .gate_type
            .as_deref()
            .ok_or_else(|| Error::Contract(format!("Human gate type missing: {}", node.id)))?;
        let source_rel = node
            .source
            .as_deref()
            .ok_or_else(|| Error::Contract(format!("Human gate source missing: {}", node.id)))?;
        let source = self.read_durable_text(project, state, manifest, source_rel)?;
        let (source_content, critique) = if gate_type == "checkpoint" {
            let mut body = String::new();
            for step in &node.revision_targets {
                let id = self.step_node_id(manifest, *step)?;
                let target = manifest
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == id)
                    .ok_or_else(|| {
                        Error::Contract(format!(
                            "Human checkpoint target node missing: STEP {step}"
                        ))
                    })?;
                let output = target.output.as_deref().ok_or_else(|| {
                    Error::Contract(format!("Human checkpoint target output missing: {id}"))
                })?;
                let content = self.read_durable_text(project, state, manifest, output)?;
                body.push_str(&format!("# STEP {step}\n\n{}\n\n", content));
            }
            (body, Some(source))
        } else {
            (source, None)
        };
        let label = node
            .label
            .clone()
            .ok_or_else(|| Error::Contract(format!("Human gate label missing: {}", node.id)))?;
        let revision_count = state
            .nodes
            .get(&node.id)
            .ok_or_else(|| Error::Contract("review gate state is missing".into()))?
            .revision_count;
        Ok(ReviewView {
            node: node.id.clone(),
            label,
            gate_type: gate_type.into(),
            source_title: if gate_type == "checkpoint" {
                "Agent Results".into()
            } else {
                "Artifact".into()
            },
            source_content,
            critique_content: critique,
            allowed: self.review_actions(state, node)?,
            revision_targets: node.revision_targets.clone(),
            revision_count,
            max_revisions: node.max_revisions,
            gate_token: self.gate_token(project, state, node)?,
        })
    }

    fn ui_stages(&self, state: &State, manifest: &Manifest) -> Vec<UiStage> {
        let next_id = self.next_node(state, manifest).map(|n| n.id.as_str());
        let mut progress = BTreeMap::<&str, (usize, usize, bool)>::new();
        for node in &manifest.nodes {
            let Some(stage_id) = node.stage.as_deref() else {
                continue;
            };
            let record = &state.nodes[&node.id];
            let stage = progress.entry(stage_id).or_default();
            stage.0 += 1;
            stage.1 += usize::from(record.status == NodeStatus::Completed);
            stage.2 |= Some(node.id.as_str()) == next_id || record.version > 0;
        }
        manifest
            .stages
            .iter()
            .map(|stage| {
                let (total, completed, active) =
                    progress.get(stage.id.as_str()).copied().unwrap_or_default();
                let status = if total > 0 && total == completed {
                    "completed"
                } else if active {
                    "in_progress"
                } else {
                    "pending"
                };
                UiStage {
                    id: stage.id.clone(),
                    label: stage.label.clone(),
                    status: status.into(),
                    description: stage.description.clone(),
                    outputs: stage.outputs.clone(),
                }
            })
            .collect()
    }
}
