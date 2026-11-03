//! Shared normalization dispatch and structural summary builder.

mod fdx;
mod fountain;

use super::{NormalizedScreenplay, ScreenplaySummary, ScriptFormat};
use std::collections::BTreeSet;

pub fn detect_format(
    path: &str,
    body: &str,
) -> Result<ScriptFormat, narrative_courier_runtime::Error> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".fountain") {
        return Ok(ScriptFormat::Fountain);
    }
    if lower.ends_with(".fdx") || body.contains("<FinalDraft") {
        return Ok(ScriptFormat::Fdx);
    }
    Err(narrative_courier_runtime::Error::Contract(
        "unsupported screenplay format: use .fountain or .fdx".into(),
    ))
}

pub fn normalize_screenplay(
    format: ScriptFormat,
    body: &str,
) -> Result<NormalizedScreenplay, narrative_courier_runtime::Error> {
    match format {
        ScriptFormat::Fountain => fountain::normalize_fountain(body),
        ScriptFormat::Fdx => fdx::normalize_fdx(body),
    }
}

#[derive(Default)]
struct Builder {
    blocks: Vec<String>,
    scenes: usize,
    cast: BTreeSet<String>,
    interiors: usize,
    exteriors: usize,
    skipped: usize,
}

impl Builder {
    fn push_block(&mut self, block: String) {
        if !block.trim().is_empty() {
            self.blocks.push(block);
        }
    }

    fn push_heading(&mut self, heading: String) {
        let upper = heading.to_ascii_uppercase();
        let mut setting_kind: Option<bool> = None;
        for token in upper.split_whitespace() {
            let slug = token.trim_end_matches('.');
            if slug == "EXT" {
                setting_kind = Some(true);
                break;
            }
            if slug == "INT" || slug == "EST" || slug == "I/E" {
                setting_kind = Some(false);
                break;
            }
        }
        match setting_kind {
            Some(true) => self.exteriors += 1,
            Some(false) => self.interiors += 1,
            None => {}
        }
        self.scenes += 1;
        self.push_block(heading);
    }

    fn finish(self, title_page: Option<String>) -> NormalizedScreenplay {
        let mut builder = self;
        if let Some(title) = title_page {
            builder.blocks.insert(0, title);
        }
        let text = format!("{}\n", builder.blocks.join("\n\n"));
        let summary = ScreenplaySummary {
            scene_count: builder.scenes,
            interior_count: builder.interiors,
            exterior_count: builder.exteriors,
            cast: builder.cast.into_iter().collect(),
            skipped_lines: builder.skipped,
        };
        NormalizedScreenplay { text, summary }
    }
}
