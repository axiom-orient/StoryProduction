//! Read-only STS views composed from integrity, catalog, and project projections.

mod catalog;
mod diagnostics;
mod project;

use super::Engine;
use crate::model::workflow::{Manifest, Node, State};
use narrative_courier_runtime::store::sha256_bytes;
use narrative_courier_runtime::{Error, Result};
use std::path::Path;

impl Engine {
    pub(in crate::engine) fn review_actions(
        &self,
        state: &State,
        node: &Node,
    ) -> Result<Vec<String>> {
        let record = state
            .nodes
            .get(&node.id)
            .ok_or_else(|| Error::Contract("review gate state is missing".into()))?;
        let mut allowed = self.human_allowed();
        if node
            .max_revisions
            .is_some_and(|limit| record.revision_count >= limit)
        {
            allowed.retain(|decision| decision != "revise");
        }
        Ok(allowed)
    }

    /// Display exactly the bytes admitted by the recorded artifact authority.
    fn read_durable_text(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
        relative: &str,
    ) -> Result<String> {
        let path = self.verify_durable_output(project, state, manifest, relative)?;
        let bytes = std::fs::read(path)?;
        let producer = manifest
            .nodes
            .iter()
            .find(|node| node.output.as_deref() == Some(relative))
            .ok_or_else(|| Error::Contract("view artifact producer is missing".into()))?;
        let expected = state
            .nodes
            .get(&producer.id)
            .and_then(|record| record.artifact_sha256.as_deref());
        if expected != Some(sha256_bytes(&bytes).as_str()) {
            return Err(Error::Contract(format!(
                "view artifact bytes changed: {relative}"
            )));
        }
        String::from_utf8(bytes)
            .map_err(|error| Error::Contract(format!("view artifact is not UTF-8: {error}")))
    }
}
