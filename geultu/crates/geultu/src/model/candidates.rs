use super::scene::{Fact, StoryThread, TimelineEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub provider: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_hash: Option<String>,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: String,
    pub kind: String,
    pub source_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writing_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    pub content: String,
    pub receipt: Receipt,
    pub status: CandidateStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChangeOperation {
    ReplaceSceneText {
        scene_id: String,
        expected_scene_revision: u64,
        text: String,
    },
    AddFact {
        fact: Fact,
    },
    AddTimeline {
        entry: TimelineEntry,
    },
    AddThread {
        thread: StoryThread,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeSetStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChangeSet {
    pub id: String,
    pub candidate_id: String,
    pub source_revision: u64,
    pub operations: Vec<ChangeOperation>,
    pub status: ChangeSetStatus,
}
