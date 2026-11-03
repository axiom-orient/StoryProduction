//! Token-bound Agent artifact validation, commit, and versioned history.

use super::super::super::{
    ensure_atomic_file_target, now_iso, push_event, resolve_directory_within_root,
    resolve_regular_file_within_root, Engine,
};
use super::super::fidelity::alignment::validate_step_08;
use super::super::fidelity::source_coverage::validate_step_00_source_coverage;
use crate::model::views::SubmitResult;
use crate::model::workflow::{Manifest, Node, NodeKind, NodeStatus, ProjectStatus};
use narrative_courier_runtime::store::{
    atomic_write, lock_project, sha256_bytes, sha256_file, timestamp_compact, ProjectLock,
};
use narrative_courier_runtime::{Error, Result};
use serde_json::json;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;

impl Engine {
    fn validate_agent_artifact(&self, node: &Node, content: &str) -> Result<()> {
        if let Some(missing) = node
            .validation
            .required_markers
            .iter()
            .find(|marker| !content.contains(marker.as_str()))
        {
            return Err(Error::Contract(format!(
                "{} omitted required domain evidence marker: {missing}",
                node.id
            )));
        }
        if let Some(forbidden) = node
            .validation
            .forbidden_markers
            .iter()
            .find(|marker| content.contains(marker.as_str()))
        {
            return Err(Error::Contract(format!(
                "{} contains forbidden domain marker: {forbidden}",
                node.id
            )));
        }
        Ok(())
    }

    pub fn submit(&self, name: &str, content: &str, provided_token: &str) -> Result<SubmitResult> {
        if content.trim().is_empty() {
            return Err(Error::Contract("empty artifact cannot be submitted".into()));
        }
        let manifest = self.load_manifest()?;
        let project = self.project_dir(name)?;
        let lock = lock_project(&project)?;
        self.submit_locked(name, content, provided_token, &manifest, &lock)
    }

    pub(crate) fn submit_with_lock(
        &self,
        name: &str,
        content: &str,
        provided_token: &str,
        _lock: &ProjectLock,
    ) -> Result<SubmitResult> {
        if content.trim().is_empty() {
            return Err(Error::Contract("empty artifact cannot be submitted".into()));
        }
        let manifest = self.load_manifest()?;
        self.submit_locked(name, content, provided_token, &manifest, _lock)
    }

    fn submit_locked(
        &self,
        name: &str,
        content: &str,
        provided_token: &str,
        manifest: &Manifest,
        _lock: &ProjectLock,
    ) -> Result<SubmitResult> {
        let project = self.project_dir(name)?;
        let mut state = self.load_state(&project, manifest)?;
        if state.status == ProjectStatus::Paused {
            return Err(Error::Contract("project is paused".into()));
        }
        let node = self
            .next_node(&state, manifest)
            .ok_or_else(|| Error::Contract("workflow completed".into()))?;
        if node.kind != NodeKind::Agent {
            return Err(Error::Contract(format!(
                "current node is human gate: {}",
                node.id
            )));
        }
        self.validate_agent_artifact(node, content)?;
        self.verify_source_snapshot(&project, &state)?;
        let inputs = self.resolved_inputs(&project, &state, manifest, node)?;
        let expected = self.task_token(&project, &state, node, &inputs)?;
        if expected != provided_token {
            return Err(Error::Contract("stale/invalid task token".into()));
        }
        if node.id == "step_00" {
            if !node.inputs.iter().any(|input| input == "@synopsis") {
                return Err(Error::Contract(
                    "step_00 source coverage requires its declared synopsis input".into(),
                ));
            }
            let config = self.load_config(&project)?;
            let synopsis_path = self.resolve_synopsis(&project, &config)?;
            let synopsis = fs::read_to_string(synopsis_path)?;
            validate_step_00_source_coverage(&synopsis, content)?;
        }
        validate_step_08(node, &inputs, content)?;
        if node.id == "step_00" && state.source_snapshot.is_none() {
            state.source_snapshot = Some(self.build_source_snapshot(&project)?);
        }
        let current_version = state
            .nodes
            .get(&node.id)
            .ok_or_else(|| Error::Contract("submission node state is missing".into()))?
            .version;
        let new_version = current_version
            .checked_add(1)
            .ok_or_else(|| Error::Contract("submission version exceeds u32 capacity".into()))?;
        let output_rel = node
            .output
            .as_ref()
            .ok_or_else(|| Error::Contract("submission output is missing".into()))?;
        let output = project.join(output_rel);
        let parent_rel = std::path::Path::new(output_rel)
            .parent()
            .and_then(|path| path.to_str())
            .ok_or_else(|| Error::Contract("submission output parent is invalid".into()))?;
        resolve_directory_within_root(&project, parent_rel)?;
        ensure_atomic_file_target(&output, "submission output")?;
        self.archive_output(
            &project,
            node,
            current_version,
            state.nodes[&node.id].artifact_sha256.as_deref(),
        )?;
        atomic_write(&output, content.as_bytes())?;
        let digest = sha256_file(&resolve_regular_file_within_root(&project, output_rel)?)?;
        if digest != sha256_bytes(content.as_bytes()) {
            return Err(Error::Contract(
                "persisted submission bytes differ from the submitted artifact".into(),
            ));
        }
        let rec = state
            .nodes
            .get_mut(&node.id)
            .ok_or_else(|| Error::Contract("submission node state is missing".into()))?;
        rec.artifact_sha256 = Some(digest);
        rec.status = NodeStatus::Completed;
        rec.version = new_version;
        rec.updated_at = Some(now_iso());
        state.active_feedback.remove(&node.id);
        push_event(
            &mut state,
            "agent_submit",
            [("node", json!(node.id)), ("version", json!(new_version))],
        );
        self.update_status(&mut state, manifest);
        self.save_state(&project, &mut state)?;
        Ok(SubmitResult {
            node: node.id.clone(),
            output: output_rel.clone(),
            version: new_version,
            status: state.status,
        })
    }

    fn archive_output(
        &self,
        project: &Path,
        node: &Node,
        version: u32,
        expected_digest: Option<&str>,
    ) -> Result<()> {
        let relative = node
            .output
            .as_deref()
            .ok_or_else(|| Error::Contract("archive output is missing".into()))?;
        let output = match resolve_regular_file_within_root(project, relative) {
            Ok(path) => path,
            Err(Error::Io(error)) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        if version > 0 && expected_digest != Some(sha256_file(&output)?.as_str()) {
            return Err(Error::Contract(
                "existing versioned output does not match its recorded hash".into(),
            ));
        }
        for relative in [
            "output/.history".to_owned(),
            format!("output/.history/{}", node.id),
        ] {
            match fs::create_dir(project.join(&relative)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
            resolve_directory_within_root(project, &relative)?;
        }
        let history =
            resolve_directory_within_root(project, &format!("output/.history/{}", node.id))?;
        let ext = output
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("md");
        let target = if version == 0 {
            history.join(format!("orphan-{}.{}", timestamp_compact(), ext))
        } else {
            history.join(format!("v{version:03}.{ext}"))
        };
        let mut archive = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)?;
        io::copy(&mut File::open(output)?, &mut archive)?;
        archive.sync_all()?;
        File::open(&history)?.sync_all()?;
        Ok(())
    }
}
