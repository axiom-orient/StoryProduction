//! Contract checks for Human gates against already-produced Agent outputs.

use super::super::super::path_parts;
use crate::model::workflow::Node;
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeSet;

pub(super) fn validate(
    node: &Node,
    available_paths: &BTreeSet<&str>,
    agent_ids: &BTreeSet<&str>,
    revision_steps: &BTreeSet<u8>,
) -> Result<()> {
    if node.role.is_some()
        || node.phase.is_some()
        || node.system_prompt.is_some()
        || node.task_prompt.is_some()
        || !node.principles.is_empty()
        || !node.inputs.is_empty()
        || node.output.is_some()
        || !node.execution_notes.is_empty()
        || !node.summary_only_inputs.is_empty()
        || !node.validation.required_markers.is_empty()
        || !node.validation.forbidden_markers.is_empty()
    {
        return Err(Error::Contract(format!(
            "{}: Human node contains Agent-only contract fields",
            node.id
        )));
    }
    let gate_type = node
        .gate_type
        .as_deref()
        .ok_or_else(|| Error::Contract(format!("{}: gate_type missing", node.id)))?;
    if !matches!(gate_type, "artifact_review" | "checkpoint") {
        return Err(Error::Contract(format!(
            "{}: unknown gate_type {gate_type}",
            node.id
        )));
    }
    if node
        .label
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        return Err(Error::Contract(format!("{}: Human label missing", node.id)));
    }
    let source = node
        .source
        .as_deref()
        .ok_or_else(|| Error::Contract(format!("{}: Human source missing", node.id)))?;
    path_parts(source)?;
    if !available_paths.contains(source) {
        return Err(Error::Contract(format!(
            "{}: Human source not produced earlier: {source}",
            node.id
        )));
    }
    if node.max_revisions == Some(0) {
        return Err(Error::Contract(format!(
            "{}: max_revisions must be > 0",
            node.id
        )));
    }
    if let Some(revision_node) = &node.revision_node {
        if !agent_ids.contains(revision_node.as_str()) {
            return Err(Error::Contract(format!(
                "{}: revision_node must reference an earlier Agent node: {revision_node}",
                node.id
            )));
        }
    }
    let mut targets = BTreeSet::new();
    for target in &node.revision_targets {
        if !targets.insert(target) || !revision_steps.contains(target) {
            return Err(Error::Contract(format!(
                "{}: revision target STEP {target} is duplicate or has no earlier directly revisable Agent node",
                node.id
            )));
        }
    }
    if (gate_type == "checkpoint" && node.revision_node.is_some())
        || (gate_type == "artifact_review"
            && (node.checkpoint.is_some() || !node.revision_targets.is_empty()))
    {
        return Err(Error::Contract(format!(
            "{}: mixed Human gate contract fields",
            node.id
        )));
    }
    match gate_type {
        "checkpoint"
            if node.checkpoint.is_none()
                || node.revision_targets.is_empty()
                || node.max_revisions.is_none() =>
        {
            return Err(Error::Contract(format!(
                "{}: checkpoint requires checkpoint, revision_targets, and max_revisions",
                node.id
            )));
        }
        "artifact_review" if node.revision_node.is_none() || node.max_revisions.is_none() => {
            return Err(Error::Contract(format!(
                "{}: artifact_review requires revision_node and max_revisions",
                node.id
            )));
        }
        _ => {}
    }
    Ok(())
}
