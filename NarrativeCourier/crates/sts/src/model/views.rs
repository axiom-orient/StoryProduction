//! Public task, decision, diagnostic, artifact, and UI projection DTOs.

use super::workflow::ProjectStatus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NextPacket {
    AgentTask {
        node: String,
        role: String,
        step: Option<u8>,
        phase: Option<String>,
        system_prompt: String,
        task_prompt: String,
        inputs: Vec<String>,
        output: String,
        contract: String,
        task_token: String,
    },
    AwaitingHuman {
        node: String,
        gate_type: Option<String>,
        label: Option<String>,
        review: Option<String>,
        allowed: Vec<String>,
        notice: Option<String>,
        gate_token: String,
    },
    Paused,
    Completed {
        final_path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResult {
    pub node: String,
    pub output: String,
    pub version: u32,
    pub status: ProjectStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResult {
    pub node: String,
    pub decision: String,
    pub status: ProjectStatus,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub invalidated: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub ok: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub ready: bool,
    pub codex_found: bool,
    pub authenticated: bool,
    pub version: Option<String>,
    pub model: String,
    pub reasoning_effort: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactListItem {
    pub node: String,
    pub role: String,
    pub step: Option<u8>,
    pub phase: Option<String>,
    pub path: String,
    pub version: u32,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactView {
    pub item: ArtifactListItem,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiStage {
    pub id: String,
    pub label: String,
    pub status: String,
    pub description: String,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectListItem {
    pub name: String,
    pub title: String,
    pub status: ProjectStatus,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewView {
    pub node: String,
    pub label: String,
    pub gate_type: String,
    pub source_title: String,
    pub source_content: String,
    pub critique_content: Option<String>,
    pub allowed: Vec<String>,
    pub revision_targets: Vec<u8>,
    pub revision_count: u32,
    pub max_revisions: Option<u32>,
    pub gate_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalView {
    pub content: String,
    pub revision_summary: String,
    pub word_count: usize,
    pub revisions: usize,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectView {
    pub name: String,
    pub title: String,
    pub status: ProjectStatus,
    pub updated_at: String,
    pub next_node: Option<String>,
    pub stages: Vec<UiStage>,
    pub review: Option<ReviewView>,
    pub final_view: Option<FinalView>,
    pub doctor: DoctorReport,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub tasks_completed: Vec<String>,
    pub stopped_on: String,
    pub project: ProjectView,
}
