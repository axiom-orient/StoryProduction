mod consistency;
mod fields;
mod free_text;
mod profile_text;
mod profiles;
mod task_contract;
mod taxonomy;

use crate::catalog::CatalogEntry;
use crate::catalog::selection::inspect_catalog_selection;
use crate::diagnostic::Diagnostic;

use self::consistency::validate_consistency;
use self::fields::common::validate_nonempty;
use self::fields::output::validate_output;
use self::fields::text::{validate_complex_output_quality, validate_constraints, validate_text};
use self::fields::visual::{
    validate_camera_and_lighting, validate_color, validate_composition, validate_elements,
    validate_scene,
};
use self::free_text::validate_free_text_budget;
use self::profiles::image_profile::validate_image_profile;
use self::profiles::render_profile::validate_render_profile;
use self::task_contract::validate_task_contract;
use self::taxonomy::{push_taxonomy_issue, validate_taxonomy_safety};
use super::model::ImagePromptRequest;
use super::specificity::inspect_description;

#[cfg(test)]
#[cfg(test)]
use self::profile_text::lexicon::TEXTLESS_UI_FORBIDDEN_TERMS;
#[cfg(test)]
use self::profile_text::negation::contains_positive_forbidden_term;
#[cfg(test)]
use super::model::ImageQuality;

const MAX_EDGE: u32 = 3840;
const MIN_PIXELS: u64 = 655_360;
const MAX_PIXELS: u64 = 8_294_400;
const EXPERIMENTAL_PIXELS: u64 = 3_686_400;
/// Free-text budget for one structured field, in characters.
///
/// Nothing else bounds these fields, so a single pasted document silently becomes a prompt
/// the image backend will reject — and that rejection costs a generation call, while this
/// check costs nothing. The bound keeps structured requests within a predictable
/// model input envelope.
const MAX_FREE_TEXT_CHARS: usize = 2_000;

pub fn validate_image_request(
    request: &ImagePromptRequest,
) -> (Vec<Diagnostic>, Vec<&'static CatalogEntry>) {
    let mut diagnostics = Vec::new();
    validate_nonempty("$.use_case", &request.use_case, &mut diagnostics);
    inspect_description("$.use_case", &request.use_case, &mut diagnostics);
    validate_task_contract(request, &mut diagnostics);
    validate_consistency(request, &mut diagnostics);

    let selected = match inspect_catalog_selection(
        request.taxonomy.category.as_deref(),
        &request.taxonomy.patterns,
    ) {
        Ok(selection) => {
            for issue in selection.issues {
                push_taxonomy_issue(issue, &mut diagnostics);
            }
            selection.entries
        }
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "IMG_TAXONOMY_LOAD",
                "$.taxonomy",
                error.to_string(),
            ));
            Vec::new()
        }
    };
    validate_taxonomy_safety(request, &selected, &mut diagnostics);
    validate_image_profile(request, &selected, &mut diagnostics);
    validate_render_profile(request, &selected, &mut diagnostics);
    validate_scene(request, &mut diagnostics);
    validate_elements(request, &mut diagnostics);
    validate_composition(request, &mut diagnostics);
    validate_camera_and_lighting(request, &mut diagnostics);
    validate_color(&request.color, request, &mut diagnostics);
    validate_text(request, &selected, &mut diagnostics);
    validate_constraints(request, &selected, &mut diagnostics);
    validate_complex_output_quality(request, &selected, &mut diagnostics);
    validate_output(request, &mut diagnostics);
    validate_free_text_budget(request, &mut diagnostics);
    (diagnostics, selected)
}

#[cfg(test)]
#[path = "validate/tests.rs"]
mod tests;
