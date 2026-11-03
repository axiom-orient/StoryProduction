//! Language and result-category inference from the brief.

use std::collections::BTreeMap;

use crate::interview::answer;
use crate::interview::image::inference::{infer_category, is_hangul};

pub(in crate::interview::image) fn infer_route_from_brief(
    brief: &str,
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    if answer(answers, "language").is_none() {
        let language = if brief.chars().any(is_hangul) {
            "ko"
        } else {
            "en"
        };
        answers.insert("language".to_owned(), language.to_owned());
        inferences.push(format!(
            "입력 문자 체계를 근거로 출력 언어를 {language}로 추정했다."
        ));
    }
    if answer(answers, "image.category").is_none()
        && let Some(category) = infer_category(brief)
    {
        answers.insert("image.category".to_owned(), category.to_owned());
        inferences.push(format!(
            "요청 키워드를 근거로 결과물 {category}를 추정했다."
        ));
    }
}
