use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::{
    CatalogEntry, CatalogError, CatalogFamily, MAX_SUPPORTING_DIRECTIONS, TextRequirement,
    find_entry,
};
use crate::json::JsonValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CatalogSelectionIssue {
    PatternLimit { found: usize },
    DuplicateId { selection_index: usize, id: String },
    UnknownCategory { id: String },
    CategoryNotPrimary { id: String, tier_1: String },
    UnknownPattern { pattern_index: usize, id: String },
    PatternIsPrimary { pattern_index: usize, id: String },
    MultipleFamily { family: CatalogFamily, count: usize },
}

impl fmt::Display for CatalogSelectionIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PatternLimit { found } => write!(
                formatter,
                "select at most {MAX_SUPPORTING_DIRECTIONS} supporting directions; found {found}"
            ),
            Self::DuplicateId { id, .. } => {
                write!(formatter, "catalog id {id:?} is selected more than once")
            }
            Self::UnknownCategory { id } => write!(formatter, "unknown category id {id:?}"),
            Self::CategoryNotPrimary { id, tier_1 } => write!(
                formatter,
                "{id} belongs to tier_1 {tier_1:?}, not the primary-output tier"
            ),
            Self::UnknownPattern { id, .. } => write!(formatter, "unknown pattern id {id:?}"),
            Self::PatternIsPrimary { id, .. } => write!(
                formatter,
                "{id:?} is a primary result and cannot be a supporting direction"
            ),
            Self::MultipleFamily { family, count } => write!(
                formatter,
                "select at most one {} entry; found {count}",
                family.as_str()
            ),
        }
    }
}

#[derive(Debug)]
pub(crate) struct CatalogSelection {
    pub(crate) entries: Vec<&'static CatalogEntry>,
    pub(crate) issues: Vec<CatalogSelectionIssue>,
}

pub(crate) fn inspect_catalog_selection(
    category: Option<&str>,
    patterns: &[String],
) -> Result<CatalogSelection, CatalogError> {
    let mut entries = Vec::new();
    let mut issues = Vec::new();
    let mut selected_ids = BTreeSet::new();
    let mut family_counts = BTreeMap::new();

    if patterns.len() > MAX_SUPPORTING_DIRECTIONS {
        issues.push(CatalogSelectionIssue::PatternLimit {
            found: patterns.len(),
        });
    }

    if let Some(category) = category {
        let normalized = category.trim().to_ascii_uppercase();
        selected_ids.insert(normalized);
        match find_entry(category)? {
            Some(entry) => {
                entries.push(entry);
                if !entry.is_primary_output() {
                    issues.push(CatalogSelectionIssue::CategoryNotPrimary {
                        id: entry.id.clone(),
                        tier_1: entry.tier_1.clone(),
                    });
                }
            }
            None => issues.push(CatalogSelectionIssue::UnknownCategory {
                id: category.to_owned(),
            }),
        }
    }

    let category_offset = usize::from(category.is_some());
    for (pattern_index, id) in patterns.iter().enumerate() {
        let normalized = id.trim().to_ascii_uppercase();
        if !selected_ids.insert(normalized) {
            issues.push(CatalogSelectionIssue::DuplicateId {
                selection_index: category_offset + pattern_index,
                id: id.clone(),
            });
            continue;
        }
        match find_entry(id)? {
            Some(entry) => {
                entries.push(entry);
                if entry.is_primary_output() {
                    issues.push(CatalogSelectionIssue::PatternIsPrimary {
                        pattern_index,
                        id: entry.id.clone(),
                    });
                } else {
                    *family_counts.entry(entry.family).or_insert(0usize) += 1;
                }
            }
            None => issues.push(CatalogSelectionIssue::UnknownPattern {
                pattern_index,
                id: id.clone(),
            }),
        }
    }

    for (family, count) in family_counts {
        if count > 1 {
            issues.push(CatalogSelectionIssue::MultipleFamily { family, count });
        }
    }

    Ok(CatalogSelection { entries, issues })
}

pub(crate) fn text_guard_is_repetition_aware(entries: &[&CatalogEntry]) -> bool {
    entries.iter().any(|entry| entry.repetition_text)
}

pub(crate) fn exact_text_is_required(entries: &[&CatalogEntry]) -> bool {
    entries
        .iter()
        .any(|entry| entry.text_requirement == TextRequirement::Required)
}

pub(crate) fn selected_catalog_to_json(entries: &[&CatalogEntry]) -> JsonValue {
    JsonValue::array(entries.iter().map(|entry| entry.to_json()))
}
