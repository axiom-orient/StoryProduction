//! Typed subject JSON projection for the image interview request.

use std::collections::BTreeMap;

use super::super::super::answer;
use super::super::inference::{ParsedSubject, contains_any, repeated_identity_anchors};
use super::geometry::{repeated_panel_placement, subject_placement};
use super::image_copy;
use crate::json::JsonValue;

pub(super) struct SubjectRenderContext<'a> {
    pub(super) category: &'a str,
    pub(super) language: &'a str,
    pub(super) repeated_count: Option<u16>,
    pub(super) repeated_action: Option<&'a str>,
    pub(super) relationship: Option<&'a str>,
    pub(super) answers: &'a BTreeMap<String, String>,
    pub(super) is_app_icon: bool,
}

pub(super) fn build_subjects(
    subjects: &[ParsedSubject],
    context: SubjectRenderContext<'_>,
) -> Vec<JsonValue> {
    let SubjectRenderContext {
        category,
        language,
        repeated_count,
        repeated_action,
        relationship,
        answers,
        is_app_icon,
    } = context;
    let subject_total = subjects.len();
    subjects
        .iter()
        .enumerate()
        .map(|(index, subject)| {
            let is_person = contains_any(
                &subject.description.to_lowercase(),
                &["인물", "사람", "모델", "person", "model"],
            );
            let repeated = subject.count > 1;
            let mut distinguishing_features = if repeated_count.is_some() {
                repeated_identity_anchors(&subject.description)
                    .into_iter()
                    .map(JsonValue::from)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if repeated {
                distinguishing_features.push(JsonValue::from(image_copy(
                    language,
                    "같은 subject id의 모든 instance에서 얼굴 기하·복장·소품·선 굵기·팔레트 역할을 일관되게 유지",
                    "Keep face geometry, costume, props, line weight, and palette roles identical across every instance of this subject id",
                )));
            }
            let appearance = if category == "C1" {
                Some(
                    answer(answers, "image.wardrobe")
                        .or_else(|| answer(answers, "image.surface"))
                        .unwrap_or_default()
                        .to_owned(),
                )
            } else {
                subject.appearance.clone()
            };
            JsonValue::object([
                (
                    "action",
                    JsonValue::from(
                        repeated_action
                            .or(relationship)
                            .unwrap_or("정지 상태"),
                    ),
                ),
                ("count", JsonValue::from(u64::from(subject.count))),
                ("description", JsonValue::from(subject.description.clone())),
                (
                    "distinguishing_features",
                    JsonValue::array(distinguishing_features),
                ),
                (
                    "face",
                    if index == 0 {
                        answer(answers, "image.face").map_or(JsonValue::Null, JsonValue::from)
                    } else {
                        JsonValue::Null
                    },
                ),
                (
                    "gaze",
                    JsonValue::from(if repeated_count.is_some() {
                        image_copy(
                            language,
                            "각 셀의 감정과 행동에 맞는 시선만 바꾸고 얼굴 기하는 유지",
                            "Vary only gaze for each cell's emotion and action; preserve face geometry",
                        )
                    } else if is_person {
                        "요청에 명시된 시선 방향을 유지"
                    } else {
                        "시선 해당 없음; 식별 가능한 대표 면을 카메라에 노출"
                    }),
                ),
                (
                    "hair",
                    if index == 0 {
                        answer(answers, "image.hair").map_or(JsonValue::Null, JsonValue::from)
                    } else {
                        JsonValue::Null
                    },
                ),
                ("id", JsonValue::from(subject.id.clone())),
                (
                    "placement",
                    if is_app_icon {
                        JsonValue::object([("zone", JsonValue::from("center"))])
                    } else if repeated_count.is_some() {
                        repeated_panel_placement()
                    } else {
                        subject_placement(subject_total)
                    },
                ),
                (
                    "pose",
                    JsonValue::from(if repeated_count.is_some() {
                        image_copy(
                            language,
                            "각 셀에 전신 instance 한 개만 배치하고 셀 경계와 서로 겹치지 않음",
                            "Place exactly one full-body instance in each cell without overlap across subjects or cell boundaries",
                        )
                    } else if is_app_icon {
                        image_copy(
                            language,
                            "32px 축소에서도 하나의 중심 실루엣이 즉시 판독됨",
                            "One core silhouette remains immediately readable when reduced to 32px",
                        )
                    } else if repeated {
                        "각 instance의 실루엣이 서로 겹치지 않고 개별 식별됨"
                    } else {
                        "주요 실루엣과 기능 면이 가려지지 않음"
                    }),
                ),
                (
                    "scale",
                    JsonValue::from(if repeated_count.is_some() {
                        image_copy(
                            language,
                            "모든 셀의 instance가 동일한 시각 비중과 기준 크기를 유지",
                            "Keep every cell instance at the same visual weight and reference size",
                        )
                    } else if repeated {
                        "모든 instance가 동일한 시각 비중으로 개별 식별됨"
                    } else if is_app_icon {
                        image_copy(
                            language,
                            "정사각형 캔버스의 약 80%를 채우는 중앙 마크",
                            "Centered mark filling roughly 80% of the square canvas",
                        )
                    } else if subject_total > 1 {
                        "다른 typed subject와 함께 프레임 안에서 완전히 식별됨"
                    } else {
                        "주 피사체의 주요 실루엣은 캔버스 높이의 58%를 점유"
                    }),
                ),
                (
                    "appearance",
                    appearance.map_or(JsonValue::Null, JsonValue::from),
                ),
            ])
        })
        .collect::<Vec<_>>()
}
