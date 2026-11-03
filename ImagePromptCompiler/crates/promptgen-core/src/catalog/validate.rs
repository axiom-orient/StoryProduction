use std::collections::{BTreeMap, BTreeSet};

use super::{
    CatalogEntry, CatalogError, EXPECTED_ENTRY_COUNT, PromptDirective, ReferenceTextMode,
    TextRequirement,
};

pub(super) fn validate_catalog(
    entries: &[CatalogEntry],
    declared_counts: &BTreeMap<String, u64>,
) -> Result<(), CatalogError> {
    if entries.len() != EXPECTED_ENTRY_COUNT {
        return Err(CatalogError(format!(
            "catalog must contain {EXPECTED_ENTRY_COUNT} entries, found {}",
            entries.len()
        )));
    }
    let mut ids = BTreeSet::new();
    let mut asset_paths = BTreeSet::new();
    let mut actual_counts: BTreeMap<String, u64> = BTreeMap::new();
    for entry in entries {
        if !ids.insert(entry.id.clone()) {
            return Err(CatalogError(format!("duplicate catalog id {}", entry.id)));
        }
        if entry.id.trim().is_empty()
            || entry.tier_1.trim().is_empty()
            || entry.tier_2.trim().is_empty()
            || entry.name_ko.trim().is_empty()
            || entry.intent.trim().is_empty()
            || entry.source_path.trim().is_empty()
            || entry.asset_path.trim().is_empty()
        {
            return Err(CatalogError(format!(
                "catalog entry {} contains an empty required field",
                entry.id
            )));
        }
        if entry
            .tier_3
            .as_ref()
            .is_some_and(|tier| tier.trim().is_empty())
        {
            return Err(CatalogError(format!(
                "catalog entry {} contains an empty optional tier_3",
                entry.id
            )));
        }
        if !entry.asset_path.starts_with("assets/")
            || !entry.asset_path.ends_with(".png")
            || entry.asset_path.contains("..")
            || !asset_paths.insert(entry.asset_path.clone())
        {
            return Err(CatalogError(format!(
                "catalog entry {} has an invalid or duplicate asset path {}",
                entry.id, entry.asset_path
            )));
        }
        if entry.default_safety_tier > 2 {
            return Err(CatalogError(format!(
                "catalog entry {} has invalid safety tier {}",
                entry.id, entry.default_safety_tier
            )));
        }
        if entry.profiles.is_empty() || !entry.profiles.iter().any(|profile| profile == "standard")
        {
            return Err(CatalogError(format!(
                "catalog entry {} must advertise the standard image profile",
                entry.id
            )));
        }
        for profile in &entry.profiles {
            if !matches!(
                profile.as_str(),
                "standard"
                    | "logo_identity"
                    | "app_icon"
                    | "app_web_ui"
                    | "information_design"
                    | "character_pose"
                    | "poomsae_pose"
            ) {
                return Err(CatalogError(format!(
                    "catalog entry {} advertises unknown image profile {}",
                    entry.id, profile
                )));
            }
        }
        if entry.default_safety_tier == 1 && entry.text_requirement != TextRequirement::Required {
            return Err(CatalogError(format!(
                "catalog entry {} defaults to text safety tier 1 but does not require exact text",
                entry.id
            )));
        }
        validate_reference_text(entry)?;
        let minimum_directives = 2;
        if entry.prompt_directives.len() < minimum_directives {
            return Err(CatalogError(format!(
                "catalog entry {} must contain at least {minimum_directives} prompt directives",
                entry.id
            )));
        }
        for (index, directive) in entry.prompt_directives.iter().enumerate() {
            validate_prompt_directive(entry, index, directive)?;
        }
        validate_suggested_tokens(entry)?;
        *actual_counts
            .entry(entry.family.as_str().to_owned())
            .or_default() += 1;
    }
    if &actual_counts != declared_counts {
        return Err(CatalogError(format!(
            "declared family counts {declared_counts:?} do not match actual counts {actual_counts:?}"
        )));
    }
    Ok(())
}

fn validate_reference_text(entry: &CatalogEntry) -> Result<(), CatalogError> {
    let has_empty_line = entry
        .reference_exact_text
        .iter()
        .any(|line| line.trim().is_empty() || line.contains('\n') || line.contains('\r'));
    if has_empty_line {
        return Err(CatalogError(format!(
            "catalog entry {} has an empty or multiline reference exact-text line",
            entry.id
        )));
    }
    match entry.reference_text_mode {
        ReferenceTextMode::None => {
            if !entry.reference_exact_text.is_empty()
                || entry.text_requirement == TextRequirement::Required
            {
                return Err(CatalogError(format!(
                    "catalog entry {} has an invalid none reference-text contract",
                    entry.id
                )));
            }
        }
        ReferenceTextMode::BlankZone => {
            if !entry.reference_exact_text.is_empty()
                || entry.text_requirement != TextRequirement::Required
            {
                return Err(CatalogError(format!(
                    "catalog entry {} has an invalid blank-zone reference-text contract",
                    entry.id
                )));
            }
        }
        ReferenceTextMode::SampleCopy => {
            if entry.reference_exact_text.is_empty()
                || entry.text_requirement != TextRequirement::Required
            {
                return Err(CatalogError(format!(
                    "catalog entry {} has an invalid sample-copy reference-text contract",
                    entry.id
                )));
            }
        }
    }
    Ok(())
}

