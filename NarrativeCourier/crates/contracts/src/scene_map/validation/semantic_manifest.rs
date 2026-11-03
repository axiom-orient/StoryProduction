//! Semantic occurrence, source-locator, and manifest-hash invariants.

mod entries;

use std::collections::{BTreeMap, BTreeSet};

use narrative_courier_runtime::token::hash_jcs;
use narrative_courier_runtime::{Error, Result};

use self::entries::{prefixed_hash, validate_entry};
use super::super::types::{NarrativeScene, SemanticManifest};

pub(super) fn validate_semantic_manifest(
    manifest: &SemanticManifest,
    scenes: &[NarrativeScene],
) -> Result<()> {
    if manifest.schema_version != 1 {
        return Err(Error::Contract(
            "semantic_manifest schema_version must be 1".into(),
        ));
    }
    prefixed_hash(
        &manifest.semantic_manifest_hash,
        "semantic_manifest.semantic_manifest_hash",
    )?;
    if manifest.entries.len() > 100_000 {
        return Err(Error::Contract(
            "semantic_manifest.entries must contain at most 100000 items".into(),
        ));
    }

    let scene_by_id: BTreeMap<&str, &NarrativeScene> = scenes
        .iter()
        .map(|scene| (scene.scene_id.as_str(), scene))
        .collect();
    let mut semantic_ids = BTreeSet::new();
    let mut locators = BTreeSet::new();
    let mut action_refs: BTreeMap<&str, BTreeSet<u32>> = scene_by_id
        .keys()
        .map(|scene_id| (*scene_id, BTreeSet::new()))
        .collect();
    let mut dialogue_refs: BTreeMap<&str, BTreeSet<u32>> = scene_by_id
        .keys()
        .map(|scene_id| (*scene_id, BTreeSet::new()))
        .collect();
    let mut directive_refs: BTreeMap<(&str, &str), BTreeSet<u32>> = BTreeMap::new();

    for (index, entry) in manifest.entries.iter().enumerate() {
        validate_entry(entry, index, &mut semantic_ids, &mut locators)?;

        let scene = scene_by_id.get(entry.scene_id.as_str()).ok_or_else(|| {
            Error::Contract(format!(
                "entry {} references unknown scene_id",
                entry.semantic_id
            ))
        })?;
        let occurrence = entry.source_locator.occurrence_index;
        match entry.kind.as_str() {
            "action" => {
                let action = scene.action.get(occurrence as usize).ok_or_else(|| {
                    Error::Contract(format!(
                        "action occurrence_index {occurrence} is missing in {}",
                        entry.scene_id
                    ))
                })?;
                if entry.source_text != *action {
                    return Err(Error::Contract(format!(
                        "action source_text drift at {}[{occurrence}]",
                        entry.scene_id
                    )));
                }
                action_refs
                    .get_mut(entry.scene_id.as_str())
                    .expect("scene checked above")
                    .insert(occurrence);
            }
            "dialogue" => {
                let dialogue_index = entry
                    .source_locator
                    .dialogue_index
                    .expect("dialogue index checked above");
                let dialogue = scene.dialogue.get(dialogue_index as usize).ok_or_else(|| {
                    Error::Contract(format!(
                        "dialogue_index {dialogue_index} is missing in {}",
                        entry.scene_id
                    ))
                })?;
                if entry.source_locator.character.as_deref() != Some(dialogue.character.as_str()) {
                    return Err(Error::Contract(format!(
                        "dialogue character drift at {}[{dialogue_index}]",
                        entry.scene_id
                    )));
                }
                if entry.source_text != dialogue.line {
                    return Err(Error::Contract(format!(
                        "dialogue source_text drift at {}[{dialogue_index}]",
                        entry.scene_id
                    )));
                }
                dialogue_refs
                    .get_mut(entry.scene_id.as_str())
                    .expect("scene checked above")
                    .insert(dialogue_index);
            }
            directive_kind => {
                directive_refs
                    .entry((entry.scene_id.as_str(), directive_kind))
                    .or_default()
                    .insert(occurrence);
            }
        }

        let mut body = serde_json::to_value(entry)?;
        body.as_object_mut()
            .ok_or_else(|| Error::Contract("semantic entry is not an object".into()))?
            .remove("entry_hash");
        let expected = format!("sha256:{}", hash_jcs(&body)?);
        if entry.entry_hash != expected {
            return Err(Error::Contract(format!(
                "entry_hash mismatch at semantic_manifest.entries[{index}]"
            )));
        }
    }

    for scene in scenes {
        let expected_actions = (0..scene.action.len() as u32).collect::<BTreeSet<_>>();
        if action_refs[scene.scene_id.as_str()] != expected_actions {
            return Err(Error::Contract(format!(
                "action manifest is not a bijection for {}",
                scene.scene_id
            )));
        }
        let expected_dialogue = (0..scene.dialogue.len() as u32).collect::<BTreeSet<_>>();
        if dialogue_refs[scene.scene_id.as_str()] != expected_dialogue {
            return Err(Error::Contract(format!(
                "dialogue manifest is not a bijection for {}",
                scene.scene_id
            )));
        }
    }
    for ((scene_id, kind), occurrences) in directive_refs {
        let expected = (0..occurrences.len() as u32).collect::<BTreeSet<_>>();
        if occurrences != expected {
            return Err(Error::Contract(format!(
                "{kind} manifest occurrence indices are not contiguous for {scene_id}"
            )));
        }
    }

    let mut manifest_value = serde_json::to_value(manifest)?;
    manifest_value
        .as_object_mut()
        .ok_or_else(|| Error::Contract("semantic manifest is not an object".into()))?
        .remove("semantic_manifest_hash");
    let expected_manifest_hash = format!("sha256:{}", hash_jcs(&manifest_value)?);
    if manifest.semantic_manifest_hash != expected_manifest_hash {
        return Err(Error::Contract("semantic_manifest_hash mismatch".into()));
    }
    Ok(())
}
