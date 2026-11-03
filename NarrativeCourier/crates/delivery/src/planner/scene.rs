//! Canonical screenplay scene-boundary extraction for deterministic delivery.

use narrative_courier_contracts::is_valid_scene_id;
use narrative_courier_runtime::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub content: String,
}

pub fn extract_scenes(screenplay: &str) -> Result<Vec<Scene>> {
    let mut headings: Vec<(usize, String)> = Vec::new();
    let mut offset = 0usize;
    for line in screenplay.split_inclusive('\n') {
        let raw = line.trim_start();
        if raw.starts_with("SCN-") {
            let id = raw
                .split_once(". ")
                .map(|(id, _)| id)
                .filter(|id| is_valid_scene_id(id))
                .ok_or_else(|| {
                    Error::Contract("LINEAGE_MISSING: malformed canonical SCN heading".into())
                })?;
            headings.push((offset, id.to_owned()));
        }
        offset += line.len();
    }
    if headings.is_empty() {
        return Err(Error::Contract(
            "LINEAGE_MISSING: final screenplay contains no canonical SCN-* headings".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    for (_, id) in &headings {
        if !seen.insert(id.as_str()) {
            return Err(Error::Contract(format!(
                "duplicate screenplay scene id: {id}"
            )));
        }
    }
    let mut scenes = Vec::new();
    for i in 0..headings.len() {
        let start = if i == 0 { 0 } else { headings[i].0 };
        let end = headings.get(i + 1).map(|v| v.0).unwrap_or(screenplay.len());
        scenes.push(Scene {
            id: headings[i].1.clone(),
            content: screenplay[start..end].to_string(),
        });
    }
    Ok(scenes)
}

#[cfg(test)]
mod tests {
    use super::extract_scenes;

    #[test]
    fn extract_scenes_preserves_canonical_order_and_exact_text() {
        let screenplay =
            "SCN-001. INT. ROOM - DAY\n\nMAYA waits.\n\nSCN-002. EXT. STREET - NIGHT\n\nJAX runs.";
        let scenes = extract_scenes(screenplay).expect("canonical scenes");
        assert_eq!(
            scenes
                .iter()
                .map(|scene| scene.id.as_str())
                .collect::<Vec<_>>(),
            vec!["SCN-001", "SCN-002"]
        );
        assert_eq!(
            scenes[0].content,
            "SCN-001. INT. ROOM - DAY\n\nMAYA waits.\n\n"
        );
        assert_eq!(
            scenes[1].content,
            "SCN-002. EXT. STREET - NIGHT\n\nJAX runs."
        );
    }
}
