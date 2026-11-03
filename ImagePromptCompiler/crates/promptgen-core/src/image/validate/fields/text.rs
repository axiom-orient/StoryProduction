use std::collections::BTreeSet;

use crate::catalog::CatalogEntry;
use crate::catalog::selection::exact_text_is_required;
use crate::diagnostic::Diagnostic;
use crate::image::model::{ImagePromptRequest, ImageQuality};
use crate::image::specificity::{
    contains_hangul_and_latin, inspect_description, inspect_exact_text, inspect_tier2_youth_terms,
};

use super::common::{validate_hex, validate_nonempty, validate_placement};

pub(in crate::image::validate) fn validate_text(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut languages = BTreeSet::new();
    if exact_text_is_required(selected) && request.text_elements.is_empty() {
        let ids = selected
            .iter()
            .filter(|entry| entry.text_requirement == crate::catalog::TextRequirement::Required)
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        diagnostics.push(
            Diagnostic::error(
                "IMG_TAXONOMY_EXACT_TEXT_REQUIRED",
                "$.text_elements",
                format!("selected visual system requires exact visible text: {ids}"),
            )
            .with_hint("supply exact text lines or choose a non-letterform visual system"),
        );
    }
    for (index, text) in request.text_elements.iter().enumerate() {
        let base = format!("$.text_elements[{index}]");
        validate_nonempty(&format!("{base}.id"), &text.id, diagnostics);
        if text.lines.is_empty() {
            diagnostics.push(Diagnostic::error(
                "IMG_TEXT_LINES_EMPTY",
                format!("{base}.lines"),
                "at least one exact text line is required",
            ));
        }
        if text.lines.len() > 6 {
            diagnostics.push(Diagnostic::warning(
                "IMG_TEXT_LINES_DENSE",
                format!("{base}.lines"),
                "more than six exact lines materially increases rendering error risk",
            ));
        }
        for (line_index, line) in text.lines.iter().enumerate() {
            let path = format!("{base}.lines[{line_index}]");
            validate_nonempty(&path, line, diagnostics);
            inspect_exact_text(&path, line, diagnostics);
            if contains_hangul_and_latin(line) {
                diagnostics.push(
                    Diagnostic::error(
                        "IMG_TEXT_MIXED_SCRIPT",
                        path,
                        "a single exact text line mixes Hangul and Latin letters",
                    )
                    .with_hint("split Korean and English copy into separate labeled text elements"),
                );
            }
        }
        if !matches!(text.language.as_str(), "ko" | "en") {
            diagnostics.push(Diagnostic::error(
                "IMG_TEXT_LANGUAGE",
                format!("{base}.language"),
                "expected ko or en",
            ));
        } else {
            languages.insert(text.language.as_str());
        }
        for (name, value) in [
            ("font_family", text.font_family.as_str()),
            ("weight", text.weight.as_str()),
            ("alignment", text.alignment.as_str()),
            ("letter_spacing", text.letter_spacing.as_str()),
            ("treatment", text.treatment.as_str()),
        ] {
            let path = format!("{base}.{name}");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
        if let Some(value) = &text.spelling_hint {
            let path = format!("{base}.spelling_hint");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
            if request.output.quality != ImageQuality::High {
                diagnostics.push(Diagnostic::warning(
                    "IMG_SPELLING_HINT_QUALITY",
                    "$.output.quality",
                    "spelling-sensitive exact text should use high quality",
                ));
            }
        }
        if text.size_percent == 0 || text.size_percent > 100 {
            diagnostics.push(Diagnostic::error(
                "IMG_TEXT_SIZE",
                format!("{base}.size_percent"),
                "text size percentage must be within 1..=100",
            ));
        }
        validate_hex(&format!("{base}.color_hex"), &text.color_hex, diagnostics);
        validate_placement(&text.placement, &format!("{base}.placement"), diagnostics);
    }
    let text_heavy = request.text_elements.len() >= 3
        || request
            .text_elements
            .iter()
            .any(|text| text.lines.len() >= 3)
        || languages.len() > 1;
    if text_heavy && request.constraints.safety_tier == 0 {
        diagnostics.push(Diagnostic::warning(
            "IMG_TEXT_TIER_RECOMMENDED",
            "$.constraints.safety_tier",
            "dense or mixed-language text benefits from tier 1 exact-text guard",
        ));
    }
    if text_heavy && request.output.quality != ImageQuality::High {
        diagnostics.push(Diagnostic::warning(
            "IMG_TEXT_QUALITY",
            "$.output.quality",
            "dense exact text should use high quality",
        ));
    }
}

pub(in crate::image::validate) fn validate_constraints(
    request: &ImagePromptRequest,
    _selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if request.constraints.safety_tier > 2 {
        diagnostics.push(Diagnostic::error(
            "IMG_SAFETY_TIER",
            "$.constraints.safety_tier",
            "safety tier must be 0, 1, or 2",
        ));
    }
    for (name, values) in [
        ("required_elements", &request.constraints.required_elements),
        ("excluded_elements", &request.constraints.excluded_elements),
    ] {
        for (index, value) in values.iter().enumerate() {
            let path = format!("$.constraints.{name}[{index}]");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
    }
    if request.constraints.safety_tier == 1 && request.text_elements.is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_TIER1_WITHOUT_TEXT",
            "$.constraints.safety_tier",
            "tier 1 is the exact-text guard lane and requires at least one text element",
        ));
    }
    if request.constraints.safety_tier == 2 {
        if !request.constraints.adult_subjects_only {
            diagnostics.push(Diagnostic::error(
                "IMG_TIER2_ADULT_ASSERT",
                "$.constraints.adult_subjects_only",
                "tier 2 requires adult_subjects_only=true",
            ));
        }
        if !request.constraints.original_characters_only {
            diagnostics.push(Diagnostic::error(
                "IMG_TIER2_ORIGINAL_CHARACTER",
                "$.constraints.original_characters_only",
                "tier 2 requires original_characters_only=true",
            ));
        }
        for (path, value) in descriptive_fields(request) {
            inspect_tier2_youth_terms(&path, value, diagnostics);
        }
    }
}

