use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostic::Diagnostic;
use crate::image::lut::{PRESET_NAMES, resolve_photo_lut};
use crate::image::model::{ColorSpec, ImagePromptRequest, PhotoLutSelection, VisualMedium};
use crate::image::specificity::inspect_description;

use super::common::{register_id, validate_hex, validate_nonempty, validate_placement};

pub(in crate::image::validate) fn validate_scene(
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let fields = [
        ("$.scene.environment", request.scene.environment.as_str()),
        ("$.scene.background", request.scene.background.as_str()),
        ("$.scene.atmosphere", request.scene.atmosphere.as_str()),
    ];
    for (path, value) in fields {
        validate_nonempty(path, value, diagnostics);
        inspect_description(path, value, diagnostics);
    }
    if let Some(value) = &request.scene.time_of_day {
        validate_nonempty("$.scene.time_of_day", value, diagnostics);
        inspect_description("$.scene.time_of_day", value, diagnostics);
    }
    if let Some(value) = &request.scene.weather {
        validate_nonempty("$.scene.weather", value, diagnostics);
        inspect_description("$.scene.weather", value, diagnostics);
    }
}

pub(in crate::image::validate) fn validate_elements(
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if request.subjects.is_empty()
        && request.text_elements.is_empty()
        && request.surfaces.is_empty()
    {
        diagnostics.push(Diagnostic::error(
            "IMG_NO_VISUAL_ELEMENT",
            "$",
            "at least one subject, text element, or surface is required",
        ));
    }
    let mut ids: BTreeMap<String, String> = BTreeMap::new();
    for (index, subject) in request.subjects.iter().enumerate() {
        let base = format!("$.subjects[{index}]");
        register_id(&mut ids, &subject.id, &format!("{base}.id"), diagnostics);
        if subject.count == 0 {
            diagnostics.push(Diagnostic::error(
                "IMG_SUBJECT_COUNT",
                format!("{base}.count"),
                "subject count must be at least 1",
            ));
        }
        let fields = [
            ("description", subject.description.as_str()),
            ("scale", subject.scale.as_str()),
            ("pose", subject.pose.as_str()),
            ("gaze", subject.gaze.as_str()),
            ("action", subject.action.as_str()),
        ];
        for (name, value) in fields {
            let path = format!("{base}.{name}");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
        if let Some(value) = &subject.face {
            validate_nonempty(&format!("{base}.face"), value, diagnostics);
            inspect_description(&format!("{base}.face"), value, diagnostics);
        }
        if let Some(value) = &subject.hair {
            validate_nonempty(&format!("{base}.hair"), value, diagnostics);
            inspect_description(&format!("{base}.hair"), value, diagnostics);
        }
        if let Some(value) = &subject.appearance {
            validate_nonempty(&format!("{base}.appearance"), value, diagnostics);
            inspect_description(&format!("{base}.appearance"), value, diagnostics);
        }
        for (feature_index, value) in subject.distinguishing_features.iter().enumerate() {
            let path = format!("{base}.distinguishing_features[{feature_index}]");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
        validate_placement(
            &subject.placement,
            &format!("{base}.placement"),
            diagnostics,
        );
    }
    for (index, surface) in request.surfaces.iter().enumerate() {
        let base = format!("$.surfaces[{index}]");
        register_id(&mut ids, &surface.id, &format!("{base}.id"), diagnostics);
        for (name, value) in [
            ("material", surface.material.as_str()),
            ("finish", surface.finish.as_str()),
            ("micro_detail", surface.micro_detail.as_str()),
            ("light_response", surface.light_response.as_str()),
        ] {
            let path = format!("{base}.{name}");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
    }
    for (index, text) in request.text_elements.iter().enumerate() {
        register_id(
            &mut ids,
            &text.id,
            &format!("$.text_elements[{index}].id"),
            diagnostics,
        );
    }
}

pub(in crate::image::validate) fn validate_composition(
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (name, value) in [
        ("framing", request.composition.framing.as_str()),
        ("viewpoint", request.composition.viewpoint.as_str()),
        ("camera_angle", request.composition.camera_angle.as_str()),
        ("balance", request.composition.balance.as_str()),
    ] {
        let path = format!("$.composition.{name}");
        validate_nonempty(&path, value, diagnostics);
        inspect_description(&path, value, diagnostics);
    }
    for (index, value) in request.composition.depth_layers.iter().enumerate() {
        let path = format!("$.composition.depth_layers[{index}]");
        validate_nonempty(&path, value, diagnostics);
        inspect_description(&path, value, diagnostics);
    }
    if request.composition.visual_hierarchy.is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_HIERARCHY_EMPTY",
            "$.composition.visual_hierarchy",
            "visual hierarchy must name every subject and exact text element in priority order",
        ));
        return;
    }
    let known = request
        .subjects
        .iter()
        .map(|subject| subject.id.as_str())
        .chain(request.text_elements.iter().map(|text| text.id.as_str()))
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for (index, id) in request.composition.visual_hierarchy.iter().enumerate() {
        if !known.contains(id.as_str()) {
            diagnostics.push(Diagnostic::error(
                "IMG_HIERARCHY_UNKNOWN",
                format!("$.composition.visual_hierarchy[{index}]"),
                format!("unknown visual element id {id:?}"),
            ));
        }
        if !seen.insert(id.as_str()) {
            diagnostics.push(Diagnostic::error(
                "IMG_HIERARCHY_DUPLICATE",
                format!("$.composition.visual_hierarchy[{index}]"),
                format!("visual element id {id:?} appears more than once"),
            ));
        }
    }
    for id in known.difference(&seen) {
        diagnostics.push(Diagnostic::error(
            "IMG_HIERARCHY_INCOMPLETE",
            "$.composition.visual_hierarchy",
            format!("visual element id {id:?} is missing from the hierarchy"),
        ));
    }
}

