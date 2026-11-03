//! Manifest-level ordering and cross-node dataflow validation.

mod agent;
mod human;

use super::super::Engine;
use crate::model::workflow::{Manifest, NodeKind};
use narrative_courier_runtime::paths::validate_path_segment;
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeSet;

pub(super) fn validate(engine: &Engine, manifest: &Manifest) -> Result<()> {
    if manifest.schema_version != 3
        || manifest.domain_id.trim().is_empty()
        || manifest.domain_version.trim().is_empty()
        || manifest.nodes.is_empty()
        || manifest.stages.is_empty()
    {
        return Err(Error::Contract(
            "workflow manifest schema/domain/version invalid".into(),
        ));
    }

    let mut stage_ids = BTreeSet::new();
    for stage in &manifest.stages {
        validate_path_segment(&stage.id, "stage id")?;
        if stage.id.trim().is_empty()
            || stage.label.trim().is_empty()
            || stage.description.trim().is_empty()
            || !stage_ids.insert(stage.id.as_str())
        {
            return Err(Error::Contract(format!(
                "duplicate/invalid stage: {}",
                stage.id
            )));
        }
    }

    let mut ids = BTreeSet::new();
    let mut agent_ids = BTreeSet::new();
    // Only outputs of earlier Agent nodes are inserted into this dataflow set.
    let mut available_paths = BTreeSet::new();
    let mut revision_steps = BTreeSet::new();
    for node in &manifest.nodes {
        validate_path_segment(&node.id, "node id")?;
        if !ids.insert(node.id.as_str()) {
            return Err(Error::Contract(format!(
                "duplicate/empty node id: {}",
                node.id
            )));
        }
        let stage = node
            .stage
            .as_deref()
            .ok_or_else(|| Error::Contract(format!("{}: stage missing", node.id)))?;
        if !stage_ids.contains(stage) {
            return Err(Error::Contract(format!(
                "{}: unknown stage {stage}",
                node.id
            )));
        }
        if node
            .validation
            .required_markers
            .iter()
            .chain(node.validation.forbidden_markers.iter())
            .any(|value| value.trim().is_empty())
        {
            return Err(Error::Contract(format!(
                "{}: artifact validation marker cannot be empty",
                node.id
            )));
        }

        let required: BTreeSet<_> = node
            .validation
            .required_markers
            .iter()
            .map(String::as_str)
            .collect();
        let forbidden: BTreeSet<_> = node
            .validation
            .forbidden_markers
            .iter()
            .map(String::as_str)
            .collect();
        if required.len() != node.validation.required_markers.len()
            || forbidden.len() != node.validation.forbidden_markers.len()
            || !required.is_disjoint(&forbidden)
        {
            return Err(Error::Contract(format!(
                "{}: duplicate or conflicting validation markers",
                node.id
            )));
        }
        match node.kind {
            NodeKind::Agent => {
                agent::validate(engine, node, &mut available_paths, &mut agent_ids)?;
                if let Some(step) = node.step {
                    if node.id == format!("step_{step:02}") {
                        revision_steps.insert(step);
                    }
                }
            }
            NodeKind::Human => {
                human::validate(node, &available_paths, &agent_ids, &revision_steps)?
            }
        }
    }
    for stage in &manifest.stages {
        let mut seen = BTreeSet::new();
        for output in &stage.outputs {
            if !seen.insert(output.as_str()) || !available_paths.contains(output.as_str()) {
                return Err(Error::Contract(format!(
                    "{}: duplicate or unproduced stage output {output}",
                    stage.id
                )));
            }
        }
    }
    Ok(())
}
