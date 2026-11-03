use super::candidates::{Candidate, ChangeSet};
use super::scene::{Fact, Scene, StoryThread, TimelineEntry};
use super::visual::VisualPlan;
use super::voice::VoiceProfile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema_version: u32,
    pub id: String,
    /// The story's actual title; it is also the Markdown export heading.
    pub title: String,
    /// Optional workspace-only label used to find the project in a long list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_label: Option<String>,
    /// Writer-authored one-line context; it never overrides scene text or facts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub revision: u64,
    pub content_revision: u64,
    pub scenes: BTreeMap<String, Scene>,
    pub facts: BTreeMap<String, Fact>,
    pub timeline: BTreeMap<String, TimelineEntry>,
    pub threads: BTreeMap<String, StoryThread>,
    pub candidates: BTreeMap<String, Candidate>,
    pub changesets: BTreeMap<String, ChangeSet>,
    pub visual_plans: BTreeMap<String, VisualPlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_profile: Option<VoiceProfile>,
    pub applied_commands: BTreeMap<String, u64>,
    pub validation_receipts: BTreeMap<String, crate::contracts::ValidationReceipt>,
    pub human_decisions: BTreeMap<String, crate::contracts::HumanDecision>,
    pub text_runs: BTreeMap<String, crate::contracts::TextRunState>,
    pub visual_runs: BTreeMap<String, crate::contracts::VisualRunState>,
    pub authoring: crate::authoring::State,
}

impl Project {
    pub fn empty(id: String, title: String) -> Self {
        Self {
            schema_version: super::SCHEMA_VERSION,
            id,
            title,
            display_label: None,
            summary: None,
            revision: 0,
            content_revision: 0,
            scenes: BTreeMap::new(),
            facts: BTreeMap::new(),
            timeline: BTreeMap::new(),
            threads: BTreeMap::new(),
            candidates: BTreeMap::new(),
            changesets: BTreeMap::new(),
            visual_plans: BTreeMap::new(),
            voice_profile: None,
            applied_commands: BTreeMap::new(),
            validation_receipts: BTreeMap::new(),
            human_decisions: BTreeMap::new(),
            text_runs: BTreeMap::new(),
            visual_runs: BTreeMap::new(),
            authoring: crate::authoring::State::default(),
        }
    }
}

/// Caller-supplied authority claims for recording a human decision.
///
/// `expected_artifact_revision` / `expected_artifact_digest` must match the
/// engine-computed subject artifact exactly; any mismatch is a fail-closed
/// conflict before an event is journaled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DecisionInput {
    pub actor_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub expected_artifact_revision: u64,
    pub expected_artifact_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectSummary {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceMeta {
    pub schema_version: u32,
    pub engine_version: String,
    pub created_at_ms: u64,
    pub projects: BTreeMap<String, ProjectSummary>,
}
