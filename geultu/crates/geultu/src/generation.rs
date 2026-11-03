mod hash;
mod registry;

pub use hash::context_digest;
pub use registry::GenerationRegistry;

use crate::domain::validation::validate_id;
use crate::error::Result;
use crate::model::{Fact, Scene, StoryThread, TimelineEntry, VoiceProfile};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::fmt;

pub const NOVEL: &str = "novel";
pub const FICTION: &str = "fiction";
pub const SYNOPSIS: &str = "synopsis";
pub const STORYBOARD: &str = "storyboard";
pub const MAX_GENERATION_PROMPT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct WritingType(String);

impl WritingType {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_id("writing type", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for WritingType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct WritingStage(String);

impl WritingStage {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_id("writing stage", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for WritingStage {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Markdown,
    PlainText,
    Fountain,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub project_id: String,
    pub writing_type: WritingType,
    #[serde(default)]
    pub stage: Option<WritingStage>,
    pub brief: String,
    #[serde(default = "empty_parameters")]
    pub parameters: Value,
    #[serde(default)]
    pub expected_content_revision: Option<u64>,
}

fn empty_parameters() -> Value {
    Value::Object(Default::default())
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerationContext {
    pub project_id: String,
    pub project_title: String,
    pub source_content_revision: u64,
    pub scenes: Vec<Scene>,
    pub facts: Vec<Fact>,
    pub timeline: Vec<TimelineEntry>,
    pub threads: Vec<StoryThread>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_profile: Option<VoiceProfile>,
}

impl GenerationContext {
    pub fn from_project(project: &crate::model::Project) -> Self {
        let mut scenes: Vec<_> = project.scenes.values().cloned().collect();
        scenes.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.id.cmp(&right.id))
        });
        let mut facts: Vec<_> = project.facts.values().cloned().collect();
        facts.sort_by(|left, right| left.id.cmp(&right.id));
        let mut timeline: Vec<_> = project.timeline.values().cloned().collect();
        timeline.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.id.cmp(&right.id))
        });
        let mut threads: Vec<_> = project.threads.values().cloned().collect();
        threads.sort_by(|left, right| left.id.cmp(&right.id));
        Self {
            project_id: project.id.clone(),
            project_title: project.title.clone(),
            source_content_revision: project.content_revision,
            scenes,
            facts,
            timeline,
            threads,
            voice_profile: project.voice_profile.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerationInput {
    pub request: GenerationRequest,
    pub context: GenerationContext,
}

/// Additive H4 planning reference: verified craft-pack identity plus its
/// agent nodes, attached to plans by the application layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CraftpackPlanRef {
    pub pack_id: String,
    pub version: String,
    pub workflow_node_count: usize,
    pub workflow_manifest_sha256: String,
    pub nodes: Vec<crate::craftpack::AgentNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationPlan {
    pub project_id: String,
    pub writing_type: WritingType,
    pub stage: WritingStage,
    pub adapter_id: String,
    pub source_content_revision: u64,
    pub prompt: String,
    pub output_format: OutputFormat,
    pub input_hash: String,
    /// Additive H4 enrichment: verified craft-pack workflow reference.
    /// Populated centrally by the application after adapter preparation
    /// (and excluded from `input_hash`), so adapters stay pack-agnostic;
    /// absent when no craft pack is resolvable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub craftpack: Option<CraftpackPlanRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedDraft {
    pub content: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

pub trait WritingAdapter: fmt::Debug + Send + Sync {
    fn id(&self) -> &str;
    fn writing_type(&self) -> &WritingType;
    fn prepare(&self, input: &GenerationInput) -> Result<GenerationPlan>;
}

pub trait GenerationProvider: fmt::Debug + Send + Sync {
    fn id(&self) -> &str;
    fn generate(&self, plan: &GenerationPlan) -> Result<GeneratedDraft>;
}
