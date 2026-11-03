//! Durable STS manifest, source binding, and workflow state models.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileFingerprint {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub at: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(flatten)]
    pub details: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenerationPolicy {
    pub model: String,
    pub reasoning_effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomainIdentity {
    pub id: String,
    pub version: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub domain_id: String,
    pub domain_version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub stages: Vec<StageSpec>,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StageSpec {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactValidation {
    #[serde(default)]
    pub required_markers: Vec<String>,
    #[serde(default)]
    pub forbidden_markers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    #[serde(default)]
    pub step: Option<u8>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub task_prompt: Option<String>,
    #[serde(default)]
    pub principles: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub gate_type: Option<String>,
    #[serde(default)]
    pub checkpoint: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub revision_targets: Vec<u8>,
    #[serde(default)]
    pub revision_node: Option<String>,
    #[serde(default)]
    pub max_revisions: Option<u32>,
    #[serde(default)]
    pub notice: Option<String>,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub execution_notes: Vec<String>,
    #[serde(default)]
    pub summary_only_inputs: Vec<String>,
    #[serde(default)]
    pub validation: ArtifactValidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Agent,
    Human,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeState {
    pub status: NodeStatus,
    pub version: u32,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub artifact_sha256: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub revision_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Initialized,
    InProgress,
    AwaitingHuman,
    Paused,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceSnapshot {
    pub config: FileFingerprint,
    pub synopsis: FileFingerprint,
    pub references: Vec<FileFingerprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevisionLog {
    pub at: String,
    pub gate: String,
    pub target: String,
    pub feedback_file: String,
    pub feedback_sha256: String,
    pub invalidated: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub schema_version: u32,
    pub project_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: ProjectStatus,
    pub generation_policy: GenerationPolicy,
    pub domain: DomainIdentity,
    pub nodes: BTreeMap<String, NodeState>,
    pub source_snapshot: Option<SourceSnapshot>,
    #[serde(default)]
    pub active_feedback: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub feedback_sha256: BTreeMap<String, String>,
    #[serde(default)]
    pub revision_log: Vec<RevisionLog>,
    #[serde(default)]
    pub events: Vec<Event>,
}
