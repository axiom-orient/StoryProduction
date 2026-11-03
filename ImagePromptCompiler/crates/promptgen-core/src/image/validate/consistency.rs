use std::collections::BTreeSet;

use crate::diagnostic::Diagnostic;
use crate::image::model::{ImageConsistencyDimension, ImagePromptRequest, ImageRenderProfile};
use crate::image::specificity::inspect_description;

use super::fields::common::validate_nonempty;

pub(super) fn validate_consistency(
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(consistency) = &request.consistency else {
        if request.render_profile == ImageRenderProfile::CinematicStoryboard {
            diagnostics.push(Diagnostic::error(
                "IMG_CINEMATIC_STORYBOARD_CONSISTENCY_REQUIRED",
                "$.consistency",
                "cinematic_storyboard requires explicit multi-image consistency dimensions, shared anchors, and temporal sequence",
            ));
        }
        return;
    };

    if consistency.dimensions.is_empty() || consistency.dimensions.len() > 4 {
        diagnostics.push(Diagnostic::error(
            "IMG_CONSISTENCY_DIMENSIONS_COUNT",
            "$.consistency.dimensions",
            "declare one to four consistency dimensions",
        ));
    }
    let mut dimensions = BTreeSet::new();
    for (index, dimension) in consistency.dimensions.iter().enumerate() {
        if !dimensions.insert(dimension.as_str()) {
            diagnostics.push(Diagnostic::error(
                "IMG_CONSISTENCY_DIMENSION_DUPLICATE",
                format!("$.consistency.dimensions[{index}]"),
                format!("consistency dimension {} is duplicated", dimension.as_str()),
            ));
        }
    }

    if consistency.shared_anchors.is_empty() || consistency.shared_anchors.len() > 8 {
        diagnostics.push(Diagnostic::error(
            "IMG_CONSISTENCY_ANCHORS_COUNT",
            "$.consistency.shared_anchors",
            "declare one to eight observable invariants shared by the requested images",
        ));
    }
    let mut anchors = BTreeSet::new();
    for (index, anchor) in consistency.shared_anchors.iter().enumerate() {
        let path = format!("$.consistency.shared_anchors[{index}]");
        validate_nonempty(&path, anchor, diagnostics);
        inspect_description(&path, anchor, diagnostics);
        if !anchors.insert(anchor.trim().to_lowercase()) {
            diagnostics.push(Diagnostic::error(
                "IMG_CONSISTENCY_ANCHOR_DUPLICATE",
                path,
                "shared_anchors contains a duplicate invariant",
            ));
        }
    }

    let has_temporal = dimensions.contains("temporal");
    match (&consistency.sequence, has_temporal) {
        (None, true) => diagnostics.push(Diagnostic::error(
            "IMG_CONSISTENCY_TEMPORAL_SEQUENCE_REQUIRED",
            "$.consistency.sequence",
            "temporal consistency requires an ordered sequence",
        )),
        (Some(sequence), true) => {
            if sequence.steps.len() < 2 || sequence.steps.len() > 24 {
                diagnostics.push(Diagnostic::error(
                    "IMG_CONSISTENCY_SEQUENCE_COUNT",
                    "$.consistency.sequence.steps",
                    "temporal sequences must contain two to twenty-four ordered steps",
                ));
            }
            if !sequence.condition_on_previous {
                diagnostics.push(Diagnostic::error(
                    "IMG_CONSISTENCY_TEMPORAL_CONDITIONING",
                    "$.consistency.sequence.condition_on_previous",
                    "temporal consistency must explicitly condition each step on the previous output",
                ));
            }
        }
        (Some(sequence), false) => {
            if sequence.steps.len() < 2 || sequence.steps.len() > 24 {
                diagnostics.push(Diagnostic::error(
                    "IMG_CONSISTENCY_SEQUENCE_COUNT",
                    "$.consistency.sequence.steps",
                    "ordered sequences must contain two to twenty-four steps",
                ));
            }
        }
        (None, false) => {}
    }
    if let Some(sequence) = &consistency.sequence {
        let mut steps = BTreeSet::new();
        for (index, step) in sequence.steps.iter().enumerate() {
            let path = format!("$.consistency.sequence.steps[{index}]");
            validate_nonempty(&path, step, diagnostics);
            inspect_description(&path, step, diagnostics);
            if !steps.insert(step.trim().to_lowercase()) {
                diagnostics.push(Diagnostic::error(
                    "IMG_CONSISTENCY_SEQUENCE_DUPLICATE",
                    path,
                    "sequence steps must be unique and ordered",
                ));
            }
        }
    }

    if request.render_profile != ImageRenderProfile::CinematicStoryboard {
        return;
    }
    let required_dimensions = [
        ImageConsistencyDimension::Character.as_str(),
        ImageConsistencyDimension::Temporal.as_str(),
        ImageConsistencyDimension::Semantic.as_str(),
    ];
    for dimension in required_dimensions {
        if !dimensions.contains(dimension) {
            diagnostics.push(Diagnostic::error(
                "IMG_CINEMATIC_STORYBOARD_CONSISTENCY_DIMENSION",
                "$.consistency.dimensions",
                format!("cinematic_storyboard requires the {dimension} consistency dimension"),
            ));
        }
    }
    let Some(storyboard) = &request.storyboard else {
        return;
    };
    for (index, identity_anchor) in storyboard.identity_anchors.iter().enumerate() {
        if !consistency
            .shared_anchors
            .iter()
            .any(|candidate| candidate.trim() == identity_anchor.trim())
        {
            diagnostics.push(Diagnostic::error(
                "IMG_CINEMATIC_STORYBOARD_ANCHOR_MISSING",
                format!("$.storyboard.identity_anchors[{index}]"),
                "every storyboard identity anchor must also be declared as a shared consistency anchor",
            ));
        }
    }
    if let Some(sequence) = &consistency.sequence {
        if sequence.steps.len() != storyboard.panels.len() {
            diagnostics.push(Diagnostic::error(
                "IMG_CINEMATIC_STORYBOARD_SEQUENCE_COUNT",
                "$.consistency.sequence.steps",
                format!(
                    "storyboard sequence must contain exactly {} panel steps, found {}",
                    storyboard.panels.len(),
                    sequence.steps.len()
                ),
            ));
        } else {
            for (index, (step, panel)) in sequence
                .steps
                .iter()
                .zip(storyboard.panels.iter())
                .enumerate()
            {
                if step.trim() != panel.id.trim() {
                    diagnostics.push(Diagnostic::error(
                        "IMG_CINEMATIC_STORYBOARD_SEQUENCE_ORDER",
                        format!("$.consistency.sequence.steps[{index}]"),
                        format!(
                            "storyboard sequence step must match panel id {:?}, found {:?}",
                            panel.id, step
                        ),
                    ));
                }
            }
        }
    }
}
