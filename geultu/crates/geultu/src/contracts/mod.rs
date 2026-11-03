//! Serde contract types mirroring `docs/canonical/contracts/*.schema.json`.
//!
//! Only the schemas consumed by the current implementation units are typed
//! here. Deserialization enforces `additionalProperties: false` through
//! `#[serde(deny_unknown_fields)]`; pattern/length/range constraints that
//! serde cannot express are enforced by the `validate` methods, which fail
//! closed with field-naming [`Error::InvalidInput`] messages.

mod artifact;
pub mod decisions;
pub mod digest;
mod effects;
mod human_decision;
pub mod receipts;
pub mod reviewed_synopsis;
pub mod route;
pub mod runs;
mod story_package;

pub(crate) mod validate;

pub use artifact::ArtifactRef;
pub use decisions::{DomainDecision, DomainDecisionKind, DOMAIN_DECISION_ID_PREFIX};
pub use effects::{EffectReceipt, EffectStatus};
pub use human_decision::{DecisionKind, HumanDecision};
pub use receipts::{
    ReceiptFinding, ReceiptOutcome, ReceiptSeverity, ReceiptValidator, ValidationReceipt,
};
pub use reviewed_synopsis::{
    GeultuReviewedSynopsisHandoff, SynopsisApproval, REVIEWED_SYNOPSIS_HANDOFF_SCHEMA_ID,
    REVIEWED_SYNOPSIS_HANDOFF_SCHEMA_VERSION,
};
pub use route::{AdmissionBudget, ReceiptSelector, RouteAdmission, RouteId};
pub use runs::{
    RunBudgets, RunFrozenInputs, TextRunState, TextRunStatus, VisualRunState, VisualRunStatus,
    TEXT_RUN_STATE_SCHEMA_ID, VISUAL_RUN_STATE_SCHEMA_ID,
};
pub use story_package::{
    GeultuStoryPackage, GeultuStoryPackageScene, GeultuStoryPackageSource,
    GEULTU_STORY_PACKAGE_SCHEMA_ID,
};