pub(in crate::image::validate) fn validate_camera_and_lighting(
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if request.medium == VisualMedium::Photo && request.camera.is_none() {
        diagnostics.push(Diagnostic::warning(
            "IMG_PHOTO_CAMERA_MISSING",
            "$.camera",
            "photo medium is more deterministic with explicit camera outcome fields",
        ));
    }
    if let Some(camera) = &request.camera {
        for (name, value) in [
            ("field_of_view", camera.field_of_view.as_str()),
            ("perspective", camera.perspective.as_str()),
            ("depth_of_field", camera.depth_of_field.as_str()),
            ("focus", camera.focus.as_str()),
            ("motion_rendering", camera.motion_rendering.as_str()),
        ] {
            let path = format!("$.camera.{name}");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
    }
    for (name, value) in [
        ("key_direction", request.lighting.key_direction.as_str()),
        ("key_quality", request.lighting.key_quality.as_str()),
        (
            "fill_description",
            request.lighting.fill_description.as_str(),
        ),
        (
            "shadow_character",
            request.lighting.shadow_character.as_str(),
        ),
        ("exposure", request.lighting.exposure.as_str()),
    ] {
        let path = format!("$.lighting.{name}");
        validate_nonempty(&path, value, diagnostics);
        inspect_description(&path, value, diagnostics);
    }
    if let Some(value) = &request.lighting.rim_description {
        validate_nonempty("$.lighting.rim_description", value, diagnostics);
        inspect_description("$.lighting.rim_description", value, diagnostics);
    }
    if let Some(kelvin) = request.lighting.key_temperature_kelvin
        && !(1_000..=20_000).contains(&kelvin)
    {
        diagnostics.push(Diagnostic::error(
            "IMG_KELVIN_RANGE",
            "$.lighting.key_temperature_kelvin",
            "color temperature must be between 1000 K and 20000 K",
        ));
    }
    if let Some(color) = &request.lighting.key_color_hex {
        validate_hex("$.lighting.key_color_hex", color, diagnostics);
    }
    if let Some(ratio) = request.lighting.key_to_fill_ratio
        && (!ratio.is_finite() || ratio <= 0.0 || ratio > 20.0)
    {
        diagnostics.push(Diagnostic::error(
            "IMG_LIGHT_RATIO",
            "$.lighting.key_to_fill_ratio",
            "key-to-fill ratio must be finite and within (0, 20]",
        ));
    }
}

pub(in crate::image::validate) fn validate_color(
    color: &ColorSpec,
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !(3..=5).contains(&color.palette.len()) {
        diagnostics.push(Diagnostic::error(
            "IMG_PALETTE_COUNT",
            "$.color.palette",
            "palette must contain 3 to 5 colors",
        ));
    }
    let mut total = 0_u16;
    let mut colors = BTreeSet::new();
    for (index, entry) in color.palette.iter().enumerate() {
        validate_hex(
            &format!("$.color.palette[{index}].hex"),
            &entry.hex,
            diagnostics,
        );
        total += u16::from(entry.proportion_percent);
        if !colors.insert(entry.hex.to_ascii_uppercase()) {
            diagnostics.push(Diagnostic::error(
                "IMG_PALETTE_DUPLICATE",
                format!("$.color.palette[{index}].hex"),
                "palette colors must be unique",
            ));
        }
        validate_nonempty(
            &format!("$.color.palette[{index}].usage"),
            &entry.usage,
            diagnostics,
        );
        inspect_description(
            &format!("$.color.palette[{index}].usage"),
            &entry.usage,
            diagnostics,
        );
    }
    if total != 100 {
        diagnostics.push(Diagnostic::error(
            "IMG_PALETTE_SUM",
            "$.color.palette",
            format!("palette proportions must sum to 100, found {total}"),
        ));
    }
    for (name, value) in [
        ("harmony", color.harmony.as_str()),
        ("contrast", color.contrast.as_str()),
        ("saturation", color.saturation.as_str()),
    ] {
        let path = format!("$.color.{name}");
        validate_nonempty(&path, value, diagnostics);
        inspect_description(&path, value, diagnostics);
    }
    if let Some(selection) = &color.photo_lut {
        validate_lut(selection, request, diagnostics);
    }
}

fn validate_lut(
    selection: &PhotoLutSelection,
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !PRESET_NAMES.contains(&selection.preset.as_str()) {
        diagnostics.push(Diagnostic::error(
            "IMG_LUT_PRESET",
            "$.color.photo_lut.preset",
            format!("unknown LUT preset {:?}", selection.preset),
        ));
    }
    for (name, value) in [
        ("strength_percent", Some(selection.strength_percent)),
        ("saturation_percent", selection.saturation_percent),
        ("grain_amount_percent", selection.grain_amount_percent),
        ("halation_percent", selection.halation_percent),
        ("vignette_percent", selection.vignette_percent),
    ] {
        if let Some(value) = value
            && value > 100
        {
            diagnostics.push(Diagnostic::error(
                "IMG_LUT_PERCENT",
                format!("$.color.photo_lut.{name}"),
                "percentage must be between 0 and 100",
            ));
        }
    }
    if let Some(kelvin) = selection.white_balance_kelvin
        && !(1_000..=20_000).contains(&kelvin)
    {
        diagnostics.push(Diagnostic::error(
            "IMG_LUT_KELVIN",
            "$.color.photo_lut.white_balance_kelvin",
            "white balance must be between 1000 K and 20000 K",
        ));
    }
    if let Some(value) = &selection.shadow_bias_hex {
        validate_hex("$.color.photo_lut.shadow_bias_hex", value, diagnostics);
    }
    if let Some(value) = &selection.highlight_bias_hex {
        validate_hex("$.color.photo_lut.highlight_bias_hex", value, diagnostics);
    }
    for (name, value) in [
        ("tint", selection.tint.as_deref()),
        ("tone_curve", selection.tone_curve.as_deref()),
        ("black_response", selection.black_response.as_deref()),
        ("contrast", selection.contrast.as_deref()),
        ("highlight_rolloff", selection.highlight_rolloff.as_deref()),
        ("skin_tone_policy", selection.skin_tone_policy.as_deref()),
        ("grain_size", selection.grain_size.as_deref()),
    ] {
        if let Some(value) = value {
            let path = format!("$.color.photo_lut.{name}");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
    }
    if resolve_photo_lut(selection).is_none() && PRESET_NAMES.contains(&selection.preset.as_str()) {
        diagnostics.push(Diagnostic::error(
            "IMG_LUT_RESOLUTION",
            "$.color.photo_lut",
            "LUT could not be resolved",
        ));
    }
    if request.medium != VisualMedium::Photo {
        diagnostics.push(Diagnostic::warning(
            "IMG_LUT_NONPHOTO",
            "$.color.photo_lut",
            "photo LUT is a prompt-level color response profile; non-photo media may interpret it loosely",
        ));
    }
}
