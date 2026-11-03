//! Scene-local structural invariants for NarrativeSceneMap v2.

use std::collections::BTreeSet;

use narrative_courier_runtime::{Error, Result};

use super::super::super::common::unique;
use super::super::types::NarrativeScene;
use super::{bounded_nonempty, is_valid_scene_id};

pub(super) fn validate_scenes(scenes: &[NarrativeScene]) -> Result<()> {
    let mut scene_ids = BTreeSet::new();
    for (index, scene) in scenes.iter().enumerate() {
        if !is_valid_scene_id(&scene.scene_id)
            || scene.scene_id.chars().count() > 64
            || !scene_ids.insert(&scene.scene_id)
        {
            return Err(Error::Contract(format!(
                "scene map scene_id is duplicate or invalid: {}",
                scene.scene_id
            )));
        }
        let expected_order = u32::try_from(index + 1).unwrap_or(u32::MAX);
        if scene.order != expected_order {
            return Err(Error::Contract(format!(
                "scene map order must be contiguous: expected {expected_order}, actual {}",
                scene.order
            )));
        }
        bounded_nonempty(&scene.heading, "scene heading", 16_384)?;
        bounded_nonempty(&scene.setting, "scene setting", 32)?;
        bounded_nonempty(&scene.location, "scene location", 4_096)?;
        bounded_nonempty(&scene.time_of_day, "scene time_of_day", 32)?;
        bounded_nonempty(&scene.time_of_day_label, "scene time_of_day_label", 256)?;
        if !matches!(scene.setting.as_str(), "INT" | "EXT" | "INT/EXT" | "I/E") {
            return Err(Error::Contract(format!(
                "scene {} has unsupported setting {}",
                scene.scene_id, scene.setting
            )));
        }
        if !matches!(
            scene.time_of_day.as_str(),
            "dawn" | "morning" | "noon" | "day" | "afternoon" | "dusk" | "night"
        ) {
            return Err(Error::Contract(format!(
                "scene {} has unsupported time_of_day {}",
                scene.scene_id, scene.time_of_day
            )));
        }
        if scene.characters.len() > 256 {
            return Err(Error::Contract(format!(
                "scene {} characters exceed 256 items",
                scene.scene_id
            )));
        }
        unique(&scene.characters, "scene character")?;
        for character in &scene.characters {
            bounded_nonempty(character, "scene character", 256)?;
        }
        if scene.action.len() > 10_000 {
            return Err(Error::Contract(format!(
                "scene {} action exceeds 10000 items",
                scene.scene_id
            )));
        }
        for action in &scene.action {
            bounded_nonempty(action, "scene action", 16_384)?;
        }
        if scene.dialogue.len() > 10_000 {
            return Err(Error::Contract(format!(
                "scene {} dialogue exceeds 10000 items",
                scene.scene_id
            )));
        }
        for dialogue in &scene.dialogue {
            bounded_nonempty(&dialogue.character, "dialogue.character", 256)?;
            bounded_nonempty(&dialogue.line, "dialogue.line", 16_384)?;
        }
        if scene.silent != scene.dialogue.is_empty() {
            return Err(Error::Contract(format!(
                "scene {} silent flag does not match dialogue",
                scene.scene_id
            )));
        }
        if scene.asset_requirements.len() > 1_000 {
            return Err(Error::Contract(format!(
                "scene {} asset requirements exceed 1000 items",
                scene.scene_id
            )));
        }
        for requirement in &scene.asset_requirements {
            bounded_nonempty(&requirement.kind, "asset requirement kind", 256)?;
            bounded_nonempty(
                &requirement.description,
                "asset requirement description",
                4_096,
            )?;
            bounded_nonempty(&requirement.source, "asset requirement source", 4_096)?;
        }
    }
    Ok(())
}
