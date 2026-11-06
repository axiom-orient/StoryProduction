//! Subtitle timing selects exact original ranges; the renderer does not author dialogue.
use crate::{codec::Value, project::Release, require, Error, Result};
use std::collections::{BTreeMap, BTreeSet};

fn range(text: &[u16], start: usize, end: usize) -> Result<String> {
    String::from_utf16(
        text.get(start..end)
            .ok_or_else(|| Error::refusal("CAPTION_SOURCE_RANGE"))?,
    )
    .map_err(|_| Error::refusal("CAPTION_RANGE_SPLITS_UNICODE"))
}

pub(crate) fn validate(plan: &Value, release: &Release) -> Result<()> {
    let panels = plan["panels"].as_array().expect("sealed panel array");
    let Some(style) = plan.get("captionStyle") else {
        return require(
            panels.iter().all(|p| p.get("captionCues").is_none()),
            "CAPTION_STYLE_REQUIRED",
        );
    };
    let font = style["fontSize"].as_u64().expect("sealed font size");
    let band = style["bandHeight"].as_u64().expect("sealed band");
    let fade = style["fadeFrames"].as_u64().expect("sealed fade");
    require(
        band >= (font * 13).div_ceil(10) * 2 + 12
            && plan["canvas"]["height"]
                .as_u64()
                .is_some_and(|h| h >= band + 64),
        "CAPTION_BAND_TOO_SMALL",
    )?;
    let entries: BTreeMap<_, _> = release.scene_map["semantic_manifest"]["entries"]
        .as_array()
        .expect("released semantics")
        .iter()
        .map(|e| (e["semantic_id"].as_str().expect("released semantic ID"), e))
        .collect();
    let mut cue_ids = BTreeSet::new();
    for panel in panels {
        let cues = panel["captionCues"]
            .as_array()
            .ok_or_else(|| Error::refusal("CAPTION_CUES_REQUIRED"))?;
        let selected: Vec<_> = panel["captions"]
            .as_array()
            .expect("sealed caption IDs")
            .iter()
            .map(|s| s.as_str().expect("caption ID"))
            .collect();
        let mut order = Vec::new();
        let mut ranges: BTreeMap<&str, Vec<(usize, usize)>> = BTreeMap::new();
        let mut previous_end = 0;
        for cue in cues {
            require(
                cue_ids.insert(cue["cueId"].as_str().expect("sealed cue ID"))
                    && cue_ids.len() <= 4096,
                "DUPLICATE_OR_EXCESSIVE_CAPTION_CUE",
            )?;
            let id = cue["semanticId"].as_str().expect("sealed semantic ID");
            require(selected.contains(&id), "CAPTION_OUTSIDE_PANEL")?;
            if order.last() != Some(&id) {
                order.push(id);
            }
            let start = cue["startFrame"].as_u64().expect("sealed start");
            let end = cue["endFrame"].as_u64().expect("sealed end");
            require(
                start >= previous_end
                    && end > start
                    && end <= panel["durationFrames"].as_u64().unwrap()
                    && end - start >= fade * 2,
                "CAPTION_CUE_TIME_ORDER",
            )?;
            previous_end = end;
            ranges.entry(id).or_default().push((
                cue["startUtf16"].as_u64().unwrap() as usize,
                cue["endUtf16"].as_u64().unwrap() as usize,
            ));
        }
        require(order == selected, "CAPTION_CUE_COVERAGE_ORDER")?;
        for id in selected {
            let original: Vec<_> = entries
                .get(id)
                .ok_or_else(|| Error::refusal("CAPTION_SEMANTIC_MISSING"))?["source_text"]
                .as_str()
                .ok_or_else(|| Error::refusal("CAPTION_SOURCE_TEXT"))?
                .encode_utf16()
                .collect();
            let mut end = 0;
            for &(a, b) in &ranges[id] {
                require(
                    a >= end
                        && b > a
                        && range(&original, end, a)?.trim().is_empty()
                        && !range(&original, a, b)?.trim().is_empty(),
                    "CAPTION_SOURCE_GAP_OR_OVERLAP",
                )?;
                end = b;
            }
            require(
                range(&original, end, original.len())?.trim().is_empty(),
                "CAPTION_SOURCE_OMISSION",
            )?;
        }
    }
    Ok(())
}
