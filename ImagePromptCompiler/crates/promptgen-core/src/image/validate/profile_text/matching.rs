use super::lexicon::{UI_RELATION_TERMS, UI_STRUCTURE_TERMS, UI_SURFACE_TERMS};

pub(super) fn is_ui_token_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
}

pub(super) fn ui_term_spans(value: &str, terms: &[&str]) -> Vec<(usize, usize)> {
    let lower = value.to_ascii_lowercase();
    terms
        .iter()
        .flat_map(|term| {
            if term
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
            {
                lower
                    .match_indices(term)
                    .filter_map(|(start, _)| {
                        let end = start + term.len();
                        let before = lower[..start].chars().next_back();
                        let after = lower[end..].chars().next();
                        (before.is_none_or(|character| !is_ui_token_character(character))
                            && after.is_none_or(|character| !is_ui_token_character(character)))
                        .then_some((start, end))
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
            } else {
                lower
                    .match_indices(term)
                    .map(|(start, _)| (start, start + term.len()))
                    .collect::<Vec<_>>()
                    .into_iter()
            }
        })
        .collect()
}

fn contains_ui_term(value: &str, terms: &[&str]) -> bool {
    !ui_term_spans(value, terms).is_empty()
}

pub(in crate::image::validate) fn contains_ui_surface_component_relation(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let surfaces = ui_term_spans(value, UI_SURFACE_TERMS);
    let components = ui_term_spans(value, UI_STRUCTURE_TERMS);
    surfaces.iter().any(|(_, surface_end)| {
        components.iter().any(|(component_start, _)| {
            *component_start >= *surface_end
                && contains_ui_term(&lower[*surface_end..*component_start], UI_RELATION_TERMS)
        })
    })
}
