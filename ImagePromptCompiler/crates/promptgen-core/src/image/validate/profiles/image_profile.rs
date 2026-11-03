use crate::catalog::CatalogEntry;
use crate::diagnostic::Diagnostic;
use crate::image::model::{ImagePromptRequest, ImageRenderProfile, ImageTaskMode, VisualMedium};
use crate::image::{BackgroundMode, CanvasPlacement, CanvasZone, ImageProfile};

use super::super::profile_text::matching::contains_ui_surface_component_relation;
use super::super::profile_text::validation::{
    validate_app_icon_content, validate_textless_ui_content,
};

pub(in crate::image::validate) fn validate_image_profile(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if request.profile == ImageProfile::Standard {
        return;
    }

    let category = selected
        .iter()
        .find(|entry| entry.is_primary_output())
        .map(|entry| entry.id.as_str());
    let expected_category = match request.profile {
        ImageProfile::LogoIdentity => "C4",
        ImageProfile::AppIcon => "C4",
        ImageProfile::AppWebUi | ImageProfile::InformationDesign => "C6",
        ImageProfile::CharacterPose | ImageProfile::PoomsaePose => "C10",
        ImageProfile::Standard => return,
    };
    if category != Some(expected_category) {
        diagnostics.push(Diagnostic::error(
            "IMG_PROFILE_CATEGORY",
            "$.profile",
            format!(
                "profile={} is category-bound to {expected_category}; select that primary catalog category",
                request.profile.as_str()
            ),
        ));
    }
    if !selected
        .iter()
        .filter(|entry| entry.is_primary_output())
        .any(|entry| {
            entry
                .profiles
                .iter()
                .any(|profile| profile == request.profile.as_str())
        })
    {
        diagnostics.push(Diagnostic::error(
            "IMG_PROFILE_CATALOG",
            "$.profile",
            format!(
                "profile={} is not advertised by the selected catalog outcome",
                request.profile.as_str()
            ),
        ));
    }

    if request.task_mode != ImageTaskMode::Generate {
        diagnostics.push(Diagnostic::error(
            "IMG_PROFILE_TASK_MODE",
            "$.profile",
            "category-bound profiles are text-directed generate prompts; use the separate render_profile contract for edits or composites",
        ));
    }

    match request.profile {
        ImageProfile::AppIcon => {
            if request.task_mode != ImageTaskMode::Generate {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_TASK_MODE",
                    "$.task_mode",
                    "app_icon is a generate-only profile",
                ));
            }
            if request.render_profile != ImageRenderProfile::Structured {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_RENDER_PROFILE",
                    "$.render_profile",
                    "app_icon uses the structured generate renderer only",
                ));
            }
            if !request.references.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_REFERENCES",
                    "$.references",
                    "app_icon does not accept reference images; provide one typed master subject",
                ));
            }
            if !request.change_contract.change_only.is_empty()
                || !request.change_contract.preserve.is_empty()
            {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_CHANGE_CONTRACT",
                    "$.change_contract",
                    "app_icon does not accept edit/change contracts",
                ));
            }
            if !matches!(
                request.medium,
                VisualMedium::GraphicDesign
                    | VisualMedium::Illustration
                    | VisualMedium::ThreeD
                    | VisualMedium::Mixed
            ) {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_MEDIUM",
                    "$.medium",
                    "app_icon requires a non-photo medium: graphic_design, illustration, 3d, or mixed",
                ));
            }
            if request.subjects.len() != 1 {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_SUBJECT_COUNT",
                    "$.subjects",
                    "app_icon requires exactly one typed subject",
                ));
            } else {
                let subject = &request.subjects[0];
                if subject.count != 1 {
                    diagnostics.push(Diagnostic::error(
                        "IMG_APP_ICON_SUBJECT_COUNT",
                        "$.subjects[0].count",
                        "app_icon requires the single typed subject to have count=1",
                    ));
                }
                if !matches!(subject.placement, CanvasPlacement::Zone(CanvasZone::Center)) {
                    diagnostics.push(Diagnostic::error(
                        "IMG_APP_ICON_SUBJECT_PLACEMENT",
                        "$.subjects[0].placement",
                        "app_icon requires the single typed subject to use the center placement zone",
                    ));
                }
            }
            if !(2..=3).contains(&request.composition.depth_layers.len()) {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_PLANES",
                    "$.composition.depth_layers",
                    "app_icon requires simple 2-3 visual planes",
                ));
            }
            if !request.text_elements.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_TEXT",
                    "$.text_elements",
                    "app_icon is textless and requires text_elements to be empty",
                ));
            }
            if !request.output.format.eq_ignore_ascii_case("png") {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_FORMAT",
                    "$.output.format",
                    "app_icon produces an opaque PNG master concept only",
                ));
            }
            if request.output.background != BackgroundMode::Opaque {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_BACKGROUND",
                    "$.output.background",
                    "app_icon requires an opaque background",
                ));
            }
            if request.output.width != 1024 || request.output.height != 1024 {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_DIMENSIONS",
                    "$.output",
                    "app_icon requires exactly 1024x1024 output",
                ));
            }
            validate_app_icon_content(request, diagnostics);
        }
        ImageProfile::LogoIdentity => {
            if !matches!(
                request.medium,
                VisualMedium::GraphicDesign | VisualMedium::ThreeD | VisualMedium::Mixed
            ) {
                diagnostics.push(Diagnostic::error(
                    "IMG_LOGO_PROFILE_MEDIUM",
                    "$.medium",
                    "logo_identity requires graphic_design, 3d, or mixed medium",
                ));
            }
            if !request.output.format.eq_ignore_ascii_case("png") {
                diagnostics.push(Diagnostic::error(
                    "IMG_LOGO_PROFILE_FORMAT",
                    "$.output.format",
                    "logo_identity produces an opaque PNG concept/brand-mark direction; SVG or vector final assets are outside this contract",
                ));
            }
            if request.output.background != BackgroundMode::Opaque {
                diagnostics.push(Diagnostic::error(
                    "IMG_LOGO_PROFILE_BACKGROUND",
                    "$.output.background",
                    "logo_identity requires an opaque background; transparent/vector final delivery is not promised",
                ));
            }
        }
        ImageProfile::AppWebUi => {
            if !matches!(
                request.medium,
                VisualMedium::GraphicDesign | VisualMedium::Mixed | VisualMedium::ThreeD
            ) {
                diagnostics.push(Diagnostic::error(
                    "IMG_UI_PROFILE_MEDIUM",
                    "$.medium",
                    "app_web_ui requires graphic_design, mixed, or 3d medium",
                ));
            }
            if request.subjects.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "IMG_UI_PROFILE_SCREEN",
                    "$.subjects",
                    "app_web_ui requires at least one typed screen or interface subject",
                ));
            } else if !request
                .subjects
                .iter()
                .any(|subject| contains_ui_surface_component_relation(&subject.description))
            {
                diagnostics.push(Diagnostic::error(
                    "IMG_UI_PROFILE_EVIDENCE",
                    "$.subjects[*].description",
                    "app_web_ui requires $.subjects[*].description to name an explicit screen/interface surface and relate it to at least one component or state (for example dashboard screen with sidebar, table, or status)",
                ));
            }
            validate_textless_ui_content(request, diagnostics);
        }
        ImageProfile::InformationDesign => {
            if !matches!(
                request.medium,
                VisualMedium::GraphicDesign | VisualMedium::Mixed | VisualMedium::Illustration
            ) {
                diagnostics.push(Diagnostic::error(
                    "IMG_INFORMATION_PROFILE_MEDIUM",
                    "$.medium",
                    "information_design requires graphic_design, mixed, or illustration medium",
                ));
            }
        }
        ImageProfile::CharacterPose => {
            if request.medium != VisualMedium::Illustration && request.medium != VisualMedium::Mixed
            {
                diagnostics.push(Diagnostic::error(
                    "IMG_CHARACTER_POSE_PROFILE_MEDIUM",
                    "$.medium",
                    "character_pose requires illustration or mixed medium",
                ));
            }
        }
        ImageProfile::PoomsaePose => {
            if request.medium != VisualMedium::Illustration && request.medium != VisualMedium::Mixed
            {
                diagnostics.push(Diagnostic::error(
                    "IMG_POOMSAE_PROFILE_MEDIUM",
                    "$.medium",
                    "poomsae_pose requires illustration or mixed medium",
                ));
            }
            let brief = format!(
                "{} {} {}",
                request.use_case,
                request
                    .subjects
                    .iter()
                    .map(|subject| format!("{} {}", subject.pose, subject.action))
                    .collect::<Vec<_>>()
                    .join(" "),
                request.scene.environment
            )
            .to_ascii_lowercase();
            if !["poomsae", "품새", "taekwondo", "태권도"]
                .iter()
                .any(|term| brief.contains(term))
            {
                diagnostics.push(Diagnostic::error(
                    "IMG_POOMSAE_PROFILE_INTENT",
                    "$.profile",
                    "poomsae_pose requires an explicit text-directed poomsae/taekwondo action in use_case, subject pose/action, or scene",
                ));
            }
        }
        ImageProfile::Standard => {}
    }
}
