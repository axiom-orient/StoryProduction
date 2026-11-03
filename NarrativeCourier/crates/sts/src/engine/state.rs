//! Durable STS state loading/saving facade and current-state initialization.

mod config;
mod inputs;
mod source_snapshot;

use super::{now_iso, resolve_regular_file_within_root, Engine};
use crate::codex::config::{selected_generation_policy, validate_generation_policy};
use crate::model::workflow::{Manifest, NodeState, NodeStatus, ProjectStatus, State};
use narrative_courier_runtime::store::atomic_write_text;
use narrative_courier_runtime::Result;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

impl Engine {
    pub(super) fn initial_state(&self, name: &str, manifest: &Manifest) -> Result<State> {
        let now = now_iso();
        let mut nodes = BTreeMap::new();
        for node in &manifest.nodes {
            nodes.insert(
                node.id.clone(),
                NodeState {
                    status: NodeStatus::Pending,
                    version: 0,
                    updated_at: None,
                    artifact_sha256: None,
                    decision: None,
                    revision_count: 0,
                },
            );
        }
        Ok(State {
            schema_version: 5,
            project_name: name.to_string(),
            created_at: now.clone(),
            updated_at: now,
            status: ProjectStatus::Initialized,
            generation_policy: selected_generation_policy()?,
            domain: self.domain_identity(manifest)?,
            nodes,
            source_snapshot: None,
            active_feedback: BTreeMap::new(),
            feedback_sha256: BTreeMap::new(),
            revision_log: vec![],
            events: vec![],
        })
    }

    pub fn load_state(&self, project: &Path, manifest: &Manifest) -> Result<State> {
        let path = resolve_regular_file_within_root(project, "state.json")?;
        let state: State = serde_json::from_slice(&fs::read(path)?)?;
        if state.schema_version != 5 {
            return Err(narrative_courier_runtime::Error::Contract(
                "state schema_version must be 5".into(),
            ));
        }
        let expected_project = self.selected_project.as_deref().unwrap_or_else(|| {
            project
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("")
        });
        if state.project_name != expected_project {
            return Err(narrative_courier_runtime::Error::Contract(
                "state project identity mismatch".into(),
            ));
        }
        if state.domain != self.domain_identity(manifest)? {
            return Err(narrative_courier_runtime::Error::Contract(
                "domain bundle changed after project creation; mixed-domain execution is blocked"
                    .into(),
            ));
        }
        let expected: std::collections::BTreeSet<_> =
            manifest.nodes.iter().map(|node| node.id.as_str()).collect();
        let actual: std::collections::BTreeSet<_> =
            state.nodes.keys().map(|value| value.as_str()).collect();
        if expected != actual {
            return Err(narrative_courier_runtime::Error::Contract(
                "state nodes do not match canonical manifest".into(),
            ));
        }
        validate_generation_policy(&state.generation_policy)?;
        Ok(state)
    }

    pub(super) fn save_state(&self, project: &Path, state: &mut State) -> Result<()> {
        state.updated_at = now_iso();
        atomic_write_text(
            &project.join("state.json"),
            &(serde_json::to_string_pretty(state)? + "\n"),
        )
    }
}
