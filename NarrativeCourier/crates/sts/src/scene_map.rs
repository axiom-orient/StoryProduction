//! Deterministic production of the STS-owned scene-map authority artifact.
//!
//! The facade binds source bytes and delegates syntax parsing, blueprint
//! annotation extraction, and occurrence identity to focused internal modules.

mod blueprint;
mod screenplay;
mod semantic_manifest;

use narrative_courier_contracts::{
    validate_narrative_scene_map, NarrativeSceneMap, NARRATIVE_SCENE_MAP_SCHEMA_VERSION,
};
use narrative_courier_runtime::store::sha256_bytes;
use narrative_courier_runtime::token::hash_jcs;

pub fn build_scene_map(
    narrative_id: &str,
    screenplay: &[u8],
    scene_ir: &[u8],
) -> narrative_courier_runtime::Result<NarrativeSceneMap> {
    let screenplay_text = std::str::from_utf8(screenplay).map_err(|error| {
        narrative_courier_runtime::Error::Contract(format!(
            "final screenplay is not UTF-8: {error}"
        ))
    })?;
    let scene_ir_text = std::str::from_utf8(scene_ir).map_err(|error| {
        narrative_courier_runtime::Error::Contract(format!("scene blueprint is not UTF-8: {error}"))
    })?;
    let annotations = blueprint::parse(scene_ir_text)?;
    let scenes = screenplay::parse_scenes(screenplay_text, &annotations)?;
    let semantic_manifest = semantic_manifest::build_semantic_manifest(&scenes, &annotations)?;
    let mut map = NarrativeSceneMap {
        schema_version: NARRATIVE_SCENE_MAP_SCHEMA_VERSION,
        map_id: String::new(),
        narrative_id: narrative_id.into(),
        screenplay_sha256: sha256_bytes(screenplay),
        scene_ir_sha256: sha256_bytes(scene_ir),
        scenes,
        semantic_manifest,
    };
    map.map_id = format!("nsm2_{}", hash_jcs(&serde_json::to_value(&map)?)?);
    validate_narrative_scene_map(&map)?;
    Ok(map)
}
