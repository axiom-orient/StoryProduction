//! Asset request vocabulary shared by delivery requests, plans, and indexes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AssetKind {
    Image,
    Vector,
    Audio,
    Video,
    Font,
    #[serde(rename = "MODEL_3D")]
    Model3D,
    Data,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum GenerationMode {
    User,
    Procedural,
    Generative,
    Derived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetSource {
    pub path: String,
    pub sha256: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub mime: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetRequest {
    pub asset_id: String,
    pub kind: AssetKind,
    pub role: String,
    pub narrative_refs: Vec<String>,
    pub generation_mode: GenerationMode,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub consistency_group: Option<String>,
    pub brief: String,
    pub hard_constraints: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub source: Option<AssetSource>,
}
