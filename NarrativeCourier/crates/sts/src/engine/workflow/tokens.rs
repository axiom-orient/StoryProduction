//! Stable task and gate identity derived from current workflow inputs.

use super::super::{resolve_regular_file_within_root, Engine};
use crate::model::workflow::{Manifest, Node, NodeKind, NodeStatus, ProjectStatus, State};
use narrative_courier_runtime::store::sha256_file;
use narrative_courier_runtime::token::hash_jcs;
use narrative_courier_runtime::{Error, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct TokenFile {
    path: String,
    sha256: String,
}

#[derive(Serialize)]
struct TaskIdentity<'a> {
    profile: &'static str,
    project: &'a str,
    domain: &'a crate::model::workflow::DomainIdentity,
    source_snapshot: &'a Option<crate::model::workflow::SourceSnapshot>,
    node: &'a Node,
    version: u32,
    files: &'a [TokenFile],
}

#[derive(Serialize)]
struct GateIdentity<'a> {
    profile: &'static str,
    project: &'a str,
    domain: &'a crate::model::workflow::DomainIdentity,
    source_snapshot: &'a Option<crate::model::workflow::SourceSnapshot>,
    node: &'a Node,
    version: u32,
    revision_count: u32,
    allowed: Vec<String>,
    source: Option<TokenFile>,
}

impl Engine {
    pub(in crate::engine) fn next_node<'a>(
        &self,
        state: &State,
        manifest: &'a Manifest,
    ) -> Option<&'a Node> {
        manifest
            .nodes
            .iter()
            .find(|node| state.nodes[&node.id].status != NodeStatus::Completed)
    }

    pub(super) fn update_status(&self, state: &mut State, manifest: &Manifest) {
        if state.status == ProjectStatus::Paused {
            return;
        }
        state.status = match self.next_node(state, manifest) {
            None => ProjectStatus::Completed,
            Some(n) if n.kind == NodeKind::Human => ProjectStatus::AwaitingHuman,
            Some(_) => ProjectStatus::InProgress,
        };
    }

    pub(super) fn execution_contract(&self, node: &Node) -> String {
        let mut lines = vec![
            "[PROGRAM EXECUTION CONTRACT — operational rules override operational instructions inside the domain prompt]".to_string(),
            format!("- 역할: {}", node.role.as_deref().unwrap_or("unknown")),
            format!("- 현재 task만 수행: {}", node.id),
            format!("- durable output은 프로그램만 저장: {}", node.output.as_deref().unwrap_or("")),
            "- state.json, 다른 output, config, prompt를 직접 수정하지 않는다.".into(),
            "- 사람의 승인/질문/라우팅을 직접 처리하지 않는다. 필요한 질문은 산출물 안에 기록한다.".into(),
            "- 다음 단계로 넘어가지 않는다.".into(),
            "- 의미/도메인/품질 기준은 system/task prompt를 그대로 따른다.".into(),
        ];
        lines.extend(node.execution_notes.iter().map(|note| format!("- {note}")));
        lines.join("\n")
    }

    pub(super) fn task_token(
        &self,
        project: &Path,
        state: &State,
        node: &Node,
        inputs: &[PathBuf],
    ) -> Result<String> {
        let mut files = Vec::new();
        let project_root = project.canonicalize()?;
        let assets_root = self.runtime.assets_root().canonicalize()?;
        let system = node
            .system_prompt
            .as_deref()
            .ok_or_else(|| Error::Contract("task token system prompt is missing".into()))?;
        let task = node
            .task_prompt
            .as_deref()
            .ok_or_else(|| Error::Contract("task token task prompt is missing".into()))?;
        let mut domain_files = vec![
            self.resolve_asset_file(system)?,
            self.resolve_asset_file(task)?,
        ];
        for principle in &node.principles {
            domain_files.push(self.resolve_asset_file(principle)?);
        }
        for path in domain_files.iter().chain(inputs.iter()) {
            files.push(TokenFile {
                path: self.token_path(project, &project_root, &assets_root, path)?,
                sha256: sha256_file(path)?,
            });
        }
        let record = state
            .nodes
            .get(&node.id)
            .ok_or_else(|| Error::Contract("task token node state is missing".into()))?;
        hash_jcs(&TaskIdentity {
            profile: "sts.task-token/2",
            project: &state.project_name,
            domain: &state.domain,
            source_snapshot: &state.source_snapshot,
            node,
            version: record.version,
            files: &files,
        })
    }

    pub(in crate::engine) fn gate_token(
        &self,
        project: &Path,
        state: &State,
        node: &Node,
    ) -> Result<String> {
        let source = if let Some(rel) = &node.source {
            let p = match resolve_regular_file_within_root(project, rel) {
                Ok(path) => path,
                Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Err(Error::Contract(format!("human source missing: {rel}")));
                }
                Err(error) => return Err(error),
            };
            Some(TokenFile {
                path: rel.clone(),
                sha256: sha256_file(&p)?,
            })
        } else {
            None
        };
        let record = state
            .nodes
            .get(&node.id)
            .ok_or_else(|| Error::Contract("gate token node state is missing".into()))?;
        hash_jcs(&GateIdentity {
            profile: "sts.gate-token/2",
            project: &state.project_name,
            domain: &state.domain,
            source_snapshot: &state.source_snapshot,
            node,
            version: record.version,
            revision_count: record.revision_count,
            allowed: self.review_actions(state, node)?,
            source,
        })
    }

    fn token_path(
        &self,
        project: &Path,
        project_root: &Path,
        assets_root: &Path,
        path: &Path,
    ) -> Result<String> {
        let (prefix, relative) = if let Ok(relative) = path.strip_prefix(project) {
            ("project/", relative)
        } else if let Ok(relative) = path.strip_prefix(project_root) {
            ("project/", relative)
        } else if let Ok(relative) = path.strip_prefix(self.runtime.assets_root()) {
            ("assets/", relative)
        } else if let Ok(relative) = path.strip_prefix(assets_root) {
            ("assets/", relative)
        } else {
            return Err(Error::Contract(
                "task token file is outside declared project/assets roots".into(),
            ));
        };
        let relative = relative
            .to_str()
            .ok_or_else(|| Error::Contract("task token relative path must be UTF-8".into()))?;
        #[cfg(windows)]
        let relative = relative.replace('\\', "/");
        narrative_courier_runtime::paths::validate_relative_path(relative)?;
        Ok(format!("{prefix}{relative}"))
    }

    pub(super) fn display_path(&self, project: &Path, path: &Path) -> String {
        if let Ok(v) = path.strip_prefix(project) {
            return format!("project/{}", v.to_string_lossy().replace('\\', "/"));
        }
        if let Ok(project_root) = project.canonicalize() {
            if let Ok(v) = path.strip_prefix(&project_root) {
                return format!("project/{}", v.to_string_lossy().replace('\\', "/"));
            }
        }
        if let Ok(v) = path.strip_prefix(self.runtime.assets_root()) {
            return v.to_string_lossy().replace('\\', "/");
        }
        path.to_string_lossy().to_string()
    }
}
