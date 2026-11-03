//! Contract checks for one executable Agent node.

use super::super::super::{path_parts, Engine};
use crate::model::workflow::Node;
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeSet;

pub(super) fn validate<'a>(
    engine: &Engine,
    node: &'a Node,
    available_paths: &mut BTreeSet<&'a str>,
    agent_ids: &mut BTreeSet<&'a str>,
) -> Result<()> {
    if node.gate_type.is_some()
        || node.checkpoint.is_some()
        || node.label.is_some()
        || node.source.is_some()
        || !node.revision_targets.is_empty()
        || node.revision_node.is_some()
        || node.max_revisions.is_some()
    {
        return Err(Error::Contract(format!(
            "{}: Agent node contains Human-only contract fields",
            node.id
        )));
    }
    let role = node
        .role
        .as_deref()
        .ok_or_else(|| Error::Contract(format!("{}: role missing", node.id)))?;
    if !matches!(role, "analyst" | "writer" | "critic") {
        return Err(Error::Contract(format!("{}: unknown role {role}", node.id)));
    }
    let system = node
        .system_prompt
        .as_deref()
        .ok_or_else(|| Error::Contract(format!("{}: system_prompt missing", node.id)))?;
    let task = node
        .task_prompt
        .as_deref()
        .ok_or_else(|| Error::Contract(format!("{}: task_prompt missing", node.id)))?;
    for prompt in [system, task] {
        path_parts(prompt)?;
        if !prompt.starts_with("prompts/") {
            return Err(Error::Contract(format!(
                "{}: prompt missing: {prompt}",
                node.id
            )));
        }
        engine.resolve_asset_file(prompt)?;
    }
    if node.principles.len() > 6 {
        return Err(Error::Contract(format!(
            "{}: at most 6 routed principles",
            node.id
        )));
    }
    let mut principles = BTreeSet::new();
    for principle in &node.principles {
        if !principles.insert(principle.as_str()) {
            return Err(Error::Contract(format!(
                "{}: duplicate routed principle {principle}",
                node.id
            )));
        }
        path_parts(principle)?;
        if !principle.starts_with("principles/") {
            return Err(Error::Contract(format!(
                "{}: principle missing: {principle}",
                node.id
            )));
        }
        engine.resolve_asset_file(principle)?;
    }
    let mut input_tokens = BTreeSet::new();
    for token in &node.inputs {
        let raw = token.strip_prefix('?').unwrap_or(token);
        if raw.is_empty() || !input_tokens.insert(raw) {
            return Err(Error::Contract(format!(
                "{}: empty or duplicate input token {raw}",
                node.id
            )));
        }
        if raw.starts_with('@') {
            if raw != "@synopsis" && raw != "@references" {
                return Err(Error::Contract(format!(
                    "{}: unknown input token {raw}",
                    node.id
                )));
            }
        } else if raw != "config.yaml" {
            path_parts(raw)?;
            if (raw.starts_with("output/") || raw.starts_with("input/human_feedback/"))
                && !available_paths.contains(raw)
            {
                return Err(Error::Contract(format!(
                    "{}: future/unproduced dependency {raw}",
                    node.id
                )));
            }
            if !raw.starts_with("output/") && !raw.starts_with("input/") {
                match engine.resolve_asset_file(raw) {
                    Ok(_) => {}
                    Err(Error::Io(error))
                        if token.starts_with('?')
                            && error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
        }
    }
    let mut summaries = BTreeSet::new();
    for summary in &node.summary_only_inputs {
        if !summaries.insert(summary.as_str())
            || !input_tokens.contains(summary.as_str())
            || !available_paths.contains(summary.as_str())
        {
            return Err(Error::Contract(format!(
                "{}: summary_only input must name a unique declared earlier output: {summary}",
                node.id
            )));
        }
    }

    let output = node
        .output
        .as_deref()
        .ok_or_else(|| Error::Contract(format!("{}: output missing", node.id)))?;
    path_parts(output)?;
    // The same ordered set checks uniqueness and makes this path available to later nodes.
    if !output.starts_with("output/") || available_paths.contains(output) {
        return Err(Error::Contract(format!(
            "{}: invalid/duplicate output {output}",
            node.id
        )));
    }
    available_paths.insert(output);
    agent_ids.insert(node.id.as_str());
    Ok(())
}
