use std::collections::BTreeMap;

use super::super::{InterviewRequest, answer};
use super::inference::{
    contains_any, dimensions, parse_patterns, parse_subjects, repeated_panel_count,
    repeated_panel_identity, selected_safety_tier, split_nonempty, subject_relationship,
    surface_fields,
};
use crate::json::JsonValue;

#[path = "builder/camera_lighting.rs"]
mod camera_lighting;
#[path = "builder/color.rs"]
mod color;
#[path = "builder/constraints.rs"]
mod constraints;
#[path = "builder/geometry.rs"]
mod geometry;
#[path = "builder/subjects.rs"]
mod subjects;
#[path = "builder/text.rs"]
mod text;

use self::camera_lighting::{
    camera_json, flat_graphic_camera_json, flat_illustration_lighting_json, lighting_json,
};
use self::color::{color_json, palette_projection};
use self::constraints::{ConstraintContext, build_constraints};
use self::geometry::{composition_balance, composition_geometry};
use self::subjects::{SubjectRenderContext, build_subjects};
use self::text::{build_text_element, use_case_without_exact_copy};

pub(super) fn build_request_json(
    request: &InterviewRequest,
    answers: &BTreeMap<String, String>,
) -> JsonValue {
    let category = answer(answers, "image.category")
        .unwrap_or("C5")
        .to_ascii_uppercase();
    let language = answer(answers, "language").unwrap_or("ko");
    let raw_profile = answer(answers, "image.profile").unwrap_or("standard");
    let is_app_icon = raw_profile == "app_icon";
    let medium = answer(answers, "image.medium").unwrap_or(if is_app_icon {
        "graphic_design"
    } else {
        "photo"
    });
    let text_mode = answer(answers, "image.text_mode").unwrap_or("none");
    let text_lines = if text_mode == "exact" {
        split_nonempty(answer(answers, "image.text").unwrap_or_default())
    } else {
        Vec::new()
    };
    let patterns = parse_patterns(answer(answers, "image.pattern").unwrap_or("none"));
    let has_text = !text_lines.is_empty();
    let use_case = use_case_without_exact_copy(request.brief.trim(), &text_lines);
    let safety_tier = selected_safety_tier(&category, &patterns, has_text);
    let subject_description = answer(answers, "image.subject")
        .unwrap_or_default()
        .to_owned();
    let panel_plan = (category == "C10")
        .then(|| answer(answers, "image.panel_plan"))
        .flatten();
    let repeated_count = panel_plan.and_then(repeated_panel_count);
    let parsed_subjects = if let Some(count) = repeated_count {
        vec![repeated_panel_identity(&subject_description, count)]
    } else {
        parse_subjects(&subject_description)
    };
    let repeated_action = repeated_count.zip(panel_plan).map(|(count, plan)| {
        if language == "en" {
            format!("Ordered panel/sticker action map, instances 1 through {count}: {plan}")
        } else {
            format!("1번부터 {count}번까지 읽기 순서를 고정한 패널·스티커 행동 맵: {plan}")
        }
    });
    let relationship = subject_relationship(&subject_description);
    let constraints = build_constraints(ConstraintContext {
        category: &category,
        language,
        answers,
        brief: &request.brief,
        subject_description: &subject_description,
        repeated_count,
        is_app_icon,
        safety_tier,
    });
    let mut hierarchy = parsed_subjects
        .iter()
        .map(|subject| JsonValue::from(subject.id.clone()))
        .collect::<Vec<_>>();
    let text_elements = if !has_text {
        JsonValue::array([])
    } else {
        hierarchy.push(JsonValue::from("headline"));
        JsonValue::array([build_text_element(answers, text_lines)])
    };
    let (width, height) = dimensions(answer(answers, "image.aspect_ratio").unwrap_or("1:1"));
    let palette = palette_projection(answers);
    let lighting = if repeated_count.is_some() {
        flat_illustration_lighting_json(language)
    } else {
        lighting_json(
            answer(answers, "image.lighting").unwrap_or("soft_daylight"),
            answer(answers, "image.lighting_detail"),
        )
    };
    let color = color_json(answers, medium, palette);
    let editorial_signal = contains_any(
        &request.brief.to_lowercase(),
        &["화보", "에디토리얼", "룩북", "editorial", "lookbook"],
    );
    let render_profile = if is_app_icon {
        "structured"
    } else if category == "C1" && !has_text && editorial_signal {
        "editorial_flat"
    } else {
        "structured"
    };
    let geometry = composition_geometry(language, is_app_icon, repeated_count, has_text, answers);
    let subject_json = build_subjects(
        &parsed_subjects,
        SubjectRenderContext {
            category: &category,
            language,
            repeated_count,
            repeated_action: repeated_action.as_deref(),
            relationship: relationship.as_deref(),
            answers,
            is_app_icon,
        },
    );
    let surface_answer = answer(answers, "image.surface").unwrap_or_default();
    let (surface_material, surface_finish, surface_micro_detail, surface_light_response) =
        if repeated_count.is_some() {
            (
                image_copy(
                    language,
                    "2D 디지털 일러스트의 잉크·브러시 선과 평면 색",
                    "Ink or brush lines and flat color planes in a 2D digital illustration",
                )
                .to_owned(),
                surface_answer.to_owned(),
                image_copy(
                    language,
                    "일정한 외곽선 굵기, 선명한 면색 경계, 동일한 다이컷 테두리",
                    "Consistent outline weight, crisp color boundaries, and identical die-cut borders",
                )
                .to_owned(),
                image_copy(
                    language,
                    "사진식 반사·림라이트 없이 색면과 외곽선 대비로 형태를 분리",
                    "Separate forms through color planes and outlines, without photographic reflections or rim light",
                )
                .to_owned(),
            )
        } else {
            let (material, finish) = surface_fields(surface_answer, &category);
            (
                material,
                finish,
                "확대 시 재질을 식별할 수 있는 미세 요철과 가장자리 마감".to_owned(),
                "표면 곡률과 마감에 맞는 연속 하이라이트와 접점 그림자".to_owned(),
            )
        };
    let composition = answer(answers, "image.composition").unwrap_or_default();
    JsonValue::object([
        (
            "camera",
            if repeated_count.is_some() {
                flat_graphic_camera_json(language)
            } else {
                camera_json(medium)
            },
        ),
        ("color", color),
        (
            "composition",
            JsonValue::object([
                (
                    "balance",
                    JsonValue::from(if is_app_icon {
                        image_copy(
                            language,
                            "하나의 지배적 중심 마크와 균일한 사방 여백",
                            "One dominant centered mark with even breathing room on all sides",
                        )
                    } else if repeated_count.is_some() {
                        image_copy(
                            language,
                            "모든 셀의 크기·간격·캐릭터 비중을 동일하게 유지",
                            "Keep cell size, spacing, and character scale uniform across the sheet",
                        )
                    } else {
                        composition_balance(composition)
                    }),
                ),
                ("camera_angle", JsonValue::from(geometry.camera_angle)),
                ("depth_layers", JsonValue::array(geometry.depth_layers)),
                ("framing", JsonValue::from(composition)),
                ("negative_space", JsonValue::array(geometry.negative_space)),
                ("viewpoint", JsonValue::from(geometry.viewpoint)),
                ("visual_hierarchy", JsonValue::array(hierarchy)),
            ]),
        ),
        ("constraints", constraints),
        (
            "language",
            JsonValue::from(answer(answers, "language").unwrap_or("ko")),
        ),
        ("profile", JsonValue::from(raw_profile)),
        ("lighting", lighting),
        ("medium", JsonValue::from(medium)),
        (
            "output",
            JsonValue::object([
                ("background", JsonValue::from("opaque")),
                ("format", JsonValue::from("png")),
                ("height", JsonValue::from(u64::from(height))),
                ("model", JsonValue::from("gpt-image-2")),
                (
                    "quality",
                    JsonValue::from(answer(answers, "image.quality").unwrap_or("medium")),
                ),
                ("width", JsonValue::from(u64::from(width))),
            ]),
        ),
        ("render_profile", JsonValue::from(render_profile)),
        (
            "scene",
            JsonValue::object([
                (
                    "atmosphere",
                    JsonValue::from(if repeated_count.is_some() {
                        image_copy(
                            language,
                            "모든 셀에서 동일한 얼굴·복장·선 굵기·팔레트 역할을 유지",
                            "Keep face, costume, line weight, and palette roles identical in every cell",
                        )
                    } else {
                        "장면 요소는 지정된 공간 관계와 시각 밀도를 유지"
                    }),
                ),
                (
                    "background",
                    JsonValue::from(answer(answers, "image.scene").unwrap_or_default()),
                ),
                (
                    "environment",
                    JsonValue::from(if repeated_count.is_some() {
                        image_copy(
                            language,
                            "정면 직교 투영의 균일한 패널·스티커 일러스트 시트",
                            "Uniform front-facing orthographic panel or sticker illustration sheet",
                        )
                    } else {
                        answer(answers, "image.scene").unwrap_or_default()
                    }),
                ),
            ]),
        ),
        ("subjects", JsonValue::array(subject_json)),
        (
            "surfaces",
            JsonValue::array([JsonValue::object([
                ("finish", JsonValue::from(surface_finish)),
                ("id", JsonValue::from("primary_surface")),
                ("light_response", JsonValue::from(surface_light_response)),
                ("material", JsonValue::from(surface_material)),
                ("micro_detail", JsonValue::from(surface_micro_detail)),
            ])]),
        ),
        (
            "taxonomy",
            JsonValue::object([
                ("category", JsonValue::from(category)),
                (
                    "patterns",
                    JsonValue::array(patterns.into_iter().map(JsonValue::from)),
                ),
            ]),
        ),
        ("text_elements", text_elements),
        ("use_case", JsonValue::from(use_case)),
    ])
}

fn image_copy<'a>(language: &str, korean: &'a str, english: &'a str) -> &'a str {
    if language == "en" { english } else { korean }
}
