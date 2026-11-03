//! Extract directly stated visual fields without category fallbacks.

use std::collections::BTreeMap;

use crate::interview::answer;
use crate::interview::image::inference::{
    contains_any, extract_composition_clause, extract_hex_palette, extract_lighting_clause,
    extract_quoted_texts, extract_scene_clause, extract_subject_clause, extract_surface_clause,
    extract_wardrobe_clause, find_ratio, infer_lighting, infer_medium, infer_text_position,
    infer_text_style, wardrobe_is_specific,
};

/// Content stated in the brief. This runs only after the route is accepted, so a
/// re-asked category never leaves subject, scene, or medium behind.
pub(in crate::interview::image) fn infer_stated_content_from_brief(
    brief: &str,
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    if answer(answers, "image.medium").is_none()
        && let Some(medium) = infer_medium(brief)
    {
        answers.insert("image.medium".to_owned(), medium.to_owned());
        inferences.push(format!("요청 키워드를 근거로 매체 {medium}을 추정했다."));
    }
    if answer(answers, "image.aspect_ratio").is_none()
        && let Some(ratio) = find_ratio(brief)
    {
        answers.insert("image.aspect_ratio".to_owned(), ratio.clone());
        inferences.push(format!("요청에 명시된 화면 비율 {ratio}를 사용한다."));
    }
    if answer(answers, "image.palette").is_none()
        && let Some(colors) = extract_hex_palette(brief)
    {
        let value = colors.join(", ");
        answers.insert("image.palette".to_owned(), value.clone());
        inferences.push(format!("요청에 명시된 HEX 팔레트 {value}를 사용한다."));
    }
    if answer(answers, "image.text_mode").is_none() {
        if contains_any(
            &brief.to_lowercase(),
            &[
                "텍스트 없음",
                "텍스트 없이",
                "문구 없음",
                "문구 없이",
                "문구와 로고 없이",
                "글자 없음",
                "글자 없이",
                "텍스트를 넣지",
                "문구를 넣지",
                "글자를 넣지",
                "no text",
                "no readable text",
                "without text",
                "exclude text",
            ],
        ) {
            answers.insert("image.text_mode".to_owned(), "none".to_owned());
            inferences.push("요청의 명시적 무문구 조건을 확인했다.".to_owned());
        } else {
            let quoted = extract_quoted_texts(brief);
            if !quoted.is_empty() {
                answers.insert("image.text_mode".to_owned(), "exact".to_owned());
                answers
                    .entry("image.text".to_owned())
                    .or_insert_with(|| quoted.join("\n"));
                inferences.push("따옴표 안 문구를 이미지 내 정확 문구로 추정했다.".to_owned());
            }
        }
    }
    if answer(answers, "image.subject").is_none()
        && let Some(subject) = extract_subject_clause(brief)
    {
        answers.insert("image.subject".to_owned(), subject.clone());
        inferences.push(format!(
            "수량·피사체 표지가 있는 절을 주 피사체로 추정했다: {subject}"
        ));
    }
    if answer(answers, "image.scene").is_none()
        && let Some(scene) = extract_scene_clause(brief)
    {
        answers.insert("image.scene".to_owned(), scene.clone());
        inferences.push(format!(
            "장소·시간·대기 표지가 있는 절을 장면으로 추정했다: {scene}"
        ));
    }
    if answer(answers, "image.composition").is_none()
        && let Some(composition) = extract_composition_clause(brief)
    {
        answers.insert("image.composition".to_owned(), composition.clone());
        inferences.push(format!("요청의 구도 표지를 사용한다: {composition}"));
    }
    if answer(answers, "image.surface").is_none()
        && let Some(surface) = extract_surface_clause(brief)
    {
        answers.insert("image.surface".to_owned(), surface.clone());
        inferences.push(format!("요청의 재질·마감 표지를 사용한다: {surface}"));
    }
    if answer(answers, "image.category").is_some_and(|category| category.eq_ignore_ascii_case("C1"))
        && answer(answers, "image.wardrobe").is_none()
        && let Some(wardrobe) = extract_wardrobe_clause(brief)
        && wardrobe_is_specific(&wardrobe)
    {
        answers.insert("image.wardrobe".to_owned(), wardrobe.clone());
        inferences.push(format!(
            "요청의 색·소재·의류종·핏 정보를 화보 의상 슬롯으로 사용한다: {wardrobe}"
        ));
    }
    if answer(answers, "image.lighting").is_none()
        && let Some(lighting) = infer_lighting(brief)
    {
        answers.insert("image.lighting".to_owned(), lighting.to_owned());
        inferences.push(format!(
            "요청의 광원·시간대 표현을 {lighting} 조명 프로필로 해석했다."
        ));
    }
    if answer(answers, "image.lighting_detail").is_none()
        && let Some(detail) = extract_lighting_clause(brief)
    {
        answers.insert("image.lighting_detail".to_owned(), detail.clone());
        inferences.push(format!(
            "요청에 명시된 광원 방향과 광질을 보존한다: {detail}"
        ));
    }
    if answer(answers, "image.text_mode") == Some("exact")
        && answer(answers, "image.text_position").is_none()
        && let Some(position) = infer_text_position(brief)
    {
        answers.insert("image.text_position".to_owned(), position.to_owned());
        inferences.push(format!(
            "요청의 위치 표현을 {position} 텍스트 영역으로 해석했다."
        ));
    }
    if answer(answers, "image.text_mode") == Some("exact")
        && answer(answers, "image.text_style").is_none()
        && let Some(style) = infer_text_style(brief)
    {
        answers.insert("image.text_style".to_owned(), style.to_owned());
        inferences.push(format!("요청의 타이포 표현을 {style} 계열로 해석했다."));
    }
}
