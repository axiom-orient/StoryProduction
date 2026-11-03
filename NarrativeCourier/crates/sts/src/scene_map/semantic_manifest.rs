//! Deterministic semantic occurrence identities and source locators.

use super::blueprint::BlueprintAnnotations;
use narrative_courier_contracts::{NarrativeScene, SemanticEntry, SemanticManifest, SourceLocator};
use narrative_courier_runtime::token::hash_jcs;
use std::collections::BTreeMap;

pub(super) fn build_semantic_manifest(
    scenes: &[NarrativeScene],
    annotations: &BTreeMap<String, BlueprintAnnotations>,
) -> narrative_courier_runtime::Result<SemanticManifest> {
    let mut entries = Vec::new();
    for scene in scenes {
        for (occurrence_index, source_text) in scene.action.iter().enumerate() {
            entries.push(new_semantic_entry(
                &scene.scene_id,
                "action",
                source_text,
                occurrence_index,
                None,
            )?);
        }
        for (occurrence_index, dialogue) in scene.dialogue.iter().enumerate() {
            entries.push(new_semantic_entry(
                &scene.scene_id,
                "dialogue",
                &dialogue.line,
                occurrence_index,
                Some(dialogue.character.as_str()),
            )?);
        }
        let Some(scene_annotations) = annotations.get(&scene.scene_id) else {
            continue;
        };
        for kind in ["emotion", "lightingTransition", "cameraMovement"] {
            for (occurrence_index, source_text) in scene_annotations
                .directives
                .get(kind)
                .into_iter()
                .flatten()
                .enumerate()
            {
                entries.push(new_semantic_entry(
                    &scene.scene_id,
                    kind,
                    source_text,
                    occurrence_index,
                    None,
                )?);
            }
        }
    }

    let mut manifest = SemanticManifest {
        schema_version: 1,
        semantic_manifest_hash: String::new(),
        entries,
    };
    let mut value = serde_json::to_value(&manifest)?;
    value
        .as_object_mut()
        .ok_or_else(|| {
            narrative_courier_runtime::Error::Contract(
                "semantic manifest did not serialize as an object".into(),
            )
        })?
        .remove("semantic_manifest_hash");
    manifest.semantic_manifest_hash = format!("sha256:{}", hash_jcs(&value)?);
    Ok(manifest)
}

fn new_semantic_entry(
    scene_id: &str,
    kind: &str,
    source_text: &str,
    occurrence_index: usize,
    character: Option<&str>,
) -> narrative_courier_runtime::Result<SemanticEntry> {
    let occurrence_index = u32::try_from(occurrence_index).map_err(|_| {
        narrative_courier_runtime::Error::Contract("semantic occurrence exceeds u32".into())
    })?;
    let mut entry = SemanticEntry {
        // Scene identity is part of the semantic ID so repeated text and
        // equal occurrence numbers in different scenes remain distinct.
        semantic_id: format!("semantic.{kind}.{scene_id}.{occurrence_index:04}"),
        entry_hash: String::new(),
        scene_id: scene_id.into(),
        kind: kind.into(),
        source_text: source_text.into(),
        source_locator: SourceLocator {
            scene_id: scene_id.into(),
            field: kind.into(),
            occurrence_index,
            character: character.map(str::to_owned),
            dialogue_index: (kind == "dialogue").then_some(occurrence_index),
        },
    };
    let mut value = serde_json::to_value(&entry)?;
    value
        .as_object_mut()
        .ok_or_else(|| {
            narrative_courier_runtime::Error::Contract(
                "semantic entry did not serialize as an object".into(),
            )
        })?
        .remove("entry_hash");
    entry.entry_hash = format!("sha256:{}", hash_jcs(&value)?);
    Ok(entry)
}
