//! Human gate decisions, revision invalidation, and approval transitions.

use super::super::{
    ensure_atomic_file_target, now_iso, push_event, resolve_directory_within_root,
    resolve_regular_file_within_root, Engine,
};
use super::fidelity::alignment::validate_step_08;
use crate::model::views::DecisionResult;
use crate::model::workflow::{Manifest, NodeKind, NodeStatus, ProjectStatus, RevisionLog, State};
use narrative_courier_runtime::store::{
    atomic_write_text, lock_project, sha256_bytes, sha256_file,
};
use narrative_courier_runtime::{Error, Result};
use serde_json::json;
use std::fs;

impl Engine {
    pub(in crate::engine) fn human_allowed(&self) -> Vec<String> {
        vec!["approve".into(), "revise".into(), "pause".into()]
    }

    pub(in crate::engine) fn step_node_id(&self, manifest: &Manifest, step: u8) -> Result<String> {
        let exact = format!("step_{step:02}");
        if manifest
            .nodes
            .iter()
            .any(|n| n.id == exact && n.kind == NodeKind::Agent && n.step == Some(step))
        {
            Ok(exact)
        } else {
            Err(Error::Contract(format!(
                "STEP {step} cannot be revised directly here"
            )))
        }
    }

    fn invalidate(
        &self,
        manifest: &Manifest,
        state: &mut State,
        start_id: &str,
        end_id: &str,
    ) -> Result<Vec<String>> {
        let start = manifest
            .nodes
            .iter()
            .position(|n| n.id == start_id)
            .ok_or_else(|| Error::Contract("revision start not found".into()))?;
        let end = manifest
            .nodes
            .iter()
            .position(|n| n.id == end_id)
            .ok_or_else(|| Error::Contract("revision end not found".into()))?;
        if start > end {
            return Err(Error::Contract("revision target after gate".into()));
        }
        let mut out = Vec::new();
        for n in &manifest.nodes[start..=end] {
            let rec = state.nodes.get_mut(&n.id).ok_or_else(|| {
                Error::Contract(format!("revision node state is missing: {}", n.id))
            })?;
            if rec.status == NodeStatus::Completed || n.id == end_id {
                rec.status = NodeStatus::Pending;
                rec.updated_at = Some(now_iso());
                if n.kind == NodeKind::Human {
                    rec.decision = None;
                }
                out.push(n.id.clone());
            }
        }
        Ok(out)
    }

    pub fn decide(
        &self,
        name: &str,
        decision: &str,
        step: Option<u8>,
        feedback: Option<&str>,
        gate_token: &str,
    ) -> Result<DecisionResult> {
        let manifest = self.load_manifest()?;
        let project = self.project_dir(name)?;
        let _lock = lock_project(&project)?;
        let mut state = self.load_state(&project, &manifest)?;
        if state.status == ProjectStatus::Paused {
            return Err(Error::Contract(
                "project is paused; resume before deciding".into(),
            ));
        }
        let node = self
            .next_node(&state, &manifest)
            .ok_or_else(|| Error::Contract("workflow completed".into()))?;
        if node.kind != NodeKind::Human {
            return Err(Error::Contract(format!(
                "current node is agent task: {}",
                node.id
            )));
        }
        self.verify_source_snapshot(&project, &state)?;
        let verified_source = node
            .source
            .as_deref()
            .map(|source| self.verify_durable_output(&project, &state, &manifest, source))
            .transpose()?;
        if self.gate_token(&project, &state, node)? != gate_token {
            return Err(Error::Contract("stale/invalid gate token".into()));
        }
        let allowed = self.review_actions(&state, node)?;
        if !allowed.iter().any(|v| v == decision) {
            return Err(Error::Contract(format!("decision not allowed: {decision}")));
        }

        if decision == "pause" {
            state.status = ProjectStatus::Paused;
            push_event(&mut state, "pause", [("node", json!(node.id))]);
            self.save_state(&project, &mut state)?;
            return Ok(DecisionResult {
                node: node.id.clone(),
                decision: decision.into(),
                status: state.status,
                target: None,
                invalidated: vec![],
            });
        }
        if decision == "approve" {
            let next_version = state
                .nodes
                .get(&node.id)
                .ok_or_else(|| Error::Contract("human gate state is missing".into()))?
                .version
                .checked_add(1)
                .ok_or_else(|| Error::Contract("human gate version exceeds u32 capacity".into()))?;
            if node.id == "final_release" {
                let source = verified_source
                    .as_deref()
                    .ok_or_else(|| Error::Contract("final release source missing".into()))?;
                let verdict = fs::read_to_string(source)?;
                if verdict.matches("STS_STATUS: READY").count() != 1
                    || verdict.contains("STS_STATUS: BLOCKED")
                {
                    return Err(Error::Contract("final release requires exactly one STS_STATUS: READY and no BLOCKED marker".into()));
                }
                let verification_node = manifest
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == "step_08_verify")
                    .ok_or_else(|| Error::Contract("STEP 8D verification node missing".into()))?;
                let verification_inputs =
                    self.resolved_inputs(&project, &state, &manifest, verification_node)?;
                validate_step_08(verification_node, &verification_inputs, &verdict)?;
                // Produce and validate the structured authority artifacts while
                // the final gate is still open. The release bundle is emitted
                // only after the Completed state has been durably committed.
                self.publish_scene_map(&project, &state, &manifest)?;
                self.publish_narrative_package(&project, &state, &manifest)?;
            }
            let rec = state
                .nodes
                .get_mut(&node.id)
                .ok_or_else(|| Error::Contract("human gate state is missing".into()))?;
            rec.status = NodeStatus::Completed;
            rec.decision = Some("approve".into());
            rec.version = next_version;
            rec.updated_at = Some(now_iso());
            push_event(&mut state, "human_approve", [("node", json!(node.id))]);
            self.update_status(&mut state, &manifest);
            self.save_state(&project, &mut state)?;
            if node.id == "final_release" {
                let committed_state = self.load_state(&project, &manifest)?;
                let package =
                    self.verify_narrative_package_at(&project, &committed_state, &manifest)?;
                self.export_release_bundle(&project, &committed_state, &manifest, &package)?;
            }
            return Ok(DecisionResult {
                node: node.id.clone(),
                decision: "approve".into(),
                status: state.status,
                target: None,
                invalidated: vec![],
            });
        }

