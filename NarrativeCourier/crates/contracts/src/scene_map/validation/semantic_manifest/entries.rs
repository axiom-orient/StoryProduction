//! Semantic-entry identifiers, field bounds, and source-locator contract.

use std::collections::BTreeSet;

use super::super::super::types::{SemanticEntry, SourceLocator};
use super::super::{bounded_nonempty, is_valid_scene_id};
use crate::common::valid_prefixed_sha256;
use narrative_courier_runtime::{Error, Result};

pub(super) const SEMANTIC_KINDS: [&str; 5] = [
    "action",
    "dialogue",
    "emotion",
    "lightingTransition",
    "cameraMovement",
];

pub(super) fn valid_semantic_id(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("semantic.") else {
        return false;
    };
    let mut chars = rest.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && rest.chars().count() <= 128
        && chars.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

pub(super) fn prefixed_hash(value: &str, field: &str) -> Result<()> {
    if valid_prefixed_sha256(value, "sha256:") {
        Ok(())
    } else {
        Err(Error::Contract(format!(
            "{field} must be sha256:<lowercase SHA-256>"
        )))
    }
}

pub(super) fn validate_source_locator(
    locator: &SourceLocator,
    entry_kind: &str,
    entry_scene: &str,
) -> Result<()> {
    if !is_valid_scene_id(&locator.scene_id) || locator.scene_id.chars().count() > 64 {
        return Err(Error::Contract("source_locator.scene_id is invalid".into()));
    }
    if locator.scene_id != entry_scene {
        return Err(Error::Contract(
            "source_locator.scene_id must equal scene_id".into(),
        ));
    }
    if !SEMANTIC_KINDS.contains(&locator.field.as_str()) {
        return Err(Error::Contract(format!(
            "source_locator.field is unsupported: {}",
            locator.field
        )));
    }
    if locator.field != entry_kind {
        return Err(Error::Contract(
            "source_locator.field must equal kind".into(),
        ));
    }
    if locator.occurrence_index > 10_000 {
        return Err(Error::Contract(
            "source_locator.occurrence_index must be at most 10000".into(),
        ));
    }
    if let Some(character) = &locator.character {
        bounded_nonempty(character, "source_locator.character", 256)?;
    }
    if let Some(dialogue_index) = locator.dialogue_index {
        if dialogue_index > 10_000 {
            return Err(Error::Contract(
                "source_locator.dialogue_index must be at most 10000".into(),
            ));
        }
    }
    if locator.field == "dialogue" {
        if locator.character.is_none() {
            return Err(Error::Contract(
                "source_locator.character is required for dialogue".into(),
            ));
        }
        let Some(dialogue_index) = locator.dialogue_index else {
            return Err(Error::Contract(
                "source_locator.dialogue_index is required for dialogue".into(),
            ));
        };
        if dialogue_index != locator.occurrence_index {
            return Err(Error::Contract(
                "source_locator.dialogue_index must equal occurrence_index".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_entry<'a>(
    entry: &'a SemanticEntry,
    index: usize,
    semantic_ids: &mut BTreeSet<&'a String>,
    locators: &mut BTreeSet<(&'a str, &'a str, u32)>,
) -> Result<()> {
    if !valid_semantic_id(&entry.semantic_id) || entry.semantic_id.chars().count() > 137 {
        return Err(Error::Contract(format!(
            "semantic_manifest.entries[{index}].semantic_id is invalid"
        )));
    }
    if !semantic_ids.insert(&entry.semantic_id) {
        return Err(Error::Contract(format!(
            "duplicate semantic_id: {}",
            entry.semantic_id
        )));
    }
    prefixed_hash(
        &entry.entry_hash,
        &format!("semantic_manifest.entries[{index}].entry_hash"),
    )?;
    if !is_valid_scene_id(&entry.scene_id) || entry.scene_id.chars().count() > 64 {
        return Err(Error::Contract(format!(
            "semantic_manifest.entries[{index}].scene_id is invalid"
        )));
    }
    if !SEMANTIC_KINDS.contains(&entry.kind.as_str()) {
        return Err(Error::Contract(format!(
            "semantic_manifest.entries[{index}].kind is unsupported: {}",
            entry.kind
        )));
    }
    bounded_nonempty(&entry.source_text, "semantic_manifest.source_text", 16_384)?;
    validate_source_locator(&entry.source_locator, &entry.kind, &entry.scene_id)?;
    let locator_key = (
        entry.source_locator.scene_id.as_str(),
        entry.source_locator.field.as_str(),
        entry.source_locator.occurrence_index,
    );
    if !locators.insert(locator_key) {
        return Err(Error::Contract(format!(
            "duplicate source_locator: {:?}",
            locator_key
        )));
    }
    Ok(())
}
