pub const SCHEMA_VERSION: u32 = 1;

/// NarrativePackage deliberately moved to a new contract identity. The
/// other delivery contracts remain on the shared schema version 1.
pub const NARRATIVE_PACKAGE_SCHEMA_VERSION: u32 = 2;

pub const NARRATIVE_SCENE_MAP_SCHEMA_VERSION: u32 = 2;

pub const NVC_RELEASE_BUNDLE_SCHEMA_VERSION: u32 = 1;

pub const NARRATIVE_PACKAGE_SCHEMA_ID: &str =
    "https://narrative-courier.local/contracts/narrative-package/v2.schema.json";

pub const NARRATIVE_SCENE_MAP_SCHEMA_ID: &str =
    "https://narrative-courier.local/contracts/narrative-scene-map/v2.schema.json";

pub const NVC_RELEASE_BUNDLE_SCHEMA_ID: &str =
    "https://narrative-courier.local/contracts/release-bundle/v1.schema.json";

mod artifact;
mod asset_index;
mod asset_types;
mod common;
mod delivery_set;
mod director;
mod narrative;
mod release_bundle;
mod request;
mod scene_map;
mod synopsis_handoff;

pub use artifact::{ArtifactRef, CanonicalNarrative, NarrativeAuthority, StsDomainRef};
pub use asset_index::{validate_asset_index, AssetIndex, AssetItem};
pub use asset_types::{AssetKind, AssetRequest, AssetSource, GenerationMode};
pub use common::is_narrative_ref;
pub use delivery_set::{
    validate_delivery_set, DeliveryReceipt, DeliverySet, DeliveryStatus, DeliveryVerification,
    VerificationLayer, VerificationStatus,
};
pub use director::{
    validate_director_plan, DeliveryPlan, DirectorPlan, GrammarAxis, PresentationStructure,
    ProjectionOperation, ProjectionUnit,
};
pub use narrative::{validate_narrative_package, NarrativePackage};
pub use release_bundle::{validate_nvc_release_bundle, NvcReleaseBundle};
pub use request::{
    validate_delivery_request, Coverage, DeliveryAssetInput, DeliveryIntent, DeliveryRequest, Fps,
    OutputFormat, OutputRequest,
};
pub use scene_map::{
    is_valid_scene_id, validate_narrative_scene_map, NarrativeAssetRequirement, NarrativeScene,
    NarrativeSceneDialogue, NarrativeSceneMap, SemanticEntry, SemanticManifest, SourceLocator,
};
pub use synopsis_handoff::{
    validate_reviewed_synopsis_handoff, validate_reviewed_synopsis_handoff_identity,
    ReviewedSynopsisApproval, ReviewedSynopsisHandoff,
};