pub(in crate::image::validate) fn validate_complex_output_quality(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let complex_output = selected.iter().any(|entry| entry.id == "C10");
    if complex_output && request.output.quality != ImageQuality::High {
        diagnostics.push(Diagnostic::warning(
            "IMG_COMPLEX_OUTPUT_QUALITY",
            "$.output.quality",
            "multi-item character, storyboard, and comic outputs should use high quality",
        ));
    }
}

fn descriptive_fields(request: &ImagePromptRequest) -> Vec<(String, &str)> {
    let mut fields = vec![
        ("$.use_case".to_owned(), request.use_case.as_str()),
        (
            "$.scene.environment".to_owned(),
            request.scene.environment.as_str(),
        ),
        (
            "$.scene.background".to_owned(),
            request.scene.background.as_str(),
        ),
        (
            "$.scene.atmosphere".to_owned(),
            request.scene.atmosphere.as_str(),
        ),
    ];
    for (index, subject) in request.subjects.iter().enumerate() {
        fields.push((
            format!("$.subjects[{index}].description"),
            subject.description.as_str(),
        ));
        fields.push((format!("$.subjects[{index}].pose"), subject.pose.as_str()));
        fields.push((
            format!("$.subjects[{index}].action"),
            subject.action.as_str(),
        ));
        if let Some(value) = &subject.face {
            fields.push((format!("$.subjects[{index}].face"), value.as_str()));
        }
        if let Some(value) = &subject.hair {
            fields.push((format!("$.subjects[{index}].hair"), value.as_str()));
        }
        if let Some(value) = &subject.appearance {
            fields.push((format!("$.subjects[{index}].appearance"), value.as_str()));
        }
    }
    for (index, reference) in request.references.iter().enumerate() {
        fields.push((
            format!("$.references[{index}].description"),
            reference.description.as_str(),
        ));
        for (use_index, value) in reference.use_for.iter().enumerate() {
            fields.push((
                format!("$.references[{index}].use_for[{use_index}]"),
                value.as_str(),
            ));
        }
    }
    for (index, value) in request.change_contract.change_only.iter().enumerate() {
        fields.push((
            format!("$.change_contract.change_only[{index}]"),
            value.as_str(),
        ));
    }
    for (index, value) in request.change_contract.preserve.iter().enumerate() {
        fields.push((
            format!("$.change_contract.preserve[{index}]"),
            value.as_str(),
        ));
    }
    if let Some(storyboard) = &request.storyboard {
        for (index, value) in storyboard.identity_anchors.iter().enumerate() {
            fields.push((
                format!("$.storyboard.identity_anchors[{index}]"),
                value.as_str(),
            ));
        }
        for (index, panel) in storyboard.panels.iter().enumerate() {
            for (name, value) in [
                ("camera_angle", panel.camera_angle.as_str()),
                ("camera_move", panel.camera_move.as_str()),
                ("action", panel.action.as_str()),
                ("emotional_beat", panel.emotional_beat.as_str()),
            ] {
                fields.push((format!("$.storyboard.panels[{index}].{name}"), value));
            }
        }
    }
    if let Some(consistency) = &request.consistency {
        for (index, value) in consistency.shared_anchors.iter().enumerate() {
            fields.push((
                format!("$.consistency.shared_anchors[{index}]"),
                value.as_str(),
            ));
        }
        if let Some(sequence) = &consistency.sequence {
            for (index, value) in sequence.steps.iter().enumerate() {
                fields.push((
                    format!("$.consistency.sequence.steps[{index}]"),
                    value.as_str(),
                ));
            }
        }
    }
    fields
}
