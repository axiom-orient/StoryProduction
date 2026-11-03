mod common;
mod generation_plan;
pub(crate) mod lut;
mod model;
mod render;
mod schema;
mod specificity;
mod validate;

pub use self::common::PromptLanguage;
pub use self::lut::available_lut_presets;
pub use self::model::{
    BackgroundMode, CameraSpec, CanvasPlacement, CanvasZone, CinematicStoryboardPanel,
    CinematicStoryboardSpec, ColorSpec, CompositionSpec, ImageChangeContract,
    ImageConsistencyDimension, ImageConsistencySequence, ImageConsistencySpec, ImageConstraints,
    ImageGenerationParameters, ImageProfile, ImagePromptRequest, ImageQuality, ImageReference,
    ImageReferenceRole, ImageRenderProfile, ImageTaskMode, LightingSpec, PaletteEntry,
    PhotoLutSelection, SceneSpec, SubjectSpec, SurfaceSpec, TaxonomySelection, TextElement,
    TextRole, VisualMedium,
};
pub use self::schema::image_schema;
pub use self::validate::validate_image_request;

use crate::diagnostic::{CompilationOutcome, Diagnostic, PromptKind};
use crate::json::JsonValue;

const MAX_RENDERED_PROMPT_CHARS: usize = 16_000;

pub fn compile_image_prompt(request: &ImagePromptRequest) -> CompilationOutcome {
    let (mut diagnostics, selected) = validate_image_request(request);
    let prompt = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error)
    {
        None
    } else {
        let prompt = render::render_image_prompt(request, &selected);
        let length = prompt.chars().count();
        if length > MAX_RENDERED_PROMPT_CHARS {
            diagnostics.push(
                Diagnostic::warning(
                    "IMG_PROMPT_TOO_LONG",
                    "$",
                    format!(
                        "rendered prompt is {length} characters, over the local {MAX_RENDERED_PROMPT_CHARS}-character execution budget"
                    ),
                )
                .with_hint("shorten the longest structured fields before spending a generation call"),
            );
        }
        if request.render_profile == ImageRenderProfile::EditorialFlat
            && !(300..=550).contains(&length)
        {
            diagnostics.push(Diagnostic::warning(
                "IMG_EDITORIAL_FLAT_LENGTH",
                "$.render_profile",
                format!(
                    "editorial_flat rendered length is {length} characters; the original Format B target band is 300..=550"
                ),
            ));
        }
        Some(prompt)
    };
    CompilationOutcome::new(
        PromptKind::Image,
        prompt,
        diagnostics,
        JsonValue::object([
            (
                "catalog_entries",
                crate::catalog::selection::selected_catalog_to_json(&selected),
            ),
            ("compiler", JsonValue::from("promptgen-core/1")),
            (
                "generation_plan",
                generation_plan::generation_plan(request, &selected),
            ),
            ("language", JsonValue::from(request.language.code())),
            ("profile", JsonValue::from(request.profile.as_str())),
            ("request", request.to_json()),
        ]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    fn example() -> ImagePromptRequest {
        ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-photo-lut.json"
            )))
            .expect("JSON"),
        )
        .expect("request")
    }

    #[test]
    fn rendered_prompt_length_covers_fields_outside_the_partial_source_budget() {
        let mut request = example();
        request.constraints.required_elements = vec!["가".repeat(MAX_RENDERED_PROMPT_CHARS)];

        let outcome = compile_image_prompt(&request);

        assert!(outcome.prompt.is_some());
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "IMG_PROMPT_TOO_LONG")
        );
    }

    #[test]
    fn pose_transfer_generation_plan_exposes_ordered_visual_qa() {
        let request = ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-pose-transfer.json"
            )))
            .expect("JSON"),
        )
        .expect("request");

        let outcome = compile_image_prompt(&request);
        let metadata = outcome.metadata.to_pretty_string();
        assert!(outcome.prompt.is_some(), "{:#?}", outcome.diagnostics);
        for token in [
            "pose_fidelity_gate",
            "whole_pose",
            "limb_detail",
            "facing_gaze",
            "character_identity",
        ] {
            assert!(metadata.contains(token), "missing {token:?}\n{metadata}");
        }
    }

    #[test]
    fn cinematic_storyboard_generation_plan_exposes_continuity_qa() {
        let request = ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-cinematic-storyboard.json"
            )))
            .expect("JSON"),
        )
        .expect("request");

        let outcome = compile_image_prompt(&request);
        let metadata = outcome.metadata.to_pretty_string();
        assert!(outcome.prompt.is_some(), "{:#?}", outcome.diagnostics);
        for token in [
            "earliest_storyboard_gate",
            "\"consistency\"",
            "multi_view",
            "consistency_character",
            "identity_anchors",
            "beat_progression",
            "screen_direction_eyeline",
            "textless_handoff",
        ] {
            assert!(metadata.contains(token), "missing {token:?}\n{metadata}");
        }
    }

    #[test]
    fn app_icon_generation_plan_exposes_purpose_silhouette_and_png_boundary() {
        let request = ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-app-icon.json"
            )))
            .expect("JSON"),
        )
        .expect("app icon request");

        let outcome = compile_image_prompt(&request);
        let metadata = outcome.metadata.to_pretty_string();
        assert!(outcome.prompt.is_some(), "{:#?}", outcome.diagnostics);
        for token in [
            "app_icon_master_concept",
            "purpose_metaphor",
            "centered_planes",
            "silhouette_32px",
            "png_boundary",
            "opaque PNG master concept",
        ] {
            assert!(metadata.contains(token), "missing {token:?}\n{metadata}");
        }
    }
}
