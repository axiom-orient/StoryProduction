use std::collections::BTreeSet;

use crate::catalog::{CatalogEntry, CatalogFamily};
use crate::diagnostic::Diagnostic;
use crate::image::model::{
    ImagePromptRequest, ImageQuality, ImageReferenceRole, ImageRenderProfile, ImageTaskMode,
    VisualMedium,
};
use crate::image::specificity::inspect_description;

use super::super::fields::common::validate_nonempty;

pub(in crate::image::validate) fn validate_render_profile(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if request.render_profile == ImageRenderProfile::PoseTransfer {
        validate_pose_transfer_profile(request, diagnostics);
        return;
    }
    if request.render_profile == ImageRenderProfile::CinematicStoryboard {
        validate_cinematic_storyboard_profile(request, selected, diagnostics);
        return;
    }
    if request.render_profile != ImageRenderProfile::EditorialFlat {
        return;
    }
    if request.task_mode != ImageTaskMode::Generate
        || !request.references.is_empty()
        || !request.change_contract.change_only.is_empty()
        || !request.change_contract.preserve.is_empty()
    {
        diagnostics.push(Diagnostic::error(
            "IMG_EDITORIAL_FLAT_TASK_MODE",
            "$.render_profile",
            "editorial_flat supports text-to-image generation only; use structured for references, edits, or composites",
        ));
    }
    if request.medium != VisualMedium::Photo {
        diagnostics.push(Diagnostic::error(
            "IMG_EDITORIAL_FLAT_MEDIUM",
            "$.medium",
            "editorial_flat is a photographic single-subject editorial profile",
        ));
    }
    if !selected.iter().any(|entry| entry.id == "C1") {
        diagnostics.push(Diagnostic::error(
            "IMG_EDITORIAL_FLAT_CATEGORY",
            "$.taxonomy.category",
            "editorial_flat requires category C1",
        ));
    }
    for entry in selected.iter().filter(|entry| !entry.is_primary_output()) {
        if !matches!(entry.family, CatalogFamily::Editorial | CatalogFamily::Look) {
            diagnostics.push(Diagnostic::error(
                "IMG_EDITORIAL_FLAT_PATTERN",
                "$.taxonomy.patterns",
                format!(
                    "editorial_flat supports only editorial and look directions; {} belongs to {}",
                    entry.id,
                    entry.family.as_str()
                ),
            ));
        }
    }
    if request.subjects.len() != 1
        || request
            .subjects
            .first()
            .is_some_and(|subject| subject.count != 1)
    {
        diagnostics.push(Diagnostic::error(
            "IMG_EDITORIAL_FLAT_SUBJECT",
            "$.subjects",
            "editorial_flat requires exactly one subject with count=1",
        ));
    }
    if !request.text_elements.is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_EDITORIAL_FLAT_TEXT",
            "$.text_elements",
            "editorial_flat is the no-text solo editorial lane; use structured for exact image text",
        ));
    }
    if request.output.aspect_ratio_label() != "2:3" {
        diagnostics.push(Diagnostic::warning(
            "IMG_EDITORIAL_FLAT_AR",
            "$.output",
            "the original Format B contract uses 2:3 as its standard editorial aspect ratio",
        ));
    }
}

