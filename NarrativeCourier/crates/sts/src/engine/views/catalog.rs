//! Read-only project and artifact catalog projections.

use super::super::Engine;
use crate::model::views::{ArtifactListItem, ArtifactView, ProjectListItem};
use crate::model::workflow::{Node, NodeKind, NodeState, NodeStatus};
use narrative_courier_runtime::{Error, Result};
use std::fs;

fn artifact_list_item(node: &Node, record: &NodeState) -> Result<ArtifactListItem> {
    let role = node
        .role
        .clone()
        .ok_or_else(|| Error::Contract(format!("Agent node role missing: {}", node.id)))?;
    let path = node
        .output
        .clone()
        .ok_or_else(|| Error::Contract(format!("Agent node output missing: {}", node.id)))?;
    Ok(ArtifactListItem {
        node: node.id.clone(),
        role,
        step: node.step,
        phase: node.phase.clone(),
        path,
        version: record.version,
        updated_at: record.updated_at.clone(),
    })
}

impl Engine {
    pub fn list_projects(&self) -> Result<Vec<ProjectListItem>> {
        let entries = match fs::read_dir(self.runtime.projects_root()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let manifest = self.load_manifest()?;
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let project = entry.path();
            let project_name = entry.file_name();
            if project_name
                .to_str()
                .is_some_and(|name| name.starts_with('.'))
            {
                continue;
            }
            let state = self.load_state(&project, &manifest)?;
            let title = self.load_config(&project)?.title;
            out.push(ProjectListItem {
                name: state.project_name,
                title,
                status: state.status,
                updated_at: state.updated_at,
            });
        }
        out.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(out)
    }

    pub fn list_artifacts(&self, name: &str) -> Result<Vec<ArtifactListItem>> {
        let manifest = self.load_manifest()?;
        let project = self.project_dir(name)?;
        let state = self.load_state(&project, &manifest)?;
        let mut items = Vec::new();
        for node in manifest
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Agent)
        {
            let record = &state.nodes[&node.id];
            if record.status != NodeStatus::Completed {
                continue;
            }
            let item = artifact_list_item(node, record)?;
            self.verify_durable_output(&project, &state, &manifest, &item.path)?;
            items.push(item);
        }
        Ok(items)
    }

    pub fn read_artifact(&self, name: &str, node_id: &str) -> Result<ArtifactView> {
        let manifest = self.load_manifest()?;
        let project = self.project_dir(name)?;
        let state = self.load_state(&project, &manifest)?;
        let node = manifest
            .nodes
            .iter()
            .find(|node| node.id == node_id && node.kind == NodeKind::Agent)
            .ok_or_else(|| Error::Contract(format!("unknown artifact node: {node_id}")))?;
        let record = &state.nodes[&node.id];
        if record.status != NodeStatus::Completed {
            return Err(Error::Contract(format!(
                "artifact is not completed: {node_id}"
            )));
        }
        let item = artifact_list_item(node, record)?;
        Ok(ArtifactView {
            content: self.read_durable_text(&project, &state, &manifest, &item.path)?,
            item,
        })
    }
}
