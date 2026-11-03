use std::collections::BTreeMap;

use super::super::inference::is_hangul;
use super::answer;
use crate::json::JsonValue;

pub(super) fn use_case_without_exact_copy(brief: &str, lines: &[String]) -> String {
    let mut value = brief.to_owned();
    let mut removed = false;
    for line in lines {
        for (open, close) in [('"', '"'), ('“', '”'), ('「', '」'), ('『', '』')] {
            let quoted = format!("{open}{line}{close}");
            if value.contains(&quoted) {
                value = value.replace(&quoted, "");
                removed = true;
            }
        }
    }
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if removed {
        format!("{value}; exact copy is defined only in text_elements.")
    } else {
        value
    }
}

pub(super) fn build_text_element(
    answers: &BTreeMap<String, String>,
    lines: Vec<String>,
) -> JsonValue {
    let position = answer(answers, "image.text_position").unwrap_or("top_left");
    let style = answer(answers, "image.text_style").unwrap_or("geometric");
    let (font, weight, treatment) = match style {
        "condensed" => (
            "condensed sans-serif with tall narrow letterforms",
            "black",
            "자형 골격을 유지한 수직 압축",
        ),
        "didone" => (
            "high-contrast didone serif",
            "bold",
            "얇은 세리프와 굵은 세로획의 대비",
        ),
        "mono" => (
            "monospace with typewriter character",
            "medium",
            "균일 폭과 직선 기준선",
        ),
        "brush" => (
            "controlled Korean brush calligraphy",
            "bold",
            "붓 가장자리는 유지하되 자모 골격은 또렷하게",
        ),
        _ => (
            "clean geometric sans-serif",
            "bold",
            "기하학적 획과 균일한 기준선",
        ),
    };
    JsonValue::object([
        (
            "alignment",
            JsonValue::from(if position.ends_with("right") {
                "right aligned"
            } else if position.contains("center") {
                "center aligned"
            } else {
                "left aligned"
            }),
        ),
        ("color_hex", JsonValue::from("#F5F5F2")),
        ("font_family", JsonValue::from(font)),
        ("id", JsonValue::from("headline")),
        (
            "language",
            JsonValue::from(if lines.iter().any(|line| line.chars().any(is_hangul)) {
                "ko"
            } else {
                "en"
            }),
        ),
        (
            "letter_spacing",
            JsonValue::from("글자 높이의 1% 이내로 통제된 자간"),
        ),
        (
            "lines",
            JsonValue::array(lines.into_iter().map(JsonValue::from)),
        ),
        (
            "placement",
            JsonValue::object([("zone", JsonValue::from(position))]),
        ),
        ("role", JsonValue::from("headline")),
        ("size_percent", JsonValue::from(12_u64)),
        ("treatment", JsonValue::from(treatment)),
        ("weight", JsonValue::from(weight)),
    ])
}
