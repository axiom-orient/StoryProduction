use std::collections::BTreeMap;

use super::{answer, image_copy};
use crate::json::JsonValue;

pub(super) struct CompositionGeometry {
    pub(super) viewpoint: &'static str,
    pub(super) camera_angle: &'static str,
    pub(super) depth_layers: Vec<JsonValue>,
    pub(super) negative_space: Vec<JsonValue>,
}

pub(super) fn composition_geometry(
    language: &str,
    is_app_icon: bool,
    repeated_count: Option<u16>,
    has_text: bool,
    answers: &BTreeMap<String, String>,
) -> CompositionGeometry {
    let viewpoint = if is_app_icon {
        image_copy(
            language,
            "캔버스 면을 정면으로 보는 직교 투영",
            "Front-facing orthographic projection of the square canvas",
        )
    } else if repeated_count.is_some() {
        image_copy(
            language,
            "캔버스 면을 정면으로 보는 직교 투영",
            "Front-facing orthographic projection of the canvas plane",
        )
    } else {
        "주 피사체의 전면과 측면 특징이 동시에 식별되는 관찰 시점"
    };
    let camera_angle = if is_app_icon {
        image_copy(
            language,
            "캔버스와 평행한 정면, 기울기·원근 왜곡 없음",
            "Straight-on and parallel to the canvas, with no tilt or perspective distortion",
        )
    } else if repeated_count.is_some() {
        image_copy(
            language,
            "캔버스 면과 평행한 정면, 기울기와 원근 왜곡 없음",
            "Straight-on and parallel to the canvas, with no tilt or perspective distortion",
        )
    } else {
        "주 피사체의 형태를 판독할 수 있는 수평 기준 10도 이내의 3/4 각도"
    };
    let depth_layers = if is_app_icon {
        vec![
            JsonValue::from(image_copy(
                language,
                "불투명 배경 평면",
                "Opaque background plane",
            )),
            JsonValue::from(image_copy(
                language,
                "핵심 은유의 단일 색면·기하",
                "Core metaphor color plane and geometry",
            )),
            JsonValue::from(image_copy(
                language,
                "절제된 깊이를 위한 선택적 보조 평면",
                "Optional restrained secondary plane for depth",
            )),
        ]
    } else if repeated_count.is_some() {
        vec![JsonValue::from(image_copy(
            language,
            "단일 평면 시트 안에서 배경, 다이컷 테두리, 캐릭터 색면만 분리",
            "Separate only the background, die-cut border, and character color planes on one flat sheet",
        ))]
    } else {
        vec![
            JsonValue::from("전경은 주 피사체의 접지면과 가장 가까운 재질"),
            JsonValue::from("중경은 주 피사체와 핵심 행동"),
            JsonValue::from("후경은 지정 장면과 분리된 배경 면"),
        ]
    };
    let negative_space = if is_app_icon || repeated_count.is_some() {
        Vec::new()
    } else {
        vec![JsonValue::from(if has_text {
            answer(answers, "image.text_position").unwrap_or("top_left")
        } else {
            "top_center"
        })]
    };
    CompositionGeometry {
        viewpoint,
        camera_angle,
        depth_layers,
        negative_space,
    }
}

pub(super) fn subject_placement(subject_total: usize) -> JsonValue {
    let (x, y, width, height) = if subject_total > 1 {
        (10_u64, 20_u64, 80_u64, 70_u64)
    } else {
        (19_u64, 24_u64, 62_u64, 62_u64)
    };
    JsonValue::object([
        ("height_percent", JsonValue::from(height)),
        ("width_percent", JsonValue::from(width)),
        ("x_percent", JsonValue::from(x)),
        ("y_percent", JsonValue::from(y)),
        ("zone", JsonValue::from("custom")),
    ])
}

pub(super) fn repeated_panel_placement() -> JsonValue {
    JsonValue::object([
        ("height_percent", JsonValue::from(90_u64)),
        ("width_percent", JsonValue::from(90_u64)),
        ("x_percent", JsonValue::from(5_u64)),
        ("y_percent", JsonValue::from(5_u64)),
        ("zone", JsonValue::from("custom")),
    ])
}

pub(super) fn composition_balance(_framing: &str) -> &'static str {
    "프레이밍에 명시된 피사체 위치와 여백 관계를 바꾸지 않고 시각 무게를 안정적으로 유지"
}
