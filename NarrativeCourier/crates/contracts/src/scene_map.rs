//! Private module composition for the NarrativeSceneMap v2 contract.
//!
//! The crate root exports the canonical DTO and validator names. DTO ownership
//! and semantic/source/hash validation stay in separate implementation modules.

mod types;
mod validation;

pub use types::{
    NarrativeAssetRequirement, NarrativeScene, NarrativeSceneDialogue, NarrativeSceneMap,
    SemanticEntry, SemanticManifest, SourceLocator,
};
pub use validation::{is_valid_scene_id, validate_narrative_scene_map};
