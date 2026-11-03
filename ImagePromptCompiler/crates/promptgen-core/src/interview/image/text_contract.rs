use std::collections::BTreeMap;

use super::super::{InterviewQuestion, answer};
use super::inference::selected_text_requirement;
use super::questions::{option, push_missing, text_mode_question};
use super::route_decisions::category_contracts::profile::is_app_icon_profile;
use super::visual_defaults::infer_default;
use crate::catalog::TextRequirement;

pub(super) fn resolve_text_mode(
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    if answer(answers, "image.text_mode").is_some() {
        return;
    }
    match selected_text_requirement(answers) {
        TextRequirement::Required => {
            answers.insert("image.text_mode".to_owned(), "exact".to_owned());
            inferences.push(
                "선택한 시각 시스템은 글자 형태가 공간 구조를 결정하므로 정확 문구 모드를 적용했다."
                    .to_owned(),
            );
        }
        TextRequirement::Recommended => {}
        TextRequirement::Optional => {
            answers.insert("image.text_mode".to_owned(), "none".to_owned());
            inferences.push(
                "선택한 결과물과 스타일이 정확 문구를 요구하지 않아 문구 없음으로 설정했다."
                    .to_owned(),
            );
        }
    }
}

pub(super) fn apply_text_defaults(
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    if answer(answers, "image.text_mode") != Some("exact") {
        return;
    }
    let category = answer(answers, "image.category")
        .unwrap_or("C5")
        .to_ascii_uppercase();
    let (position, style) = match category.as_str() {
        "C4" => ("middle_right", "mono"),
        "C11" => ("top_left", "condensed"),
        _ => ("top_left", "geometric"),
    };
    infer_default(
        answers,
        inferences,
        "image.text_position",
        position.to_owned(),
        "선택한 결과물의 기본 읽기 순서",
    );
    infer_default(
        answers,
        inferences,
        "image.text_style",
        style.to_owned(),
        "선택한 결과물의 기본 타이포그래피",
    );
}

pub(super) fn collect_contract_questions(
    answers: &BTreeMap<String, String>,
) -> Vec<InterviewQuestion> {
    let mut questions = Vec::new();
    if let Some(value) = answer(answers, "image.text_mode")
        && !matches!(value, "none" | "exact")
    {
        questions.push(text_mode_question().with_error(
            "지원하지 않는 문구 모드입니다. 문구 없음 또는 정확 문구 입력 중 하나를 선택하세요.",
        ));
        return questions;
    }
    if is_app_icon_profile(answers) && answer(answers, "image.text_mode") == Some("exact") {
        return vec![
            InterviewQuestion::choice(
                "image.text_mode",
                "앱 아이콘은 문구 없는 마스터 콘셉트만 지원합니다.",
                "앱 아이콘 안에는 텍스트·문자·숫자·의사문자를 넣지 않습니다.",
                vec![option(
                    "none",
                    "문구 없음",
                    "읽을 수 있는 문자와 문구를 생성하지 않음",
                )],
            )
            .with_error("app_icon은 image.text_mode=none만 허용합니다."),
        ];
    }
    let text_requirement = selected_text_requirement(answers);
    if text_requirement == TextRequirement::Required
        && answer(answers, "image.text_mode") == Some("none")
    {
        questions.push(
            InterviewQuestion::choice(
                "image.text_mode",
                "선택한 시각 시스템에 정확 문구가 필요합니다.",
                "글자 형태가 마스크·터널·조각·공간 구조를 만들기 때문에 무문구 상태로는 해당 패턴을 성립시킬 수 없습니다.",
                vec![option(
                    "exact",
                    "정확 문구 입력",
                    "지정한 문구와 줄바꿈을 그대로 사용",
                )],
            )
            .with_error("정확 문구를 입력하거나 글자 의존 패턴을 제거해야 합니다."),
        );
        return questions;
    }
    if answer(answers, "image.text_mode").is_none() {
        questions.push(text_mode_question());
    }
    if answer(answers, "image.text_mode") == Some("exact") {
        push_missing(
            answers,
            "image.text",
            InterviewQuestion::multiline(
                "image.text",
                "정확히 렌더할 문구를 줄마다 입력해 주세요.",
                "한 줄에는 한 문자 체계만 사용합니다. 줄바꿈은 그대로 보존됩니다.",
                "봄의 한 잔\nSPRING CUP",
            ),
            &mut questions,
        );
    }
    questions
}
