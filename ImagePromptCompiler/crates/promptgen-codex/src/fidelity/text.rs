use std::collections::BTreeSet;

use promptgen_core::image::ImagePromptRequest;

use super::TextFidelityCheck;
use super::placement::placement_label;

pub(super) fn text_matches_request(
    checks: &[TextFidelityCheck],
    request: &ImagePromptRequest,
) -> bool {
    let mut ids = BTreeSet::new();
    checks.len() == request.text_elements.len()
        && checks.iter().all(|check| ids.insert(check.id.as_str()))
        && request.text_elements.iter().all(|element| {
            checks
                .iter()
                .find(|check| check.id == element.id)
                .is_some_and(|check| check.exact && check.observed_text == element.lines.join("\n"))
        })
}

pub(super) fn text_failure_details(
    checks: &[TextFidelityCheck],
    request: &ImagePromptRequest,
) -> (String, String) {
    for element in &request.text_elements {
        let expected = element.lines.join("\n");
        let expected_visible = expected.replace('\n', "\\n");
        let Some(check) = checks.iter().find(|check| check.id == element.id) else {
            return (
                format!(
                    "text element {} is missing from the visual check",
                    element.id
                ),
                format!(
                    "Render text element {} exactly as \"{}\" at {} and preserve everything else.",
                    element.id,
                    expected_visible,
                    placement_label(&element.placement)
                ),
            );
        };
        if check.observed_text != expected {
            return (
                format!(
                    "text element {} differs from the requested character sequence or line breaks",
                    element.id
                ),
                format!(
                    "Replace only text element {} with exactly \"{}\", preserving its line breaks and {} placement; preserve everything else.",
                    element.id,
                    expected_visible,
                    placement_label(&element.placement)
                ),
            );
        }
        if !check.exact {
            return (
                format!(
                    "text element {} placement or glyph fidelity is NOT_PROVEN by the visual check",
                    element.id
                ),
                format!(
                    "Make text element {} directly legible as \"{}\" at {}; preserve everything else.",
                    element.id,
                    expected_visible,
                    placement_label(&element.placement)
                ),
            );
        }
    }
    (
        "text checks do not enumerate every typed text element exactly once".to_owned(),
        "Return exactly one visual text check for each typed text element and preserve everything else."
            .to_owned(),
    )
}
