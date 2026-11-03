//! Ordered resolution and hash verification for manifest-selected node inputs.

use super::super::{resolve_regular_file_within_root, Engine};
use crate::model::workflow::{Manifest, Node, NodeKind, NodeStatus, State};
use narrative_courier_runtime::store::sha256_file;
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

impl Engine {
    fn producer_for_output<'a>(&self, manifest: &'a Manifest, relative: &str) -> Option<&'a Node> {
        manifest
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Agent && node.output.as_deref() == Some(relative))
    }

    pub(in crate::engine) fn verify_durable_output(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
        relative: &str,
    ) -> Result<PathBuf> {
        let path = resolve_regular_file_within_root(project, relative)?;
        self.verify_durable_output_at(state, manifest, relative, path)
    }

    fn verify_durable_output_at(
        &self,
        state: &State,
        manifest: &Manifest,
        relative: &str,
        path: PathBuf,
    ) -> Result<PathBuf> {
        let producer = self
            .producer_for_output(manifest, relative)
            .ok_or_else(|| Error::Contract(format!("no producer for output: {relative}")))?;
        let record = state.nodes.get(&producer.id).ok_or_else(|| {
            Error::Contract(format!(
                "missing state for output producer: {}",
                producer.id
            ))
        })?;
        if record.status != NodeStatus::Completed {
            return Err(Error::Contract(format!(
                "producer not completed: {}",
                producer.id
            )));
        }
        let actual = sha256_file(&path)?;
        if record.artifact_sha256.as_deref() != Some(actual.as_str()) {
            return Err(Error::Contract(format!(
                "durable output integrity failure: {relative}"
            )));
        }
        Ok(path)
    }

    pub(in crate::engine) fn verify_feedback(
        &self,
        project: &Path,
        state: &State,
        relative: &str,
    ) -> Result<PathBuf> {
        let path = resolve_regular_file_within_root(project, relative)?;
        self.verify_feedback_at(state, relative, path)
    }

    fn verify_feedback_at(&self, state: &State, relative: &str, path: PathBuf) -> Result<PathBuf> {
        if !relative.starts_with("input/human_feedback/") {
            return Err(Error::Contract(format!(
                "feedback path is outside feedback storage: {relative}"
            )));
        }
        let actual = sha256_file(&path)?;
        if state.feedback_sha256.get(relative).map(String::as_str) != Some(actual.as_str()) {
            return Err(Error::Contract(format!(
                "feedback integrity failure: {relative}"
            )));
        }
        Ok(path)
    }

    pub(in crate::engine) fn resolved_inputs(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
        node: &Node,
    ) -> Result<Vec<PathBuf>> {
        let config = self.load_config(project)?;
        let synopsis = self.resolve_synopsis(project, &config)?;
        let references = self.resolve_references(project, &config)?;
        let mut resolved = Vec::new();
        let mut seen = BTreeSet::new();
        for token in &node.inputs {
            let (optional, raw) = match token.strip_prefix('?') {
                Some(raw) => (true, raw),
                None => (false, token.as_str()),
            };
            if raw == "@references" {
                for path in &references {
                    if seen.insert(path.clone()) {
                        resolved.push(path.clone());
                    }
                }
                continue;
            }
            let path = match raw {
                "@synopsis" => Ok(synopsis.clone()),
                "config.yaml" => resolve_regular_file_within_root(project, raw),
                _ if raw.starts_with("output/") || raw.starts_with("input/") => {
                    resolve_regular_file_within_root(project, raw)
                }
                _ => self.resolve_asset_file(raw),
            };
            let path = match path {
                Ok(path) => path,
                Err(Error::Io(error))
                    if optional && error.kind() == std::io::ErrorKind::NotFound =>
                {
                    continue
                }
                Err(error) => return Err(error),
            };
            // Classify by manifest token, never by a transformed/canonical filesystem prefix.
            let path = if raw.starts_with("output/") {
                self.verify_durable_output_at(state, manifest, raw, path)?
            } else if raw.starts_with("input/human_feedback/") {
                self.verify_feedback_at(state, raw, path)?
            } else {
                path
            };
            if seen.insert(path.clone()) {
                resolved.push(path);
            }
        }
        if let Some(extra) = state.active_feedback.get(&node.id) {
            for relative in extra {
                let path = self.verify_feedback(project, state, relative)?;
                if seen.insert(path.clone()) {
                    resolved.push(path);
                }
            }
        }
        Ok(resolved)
    }
}