fn validate_cinematic_storyboard_profile(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if request.task_mode != ImageTaskMode::Generate
        || !request.references.is_empty()
        || !request.change_contract.change_only.is_empty()
        || !request.change_contract.preserve.is_empty()
    {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_TASK_MODE",
            "$.render_profile",
            "cinematic_storyboard supports text-to-image generation only; preserve references and edits through structured or pose_transfer",
        ));
    }
    if request.medium != VisualMedium::Illustration {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_MEDIUM",
            "$.medium",
            "cinematic_storyboard requires illustration so panel line, color, and character continuity remain explicit",
        ));
    }
    if !selected.iter().any(|entry| entry.id == "C10") {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_CATEGORY",
            "$.taxonomy.category",
            "cinematic_storyboard is the C10 illustration-and-story outcome profile",
        ));
    }
    if !request.text_elements.is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_TEXT",
            "$.text_elements",
            "cinematic_storyboard is a textless visual handoff; keep labels, dialogue, panel numbers, logos, and UI outside the generated sheet",
        ));
    }
    if request.output.quality != ImageQuality::High {
        diagnostics.push(
            Diagnostic::warning(
                "IMG_CINEMATIC_STORYBOARD_QUALITY",
                "$.output.quality",
                "panel-to-panel character, gaze, and framing continuity are most reliable at high quality",
            )
            .with_hint("use quality=high unless generation cost is the deliberate tradeoff"),
        );
    }

    let Some(storyboard) = &request.storyboard else {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_REQUIRED",
            "$.storyboard",
            "cinematic_storyboard requires a typed layout, identity anchors, screen direction, and ordered panel plan",
        ));
        return;
    };

    let expected_count = match storyboard.layout.as_str() {
        "three_panel_strip" => 3,
        "two_by_two" => 4,
        "three_by_two" => 6,
        _ => {
            diagnostics.push(Diagnostic::error(
                "IMG_CINEMATIC_STORYBOARD_LAYOUT",
                "$.storyboard.layout",
                "layout must be three_panel_strip, two_by_two, or three_by_two",
            ));
            0
        }
    };
    if storyboard.panels.len() != expected_count {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_PANEL_COUNT",
            "$.storyboard.panels",
            format!(
                "{} requires exactly {expected_count} ordered panels, found {}",
                storyboard.layout,
                storyboard.panels.len()
            ),
        ));
    }
    if !matches!(
        storyboard.screen_direction.as_str(),
        "left_to_right" | "right_to_left"
    ) {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_DIRECTION",
            "$.storyboard.screen_direction",
            "screen_direction must be left_to_right or right_to_left",
        ));
    }
    if !(3..=6).contains(&storyboard.identity_anchors.len()) {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_IDENTITY",
            "$.storyboard.identity_anchors",
            "declare 3 to 6 visible identity anchors such as hair, costume, prop, silhouette, and palette role",
        ));
    }
    let mut ids = BTreeSet::new();
    for (index, panel) in storyboard.panels.iter().enumerate() {
        let base = format!("$.storyboard.panels[{index}]");
        if !ids.insert(panel.id.trim()) {
            diagnostics.push(Diagnostic::error(
                "IMG_CINEMATIC_STORYBOARD_PANEL_ID",
                format!("{base}.id"),
                "every storyboard panel id must be unique",
            ));
        }
        for (name, value) in [
            ("id", panel.id.as_str()),
            ("camera_angle", panel.camera_angle.as_str()),
            ("camera_move", panel.camera_move.as_str()),
            ("action", panel.action.as_str()),
            ("emotional_beat", panel.emotional_beat.as_str()),
        ] {
            validate_nonempty(&format!("{base}.{name}"), value, diagnostics);
            inspect_description(&format!("{base}.{name}"), value, diagnostics);
        }
    }
}

fn validate_pose_transfer_profile(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    if request.task_mode != ImageTaskMode::Composite {
        diagnostics.push(Diagnostic::error(
            "IMG_POSE_TRANSFER_TASK_MODE",
            "$.task_mode",
            "pose_transfer requires composite mode because pose geometry and character identity come from separate references",
        ));
    }

    let pose_count = request
        .references
        .iter()
        .filter(|reference| reference.role == ImageReferenceRole::Pose)
        .count();
    let subject_count = request
        .references
        .iter()
        .filter(|reference| reference.role == ImageReferenceRole::Subject)
        .count();
    if request.references.len() != 2 || pose_count != 1 || subject_count != 1 {
        diagnostics.push(Diagnostic::error(
            "IMG_POSE_TRANSFER_REFERENCES",
            "$.references",
            "pose_transfer requires exactly two references: one role=pose source and one role=subject character sheet",
        ));
    }

    if request.subjects.len() != 1
        || request
            .subjects
            .first()
            .is_some_and(|subject| subject.count != 1)
    {
        diagnostics.push(Diagnostic::error(
            "IMG_POSE_TRANSFER_SUBJECT",
            "$.subjects",
            "pose_transfer requires exactly one output subject with count=1",
        ));
    }

    if request.output.quality != ImageQuality::High {
        diagnostics.push(
            Diagnostic::warning(
                "IMG_POSE_TRANSFER_QUALITY",
                "$.output.quality",
                "pose, hands, feet, facing, and gaze are most reliable at high quality",
            )
            .with_hint("use quality=high unless generation cost is the deliberate tradeoff"),
        );
    }
}
