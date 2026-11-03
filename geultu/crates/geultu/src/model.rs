//! Stable facade for persisted project values and public model DTOs.
//!
//! The child modules separate narrative content, candidates, feature values,
//! project state, and report projections while keeping the existing
//! `geultu::model::*` paths and serde contracts.

mod candidates;
mod reports;
mod scene;
mod visual;
mod voice;
mod workspace;

pub const SCHEMA_VERSION: u32 = 2;
pub const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;
pub const VOICE_PROFILE_SCHEMA_VERSION: u32 = 1;

pub use self::candidates::{
    Candidate, CandidateStatus, ChangeOperation, ChangeSet, ChangeSetStatus, Receipt,
};
pub use self::reports::{Finding, ProofReport, ReviewReport, Severity, StyleMetrics};
pub use self::scene::{EvidenceRef, Fact, Scene, StoryThread, ThreadStatus, TimelineEntry};
pub use self::visual::{Shot, VisualPlan};
pub use self::voice::{VoiceComparison, VoiceGuide, VoiceMeasurements, VoiceProfile};
pub use self::workspace::{DecisionInput, Project, ProjectSummary, WorkspaceMeta};
