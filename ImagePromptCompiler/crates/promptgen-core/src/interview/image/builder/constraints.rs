//! Category, safety, and required/excluded element projection.

use std::collections::BTreeMap;

use super::super::super::answer;
use super::super::inference::split_nonempty;
use super::super::route_decisions::category_contracts::category::category_contract_value;
use super::image_copy;
use crate::json::JsonValue;

pub(super) struct ConstraintContext<'a> {
    pub(super) category: &'a String,
    pub(super) language: &'a str,
    pub(super) answers: &'a BTreeMap<String, String>,
    pub(super) brief: &'a str,
    pub(super) subject_description: &'a str,
    pub(super) repeated_count: Option<u16>,
    pub(super) is_app_icon: bool,
    pub(super) safety_tier: u8,
}

pub(super) fn build_constraints(context: ConstraintContext<'_>) -> JsonValue {
    let ConstraintContext {
        category,
        language,
        answers,
        brief,
        subject_description,
        repeated_count,
        is_app_icon,
        safety_tier,
    } = context;
    let mut required_elements = vec![JsonValue::from(image_copy(
        language,
        "각 typed subject의 count와 placement를 정확히 유지",
        "Preserve every typed subject count and placement exactly",
    ))];
    if is_app_icon {
        required_elements.extend([
            JsonValue::from(image_copy(
                language,
                "제품 목적에서 도출한 하나의 핵심 은유와 컴포넌트 기하",
                "One core metaphor and component geometry derived from the product purpose",
            )),
            JsonValue::from(image_copy(
                language,
                "단순한 2–3개 시각 평면과 32px 실루엣 판독성",
                "Simple 2–3 visual planes and a silhouette readable at 32px",
            )),
        ]);
    }
    if category == "C10" {
        if repeated_count.is_some() {
            required_elements.push(JsonValue::from(image_copy(
                language,
                "패널·스티커 개수와 subject action의 읽기 순서를 정확히 유지",
                "Preserve the panel/sticker count and typed subject action order exactly",
            )));
        } else if let Some((_, value)) = category_contract_value(answers)
            && value.trim() != subject_description.trim()
            && !brief.contains(value.trim())
        {
            required_elements.push(JsonValue::from(if language == "en" {
                format!("Panel plan: {value}")
            } else {
                format!("패널 계획: {value}")
            }));
        }
    } else if category != "C1"
        && let Some((label, value)) = category_contract_value(answers)
        && value.trim() != subject_description.trim()
        && !brief.contains(value.trim())
    {
        required_elements.push(JsonValue::from(format!("{label}: {value}")));
    }
    JsonValue::object([
        ("adult_subjects_only", JsonValue::from(safety_tier == 2)),
        ("clean_unbranded_finish", JsonValue::from(true)),
        (
            "excluded_elements",
            string_array(answer(answers, "image.exclusions").unwrap_or_default()),
        ),
        (
            "original_characters_only",
            JsonValue::from(matches!(category.as_str(), "C1" | "C10" | "C11")),
        ),
        ("required_elements", JsonValue::array(required_elements)),
        ("safety_tier", JsonValue::from(u64::from(safety_tier))),
    ])
}

fn string_array(value: &str) -> JsonValue {
    JsonValue::array(split_nonempty(value).into_iter().map(JsonValue::from))
}