        let text = feedback
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::Contract("revision feedback is required".into()))?;
        let current_revisions = state
            .nodes
            .get(&node.id)
            .ok_or_else(|| Error::Contract("human gate state is missing".into()))?
            .revision_count;
        if let Some(max) = node.max_revisions {
            if current_revisions >= max {
                return Err(Error::Contract(format!("max revisions reached ({max})")));
            }
        }
        let target = if let Some(revision_node) = &node.revision_node {
            if step.is_some() {
                return Err(Error::Contract(
                    "artifact review revises its declared node; step must be omitted".into(),
                ));
            }
            revision_node.clone()
        } else {
            let s = step.ok_or_else(|| {
                Error::Contract("checkpoint revision requires target step".into())
            })?;
            if !node.revision_targets.contains(&s) {
                return Err(Error::Contract(format!(
                    "STEP {s} not allowed at this checkpoint"
                )));
            }
            self.step_node_id(&manifest, s)?
        };
        let seq = current_revisions
            .checked_add(1)
            .ok_or_else(|| Error::Contract("human revision count exceeds u32 capacity".into()))?;
        let next_version = state
            .nodes
            .get(&node.id)
            .ok_or_else(|| Error::Contract("human gate state is missing".into()))?
            .version
            .checked_add(1)
            .ok_or_else(|| Error::Contract("human gate version exceeds u32 capacity".into()))?;
        let invalidated = self.invalidate(&manifest, &mut state, &target, &node.id)?;
        {
            let rec = state
                .nodes
                .get_mut(&node.id)
                .ok_or_else(|| Error::Contract("human gate state is missing".into()))?;
            rec.revision_count = seq;
            rec.version = next_version;
            rec.updated_at = Some(now_iso());
        }
        let feedback_rel = format!("input/human_feedback/{}_r{seq:02}_{}.md", node.id, target);
        resolve_directory_within_root(&project, "input/human_feedback")?;
        ensure_atomic_file_target(&project.join(&feedback_rel), "human feedback")?;
        if state.feedback_sha256.contains_key(&feedback_rel)
            || project.join(&feedback_rel).try_exists()?
        {
            return Err(Error::Contract(
                "revision feedback already exists; explicit recovery is required".into(),
            ));
        }
        let feedback_body = format!(
            "# Human Feedback\n\n- gate: {}\n- target: {}\n- recorded_at: {}\n\n{}\n",
            node.id,
            target,
            now_iso(),
            text.trim()
        );
        let feedback_hash = sha256_bytes(feedback_body.as_bytes());
        atomic_write_text(&project.join(&feedback_rel), &feedback_body)?;
        if sha256_file(&resolve_regular_file_within_root(&project, &feedback_rel)?)?
            != feedback_hash
        {
            return Err(Error::Contract(
                "persisted feedback differs from the submitted review".into(),
            ));
        }
        state
            .feedback_sha256
            .insert(feedback_rel.clone(), feedback_hash.clone());
        state
            .active_feedback
            .entry(target.clone())
            .or_default()
            .push(feedback_rel.clone());
        state.revision_log.push(RevisionLog {
            at: now_iso(),
            gate: node.id.clone(),
            target: target.clone(),
            feedback_file: feedback_rel,
            feedback_sha256: feedback_hash,
            invalidated: invalidated.clone(),
        });
        state.status = ProjectStatus::InProgress;
        push_event(
            &mut state,
            "human_revise",
            [("node", json!(node.id)), ("target", json!(target))],
        );
        self.save_state(&project, &mut state)?;
        Ok(DecisionResult {
            node: node.id.clone(),
            decision: "revise".into(),
            status: state.status,
            target: Some(target),
            invalidated,
        })
    }
}
