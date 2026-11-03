use crate::diagnostic::Diagnostic;
use crate::image::model::ImagePromptRequest;

use super::MAX_FREE_TEXT_CHARS;

/// Warns when one free-text field is large enough to hide multiple responsibilities.
///
/// The complete rendered prompt is measured after rendering in `compile_image_prompt`; measuring
/// only a hand-picked subset of source fields cannot establish the actual execution payload size.
pub(super) fn validate_free_text_budget(
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut fields: Vec<(String, usize)> = vec![
        ("$.use_case".to_owned(), request.use_case.chars().count()),
        (
            "$.scene.environment".to_owned(),
            request.scene.environment.chars().count(),
        ),
        (
            "$.scene.background".to_owned(),
            request.scene.background.chars().count(),
        ),
        (
            "$.scene.atmosphere".to_owned(),
            request.scene.atmosphere.chars().count(),
        ),
    ];
    for (index, subject) in request.subjects.iter().enumerate() {
        fields.push((
            format!("$.subjects[{index}].description"),
            subject.description.chars().count(),
        ));
        fields.push((
            format!("$.subjects[{index}].pose"),
            subject.pose.chars().count(),
        ));
    }
    for (index, reference) in request.references.iter().enumerate() {
        fields.push((
            format!("$.references[{index}].description"),
            reference.description.chars().count(),
        ));
        for (use_index, value) in reference.use_for.iter().enumerate() {
            fields.push((
                format!("$.references[{index}].use_for[{use_index}]"),
                value.chars().count(),
            ));
        }
    }
    for (index, value) in request.change_contract.change_only.iter().enumerate() {
        fields.push((
            format!("$.change_contract.change_only[{index}]"),
            value.chars().count(),
        ));
    }
    for (index, value) in request.change_contract.preserve.iter().enumerate() {
        fields.push((
            format!("$.change_contract.preserve[{index}]"),
            value.chars().count(),
        ));
    }
    if let Some(storyboard) = &request.storyboard {
        for (index, value) in storyboard.identity_anchors.iter().enumerate() {
            fields.push((
                format!("$.storyboard.identity_anchors[{index}]"),
                value.chars().count(),
            ));
        }
        for (index, panel) in storyboard.panels.iter().enumerate() {
            for (name, value) in [
                ("camera_angle", panel.camera_angle.as_str()),
                ("camera_move", panel.camera_move.as_str()),
                ("action", panel.action.as_str()),
                ("emotional_beat", panel.emotional_beat.as_str()),
            ] {
                fields.push((
                    format!("$.storyboard.panels[{index}].{name}"),
                    value.chars().count(),
                ));
            }
        }
    }
    if let Some(consistency) = &request.consistency {
        for (index, value) in consistency.shared_anchors.iter().enumerate() {
            fields.push((
                format!("$.consistency.shared_anchors[{index}]"),
                value.chars().count(),
            ));
        }
        if let Some(sequence) = &consistency.sequence {
            for (index, value) in sequence.steps.iter().enumerate() {
                fields.push((
                    format!("$.consistency.sequence.steps[{index}]"),
                    value.chars().count(),
                ));
            }
        }
    }
    for (index, text) in request.text_elements.iter().enumerate() {
        if let Some(value) = &text.spelling_hint {
            fields.push((
                format!("$.text_elements[{index}].spelling_hint"),
                value.chars().count(),
            ));
        }
    }

    for (path, count) in &fields {
        if *count > MAX_FREE_TEXT_CHARS {
            diagnostics.push(
                Diagnostic::warning(
                    "IMG_FIELD_TOO_LONG",
                    path,
                    format!(
                        "{count} characters exceeds the {MAX_FREE_TEXT_CHARS}-character field budget"
                    ),
                )
                .with_hint("split the field into the structured slots it describes"),
            );
        }
    }
}
