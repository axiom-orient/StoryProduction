//! Stable DirectorPlan DTO and validator facade.

mod validation;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::asset_types::AssetRequest;

pub use self::validation::validate_director_plan;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GrammarAxis {
    pub must: Vec<String>,
    pub avoid: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ProjectionOperation {
    Preserve,
    Compress,
    Excerpt,
    Reorder,
    Abstract,
    Omit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectionUnit {
    pub unit_id: String,
    pub order: u32,
    pub narrative_refs: Vec<String>,
    pub operation: ProjectionOperation,
    pub direction: String,
    pub asset_refs: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub content: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub duration_frames: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PresentationStructure {
    Linear,
    Scroll,
    Grid,
    Paged,
    Timed,
    Graph,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeliveryPlan {
    pub output_request_id: String,
    pub presentation_structure: PresentationStructure,
    pub units: Vec<ProjectionUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DirectorPlan {
    pub schema_version: u32,
    pub plan_id: String,
    pub narrative_package_sha256: String,
    pub delivery_request_sha256: String,
    pub director_bundle_sha256: String,
    pub visual_grammar: BTreeMap<String, GrammarAxis>,
    pub asset_requests: Vec<AssetRequest>,
    pub deliveries: Vec<DeliveryPlan>,
}
