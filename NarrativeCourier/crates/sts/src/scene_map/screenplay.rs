//! Narrow screenplay syntax parser facade for canonical SCN headings/dialogue.

mod dialogue;
mod heading;

use super::blueprint::BlueprintAnnotations;
use narrative_courier_contracts::{NarrativeScene, NarrativeSceneDialogue};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn parse_scenes(
    screenplay: &str,
    annotations: &BTreeMap<String, BlueprintAnnotations>,
) -> narrative_courier_runtime::Result<Vec<NarrativeScene>> {
    let mut scenes = Vec::new();
    let mut scene_ids = BTreeSet::new();
    let mut current: Option<ParsedScene> = None;
    for raw_line in screenplay.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.trim_start().starts_with("SCN-") {
            if let Some(scene) = current.take() {
                scenes.push(scene.finish(annotations));
            }
            let next = heading::parse_heading(line)?;
            if !scene_ids.insert(next.scene_id.clone()) {
                return Err(narrative_courier_runtime::Error::Contract(format!(
                    "duplicate screenplay scene: {}",
                    next.scene_id
                )));
            }
            current = Some(next);
            continue;
        }
        if let Some(scene) = current.as_mut() {
            scene.consume(line);
        }
    }
    if let Some(scene) = current.take() {
        scenes.push(scene.finish(annotations));
    }
    if scenes.is_empty() {
        return Err(narrative_courier_runtime::Error::Contract(
            "final screenplay contains no SCN-* scene headings".into(),
        ));
    }
    for scene_id in annotations.keys() {
        if !scene_ids.contains(scene_id) {
            return Err(narrative_courier_runtime::Error::Contract(format!(
                "scene blueprint references a scene absent from the screenplay: {scene_id}"
            )));
        }
    }
    for (index, scene) in scenes.iter_mut().enumerate() {
        scene.order = u32::try_from(index + 1).map_err(|_| {
            narrative_courier_runtime::Error::Contract("scene count exceeds u32".into())
        })?;
    }
    Ok(scenes)
}

#[derive(Debug)]
struct ParsedScene {
    scene_id: String,
    heading: String,
    setting: String,
    location: String,
    time_of_day: String,
    time_of_day_label: String,
    characters: Vec<String>,
    action: Vec<String>,
    dialogue: Vec<NarrativeSceneDialogue>,
    active_character: Option<String>,
}

impl ParsedScene {
    fn consume(&mut self, raw_line: &str) {
        let line = raw_line.trim();
        if line.is_empty() {
            self.active_character = None;
            return;
        }
        // Parentheticals annotate the preceding cue; they are not dialogue
        // by themselves and must not turn a silent scene into a speaking one.
        if dialogue::is_parenthetical(line) {
            return;
        }
        if raw_line
            .chars()
            .next()
            .is_some_and(|value| value.is_whitespace())
        {
            if let Some(character) = self.active_character.take() {
                self.add_dialogue(&character, line);
                self.active_character = Some(character);
                return;
            }
        }
        // Keep the original line for inline-dialogue detection. Trimming it
        // first loses the indentation that distinguishes a screenplay
        // dialogue line from an action line.
        if let Some((character, dialogue)) = dialogue::inline_dialogue(raw_line) {
            self.active_character = None;
            self.add_dialogue(character, dialogue);
            return;
        }
        if let Some(character) = dialogue::cue(line) {
            self.active_character = Some(character.to_string());
            return;
        }
        if let Some(character) = self.active_character.take() {
            self.add_dialogue(&character, line);
            return;
        }
        self.action.push(line.to_string());
    }

    fn add_dialogue(&mut self, character: &str, line: &str) {
        let character = character.trim();
        let line = line.trim();
        if character.is_empty() || line.is_empty() {
            return;
        }
        if !self.characters.iter().any(|value| value == character) {
            self.characters.push(character.to_string());
        }
        self.dialogue.push(NarrativeSceneDialogue {
            character: character.to_string(),
            line: line.to_string(),
        });
    }

    fn finish(self, annotations: &BTreeMap<String, BlueprintAnnotations>) -> NarrativeScene {
        let asset_requirements = annotations
            .get(&self.scene_id)
            .map(|value| value.requirements.clone())
            .unwrap_or_default();
        NarrativeScene {
            scene_id: self.scene_id,
            order: 0,
            heading: self.heading,
            setting: self.setting,
            location: self.location,
            time_of_day: self.time_of_day,
            time_of_day_label: self.time_of_day_label,
            silent: self.dialogue.is_empty(),
            characters: self.characters,
            action: self.action,
            dialogue: self.dialogue,
            asset_requirements,
        }
    }
}
