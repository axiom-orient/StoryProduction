//! Canonical scene-heading grammar and time-of-day normalization.

use super::ParsedScene;
use narrative_courier_contracts::is_valid_scene_id;

pub(super) fn parse_heading(line: &str) -> narrative_courier_runtime::Result<ParsedScene> {
    let heading = line.trim().to_string();
    let (scene_id, body) = heading.split_once(". ").ok_or_else(|| {
        narrative_courier_runtime::Error::Contract(format!(
            "malformed scene heading (expected SCN-###. INT./EXT. LOCATION - TIME): {heading}"
        ))
    })?;
    if !is_valid_scene_id(scene_id) {
        return Err(narrative_courier_runtime::Error::Contract(format!(
            "invalid scene id in heading: {scene_id}"
        )));
    }
    let (setting, location_time) = body.split_once(". ").ok_or_else(|| {
        narrative_courier_runtime::Error::Contract(format!(
            "scene heading is missing setting: {heading}"
        ))
    })?;
    let setting = setting.trim().to_ascii_uppercase();
    if !matches!(setting.as_str(), "INT" | "EXT" | "INT/EXT" | "I/E") {
        return Err(narrative_courier_runtime::Error::Contract(format!(
            "unsupported scene setting in heading: {heading}"
        )));
    }
    let (location, time) = location_time.rsplit_once(" - ").ok_or_else(|| {
        narrative_courier_runtime::Error::Contract(format!(
            "scene heading is missing time of day: {heading}"
        ))
    })?;
    let location = location.trim();
    let time = time.trim();
    if location.is_empty() || time.is_empty() {
        return Err(narrative_courier_runtime::Error::Contract(format!(
            "scene heading location/time must not be empty: {heading}"
        )));
    }
    let canonical_time = canonical_time(time).ok_or_else(|| {
        narrative_courier_runtime::Error::Contract(format!(
            "scene heading has unsupported time of day '{time}': {heading}"
        ))
    })?;
    Ok(ParsedScene {
        scene_id: scene_id.to_string(),
        heading: heading.clone(),
        setting,
        location: location.to_string(),
        time_of_day: canonical_time.to_string(),
        time_of_day_label: time.to_string(),
        characters: Vec::new(),
        action: Vec::new(),
        dialogue: Vec::new(),
        active_character: None,
    })
}

fn canonical_time(value: &str) -> Option<&'static str> {
    let lower = value.to_ascii_lowercase();
    if matches!(value, "새벽" | "여명" | "동틀 무렵")
        || lower.contains("dawn")
        || lower.contains("sunrise")
    {
        Some("dawn")
    } else if value == "아침" || lower.contains("morning") {
        Some("morning")
    } else if matches!(value, "정오" | "한낮") || lower.contains("noon") || lower.contains("midday")
    {
        Some("noon")
    } else if value == "낮" || lower == "day" {
        Some("day")
    } else if value == "오후" || lower.contains("afternoon") {
        Some("afternoon")
    } else if matches!(value, "황혼" | "해질녘" | "저녁")
        || lower.contains("dusk")
        || lower.contains("sunset")
        || lower.contains("evening")
    {
        Some("dusk")
    } else if value == "밤"
        || lower.contains("night")
        || lower.contains("midnight")
        || lower.contains("late")
    {
        Some("night")
    } else {
        None
    }
}
