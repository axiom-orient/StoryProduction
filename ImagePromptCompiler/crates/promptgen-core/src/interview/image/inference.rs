#[path = "inference/subjects/appearance.rs"]
mod appearance;
#[path = "inference/subjects/counts.rs"]
mod counts;
#[path = "inference/defaults.rs"]
mod defaults;
#[path = "inference/fields/graphics.rs"]
mod field_graphics;
#[path = "inference/fields/spatial.rs"]
mod field_spatial;
#[path = "inference/fields/visual_attributes.rs"]
mod field_visual_attributes;
#[path = "inference/subjects/lexicon.rs"]
mod lexicon;
#[path = "inference/subjects/panels.rs"]
mod panels;
#[path = "inference/subjects/parser.rs"]
mod parser;
#[path = "inference/routing.rs"]
mod routing;
#[path = "inference/subjects/text.rs"]
mod text;

pub(super) use self::appearance::{extract_finish, material_terms, remove_finish_terms};
pub(super) use self::defaults::{
    aspect_ratio_preset, catalog_palette, default_aspect_ratio, default_composition,
    default_exclusions, default_lighting, default_lut, default_surface, dimensions,
    fallback_palette, find_ratio, parse_hex_palette, parse_patterns, proportions,
    selected_safety_tier, split_nonempty,
};
pub(super) use self::field_graphics::{
    extract_hex_palette, extract_quoted_texts, infer_text_position, infer_text_style,
};
pub(super) use self::field_spatial::{extract_composition_clause, extract_scene_clause};
pub(super) use self::field_visual_attributes::{
    extract_lighting_clause, extract_surface_clause, extract_wardrobe_clause, infer_lighting,
    surface_fields, wardrobe_is_specific,
};
pub(super) use self::lexicon::contains_subject_marker;
pub(super) use self::panels::{
    extract_grid, repeated_identity_anchors, repeated_panel_count, repeated_panel_identity,
    subject_relationship,
};
pub(super) use self::parser::{ParsedSubject, extract_subject_clause, parse_subjects};
pub(super) use self::routing::{
    infer_category, infer_medium, infer_profile, invalid_pattern_tokens, needs_high_quality,
    selected_text_requirement,
};

use super::{ASPECT_RATIO_PRESETS, AspectRatioPreset};

pub(super) fn split_clauses(value: &str) -> Vec<String> {
    value
        .split(['\n', ',', ';', '.', '。', '!', '?'])
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn contains_any(value: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| value.contains(term))
}

pub(super) fn is_hangul(character: char) -> bool {
    matches!(character as u32, 0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF)
}
