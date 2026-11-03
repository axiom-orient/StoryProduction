//! Category and profile clarification questions for image interviews.

use std::collections::BTreeMap;

use super::category::{app_icon_contract_spec, category_contract_spec};
use super::profile::{is_app_icon_profile, profile_is_compatible};
use crate::interview::image::inference::{invalid_pattern_tokens, wardrobe_is_specific};
use crate::interview::image::questions::{
    category_question, is_primary_category, option, profile_question, push_missing,
};
use crate::interview::{InterviewQuestion, answer};

pub(in crate::interview::image) fn category_routing_question(
    answers: &BTreeMap<String, String>,
) -> Option<InterviewQuestion> {
    match answer(answers, "image.category") {
        None => Some(category_question()),
        Some(category) if !is_primary_category(category) => {
            Some(category_question().with_error("지원하지 않는 결과물 종류입니다."))
        }
        Some(_) => None,
    }
}

pub(in crate::interview::image) fn collect_routing_questions(
    answers: &BTreeMap<String, String>,
) -> Vec<InterviewQuestion> {
    let mut questions = Vec::new();
    debug_assert!(category_routing_question(answers).is_none());

    if let Some(question) = missing_category_contract_question(answers) {
        questions.push(question);
        return questions;
    }

    if let Some(profile) = answer(answers, "image.profile")
        && !profile_is_compatible(
            answer(answers, "image.category").unwrap_or_default(),
            profile,
        )
    {
        questions.push(
            profile_question(answer(answers, "image.category").unwrap_or_default())
                .with_error("선택한 프로필이 결과물 카테고리와 호환되지 않습니다."),
        );
        return questions;
    }

    push_missing(
        answers,
        "image.subject",
        InterviewQuestion::multiline(
            "image.subject",
            "주 피사체를 눈에 보이게 정의해 주세요.",
            "수량, 형태, 자세 또는 방향, 표면 특징을 포함합니다.",
            "무광 아이보리 도자기 컵 1개, 손잡이는 오른쪽 3시 방향, 짙은 필터 커피가 보인다.",
        ),
        &mut questions,
    );
    push_missing(
        answers,
        "image.scene",
        InterviewQuestion::multiline(
            "image.scene",
            "장면과 배경을 어떻게 구성합니까?",
            "환경, 후경, 시간대, 대기 상태를 관찰 가능한 문장으로 적습니다.",
            "오전의 콘크리트 로스터리, 후면은 중간 회색 벽, 창을 통과한 확산 일광.",
        ),
        &mut questions,
    );

    match answer(answers, "image.pattern") {
        Some(value) if invalid_pattern_tokens(value) => questions.push(
            InterviewQuestion::choice(
                "image.pattern",
                "레거시 스타일 선택을 제거해 주세요.",
                "스타일은 별도 카테고리가 아니라 매체·조명·색·재질 입력으로 직접 지정합니다.",
                vec![option(
                    "none",
                    "별도 스타일 없음",
                    "결과물 1개와 구조화된 시각 필드만 사용",
                )],
            )
            .with_error("이 버전은 보조 스타일 ID를 지원하지 않습니다."),
        ),
        _ => {}
    }
    questions
}

fn missing_category_contract_question(
    answers: &BTreeMap<String, String>,
) -> Option<InterviewQuestion> {
    let category = answer(answers, "image.category")?.to_ascii_uppercase();
    if category == "C1" {
        let question = InterviewQuestion::multiline(
            "image.wardrobe",
            "의상을 색·소재·의류종·핏으로 정의해 주세요.",
            "화보에서 결과를 크게 바꾸는 의상 제품과 실루엣만 확인합니다.",
            "아이보리 오버사이즈 울 코트, 크림 터틀넥 니트, 하이웨이스트 와이드 슬랙스, 여유로운 핏.",
        );
        return match answer(answers, "image.wardrobe") {
            None => Some(question),
            Some(value) if !wardrobe_is_specific(value) => Some(question.with_error(
                "색, 소재, 의류종, 핏/재단 중 최소 3가지를 관찰 가능한 말로 지정해 주세요.",
            )),
            _ => None,
        };
    }
    let spec = if category == "C4" && is_app_icon_profile(answers) {
        app_icon_contract_spec()
    } else {
        category_contract_spec(&category)?
    };
    answer(answers, spec.key)
        .is_none()
        .then(|| InterviewQuestion::multiline(spec.key, spec.label, spec.help, spec.placeholder))
}
