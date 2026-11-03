//! Caller-authored DeliveryRequest DTO and closed validation contract.

mod validation;

use serde::{Deserialize, Serialize};

use super::asset_types::{AssetKind, AssetSource, GenerationMode};

pub use self::validation::validate_delivery_request;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeliveryIntent {
    Analysis,
    Development,
    Narrative,
    Presentation,
    Promotion,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Coverage {
    Full,
    Summary,
    Selection,
    Excerpt,
    Mood,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "UPPERCASE")]
pub enum OutputFormat {
    Html,
    Png,
    Webp,
    Svg,
    Pdf,
    Mp4,
    Webm,
    Json,
    Otio,
    Pptx,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Fps {
    pub num: u32,
    pub den: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutputRequest {
    pub output_id: String,
    pub format: OutputFormat,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub backend_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub width: Option<u32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub height: Option<u32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub aspect_ratio: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub fps: Option<Fps>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub duration_frames: Option<u32>,
    #[serde(default)]
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeliveryAssetInput {
    pub asset_id: String,
    pub kind: AssetKind,
    pub role: String,
    pub narrative_refs: Vec<String>,
    pub generation_mode: GenerationMode,
    pub source: AssetSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeliveryRequest {
    pub schema_version: u32,
    pub request_id: String,
    pub narrative_package_sha256: String,
    pub intent: DeliveryIntent,
    pub coverage: Coverage,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub audience: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub locale: Option<String>,
    #[serde(default)]
    pub must_include_refs: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub asset_inputs: Vec<DeliveryAssetInput>,
    pub outputs: Vec<OutputRequest>,
}
