//! Scene-blueprint annotation routing owned by the STS scene-map projection.

use narrative_courier_contracts::{is_valid_scene_id, NarrativeAssetRequirement};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(super) struct BlueprintAnnotations {
    pub(super) requirements: Vec<NarrativeAssetRequirement>,
    pub(super) directives: BTreeMap<String, Vec<String>>,
}

pub(super) fn parse(
    text: &str,
) -> narrative_courier_runtime::Result<BTreeMap<String, BlueprintAnnotations>> {
    let mut out: BTreeMap<String, BlueprintAnnotations> = BTreeMap::new();
    let mut current = None;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with("SCN-") {
            let scene_id = line
                .split_once(". ")
                .map(|(value, _)| value)
                .filter(|value| is_valid_scene_id(value))
                .ok_or_else(|| {
                    narrative_courier_runtime::Error::Contract(format!(
                        "malformed scene blueprint heading: {line}"
                    ))
                })?;
            if out
                .insert(scene_id.to_owned(), BlueprintAnnotations::default())
                .is_some()
            {
                return Err(narrative_courier_runtime::Error::Contract(format!(
                    "duplicate scene blueprint heading: {scene_id}"
                )));
            }
            current = Some(scene_id);
            continue;
        }
        let directive = directive_line(line);
        let requirement = requirement_line(line);
        if directive.is_none() && requirement.is_none() {
            continue;
        }
        let scene_id = current.ok_or_else(|| {
            narrative_courier_runtime::Error::Contract(
                "scene blueprint annotation precedes any scene heading".into(),
            )
        })?;
        if let Some((kind, description)) = directive {
            if description.is_empty() {
                return Err(narrative_courier_runtime::Error::Contract(format!(
                    "scene blueprint directive {kind} must not be empty in {scene_id}"
                )));
            }
            out.get_mut(scene_id)
                .expect("scene heading was admitted above")
                .directives
                .entry(kind.into())
                .or_default()
                .push(description.into());
            continue;
        }
        let Some((kind, description)) = requirement else {
            continue;
        };
        if description.is_empty() {
            return Err(narrative_courier_runtime::Error::Contract(format!(
                "scene blueprint asset requirement is empty in {scene_id}"
            )));
        }
        let item = NarrativeAssetRequirement {
            kind: kind.into(),
            description: description.into(),
            source: line.into(),
        };
        let entries = &mut out
            .get_mut(scene_id)
            .expect("scene heading was admitted above")
            .requirements;
        if !entries.iter().any(|existing| existing == &item) {
            entries.push(item);
        }
    }
    Ok(out)
}

fn directive_line(line: &str) -> Option<(&'static str, &str)> {
    let (label, description) = bullet_label_description(line)?;
    let kind = if label.eq_ignore_ascii_case("Emotion") {
        "emotion"
    } else if label.eq_ignore_ascii_case("Lighting transition") {
        "lightingTransition"
    } else if label.eq_ignore_ascii_case("Camera movement") {
        "cameraMovement"
    } else {
        return None;
    };
    Some((kind, description.trim()))
}

fn requirement_line(line: &str) -> Option<(&str, &str)> {
    let (label, description) = bullet_label_description(line)?;
    let lower = label.to_ascii_lowercase();
    let hinted = lower.contains("asset")
        || lower.contains("prop")
        || label.contains("소품")
        || label.contains("자산");
    hinted.then_some(("asset", description))
}

fn bullet_label_description(line: &str) -> Option<(&str, &str)> {
    let value = line.strip_prefix("- ")?;
    let (label, description) = value.split_once(':')?;
    Some((label.trim(), description.trim()))
}
