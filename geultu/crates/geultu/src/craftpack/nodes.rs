//! Closed serde types for the scriptforge craft-pack manifests.
//!
//! `craft-pack.json` and `prompts/workflow_manifest.json` are mirrored 1:1
//! with `#[serde(deny_unknown_fields)]`; pattern-free shape constraints the
//! derive cannot express (unknown node kinds) are rejected by the manual
//! [`WorkflowNode`] dispatch.

use serde::{Deserialize, Serialize};

/// Typed `craft-pack.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CraftPackManifest {
    pub api_version: String,
    pub pack_id: String,
    pub version: String,
    pub source_archive: String,
    pub source_archive_sha256: String,
    pub workflow_manifest: String,
    pub workflow_manifest_sha256: String,
    pub prompt_asset_count: u64,
    pub license: String,
    pub redistribution_policy: String,
}

/// Agent node of the workflow manifest; unknown fields are rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowAgentNode {
    pub kind: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    pub role: String,
    pub system_prompt: String,
    pub task_prompt: String,
    pub inputs: Vec<String>,
    pub output: String,
}

/// Human gate of the workflow manifest; unknown fields are rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowHumanNode {
    pub kind: String,
    pub id: String,
    pub gate_type: String,
    pub label: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_targets: Option<Vec<u64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_revisions: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_node: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional_output: Option<String>,
}

/// Workflow manifest node, discriminated by the required `kind` field.
/// Manual dispatch retains the flat `kind` field in each closed variant body.
/// Serialization uses the same flat manifest shape, and unknown kinds fail closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum WorkflowNode {
    Agent(WorkflowAgentNode),
    Human(WorkflowHumanNode),
}

impl<'de> Deserialize<'de> for WorkflowNode {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as DeError;
        let value = serde_json::Value::deserialize(deserializer)?;
        let kind = value
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| D::Error::custom("workflow node requires a string 'kind' field"))?;
        match kind {
            "agent" => Ok(Self::Agent(
                serde_json::from_value(value).map_err(D::Error::custom)?,
            )),
            "human" => Ok(Self::Human(
                serde_json::from_value(value).map_err(D::Error::custom)?,
            )),
            other => Err(D::Error::custom(format!(
                "unknown workflow node kind '{other}'; expected 'agent' or 'human'"
            ))),
        }
    }
}

impl WorkflowNode {
    /// Node identity shared by both kinds.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Agent(node) => &node.id,
            Self::Human(node) => &node.id,
        }
    }
}

/// Typed `prompts/workflow_manifest.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowManifest {
    pub schema_version: u64,
    pub description: String,
    pub nodes: Vec<WorkflowNode>,
}

impl WorkflowManifest {
    /// Number of agent-executed nodes.
    #[must_use]
    pub fn agent_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| matches!(node, WorkflowNode::Agent(_)))
            .count()
    }

    /// Number of human gates.
    #[must_use]
    pub fn human_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| matches!(node, WorkflowNode::Human(_)))
            .count()
    }

    /// Agent nodes as planning references, in manifest document order —
    /// per the manifest, the document alone defines execution ordering.
    #[must_use]
    pub fn agent_nodes(&self) -> Vec<AgentNode> {
        self.nodes
            .iter()
            .filter_map(|node| match node {
                WorkflowNode::Agent(agent) => Some(AgentNode {
                    id: agent.id.clone(),
                    role: agent.role.clone(),
                    task_prompt: agent.task_prompt.clone(),
                }),
                WorkflowNode::Human(_) => None,
            })
            .collect()
    }
}

/// Flattened agent-node reference used for status and plan citations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentNode {
    pub id: String,
    pub role: String,
    pub task_prompt: String,
}
