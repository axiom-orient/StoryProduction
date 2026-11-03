mod approval;
mod dispatch;
mod generation;
mod packs;
mod requests;
pub(crate) mod support;
mod tools;

use approval::{commit_decision, decision_input};
use support::{command_id, commit, parse};

use crate::error::Result;
use crate::generation::GenerationRegistry;
#[cfg(test)]
use crate::store::now_ms;
use crate::store::Workspace;
use serde::Serialize;
use serde_json::Value;

/// MCP can read candidates and their subjects, but final decisions stay with
/// the local human entrypoint.
pub fn is_human_only_tool(name: &str) -> bool {
    tools::lookup(name).is_some_and(|tool| tool.human_only)
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Clone)]
pub struct Application {
    workspace: Workspace,
    generation: GenerationRegistry,
    craftpack_paths: Option<crate::craftpack::CraftpackPaths>,
}

impl Application {
    pub fn open(root: impl AsRef<std::path::Path>) -> Result<Self> {
        Ok(Self::from_workspace(Workspace::open(root)?))
    }

    pub fn from_workspace(workspace: Workspace) -> Self {
        Self {
            workspace,
            generation: GenerationRegistry::default(),
            craftpack_paths: None,
        }
    }
    pub fn with_generation_registry(mut self, generation: GenerationRegistry) -> Self {
        self.generation = generation;
        self
    }
    /// Configures explicit asset locations for the H4/H5 pack loaders;
    /// without it the loaders discover packs via `GEULTU_ASSETS_ROOT`,
    /// `GEULTU_CRAFT_PACK_ROOT`, and the repository layout.
    #[must_use]
    pub fn with_craftpack_paths(mut self, paths: crate::craftpack::CraftpackPaths) -> Self {
        self.craftpack_paths = Some(paths);
        self
    }
    pub fn with_writing_adapters<I, A>(mut self, adapters: I) -> Self
    where
        I: IntoIterator<Item = A>,
        A: crate::generation::WritingAdapter + 'static,
    {
        for adapter in adapters {
            self.generation.add_writing_adapter(adapter);
        }
        self
    }
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        tools::registered_tools()
            .iter()
            .map(|tool| tool.definition.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    fn app() -> (std::path::PathBuf, Application) {
        let root =
            std::env::temp_dir().join(format!("geultu-app-{}-{}", std::process::id(), now_ms()));
        let _ = fs::remove_dir_all(&root);
        let workspace = Workspace::init(&root).unwrap();
        (root, Application::from_workspace(workspace))
    }

    #[test]
    fn complete_approval_flow_is_revision_checked() {
        let (root, app) = app();
        app.call(
            "project.create",
            json!({"project_id":"book","title":"Book","command_id":"c1"}),
        )
        .unwrap();
        app.call("scene.add", json!({"project_id":"book","scene_id":"scene_1","title":"One","text":"old","command_id":"c2"})).unwrap();
        let candidate_result = app.call("candidate.create", json!({"project_id":"book","candidate_id":"cand","kind":"rewrite","content":"new","provider":"test","model":"fixture","command_id":"c3"})).unwrap();
        assert_eq!(candidate_result["revision"], 3);
        app.call("changeset.create", json!({"project_id":"book","candidate_id":"cand","changeset_id":"change","operations":[{"type":"replace_scene_text","scene_id":"scene_1","expected_scene_revision":0,"text":"new"}],"command_id":"c4"})).unwrap();
        let subject = app
            .call(
                "candidate.subject",
                json!({"project_id":"book","candidate_id":"cand"}),
            )
            .unwrap();
        app.call(
            "changeset.approve",
            json!({
                "project_id":"book",
                "changeset_id":"change",
                "expected_project_revision":4,
                "actor_id":"author",
                "expected_artifact_revision":subject["revision"],
                "expected_artifact_digest":subject["digest"],
                "command_id":"c5"
            }),
        )
        .unwrap();
        let scene = app
            .call(
                "scene.get",
                json!({"project_id":"book","scene_id":"scene_1"}),
            )
            .unwrap();
        assert_eq!(scene["text"], "new");
        assert_eq!(scene["revision"], 1);
        let _ = fs::remove_dir_all(root);
    }
}