fn validate_prompt_directive(
    entry: &CatalogEntry,
    index: usize,
    directive: &PromptDirective,
) -> Result<(), CatalogError> {
    let text = directive.text.trim();
    if text.is_empty() {
        return Err(CatalogError(format!(
            "catalog entry {} prompt_directives[{index}] is empty",
            entry.id
        )));
    }
    let lower = text.to_lowercase();
    let structural_forbidden = [
        ("{", "placeholder brace"),
        ("}", "placeholder brace"),
        ("주의:", "source note"),
        ("references/", "source path"),
        ("skills/", "source path"),
        ("recipe", "implementation label"),
        ("```", "markdown fence"),
    ];
    // The markers are authored lowercase; the text is lowered once above.
    for (marker, kind) in structural_forbidden {
        if lower.contains(marker) {
            return Err(CatalogError(format!(
                "catalog entry {} prompt_directives[{index}] contains forbidden {kind}: {marker:?}",
                entry.id
            )));
        }
    }
    let provenance_tokens = [
        entry.source_path.as_str(),
        entry.source_sha256.as_str(),
        entry.asset_path.as_str(),
    ];
    for token in provenance_tokens {
        if !token.is_empty() && lower.contains(&token.to_lowercase()) {
            return Err(CatalogError(format!(
                "catalog entry {} prompt_directives[{index}] leaks entry-derived provenance {token:?}",
                entry.id
            )));
        }
    }
    if contains_identifier(text, &entry.id)
        || lower.contains(&format!("directive_{}", entry.id.to_lowercase()))
        || (entry.name_en.contains('_') && lower.contains(&entry.name_en.to_lowercase()))
    {
        return Err(CatalogError(format!(
            "catalog entry {} prompt_directives[{index}] leaks an entry-derived identifier",
            entry.id
        )));
    }
    let authoring_verbs = [
        "choose ",
        "declare ",
        "define ",
        "describe ",
        "specify ",
        "state ",
    ];
    if authoring_verbs.iter().any(|verb| lower.starts_with(verb)) {
        return Err(CatalogError(format!(
            "catalog entry {} prompt_directives[{index}] starts with an authoring-mode verb instead of an observable image instruction",
            entry.id
        )));
    }
    if lower.starts_with("ar ")
        || lower.contains(" default ar")
        || lower.contains("aspect ratio")
        || contains_aspect_ratio_token(text)
    {
        return Err(CatalogError(format!(
            "catalog entry {} prompt_directives[{index}] attempts to override typed output authority",
            entry.id
        )));
    }
    Ok(())
}

fn contains_identifier(text: &str, identifier: &str) -> bool {
    text.split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
        .any(|token| token.eq_ignore_ascii_case(identifier))
}

fn contains_aspect_ratio_token(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        let token =
            token.trim_matches(|character: char| !character.is_ascii_digit() && character != ':');
        let Some((left, right)) = token.split_once(':') else {
            return false;
        };
        !left.is_empty()
            && !right.is_empty()
            && left.chars().all(|character| character.is_ascii_digit())
            && right.chars().all(|character| character.is_ascii_digit())
    })
}

fn validate_suggested_tokens(entry: &CatalogEntry) -> Result<(), CatalogError> {
    let valid_ratios = ["1:1", "2:3", "3:2", "3:4", "4:5", "4:3", "16:9", "9:16"];
    let mut seen_ratios = BTreeSet::new();
    for ratio in &entry.suggested_aspect_ratios {
        if !valid_ratios.contains(&ratio.as_str()) || !seen_ratios.insert(ratio) {
            return Err(CatalogError(format!(
                "catalog entry {} has invalid or duplicate suggested aspect ratio {ratio:?}",
                entry.id
            )));
        }
    }
    if entry.suggested_palette.len() > 5 {
        return Err(CatalogError(format!(
            "catalog entry {} has more than five suggested palette colors",
            entry.id
        )));
    }
    let mut seen_colors = BTreeSet::new();
    for color in &entry.suggested_palette {
        let valid = color.len() == 7
            && color.starts_with('#')
            && color[1..]
                .chars()
                .all(|character| character.is_ascii_hexdigit())
            && color
                .chars()
                .all(|character| !character.is_ascii_lowercase());
        if !valid || !seen_colors.insert(color) {
            return Err(CatalogError(format!(
                "catalog entry {} has invalid or duplicate suggested palette color {color:?}",
                entry.id
            )));
        }
    }
    Ok(())
}
