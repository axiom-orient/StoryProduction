//! Agent task projection and bounded prompt/input materialization.

use super::super::super::{is_extractable_document, is_text_file, Engine};
use crate::model::views::NextPacket;
use crate::model::workflow::{GenerationPolicy, Node, NodeKind, ProjectStatus};
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

impl Engine {
    pub fn next(&self, name: &str) -> Result<NextPacket> {
        let manifest = self.load_manifest()?;
        let project = self.project_dir(name)?;
        let state = self.load_state(&project, &manifest)?;
        if state.status == ProjectStatus::Paused {
            return Ok(NextPacket::Paused);
        }
        let Some(node) = self.next_node(&state, &manifest) else {
            self.verify_source_snapshot(&project, &state)?;
            self.verify_durable_output(&project, &state, &manifest, "output/final_screenplay.md")?;
            return Ok(NextPacket::Completed {
                final_path: project
                    .join("output/final_screenplay.md")
                    .to_string_lossy()
                    .into(),
            });
        };
        if node.kind == NodeKind::Human {
            self.verify_source_snapshot(&project, &state)?;
            if let Some(source) = &node.source {
                self.verify_durable_output(&project, &state, &manifest, source)?;
            }
            return Ok(NextPacket::AwaitingHuman {
                node: node.id.clone(),
                gate_type: node.gate_type.clone(),
                label: node.label.clone(),
                review: node.source.clone(),
                allowed: self.review_actions(&state, node)?,
                notice: node.notice.clone(),
                gate_token: self.gate_token(&project, &state, node)?,
            });
        }
        self.verify_source_snapshot(&project, &state)?;
        let inputs = self.resolved_inputs(&project, &state, &manifest, node)?;
        Ok(NextPacket::AgentTask {
            node: node.id.clone(),
            role: node.role.clone().unwrap(),
            step: node.step,
            phase: node.phase.clone(),
            system_prompt: node.system_prompt.clone().unwrap(),
            task_prompt: node.task_prompt.clone().unwrap(),
            inputs: inputs
                .iter()
                .map(|p| self.display_path(&project, p))
                .collect(),
            output: node.output.clone().unwrap(),
            contract: self.execution_contract(node),
            task_token: self.task_token(&project, &state, node, &inputs)?,
        })
    }

    pub fn task_text(&self, name: &str) -> Result<(String, String)> {
        let (token, text, _, _) = self.task_material(name)?;
        Ok((token, text))
    }

    pub(crate) fn task_material(
        &self,
        name: &str,
    ) -> Result<(String, String, Vec<PathBuf>, GenerationPolicy)> {
        let manifest = self.load_manifest()?;
        let project = self.project_dir(name)?;
        let state = self.load_state(&project, &manifest)?;
        let node = self
            .next_node(&state, &manifest)
            .ok_or_else(|| Error::Contract("workflow completed".into()))?;
        if node.kind != NodeKind::Agent {
            return Err(Error::Contract("current node is human gate".into()));
        }
        self.verify_source_snapshot(&project, &state)?;
        let inputs = self.resolved_inputs(&project, &state, &manifest, node)?;
        let token = self.task_token(&project, &state, node, &inputs)?;
        let system =
            fs::read_to_string(self.resolve_asset_file(node.system_prompt.as_ref().unwrap())?)?;
        let task =
            fs::read_to_string(self.resolve_asset_file(node.task_prompt.as_ref().unwrap())?)?;
        let mut routed = String::new();
        for rel in &node.principles {
            routed.push_str(&format!(
                "\n## {}\n{}\n",
                rel,
                fs::read_to_string(self.resolve_asset_file(rel)?)?
            ));
        }
        let mut binary_inputs = Vec::new();
        let mut text = format!("{}\n\n# SYSTEM PROMPT\n{}\n\n# ROUTED PRINCIPLES\n{}\n# TASK PROMPT\n{}\n\n# RESOLVED INPUTS\n", self.execution_contract(node), system, routed, task);
        for p in &inputs {
            text.push_str(&format!("\n## {}\n", self.display_path(&project, p)));
            if is_text_file(p) {
                let content = fs::read_to_string(p)?;
                let task_content = self.semantic_input_for_node(&project, node, p, &content)?;
                text.push_str(&task_content);
                if !task_content.ends_with('\n') {
                    text.push('\n');
                }
            } else if is_extractable_document(p) {
                let content = narrative_courier_runtime::document::extract_document_text(p)?;
                text.push_str("[PROGRAM-NORMALIZED DOCUMENT TEXT]\n");
                text.push_str(&content);
                if !content.ends_with('\n') {
                    text.push('\n');
                }
            } else {
                binary_inputs.push(p.clone());
                text.push_str(&format!("[IMAGE INPUT: {}]\n", p.display()));
            }
        }
        text.push_str("\n# RESPONSE CONTRACT\nReturn ONLY the durable artifact body in Markdown/plain text. Do not wrap it in a code fence. Do not explain what you did. Do not modify files.\n");
        Ok((token, text, binary_inputs, state.generation_policy.clone()))
    }

    fn semantic_input_for_node(
        &self,
        project: &Path,
        node: &Node,
        path: &Path,
        content: &str,
    ) -> Result<String> {
        let project = if project.is_relative() {
            project.to_path_buf()
        } else {
            fs::canonicalize(project)?
        };
        let relative = path
            .strip_prefix(&project)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if node
            .summary_only_inputs
            .iter()
            .any(|input| input == &relative)
        {
            let marker = "## [TRANSFER CAPSULE]";
            let start = content.find(marker).ok_or_else(|| {
                Error::Contract(format!(
                    "summary-only input is missing its transfer summary: {relative}"
                ))
            })?;
            return Ok(format!(
                "[PROGRAM CONTEXT REDUCTION: 승인된 전달 요약만 제공]\n{}",
                &content[start..]
            ));
        }
        Ok(content.to_string())
    }
}
