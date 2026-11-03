//! Profile/category compatibility and route-derived medium selection.

use std::collections::BTreeMap;

use crate::interview::image::questions::medium_options;
use crate::interview::{InterviewOption, InterviewQuestion, answer};

pub(super) fn profile_is_compatible(category: &str, profile: &str) -> bool {
    match profile.to_ascii_lowercase().as_str() {
        "standard" => true,
        "logo_identity" | "app_icon" => category.eq_ignore_ascii_case("C4"),
        "app_web_ui" | "information_design" => category.eq_ignore_ascii_case("C6"),
        "character_pose" | "poomsae_pose" => category.eq_ignore_ascii_case("C10"),
        _ => false,
    }
}

pub(super) fn profile_is_known(profile: &str) -> bool {
    matches!(
        profile.to_ascii_lowercase().as_str(),
        "standard"
            | "logo_identity"
            | "app_icon"
            | "app_web_ui"
            | "information_design"
            | "character_pose"
            | "poomsae_pose"
    )
}

pub(in crate::interview::image) fn is_app_icon_profile(answers: &BTreeMap<String, String>) -> bool {
    answer(answers, "image.profile").is_some_and(|profile| profile.eq_ignore_ascii_case("app_icon"))
}

pub(in crate::interview::image) fn infer_medium_from_route(
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    if answer(answers, "image.medium").is_some() {
        return;
    }
    let Some(category) = answer(answers, "image.category") else {
        return;
    };
    let profile = answer(answers, "image.profile").map(str::to_ascii_lowercase);
    let medium = match (category.to_ascii_uppercase().as_str(), profile.as_deref()) {
        ("C4", Some("logo_identity" | "app_icon")) => "graphic_design",
        ("C6", Some("app_web_ui" | "information_design")) => "graphic_design",
        ("C10", Some("character_pose" | "poomsae_pose")) => "illustration",
        ("C6", _) => "graphic_design",
        ("C10" | "C11", _) => "illustration",
        _ => "photo",
    };
    answers.insert("image.medium".to_owned(), medium.to_owned());
    inferences.push(format!("선택한 결과물의 기본 매체 {medium}을 적용했다."));
}

pub(in crate::interview::image) fn profile_medium_options(
    profile: &str,
) -> Option<Vec<InterviewOption>> {
    let allowed = match profile.to_ascii_lowercase().as_str() {
        "logo_identity" => &["graphic_design", "3d", "mixed"][..],
        "app_icon" => &["graphic_design", "illustration", "3d", "mixed"][..],
        "app_web_ui" => &["graphic_design", "mixed", "3d"][..],
        "information_design" => &["graphic_design", "mixed", "illustration"][..],
        "character_pose" | "poomsae_pose" => &["illustration", "mixed"][..],
        _ => return None,
    };
    let options = medium_options();
    Some(
        allowed
            .iter()
            .filter_map(|value| {
                options
                    .iter()
                    .find(|option| option.value == *value)
                    .cloned()
            })
            .collect(),
    )
}

pub(in crate::interview::image) fn profile_medium_question(
    profile: &str,
    medium: &str,
) -> Option<InterviewQuestion> {
    let options = profile_medium_options(profile)?;
    Some(
        InterviewQuestion::choice(
            "image.medium",
            "선택한 프로필과 호환되는 시각 매체를 선택해 주세요.",
            "현재 매체를 자동으로 덮어쓰지 않습니다. 아래 호환 옵션 중 하나를 선택하면 다시 검증합니다.",
            options,
        )
        .with_error(format!(
            "{profile} 프로필은 현재 {medium} 매체와 호환되지 않습니다. 아래 호환 매체 중 하나를 선택해 주세요."
        )),
    )
}
