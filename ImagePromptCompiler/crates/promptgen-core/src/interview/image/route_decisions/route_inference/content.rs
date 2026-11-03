//! Category-contract subject and scene content completion.

use std::collections::BTreeMap;

use crate::interview::answer;
use crate::interview::image::route_decisions::category_contracts::category::{
    category_contract_value, default_route_scene,
};
use crate::interview::image::route_decisions::category_contracts::profile::is_app_icon_profile;

pub(in crate::interview::image) fn infer_route_content(
    brief: &str,
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    let Some(category) = answer(answers, "image.category").map(|value| value.to_ascii_uppercase())
    else {
        return;
    };

    if answer(answers, "image.subject").is_none() {
        let contract = category_contract_value(answers).map(|(_, value)| value.to_owned());
        let fallback = (!brief.trim().is_empty()).then(|| brief.trim().to_owned());
        if let Some(subject) = contract.or(fallback) {
            answers.insert("image.subject".to_owned(), subject);
            inferences.push(
                "결과물 전용 답변과 요청 본문을 주 피사체·핵심 내용으로 정리했다.".to_owned(),
            );
        }
    }

    if answer(answers, "image.scene").is_none() {
        let scene = if category == "C4" && is_app_icon_profile(answers) {
            "opaque neutral canvas for one isolated centered app-icon mark"
        } else {
            default_route_scene(&category)
        };
        answers.insert("image.scene".to_owned(), scene.to_owned());
        inferences.push(format!(
            "{category} 결과물의 제작 캔버스 기본값을 장면으로 적용했다."
        ));
    }
}
