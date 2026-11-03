//! Wire DTOs and public vocabulary for NarrativeSceneMap v2.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NarrativeSceneDialogue {
    pub character: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NarrativeAssetRequirement {
    pub kind: String,
    pub description: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NarrativeScene {
    pub scene_id: String,
    pub order: u32,
    pub heading: String,
    pub setting: String,
    pub location: String,
    pub time_of_day: String,
    pub time_of_day_label: String,
    pub silent: bool,
    pub characters: Vec<String>,
    pub action: Vec<String>,
    pub dialogue: Vec<NarrativeSceneDialogue>,
    pub asset_requirements: Vec<NarrativeAssetRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NarrativeSceneMap {
    pub schema_version: u32,
    pub map_id: String,
    pub narrative_id: String,
    pub screenplay_sha256: String,
    pub scene_ir_sha256: String,
    pub scenes: Vec<NarrativeScene>,
    pub semantic_manifest: SemanticManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SemanticManifest {
    pub schema_version: u32,
    pub semantic_manifest_hash: String,
    pub entries: Vec<SemanticEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SemanticEntry {
    pub semantic_id: String,
    pub entry_hash: String,
    pub scene_id: String,
    pub kind: String,
    pub source_text: String,
    pub source_locator: SourceLocator,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceLocator {
    pub scene_id: String,
    pub field: String,
    pub occurrence_index: u32,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::super::common::deserialize_present"
    )]
    pub character: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::super::common::deserialize_present"
    )]
    pub dialogue_index: Option<u32>,
}
