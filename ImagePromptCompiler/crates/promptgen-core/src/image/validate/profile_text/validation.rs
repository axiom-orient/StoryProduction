use crate::diagnostic::Diagnostic;
use crate::image::model::ImagePromptRequest;

use super::lexicon::{APP_ICON_FORBIDDEN_TERMS, TEXTLESS_UI_FORBIDDEN_TERMS};
use super::negation::contains_positive_forbidden_term;
use super::rendered_fields::collect_rendered_free_text;

pub(in crate::image::validate) fn validate_textless_ui_content(
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !request.text_elements.is_empty() {
        return;
    }
    for (path, value) in collect_rendered_free_text(request) {
        if contains_positive_forbidden_term(&value, TEXTLESS_UI_FORBIDDEN_TERMS) {
            diagnostics.push(
                Diagnostic::error(
                    "IMG_UI_TEXTLESS_CONTENT",
                    path,
                    "textless app/web/agent screens may describe only visual structure, navigation, state, and component relationships; readable text, numerals, pseudo-writing, invented data, and charts are not allowed",
                )
                .with_hint("keep text_elements empty and describe geometry/state without copy or data content"),
            );
        }
    }
}

pub(in crate::image::validate) fn validate_app_icon_content(
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (path, value) in collect_rendered_free_text(request) {
        if contains_positive_forbidden_term(&value, APP_ICON_FORBIDDEN_TERMS) {
            diagnostics.push(
                Diagnostic::error(
                    "IMG_APP_ICON_FORBIDDEN_CONTENT",
                    path,
                    "app_icon forbids text, letters, numerals, pseudo-writing, UI screenshots, device mockups, rounded-square masks, app-icon frames, and watermarks",
                )
                .with_hint("describe one abstract core metaphor and keep prohibited content in exclusions only"),
            );
        }
    }
}
