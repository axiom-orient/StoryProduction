//! Remove answer state that no longer applies to the selected image route.

use std::collections::BTreeMap;

use super::profile::{is_app_icon_profile, profile_is_compatible, profile_is_known};
use crate::interview::answer;

pub(in crate::interview::image) fn prune_irrelevant_category_answers(
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    const CATEGORY_KEYS: &[&str] = &[
        "image.wardrobe",
        "image.adult_editorial_confirm",
        "image.variants",
        "image.formulation",
        "image.product_guide",
        "image.app_icon_concept",
        "image.campaign_plan",
        "image.information_plan",
        "image.card_plan",
        "image.brand_applications",
        "image.icon_set",
        "image.panel_plan",
        "image.keyart_plan",
        "image.deck_plan",
        "image.character_sheet",
        "image.occlusion_plan",
        "image.series_plan",
        "image.meta_ui_plan",
        "image.collage_plan",
        "image.storyboard_plan",
        "image.stage_plan",
    ];
    let category = answer(answers, "image.category")
        .map(str::to_ascii_uppercase)
        .unwrap_or_default();
    let allowed = match category.as_str() {
        "C1" => &["image.wardrobe"][..],
        "C4" if is_app_icon_profile(answers) => &["image.app_icon_concept"][..],
        "C4" => &["image.product_guide"][..],
        "C5" => &["image.campaign_plan"][..],
        "C6" => &["image.information_plan"][..],
        "C10" => &["image.panel_plan"][..],
        "C11" => &["image.keyart_plan"][..],
        _ => &[][..],
    };
    let removed = CATEGORY_KEYS
        .iter()
        .filter(|key| !allowed.contains(key))
        .filter_map(|key| answers.remove(*key).map(|_| *key))
        .collect::<Vec<_>>();
    if !removed.is_empty() {
        inferences.push(format!(
            "현재 결과물 경로와 무관한 이전 전용 답변을 제거했다: {}",
            removed.join(", ")
        ));
    }
    if let Some(profile) = answers.get("image.profile").cloned()
        && profile_is_known(&profile)
        && !profile_is_compatible(&category, &profile)
    {
        answers.remove("image.profile");
        inferences.push(format!(
            "현재 {category} 결과물과 호환되지 않는 이전 image.profile={profile} 답변을 제거했다."
        ));
    }
}
