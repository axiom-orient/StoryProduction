//! Stable validator facade for the NarrativeSceneMap v2 contract.

use narrative_courier_runtime::token::hash_jcs;
use narrative_courier_runtime::{Error, Result};

use super::super::common::{nonempty, sha, valid_prefixed_sha256};
use super::super::NARRATIVE_SCENE_MAP_SCHEMA_VERSION;
use super::types::NarrativeSceneMap;

mod scenes;
mod semantic_manifest;

pub(super) fn bounded_nonempty(value: &str, field: &str, maximum: usize) -> Result<()> {
    nonempty(value, field)?;
    if value.chars().count() > maximum {
        return Err(Error::Contract(format!(
            "{field} must contain at most {maximum} character(s)"
        )));
    }
    Ok(())
}

pub fn validate_narrative_scene_map(v: &NarrativeSceneMap) -> Result<()> {
    if v.schema_version != NARRATIVE_SCENE_MAP_SCHEMA_VERSION {
        return Err(Error::Contract(
            "NarrativeSceneMap schema_version must be 2".into(),
        ));
    }
    if !valid_prefixed_sha256(&v.map_id, "nsm2_") {
        return Err(Error::Contract(
            "NarrativeSceneMap map_id must be nsm2_<lowercase SHA-256>".into(),
        ));
    }
    bounded_nonempty(&v.narrative_id, "narrative_id", 512)?;
    sha(&v.screenplay_sha256, "screenplay_sha256")?;
    sha(&v.scene_ir_sha256, "scene_ir_sha256")?;
    if v.scenes.is_empty() || v.scenes.len() > 10_000 {
        return Err(Error::Contract(
            "NarrativeSceneMap scenes must contain between 1 and 10000 items".into(),
        ));
    }
    scenes::validate_scenes(&v.scenes)?;
    semantic_manifest::validate_semantic_manifest(&v.semantic_manifest, &v.scenes)?;

    let mut map_value = serde_json::to_value(v)?;
    map_value
        .as_object_mut()
        .ok_or_else(|| Error::Contract("scene map is not an object".into()))?
        .insert("map_id".into(), serde_json::Value::String(String::new()));
    let expected_map_id = format!("nsm2_{}", hash_jcs(&map_value)?);
    if v.map_id != expected_map_id {
        return Err(Error::Contract("map_id hash mismatch".into()));
    }
    Ok(())
}

pub fn is_valid_scene_id(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("SCN-") else {
        return false;
    };
    rest.len() >= 3 && rest.bytes().all(|byte| byte.is_ascii_digit())
}
