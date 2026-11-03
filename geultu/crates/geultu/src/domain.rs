mod command;
mod reducer;
pub(crate) mod validation;

pub use command::decide;
pub use reducer::evolve;

use crate::contracts::{HumanDecision, TextRunState, ValidationReceipt, VisualRunState};
use crate::model::{
    Candidate, ChangeOperation, ChangeSet, Fact, Scene, StoryThread, ThreadStatus, TimelineEntry,
    VisualPlan, VoiceProfile,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Authoring {
        mutation: crate::authoring::Mutation,
    },
    AddScene {
        scene: Scene,
    },
    SetSceneText {
        scene_id: String,
        text: String,
        expected_scene_revision: Option<u64>,
        #[serde(default)]
        run_id: Option<String>,
    },
    ImportScenes {
        scenes: Vec<Scene>,
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
    SetThreadStatus {
        thread_id: String,
        status: ThreadStatus,
    },
    AddCandidate {
        candidate: Candidate,
    },
    PrepareSceneCandidate {
        candidate: Candidate,
        scene_id: String,
        expected_scene_revision: u64,
        changeset_id: String,
    },
    RejectCandidate {
        candidate_id: String,
        decision: HumanDecision,
    },
    AddChangeSet {
        changeset: ChangeSet,
    },
    ApproveChangeSet {
        changeset_id: String,
        expected_project_revision: u64,
        #[serde(default)]
        run_id: Option<String>,
        decision: HumanDecision,
    },
    RejectChangeSet {
        changeset_id: String,
        decision: HumanDecision,
    },
    AddVisualPlan {
        plan: VisualPlan,
    },
    ActivateVoiceProfile {
        profile: VoiceProfile,
        expected_project_revision: u64,
    },
    RecordValidationReceipt {
        receipt: ValidationReceipt,
    },
    SetProjectMetadata {
        title: String,
        display_label: Option<String>,
        summary: Option<String>,
    },
    OpenTextRun {
        run: TextRunState,
    },
    AdmitVisualPackage {
        run: VisualRunState,
        receipt: ValidationReceipt,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    AuthoringApplied {
        mutation: crate::authoring::Mutation,
    },
    ProjectCreated {
        id: String,
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },
    ProjectMetadataSet {
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },
    SceneAdded {
        scene: Scene,
    },
    SceneTextSet {
        scene_id: String,
        text: String,
    },
    ScenesImported {
        scenes: Vec<Scene>,
    },
    FactAdded {
        fact: Fact,
    },
    TimelineAdded {
        entry: TimelineEntry,
    },
    ThreadAdded {
        thread: StoryThread,
    },
    ThreadStatusSet {
        thread_id: String,
        status: ThreadStatus,
    },
    CandidateAdded {
        candidate: Candidate,
    },
    CandidateRejected {
        candidate_id: String,
    },
    ChangeSetAdded {
        changeset: ChangeSet,
    },
    ChangeSetApproved {
        changeset_id: String,
        candidate_id: String,
        operations: Vec<ChangeOperation>,
    },
    ChangeSetRejected {
        changeset_id: String,
    },
    VisualPlanAdded {
        plan: VisualPlan,
    },
    VoiceProfileActivated {
        profile: VoiceProfile,
    },
    ValidationReceiptRecorded {
        receipt: ValidationReceipt,
    },
    HumanDecisionRecorded {
        decision: HumanDecision,
    },
    TextRunOpened {
        run: TextRunState,
    },
    TextRunBudgetSpent {
        run_id: String,
        revision_units: u64,
    },
    VisualRunAdmitted {
        run: VisualRunState,
    },
}

impl Event {
    pub(crate) fn affects_content_revision(&self) -> bool {
        matches!(
            self,
            Self::SceneAdded { .. }
                | Self::SceneTextSet { .. }
                | Self::ScenesImported { .. }
                | Self::FactAdded { .. }
                | Self::TimelineAdded { .. }
                | Self::ThreadAdded { .. }
                | Self::ThreadStatusSet { .. }
                | Self::ChangeSetApproved { .. }
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventEnvelope {
    pub schema_version: u32,
    pub seq: u64,
    pub command_id: String,
    pub timestamp_ms: u64,
    pub event: Event,
}
