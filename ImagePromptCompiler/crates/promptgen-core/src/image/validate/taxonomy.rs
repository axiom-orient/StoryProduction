use crate::catalog::CatalogEntry;
use crate::catalog::selection::CatalogSelectionIssue;
use crate::diagnostic::Diagnostic;
use crate::image::model::ImagePromptRequest;

pub(super) fn push_taxonomy_issue(issue: CatalogSelectionIssue, diagnostics: &mut Vec<Diagnostic>) {
    let (code, path) = match &issue {
        CatalogSelectionIssue::PatternLimit { .. } => (
            "IMG_TAXONOMY_PATTERN_LIMIT",
            "$.taxonomy.patterns".to_owned(),
        ),
        CatalogSelectionIssue::DuplicateId {
            selection_index, ..
        } => (
            "IMG_TAXONOMY_DUPLICATE",
            format!("$.taxonomy.ids[{selection_index}]"),
        ),
        CatalogSelectionIssue::UnknownCategory { .. } => {
            ("IMG_TAXONOMY_UNKNOWN", "$.taxonomy.category".to_owned())
        }
        CatalogSelectionIssue::CategoryNotPrimary { .. } => (
            "IMG_TAXONOMY_CATEGORY_TIER",
            "$.taxonomy.category".to_owned(),
        ),
        CatalogSelectionIssue::UnknownPattern { pattern_index, .. } => (
            "IMG_TAXONOMY_UNKNOWN",
            format!("$.taxonomy.patterns[{pattern_index}]"),
        ),
        CatalogSelectionIssue::PatternIsPrimary { pattern_index, .. } => (
            "IMG_TAXONOMY_PATTERN_TIER",
            format!("$.taxonomy.patterns[{pattern_index}]"),
        ),
        CatalogSelectionIssue::MultipleFamily { .. } => (
            "IMG_TAXONOMY_MULTIPLE_FAMILY",
            "$.taxonomy.patterns".to_owned(),
        ),
    };
    diagnostics.push(Diagnostic::error(code, path, issue.to_string()));
}

pub(super) fn validate_taxonomy_safety(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for entry in selected {
        if request.constraints.safety_tier < entry.default_safety_tier {
            diagnostics.push(Diagnostic::error(
                "IMG_TAXONOMY_TIER",
                "$.constraints.safety_tier",
                format!(
                    "{} requires safety tier {}, request declares {}",
                    entry.id, entry.default_safety_tier, request.constraints.safety_tier
                ),
            ));
        }
    }
}
